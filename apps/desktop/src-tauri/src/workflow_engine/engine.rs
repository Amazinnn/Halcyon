//! Workflow executor (M4, ADR-0012; v2, ADR-0017). Runs a node+edge graph with
//! a run-local data context ({{nodeId.field}} and {{system.*}} references),
//! fail-fast semantics, multi-way branch routing, agent fill-slots and loop
//! (back-edge) support. Depends only on injected traits so it is unit-testable
//! without Tauri.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::model::{
    validate_workflow, CharacterInfo, NodeDef, NodeLogEntry, RunStatus, WorkflowDef,
};

/// One-shot agent invocation. wait=true blocks until the agent turn finishes
/// and returns (result_text, thread_id); wait=false returns None right
/// after dispatch (no output for downstream nodes).
/// Workflow calls always suppress provider-facing chat events. The engine
/// separately owns the single final result event and bubble.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentDisplay {
    pub show_initial: bool,
    pub show_thinking: bool,
    pub show_result: bool,
}

impl AgentDisplay {
    /// Direct-chat adapters emit only Provider-explicit public text through
    /// this boundary. The legacy fields stay for serialized workflow plans,
    /// but are never interpreted as first text versus later "thinking".
    pub const fn shows_public_text_deltas(&self) -> bool {
        self.show_initial && self.show_thinking
    }
}

impl Default for AgentDisplay {
    fn default() -> Self {
        Self {
            show_initial: true,
            show_thinking: false,
            show_result: true,
        }
    }
}

pub trait AgentCall: Send + Sync {
    fn call_one_shot(
        &self,
        character: &CharacterInfo,
        prompt: &str,
        wait: bool,
        cancel: &AtomicBool,
        display: AgentDisplay,
    ) -> Result<Option<(String, String)>, String>;

    /// v1.11 (ADR-0020): resolve a per-node target character by id. Returns
    /// None when the id is empty or unknown (callers keep the workflow
    /// character in that case).
    fn resolve_character(&self, id: &str) -> Option<CharacterInfo>;
}

pub trait EventSink: Send + Sync {
    fn bubble(&self, text: &str, priority: &str, agent_id: Option<&str>);
    fn agent_result(&self, workflow_id: &str, workflow_name: &str, agent_id: &str, text: &str);
}

pub trait WindowOps: Send + Sync {
    fn show_window(&self, label: &str) -> Result<(), String>;
}

/// Deterministic system actions + state exposed to workflow nodes (v2, ADR-0017).
pub trait SystemActions: Send + Sync {
    fn focus(&self, seconds: i64) -> Result<(), String>;
    fn idle(&self, seconds: i64) -> Result<(), String>;
    fn ring(&self, seconds: i64) -> Result<(), String>;
    /// Current focus state: idle | focus | rest.
    fn focus_state(&self) -> String;
    /// Local wall-clock "HH:MM".
    fn now_hhmm(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub status: RunStatus,
    pub error: Option<String>,
    pub node_log: Vec<NodeLogEntry>,
}

pub fn execute_run(
    wf: &WorkflowDef,
    character: &CharacterInfo,
    agent: &dyn AgentCall,
    events: &dyn EventSink,
    windows: &dyn WindowOps,
    system: &dyn SystemActions,
    cancel: &AtomicBool,
) -> RunOutcome {
    if let Err(e) = validate_workflow(wf) {
        return RunOutcome {
            status: RunStatus::Failed,
            error: Some(e),
            node_log: vec![],
        };
    }

    let nodes: HashMap<&str, &NodeDef> = wf.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Classify back edges (edges that close a cycle, incl. self-loops) with a
    // DFS. Back edges never block entry and re-enter their target when taken.
    let edges: Vec<&crate::workflow_engine::model::EdgeDef> = wf.edges.iter().collect();
    let mut back: HashSet<usize> = HashSet::new();
    {
        fn dfs<'a>(
            id: &'a str,
            edges: &[&'a crate::workflow_engine::model::EdgeDef],
            state: &mut HashMap<&'a str, u8>,
            back: &mut HashSet<usize>,
        ) {
            state.insert(id, 1);
            for (ei, e) in edges.iter().enumerate() {
                if e.source != id {
                    continue;
                }
                match state.get(e.target.as_str()).copied().unwrap_or(0) {
                    1 => {
                        back.insert(ei);
                    }
                    2 => {}
                    _ => {
                        dfs(&e.target, edges, state, back);
                    }
                }
            }
            state.insert(id, 2);
        }
        let mut state: HashMap<&str, u8> = HashMap::new();
        for n in &wf.nodes {
            if state.get(n.id.as_str()).copied().unwrap_or(0) == 0 {
                dfs(n.id.as_str(), &edges, &mut state, &mut back);
            }
        }
    }

    let mut indegree: HashMap<&str, usize> = nodes.keys().map(|k| (*k, 0usize)).collect();
    let mut out_edges: HashMap<&str, Vec<(&crate::workflow_engine::model::EdgeDef, bool)>> =
        HashMap::new();
    for (ei, e) in edges.iter().enumerate() {
        let is_back = back.contains(&ei);
        if !is_back {
            *indegree.get_mut(e.target.as_str()).unwrap() += 1;
        }
        out_edges
            .entry(e.source.as_str())
            .or_default()
            .push((e, is_back));
    }

    let mut remaining: HashMap<&str, usize> = indegree.clone();
    let mut queue: VecDeque<&str> = wf
        .nodes
        .iter()
        .filter(|n| indegree.get(n.id.as_str()) == Some(&0))
        .map(|n| n.id.as_str())
        .collect();

    // v1.10.5 (ADR-0018): first non-back incoming edge source per node. Branch
    // nodes read the upstream Agent's fill-slot through this map (no UI vars).
    let mut incoming: HashMap<&str, &str> = HashMap::new();
    for (e, is_back) in out_edges.iter().flat_map(|(_, v)| v.iter()) {
        if !*is_back {
            incoming
                .entry(e.target.as_str())
                .or_insert(e.source.as_str());
        }
    }

    let mut data: HashMap<String, Value> = HashMap::new();
    let mut log: Vec<NodeLogEntry> = Vec::new();
    let mut done: HashSet<&str> = HashSet::new();
    let mut status = RunStatus::Success;
    let mut error: Option<String> = None;

    while let Some(id) = queue.pop_front() {
        if done.contains(id) {
            continue;
        }
        if cancel.load(Ordering::Relaxed) {
            status = RunStatus::Cancelled;
            error = Some("已取消".into());
            break;
        }
        let node = nodes[id];
        match run_node(
            node, &data, &incoming, &wf.id, &wf.name, character, agent, events, windows, system,
            cancel,
        ) {
            Ok((output, taken_handle)) => {
                data.insert(id.to_string(), output.clone());
                log.push(NodeLogEntry {
                    node_id: id.to_string(),
                    kind: node.kind.clone(),
                    status: "ok".into(),
                    output: Some(output),
                    error: None,
                });
                done.insert(id);
                if let Some(edges) = out_edges.get(id) {
                    for (e, is_back) in edges {
                        if e.source_handle != taken_handle {
                            continue;
                        }
                        if *is_back {
                            // Loop: reset the whole forward subgraph from the
                            // target so the loop re-propagates cleanly.
                            reset_forward(
                                e.target.as_str(),
                                &out_edges,
                                &indegree,
                                &mut remaining,
                                &mut done,
                                &mut queue,
                            );
                        } else {
                            let rem = remaining.get_mut(e.target.as_str()).unwrap();
                            *rem -= 1;
                            if *rem == 0 {
                                queue.push_back(e.target.as_str());
                            }
                        }
                    }
                }
            }
            Err(err) => {
                let cancelled = cancel.load(Ordering::Relaxed);
                status = if cancelled {
                    RunStatus::Cancelled
                } else {
                    RunStatus::Failed
                };
                error = Some(if cancelled {
                    "已取消".into()
                } else {
                    err.clone()
                });
                log.push(NodeLogEntry {
                    node_id: id.to_string(),
                    kind: node.kind.clone(),
                    status: if cancelled { "cancelled" } else { "failed" }.into(),
                    output: None,
                    error: Some(err),
                });
                break;
            }
        }
    }

    // Nodes that never ran (downstream of an untaken branch / cancelled) are
    // marked skipped.
    for n in &wf.nodes {
        if !done.contains(n.id.as_str()) {
            log.push(NodeLogEntry {
                node_id: n.id.clone(),
                kind: n.kind.clone(),
                status: "skipped".into(),
                output: None,
                error: None,
            });
        }
    }

    RunOutcome {
        status,
        error,
        node_log: log,
    }
}

/// v2 loop support: un-done the target and every forward-descendant, reset
/// their remaining counters to the forward indegree and re-enqueue the target.
fn reset_forward<'a>(
    id: &'a str,
    out_edges: &HashMap<&'a str, Vec<(&'a crate::workflow_engine::model::EdgeDef, bool)>>,
    indegree: &HashMap<&'a str, usize>,
    remaining: &mut HashMap<&'a str, usize>,
    done: &mut HashSet<&'a str>,
    queue: &mut VecDeque<&'a str>,
) {
    let mut stack = vec![id];
    let mut seen: HashSet<&str> = HashSet::new();
    while let Some(nid) = stack.pop() {
        if !seen.insert(nid) {
            continue;
        }
        remaining.insert(nid, indegree[nid]);
        done.remove(nid);
        if let Some(edges) = out_edges.get(nid) {
            for (e, is_back) in edges {
                if !*is_back {
                    stack.push(e.target.as_str());
                }
            }
        }
    }
    queue.push_back(id);
}

fn run_node(
    node: &NodeDef,
    data: &HashMap<String, Value>,
    incoming: &HashMap<&str, &str>,
    workflow_id: &str,
    workflow_name: &str,
    character: &CharacterInfo,
    agent: &dyn AgentCall,
    events: &dyn EventSink,
    windows: &dyn WindowOps,
    system: &dyn SystemActions,
    cancel: &AtomicBool,
) -> Result<(Value, String), String> {
    match node.kind.as_str() {
        "agent" => {
            let prompt = param_str(node, "prompt")?;
            let prompt = resolve_with_system(&prompt, data, system)?;
            let wait = param_bool(node, "wait").unwrap_or(true);
            // v1.11 (ADR-0020): per-node target character overrides the
            // workflow character; empty/unknown falls back to the workflow one.
            let target = param_str(node, "characterId")
                .ok()
                .filter(|s| !s.is_empty());
            let caller = if let Some(id) = target {
                agent
                    .resolve_character(&id)
                    .unwrap_or_else(|| character.clone())
            } else {
                character.clone()
            };
            let show_result = param_bool(node, "showResult").unwrap_or(true);
            // Raw workflow turns are never rendered as ordinary provider chat.
            // Saved showInitial/showThinking fields remain tolerated but inert.
            let display = AgentDisplay {
                show_initial: false,
                show_thinking: false,
                show_result: false,
            };
            match agent.call_one_shot(&caller, &prompt, wait, cancel, display) {
                Ok(Some((result, thread_id))) => {
                    // Workflow results have one engine-owned chat event and
                    // one bubble, both gated by the saved showResult flag.
                    if show_result {
                        events.agent_result(workflow_id, workflow_name, &caller.id, &result);
                        events.bubble(&result, "normal", Some(&caller.id));
                    }
                    let mut out =
                        json!({ "result": result, "threadId": thread_id, "status": "completed" });
                    // v2 fill-slot: the agent answers a single-choice question;
                    // the slot value is the option contained in the reply, or the
                    // raw reply when fillOptions is empty (free-form fill).
                    let options = param_str_array(node, "fillOptions").unwrap_or_default();
                    let slot = if options.is_empty() {
                        result.trim().to_string()
                    } else {
                        let lower = result.to_lowercase();
                        options
                            .iter()
                            .find(|o| lower.contains(&o.to_lowercase()))
                            .cloned()
                            .unwrap_or_else(|| result.trim().to_string())
                    };
                    if let Some(obj) = out.as_object_mut() {
                        obj.insert("slot".into(), json!(slot));
                    }
                    Ok((out, "out".into()))
                }
                Ok(None) => Ok((json!({ "status": "sent" }), "out".into())),
                Err(e) => Err(e),
            }
        }
        "show_window" => {
            let target = param_str(node, "target")?;
            windows.show_window(&target)?;
            Ok((json!({ "opened": true }), "out".into()))
        }
        "wait" => {
            let secs = param_i64(node, "seconds").unwrap_or(1).clamp(1, 3600);
            let deadline = Instant::now() + Duration::from_secs(secs as u64);
            while Instant::now() < deadline {
                if cancel.load(Ordering::Relaxed) {
                    return Err("已取消".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok((json!({ "elapsedSec": secs }), "out".into()))
        }
        "branch" => {
            // v1.10.5 (ADR-0018): friendly conditions only — upstream Agent
            // fill-slot (auto via incoming edge) or current focus state. No
            // user-visible variables.
            let condition = param_str(node, "condition").unwrap_or_else(|_| "slot".into());
            if condition == "focus_state" {
                let state = system.focus_state();
                let want = param_str(node, "focusState").unwrap_or_else(|_| "focus".into());
                let matched = state == want;
                Ok((
                    json!({ "matched": matched, "value": state, "option": "" }),
                    if matched { "true" } else { "false" }.into(),
                ))
            } else {
                let up = incoming
                    .get(node.id.as_str())
                    .copied()
                    .ok_or_else(|| format!("分支 {} 需要连接上游 Agent", node.id))?;
                let resolved = data
                    .get(up)
                    .and_then(|o| o.get("slot"))
                    .map(value_to_string)
                    .unwrap_or_default();
                let options = param_str_array(node, "options").unwrap_or_default();
                let idx = options.iter().position(|o| o.trim() == resolved.trim());
                match idx {
                    Some(k) => {
                        let handle = format!("option{}", k + 1);
                        Ok((
                            json!({ "matched": true, "value": resolved, "option": options[k] }),
                            handle,
                        ))
                    }
                    None => {
                        // No matching option: the flow stops at this node.
                        Ok((
                            json!({ "matched": false, "value": resolved, "option": "" }),
                            "none".into(),
                        ))
                    }
                }
            }
        }
        "focus" => {
            let secs = param_i64(node, "seconds").unwrap_or(1).clamp(1, 3600);
            system.focus(secs)?;
            // v1.11.1: block until the countdown finishes (or cancel), so a
            // workflow loop matches real time instead of spinning.
            sleep_wait(secs, cancel)?;
            Ok((
                json!({ "completed": true, "elapsedSec": secs }),
                "out".into(),
            ))
        }
        "idle" => {
            let secs = param_i64(node, "seconds").unwrap_or(1).clamp(1, 3600);
            system.idle(secs)?;
            // v1.11.1: same blocking wait as focus (root cause of the endless
            // ring loop: idle returned instantly and the cycle spun).
            sleep_wait(secs, cancel)?;
            Ok((
                json!({ "completed": true, "elapsedSec": secs }),
                "out".into(),
            ))
        }
        "ring" => {
            let secs = param_i64(node, "seconds").unwrap_or(1).clamp(1, 120);
            system.ring(secs)?;
            // v1.11.1: ring for the requested duration; cancel stops it.
            sleep_wait(secs, cancel)?;
            Ok((json!({ "played": true, "seconds": secs }), "out".into()))
        }
        other => Err(format!("未知节点类型: {other}")),
    }
}

/// v1.11.1: sleep for `secs` seconds, polling the cancel flag every 100ms so
/// long-running workflow nodes (focus/idle/ring) stop promptly on cancel.
fn sleep_wait(secs: i64, cancel: &AtomicBool) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(secs as u64);
    while Instant::now() < deadline {
        if cancel.load(Ordering::Relaxed) {
            return Err("已取消".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

fn param_str(node: &NodeDef, key: &str) -> Result<String, String> {
    node.params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("节点 {} 缺少参数 {}", node.id, key))
}

fn param_bool(node: &NodeDef, key: &str) -> Option<bool> {
    node.params.get(key).and_then(Value::as_bool)
}

fn param_i64(node: &NodeDef, key: &str) -> Option<i64> {
    node.params.get(key).and_then(Value::as_i64)
}

fn param_str_array(node: &NodeDef, key: &str) -> Result<Vec<String>, String> {
    node.params
        .get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<Vec<String>>()
        })
        .ok_or_else(|| format!("节点 {} 缺少参数数组 {}", node.id, key))
}

/// v2: like `resolve` but also handles {{system.<field>}} refs (focus state /
/// wall-clock time) through the injected SystemActions.
pub fn resolve_with_system(
    text: &str,
    data: &HashMap<String, Value>,
    system: &dyn SystemActions,
) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err("未闭合的引用 {{".into());
        };
        let token = after[..end].trim();
        if let Some(name) = token.strip_prefix("system.") {
            let v = match name {
                "focus_state" => system.focus_state(),
                "time" => system.now_hhmm(),
                _ => return Err(format!("未知系统字段: {token}")),
            };
            out.push_str(&v);
        } else {
            let (node_id, field) = token
                .split_once('.')
                .ok_or_else(|| format!("引用格式应为 {{节点.字段}}: {token}"))?;
            let v = data
                .get(node_id)
                .and_then(|o| o.get(field))
                .ok_or_else(|| format!("引用缺失: {token}"))?;
            out.push_str(&value_to_string(v));
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_engine::model::EdgeDef;
    use std::sync::{Arc, Mutex};

    struct MockSystem {
        focus_calls: Mutex<Vec<i64>>,
        idle_calls: Mutex<Vec<i64>>,
        ring_calls: Mutex<Vec<i64>>,
        fail_actions: bool,
        focus_state: String,
        now_hhmm: String,
    }
    impl MockSystem {
        fn new(focus_state: &str) -> Self {
            Self {
                focus_calls: Mutex::new(vec![]),
                idle_calls: Mutex::new(vec![]),
                ring_calls: Mutex::new(vec![]),
                fail_actions: false,
                focus_state: focus_state.into(),
                now_hhmm: "12:00".into(),
            }
        }
        fn calls(&self) -> (Vec<i64>, Vec<i64>, Vec<i64>) {
            (
                self.focus_calls.lock().unwrap().clone(),
                self.idle_calls.lock().unwrap().clone(),
                self.ring_calls.lock().unwrap().clone(),
            )
        }
    }
    impl SystemActions for MockSystem {
        fn focus(&self, seconds: i64) -> Result<(), String> {
            if self.fail_actions {
                return Err("system boom".into());
            }
            self.focus_calls.lock().unwrap().push(seconds);
            Ok(())
        }
        fn idle(&self, seconds: i64) -> Result<(), String> {
            if self.fail_actions {
                return Err("system boom".into());
            }
            self.idle_calls.lock().unwrap().push(seconds);
            Ok(())
        }
        fn ring(&self, seconds: i64) -> Result<(), String> {
            if self.fail_actions {
                return Err("system boom".into());
            }
            self.ring_calls.lock().unwrap().push(seconds);
            Ok(())
        }
        fn focus_state(&self) -> String {
            self.focus_state.clone()
        }
        fn now_hhmm(&self) -> String {
            self.now_hhmm.clone()
        }
    }

    struct MockAgent {
        result: String,
        fail: bool,
        prompts: Mutex<Vec<String>>,
        targets: Mutex<Vec<String>>,
        displays: Mutex<Vec<AgentDisplay>>,
    }
    impl MockAgent {
        fn new(result: &str) -> Self {
            Self {
                result: result.into(),
                fail: false,
                prompts: Mutex::new(vec![]),
                targets: Mutex::new(vec![]),
                displays: Mutex::new(vec![]),
            }
        }
    }
    impl AgentCall for MockAgent {
        fn resolve_character(&self, id: &str) -> Option<CharacterInfo> {
            if id.is_empty() {
                None
            } else {
                Some(CharacterInfo {
                    id: id.into(),
                    name: id.into(),
                    persona: "p".into(),
                })
            }
        }

        fn call_one_shot(
            &self,
            c: &CharacterInfo,
            prompt: &str,
            wait: bool,
            _cancel: &AtomicBool,
            display: AgentDisplay,
        ) -> Result<Option<(String, String)>, String> {
            self.prompts.lock().unwrap().push(prompt.to_string());
            self.targets.lock().unwrap().push(c.id.clone());
            self.displays.lock().unwrap().push(display);
            if self.fail {
                return Err("agent boom".into());
            }
            if wait {
                Ok(Some((
                    format!("{} => {}", self.result, prompt),
                    "th-1".into(),
                )))
            } else {
                Ok(None)
            }
        }
    }
    #[derive(Default)]
    struct Sink {
        bubbles: Mutex<Vec<(String, String, Option<String>)>>,
        agent_results: Mutex<Vec<(String, String, String, String)>>,
    }
    impl EventSink for Sink {
        fn bubble(&self, text: &str, priority: &str, agent_id: Option<&str>) {
            self.bubbles.lock().unwrap().push((
                text.into(),
                priority.into(),
                agent_id.map(str::to_owned),
            ));
        }

        fn agent_result(&self, workflow_id: &str, workflow_name: &str, agent_id: &str, text: &str) {
            self.agent_results.lock().unwrap().push((
                workflow_id.into(),
                workflow_name.into(),
                agent_id.into(),
                text.into(),
            ));
        }
    }
    struct Win {
        opened: Mutex<Vec<String>>,
    }
    impl WindowOps for Win {
        fn show_window(&self, label: &str) -> Result<(), String> {
            self.opened.lock().unwrap().push(label.into());
            Ok(())
        }
    }

    fn wf(nodes: Vec<NodeDef>, edges: Vec<EdgeDef>) -> WorkflowDef {
        WorkflowDef {
            id: "w".into(),
            character_id: "c".into(),
            name: "t".into(),
            trigger: "manual".into(),
            schedule_type: None,
            interval_minutes: None,
            daily_time: None,
            weekly_day: None,
            weekly_time: None,
            guard: "none".into(),
            nodes,
            edges,
            enabled: true,
            next_run_at: None,
        }
    }
    fn node(id: &str, kind: &str, params: Value) -> NodeDef {
        NodeDef {
            id: id.into(),
            kind: kind.into(),
            params,
            x: 0.0,
            y: 0.0,
        }
    }
    fn edge(source: &str, handle: &str, target: &str) -> EdgeDef {
        EdgeDef {
            id: format!("{source}-{handle}-{target}"),
            source: source.into(),
            source_handle: handle.into(),
            target: target.into(),
        }
    }
    fn char_info() -> CharacterInfo {
        CharacterInfo {
            id: "c".into(),
            name: "c".into(),
            persona: "p".into(),
        }
    }
    fn run(w: &WorkflowDef, agent: &MockAgent, system: &MockSystem) -> RunOutcome {
        let sink = Sink::default();
        let win = Win {
            opened: Mutex::new(vec![]),
        };
        let cancel = AtomicBool::new(false);
        execute_run(w, &char_info(), agent, &sink, &win, system, &cancel)
    }
    fn run_with_sink(
        w: &WorkflowDef,
        agent: &MockAgent,
        system: &MockSystem,
        sink: &Sink,
    ) -> RunOutcome {
        let win = Win {
            opened: Mutex::new(vec![]),
        };
        let cancel = AtomicBool::new(false);
        execute_run(w, &char_info(), agent, sink, &win, system, &cancel)
    }

    #[test]
    fn agent_node_character_id_overrides_workflow_character() {
        let agent = MockAgent::new("R");
        let system = MockSystem::new("idle");
        // wf character_id is "c" (see wf()); the node targets "char-b".
        let w = wf(
            vec![node(
                "a",
                "agent",
                json!({ "prompt": "go", "wait": true, "characterId": "char-b" }),
            )],
            vec![],
        );
        let out = run(&w, &agent, &system);
        assert_eq!(out.status, RunStatus::Success);
        let targets = agent.targets.lock().unwrap();
        assert_eq!(*targets, vec!["char-b".to_string()]);
    }

    #[test]
    fn agent_node_empty_character_id_keeps_workflow_character() {
        let agent = MockAgent::new("R");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![node(
                "a",
                "agent",
                json!({ "prompt": "go", "wait": true, "characterId": "" }),
            )],
            vec![],
        );
        let out = run(&w, &agent, &system);
        assert_eq!(out.status, RunStatus::Success);
        let targets = agent.targets.lock().unwrap();
        assert_eq!(*targets, vec!["c".to_string()]);
    }

    #[test]
    fn agent_success_emits_one_result_and_bubble_for_target_agent() {
        let sink = Sink::default();
        let agent = MockAgent::new("我比较专注吧");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![node(
                "a",
                "agent",
                json!({ "prompt": "自检", "wait": true, "characterId": "char-b", "showInitial": true, "showThinking": true, "showResult": true }),
            )],
            vec![],
        );
        let out = run_with_sink(&w, &agent, &system, &sink);
        assert_eq!(out.status, RunStatus::Success);
        let bubbles: Vec<String> = sink
            .bubbles
            .lock()
            .unwrap()
            .iter()
            .map(|(t, _p, _agent_id)| t.clone())
            .collect();
        assert_eq!(bubbles.len(), 1);
        assert!(bubbles[0].starts_with("我比较专注吧 => "));
        assert_eq!(sink.bubbles.lock().unwrap()[0].1, "normal");
        assert_eq!(sink.bubbles.lock().unwrap()[0].2.as_deref(), Some("char-b"));
        assert_eq!(
            *agent.displays.lock().unwrap(),
            vec![AgentDisplay {
                show_initial: false,
                show_thinking: false,
                show_result: false
            }],
        );
        let results = sink.agent_results.lock().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "w");
        assert_eq!(results[0].1, "t");
        assert_eq!(results[0].2, "char-b");
        assert!(results[0].3.starts_with("我比较专注吧 => "));
    }

    #[test]
    fn agent_no_wait_does_not_bubble() {
        let sink = Sink::default();
        let agent = MockAgent::new("R");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![
                node("a", "agent", json!({ "prompt": "go", "wait": false })),
                node("b", "show_window", json!({ "target": "chat" })),
            ],
            vec![edge("a", "out", "b")],
        );
        let out = run_with_sink(&w, &agent, &system, &sink);
        assert_eq!(out.status, RunStatus::Success);
        assert!(sink.bubbles.lock().unwrap().is_empty());
        assert_eq!(
            out.node_log
                .iter()
                .find(|l| l.node_id == "a")
                .unwrap()
                .output,
            Some(json!({ "status": "sent" }))
        );
    }

    #[test]
    fn agent_show_result_false_emits_neither_result_nor_bubble() {
        let sink = Sink::default();
        let agent = MockAgent::new("hidden");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![node(
                "a",
                "agent",
                json!({ "prompt": "go", "wait": true, "showResult": false }),
            )],
            vec![],
        );

        let out = run_with_sink(&w, &agent, &system, &sink);

        assert_eq!(out.status, RunStatus::Success);
        assert!(sink.agent_results.lock().unwrap().is_empty());
        assert!(sink.bubbles.lock().unwrap().is_empty());
    }

    #[test]
    fn branch_slot_auto_routes_option() {
        let agent = MockAgent::new("我现在很专注");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![
                node(
                    "a",
                    "agent",
                    json!({ "prompt": "自检", "wait": true, "fillOptions": ["专注", "分心"] }),
                ),
                node(
                    "b",
                    "branch",
                    json!({ "condition": "slot", "options": ["专注", "分心"] }),
                ),
                node("w1", "wait", json!({ "seconds": 1 })),
                node("w2", "wait", json!({ "seconds": 1 })),
            ],
            vec![
                edge("a", "out", "b"),
                edge("b", "option1", "w1"),
                edge("b", "option2", "w2"),
            ],
        );
        let out = run(&w, &agent, &system);
        assert_eq!(out.status, RunStatus::Success);
        assert_eq!(
            out.node_log
                .iter()
                .find(|l| l.node_id == "w1")
                .unwrap()
                .status,
            "ok"
        );
        assert_eq!(
            out.node_log
                .iter()
                .find(|l| l.node_id == "w2")
                .unwrap()
                .status,
            "skipped"
        );
        let slot = out
            .node_log
            .iter()
            .find(|l| l.node_id == "a")
            .unwrap()
            .output
            .as_ref()
            .unwrap()
            .get("slot")
            .unwrap();
        assert_eq!(slot, "专注");
    }

    #[test]
    fn branch_focus_state_routes_true_false() {
        let agent = MockAgent::new("x");
        let system = MockSystem::new("focus");
        let w = wf(
            vec![
                node(
                    "b",
                    "branch",
                    json!({ "condition": "focus_state", "focusState": "focus" }),
                ),
                node("t", "wait", json!({ "seconds": 1 })),
                node("f", "wait", json!({ "seconds": 1 })),
            ],
            vec![edge("b", "true", "t"), edge("b", "false", "f")],
        );
        let out = run(&w, &agent, &system);
        assert_eq!(out.status, RunStatus::Success);
        assert_eq!(
            out.node_log
                .iter()
                .find(|l| l.node_id == "t")
                .unwrap()
                .status,
            "ok"
        );
        assert_eq!(
            out.node_log
                .iter()
                .find(|l| l.node_id == "f")
                .unwrap()
                .status,
            "skipped"
        );

        let system2 = MockSystem::new("rest");
        let out2 = run(&w, &agent, &system2);
        assert_eq!(
            out2.node_log
                .iter()
                .find(|l| l.node_id == "t")
                .unwrap()
                .status,
            "skipped"
        );
        assert_eq!(
            out2.node_log
                .iter()
                .find(|l| l.node_id == "f")
                .unwrap()
                .status,
            "ok"
        );
    }

    #[test]
    fn branch_without_upstream_fails() {
        let agent = MockAgent::new("x");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![node(
                "b",
                "branch",
                json!({ "condition": "slot", "options": ["专注"] }),
            )],
            vec![],
        );
        let out = run(&w, &agent, &system);
        assert_eq!(out.status, RunStatus::Failed);
        assert!(out.error.as_deref().unwrap().contains("连接上游"));
    }

    #[test]
    fn agent_prompt_resolves_system_refs() {
        let agent = MockAgent::new("R");
        let system = MockSystem::new("focus");
        let w = wf(
            vec![node(
                "a",
                "agent",
                json!({ "prompt": "{{system.focus_state}} @ {{system.time}}", "wait": true }),
            )],
            vec![],
        );
        let out = run(&w, &agent, &system);
        assert_eq!(out.status, RunStatus::Success);
        assert_eq!(agent.prompts.lock().unwrap()[0], "focus @ 12:00");
    }

    #[test]
    fn fill_slot_hit_and_free_fallback() {
        let agent = MockAgent::new("我比较专注吧");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![node(
                "a",
                "agent",
                json!({ "prompt": "自检", "wait": true, "fillOptions": ["专注", "分心"] }),
            )],
            vec![],
        );
        let out = run(&w, &agent, &system);
        let slot = out
            .node_log
            .iter()
            .find(|l| l.node_id == "a")
            .unwrap()
            .output
            .as_ref()
            .unwrap()
            .get("slot")
            .unwrap();
        assert_eq!(slot, "专注");

        let w2 = wf(
            vec![node(
                "a",
                "agent",
                json!({ "prompt": "自由回答", "wait": true }),
            )],
            vec![],
        );
        let out2 = run(&w2, &agent, &system);
        let slot2 = out2
            .node_log
            .iter()
            .find(|l| l.node_id == "a")
            .unwrap()
            .output
            .as_ref()
            .unwrap()
            .get("slot")
            .unwrap();
        assert_eq!(slot2, "我比较专注吧 => 自由回答");

        let agent2 = MockAgent::new("随便说说");
        let out3 = run(&w, &agent2, &system);
        let slot3 = out3
            .node_log
            .iter()
            .find(|l| l.node_id == "a")
            .unwrap()
            .output
            .as_ref()
            .unwrap()
            .get("slot")
            .unwrap();
        assert_eq!(slot3, "随便说说 => 自检");
    }

    #[test]
    fn cycle_executes_and_cancel_escapes() {
        let agent = MockAgent::new("x");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![
                node("a", "wait", json!({ "seconds": 1 })),
                node("b", "wait", json!({ "seconds": 1 })),
            ],
            vec![edge("a", "out", "b"), edge("b", "out", "a")],
        );
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_ref = cancel.clone();
        let cancel_thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(2200));
            cancel_ref.store(true, Ordering::Relaxed);
        });
        let sink = Sink::default();
        let win = Win {
            opened: Mutex::new(vec![]),
        };
        let out = execute_run(
            &w,
            &char_info(),
            &agent,
            &sink,
            &win,
            &system,
            cancel.as_ref(),
        );
        let _ = cancel_thread.join();
        assert_eq!(out.status, RunStatus::Cancelled);
    }

    #[test]
    fn focus_idle_ring_actions_called() {
        let agent = MockAgent::new("x");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![
                node("f", "focus", json!({ "seconds": 1 })),
                node("i", "idle", json!({ "seconds": 1 })),
                node("r", "ring", json!({ "seconds": 1 })),
            ],
            vec![edge("f", "out", "i"), edge("i", "out", "r")],
        );
        let out = run(&w, &agent, &system);
        assert_eq!(out.status, RunStatus::Success);
        assert_eq!(system.calls(), (vec![1], vec![1], vec![1]));
    }

    #[test]
    fn system_action_failure_fails_fast() {
        let agent = MockAgent::new("x");
        let mut system = MockSystem::new("idle");
        system.fail_actions = true;
        let w = wf(
            vec![
                node("f", "focus", json!({ "seconds": 1500 })),
                node("w", "wait", json!({ "seconds": 1 })),
            ],
            vec![edge("f", "out", "w")],
        );
        let out = run(&w, &agent, &system);
        assert_eq!(out.status, RunStatus::Failed);
        assert!(out.error.as_deref().unwrap().contains("system boom"));
        assert_eq!(
            out.node_log
                .iter()
                .find(|l| l.node_id == "w")
                .unwrap()
                .status,
            "skipped"
        );
    }

    #[test]
    fn fail_fast_stops_run() {
        let sink = Sink::default();
        let mut agent = MockAgent::new("x");
        agent.fail = true;
        let system = MockSystem::new("idle");
        let w = wf(
            vec![
                node("a", "agent", json!({ "prompt": "x", "wait": true })),
                node("w", "wait", json!({ "seconds": 1 })),
            ],
            vec![edge("a", "out", "w")],
        );
        let out = run_with_sink(&w, &agent, &system, &sink);
        assert_eq!(out.status, RunStatus::Failed);
        assert!(out.error.as_deref().unwrap().contains("agent boom"));
        assert!(sink.bubbles.lock().unwrap().is_empty());
    }

    #[test]
    fn wait_node_cancellation() {
        let agent = MockAgent::new("x");
        let system = MockSystem::new("idle");
        let cancel = AtomicBool::new(true);
        let sink = Sink::default();
        let win = Win {
            opened: Mutex::new(vec![]),
        };
        let w = wf(vec![node("w", "wait", json!({ "seconds": 5 }))], vec![]);
        let out = execute_run(&w, &char_info(), &agent, &sink, &win, &system, &cancel);
        assert_eq!(out.status, RunStatus::Cancelled);
    }

    #[test]
    fn focus_idle_ring_block_and_cancel() {
        // v1.11.1: focus/idle/ring block until the duration elapses; a set
        // cancel flag interrupts them immediately (Cancelled).
        let cancel = AtomicBool::new(true);
        let agent = MockAgent::new("x");
        let system = MockSystem::new("idle");
        let sink = Sink::default();
        let win = Win {
            opened: Mutex::new(vec![]),
        };
        for kind in ["focus", "idle", "ring"] {
            let w = wf(vec![node("f", kind, json!({ "seconds": 60 }))], vec![]);
            let out = execute_run(&w, &char_info(), &agent, &sink, &win, &system, &cancel);
            assert_eq!(out.status, RunStatus::Cancelled, "{kind} should cancel");
        }
    }

    #[test]
    fn missing_ref_fails_fast() {
        let agent = MockAgent::new("x");
        let system = MockSystem::new("idle");
        let w = wf(
            vec![node(
                "a",
                "agent",
                json!({ "prompt": "{{ghost.result}}", "wait": true }),
            )],
            vec![],
        );
        let out = run(&w, &agent, &system);
        assert_eq!(out.status, RunStatus::Failed);
        assert!(out.error.as_deref().unwrap().contains("引用缺失"));
    }

    #[test]
    fn resolve_with_system_fields() {
        let system = MockSystem::new("rest");
        let mut data = HashMap::new();
        data.insert("a".into(), json!({ "slot": "专注" }));
        assert_eq!(
            resolve_with_system(
                "s={{system.focus_state}} t={{system.time}} slot={{a.slot}}",
                &data,
                &system
            )
            .unwrap(),
            "s=rest t=12:00 slot=专注"
        );
    }
}
