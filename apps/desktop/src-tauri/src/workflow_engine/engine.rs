//! Workflow executor (M4, ADR-0012). Runs a node+edge graph with a run-local
//! data context ({{nodeId.field}} references), fail-fast semantics and a
//! cancellation flag. Depends only on injected traits so it is unit-testable
//! without Tauri.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::model::{CharacterInfo, NodeDef, NodeLogEntry, RunStatus, WorkflowDef, validate_workflow};

/// One-shot agent invocation. wait=true blocks until the agent turn finishes
/// and returns (result_text, thread_id); wait=false returns None right
/// after dispatch (no output for downstream nodes).
pub trait AgentCall: Send + Sync {
    fn call_one_shot(
        &self,
        character: &CharacterInfo,
        prompt: &str,
        wait: bool,
        cancel: &AtomicBool,
    ) -> Result<Option<(String, String)>, String>;
}

pub trait EventSink: Send + Sync {
    fn bubble(&self, text: &str, priority: &str);
}

pub trait WindowOps: Send + Sync {
    fn show_window(&self, label: &str) -> Result<(), String>;
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
    let mut indegree: HashMap<&str, usize> = nodes.keys().map(|k| (*k, 0usize)).collect();
    let mut out_edges: HashMap<&str, Vec<&crate::workflow_engine::model::EdgeDef>> = HashMap::new();
    for e in &wf.edges {
        *indegree.get_mut(e.target.as_str()).unwrap() += 1;
        out_edges.entry(e.source.as_str()).or_default().push(e);
    }

    let mut remaining: HashMap<&str, usize> = indegree.clone();
    let mut queue: VecDeque<&str> = wf
        .nodes
        .iter()
        .filter(|n| indegree.get(n.id.as_str()) == Some(&0))
        .map(|n| n.id.as_str())
        .collect();

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
        match run_node(node, &data, character, agent, events, windows, cancel) {
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
                    for e in edges {
                        if e.source_handle == taken_handle {
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
                status = if cancelled { RunStatus::Cancelled } else { RunStatus::Failed };
                error = Some(if cancelled { "已取消".into() } else { err.clone() });
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

    // Nodes that never ran (downstream of an untaken branch) are marked skipped.
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

    RunOutcome { status, error, node_log: log }
}

fn run_node(
    node: &NodeDef,
    data: &HashMap<String, Value>,
    character: &CharacterInfo,
    agent: &dyn AgentCall,
    events: &dyn EventSink,
    windows: &dyn WindowOps,
    cancel: &AtomicBool,
) -> Result<(Value, String), String> {
    match node.kind.as_str() {
        "bubble" => {
            let text = param_str(node, "text")?;
            let text = resolve(&text, data)?;
            let priority = param_str(node, "priority").unwrap_or_else(|_| "normal".into());
            events.bubble(&text, &priority);
            Ok((json!({ "text": text }), "out".into()))
        }
        "agent" => {
            let prompt = param_str(node, "prompt")?;
            let prompt = resolve(&prompt, data)?;
            let wait = param_bool(node, "wait").unwrap_or(true);
            match agent.call_one_shot(character, &prompt, wait, cancel) {
                Ok(Some((result, thread_id))) => Ok((
                    json!({ "result": result, "threadId": thread_id, "status": "completed" }),
                    "out".into(),
                )),
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
        "if" => {
            let source = param_str(node, "source").unwrap_or_default();
            let resolved = resolve(&source, data)?;
            let op = param_str(node, "op").unwrap_or_else(|_| "not_empty".into());
            let value = param_str(node, "value").unwrap_or_default();
            let matched = match op.as_str() {
                "contains" => resolved.contains(&value),
                "equals" => resolved == value,
                _ => !resolved.trim().is_empty(),
            };
            Ok((json!({ "matched": matched }), if matched { "true" } else { "false" }.into()))
        }
        other => Err(format!("未知节点类型: {other}")),
    }
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

/// Resolve {{nodeId.field}} references against the run-local data context.
/// Missing refs are hard errors (fail-fast, ADR-0012).
pub fn resolve(text: &str, data: &HashMap<String, Value>) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err("未闭合的引用 {{".into());
        };
        let token = after[..end].trim();
        let (node_id, field) = token
            .split_once('.')
            .ok_or_else(|| format!("引用格式应为 {{节点.字段}}: {token}"))?;
        let v = data
            .get(node_id)
            .and_then(|o| o.get(field))
            .ok_or_else(|| format!("引用缺失: {token}"))?;
        out.push_str(&value_to_string(v));
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
    use crate::workflow_engine::model::{EdgeDef, NodeDef};

    struct MockAgent {
        result: String,
        fail: bool,
    }
    impl AgentCall for MockAgent {
        fn call_one_shot(
            &self,
            _c: &CharacterInfo,
            prompt: &str,
            wait: bool,
            _cancel: &AtomicBool,
        ) -> Result<Option<(String, String)>, String> {
            if self.fail {
                return Err("agent boom".into());
            }
            if wait {
                Ok(Some((format!("{} => {}", self.result, prompt), "th-1".into())))
            } else {
                Ok(None)
            }
        }
    }
    struct Sink {
        bubbles: std::sync::Mutex<Vec<(String, String)>>,
    }
    impl EventSink for Sink {
        fn bubble(&self, text: &str, priority: &str) {
            self.bubbles.lock().unwrap().push((text.into(), priority.into()));
        }
    }
    struct Win {
        opened: std::sync::Mutex<Vec<String>>,
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
            guard: "none".into(),
            nodes,
            edges,
            enabled: true,
            next_run_at: None,
        }
    }
    fn node(id: &str, kind: &str, params: Value) -> NodeDef {
        NodeDef { id: id.into(), kind: kind.into(), params, x: 0.0, y: 0.0 }
    }
    fn edge(source: &str, handle: &str, target: &str) -> EdgeDef {
        EdgeDef { id: format!("{source}-{handle}-{target}"), source: source.into(), source_handle: handle.into(), target: target.into() }
    }

    #[test]
    fn bubble_output_and_ref_resolution() {
        let sink = Sink { bubbles: Default::default() };
        let win = Win { opened: Default::default() };
        let agent = MockAgent { result: "R".into(), fail: false };
        let cancel = AtomicBool::new(false);
        let w = wf(
            vec![
                node("a", "agent", json!({ "prompt": "hi", "wait": true })),
                node("b", "bubble", json!({ "text": "got: {{a.result}}" })),
            ],
            vec![edge("a", "out", "b")],
        );
        let out = execute_run(&w, &CharacterInfo { id: "c".into(), name: "c".into(), persona: "p".into() }, &agent, &sink, &win, &cancel);
        assert_eq!(out.status, RunStatus::Success);
        assert_eq!(sink.bubbles.lock().unwrap()[0].0, "got: R => hi");
    }

    #[test]
    fn if_branch_routes_to_true_only() {
        let sink = Sink { bubbles: Default::default() };
        let win = Win { opened: Default::default() };
        let agent = MockAgent { result: "分心了".into(), fail: false };
        let cancel = AtomicBool::new(false);
        let w = wf(
            vec![
                node("a", "agent", json!({ "prompt": "自检", "wait": true })),
                node("i", "if", json!({ "source": "{{a.result}}", "op": "contains", "value": "分心" })),
                node("t", "bubble", json!({ "text": "警告" })),
                node("f", "bubble", json!({ "text": "else" })),
            ],
            vec![
                edge("a", "out", "i"),
                edge("i", "true", "t"),
                edge("i", "false", "f"),
            ],
        );
        let out = execute_run(&w, &CharacterInfo { id: "c".into(), name: "c".into(), persona: "".into() }, &agent, &sink, &win, &cancel);
        assert_eq!(out.status, RunStatus::Success);
        let bubbles: Vec<String> = sink.bubbles.lock().unwrap().iter().map(|(t, _)| t.clone()).collect();
        assert_eq!(bubbles, vec!["警告"]);
        let skipped = out.node_log.iter().filter(|l| l.status == "skipped").count();
        assert_eq!(skipped, 1);
        assert_eq!(out.node_log.iter().find(|l| l.node_id == "f").unwrap().status, "skipped");
    }

    #[test]
    fn fail_fast_stops_run() {
        let sink = Sink { bubbles: Default::default() };
        let win = Win { opened: Default::default() };
        let agent = MockAgent { result: "".into(), fail: true };
        let cancel = AtomicBool::new(false);
        let w = wf(
            vec![
                node("a", "agent", json!({ "prompt": "x", "wait": true })),
                node("b", "bubble", json!({ "text": "later" })),
            ],
            vec![edge("a", "out", "b")],
        );
        let out = execute_run(&w, &CharacterInfo { id: "c".into(), name: "c".into(), persona: "".into() }, &agent, &sink, &win, &cancel);
        assert_eq!(out.status, RunStatus::Failed);
        assert!(out.error.as_deref().unwrap().contains("agent boom"));
        assert!(sink.bubbles.lock().unwrap().is_empty());
    }

    #[test]
    fn wait_node_cancellation() {
        let sink = Sink { bubbles: Default::default() };
        let win = Win { opened: Default::default() };
        let agent = MockAgent { result: "".into(), fail: false };
        let cancel = AtomicBool::new(true);
        let w = wf(vec![node("w", "wait", json!({ "seconds": 5 }))], vec![]);
        let out = execute_run(&w, &CharacterInfo { id: "c".into(), name: "c".into(), persona: "".into() }, &agent, &sink, &win, &cancel);
        assert_eq!(out.status, RunStatus::Cancelled);
    }

    #[test]
    fn no_wait_agent_has_no_result_output() {
        let sink = Sink { bubbles: Default::default() };
        let win = Win { opened: Default::default() };
        let agent = MockAgent { result: "".into(), fail: false };
        let cancel = AtomicBool::new(false);
        let w = wf(
            vec![
                node("a", "agent", json!({ "prompt": "go", "wait": false })),
                node("b", "show_window", json!({ "target": "chat" })),
            ],
            vec![edge("a", "out", "b")],
        );
        let out = execute_run(&w, &CharacterInfo { id: "c".into(), name: "c".into(), persona: "".into() }, &agent, &sink, &win, &cancel);
        assert_eq!(out.status, RunStatus::Success);
        assert_eq!(win.opened.lock().unwrap()[0], "chat");
        assert_eq!(out.node_log.iter().find(|l| l.node_id == "a").unwrap().output, Some(json!({ "status": "sent" })));
    }

    #[test]
    fn missing_ref_fails_fast() {
        let sink = Sink { bubbles: Default::default() };
        let win = Win { opened: Default::default() };
        let agent = MockAgent { result: "".into(), fail: false };
        let cancel = AtomicBool::new(false);
        let w = wf(vec![node("b", "bubble", json!({ "text": "{{ghost.result}}" }))], vec![]);
        let out = execute_run(&w, &CharacterInfo { id: "c".into(), name: "c".into(), persona: "".into() }, &agent, &sink, &win, &cancel);
        assert_eq!(out.status, RunStatus::Failed);
        assert!(out.error.as_deref().unwrap().contains("引用缺失"));
    }

    #[test]
    fn resolve_plain_text_and_missing() {
        let mut data = HashMap::new();
        data.insert("a".into(), json!({ "result": "ok", "n": 3 }));
        assert_eq!(resolve("x {{a.result}} y", &data).unwrap(), "x ok y");
        assert_eq!(resolve("n={{a.n}}", &data).unwrap(), "n=3");
        assert!(resolve("{{a.missing}}", &data).is_err());
        assert_eq!(resolve("no refs", &data).unwrap(), "no refs");
    }
}