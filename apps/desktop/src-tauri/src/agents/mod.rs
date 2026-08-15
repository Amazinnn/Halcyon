//! Agent providers (ADR-0007/0025): Focus embeds real Codex and Claude CLIs
//! and keeps the scripted mock for Rust test injection. All implement the same
//! `AgentProvider` trait; events are published as AgentEvent v1 envelopes over
//! the core event bus (`agent:event` / `pet:state_changed` /
//! `bubble:requested`).

pub mod codex;
pub mod claude;
pub mod mock;

use serde::Serialize;
use serde_json::Value;

/// M5 (ADR-0022): full display for direct conversation — initial short
/// sentence, thinking stream and final result all shown.
pub fn agent_display_full(streaming_enabled: bool) -> crate::workflow_engine::engine::AgentDisplay {
    crate::workflow_engine::engine::AgentDisplay {
        show_initial: streaming_enabled,
        show_thinking: streaming_enabled,
        show_result: true,
    }
}

/// M5 (ADR-0022): system-level output discipline — hard-coded, injected into
/// EVERY agent turn (conversation and workflow calls alike). Short sentences,
/// newline-separated; no Markdown; short total. The frontend later truncates
/// pet-bubble sentences at newlines (mechanism deferred).
pub const OUTPUT_DISCIPLINE: &str = "给用户的输出规范：请用简洁的中文短句回答，句间用单个换行分隔；不要使用 Markdown、列表、代码块或长段落；总长度不超过约 200 字；只输出需要直接展示给用户看的内容。";

/// Embedded AgentEvent schema (v1) so emitted samples can be validated in tests.
pub const SCHEMA_JSON: &str =
    include_str!("../../../../../packages/event-schema/agent-event.schema.json");

pub const ACTIVE_TURN_ERROR: &str =
    "Agent already has an active turn; wait for it to finish or stop it first";
pub const PROVIDER_SWITCH_BUSY_ERROR: &str =
    "该宠物的 Agent 正在运行；请等待完成或先中断，再切换 Provider";

pub fn is_busy_turn_error(error: &str) -> bool {
    error == ACTIVE_TURN_ERROR
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentProviderKind {
    Codex,
    Claude,
    #[cfg(test)]
    Mock,
}

impl AgentProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentProviderKind::Codex => "codex",
            AgentProviderKind::Claude => "claude",
            #[cfg(test)]
            AgentProviderKind::Mock => "mock",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }
}

/// Turn completion signal (M4 workflow engine, ADR-0012): lets a one-shot
/// agent node wait for its turn to finish instead of polling events.
#[derive(Debug, Clone)]
pub struct TurnDone {
    pub thread_id: Option<String>,
    pub status: String, // completed | interrupted | error
    pub result: Option<String>,
}

/// A thread summary returned to the UI (subset of the app-server Thread).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentThreadInfo {
    pub id: String,
    pub preview: String,
    pub cwd: String,
    pub status: String,
    pub updated_at: i64,
    /// Marked when the thread was created by a workflow agent node
    /// (ADR-0012 automation threads stay visible with a badge).
    #[serde(default)]
    pub automation: bool,
}

/// Common agent contract implemented by the real (Codex) and mock providers.
/// Methods are synchronous; streaming events arrive on the core event bus.
/// `display` controls provider event visibility. Direct chat passes full
/// display; workflow calls pass all false and let the engine own presentation.
pub trait AgentProvider: Send + Sync {

    /// Create a new thread and start a turn (initial message may be empty).
    fn start_thread(
        &mut self,
        workspace_dir: &str,
        initial_message: &str,
        display: crate::workflow_engine::engine::AgentDisplay,
    ) -> Result<AgentThreadInfo, String>;

    /// Load an existing thread so turns can be appended.
    fn resume_thread(&mut self, thread_id: &str) -> Result<AgentThreadInfo, String>;

    /// Resume an existing thread and immediately start a new turn.
    fn resume_and_send(
        &mut self,
        thread_id: &str,
        text: &str,
        display: crate::workflow_engine::engine::AgentDisplay,
    ) -> Result<AgentThreadInfo, String> {
        let info = self.resume_thread(thread_id)?;
        self.send(&info.id, text, display)?;
        Ok(info)
    }

    fn list_threads(&mut self) -> Result<Vec<AgentThreadInfo>, String>;

    /// Append a user turn to an active thread (streaming events follow).
    fn send(&mut self, thread_id: &str, text: &str, display: crate::workflow_engine::engine::AgentDisplay) -> Result<(), String>;

    fn interrupt(&mut self, thread_id: &str) -> Result<(), String>;
}

/// The provider slot held in AppState. Real providers are Arc<Mutex<..>> so
/// each character owns an isolated runtime while the registry remains cheap to clone.
pub enum AgentRuntime {
    Codex(std::sync::Arc<std::sync::Mutex<codex::CodexProvider>>),
    Claude(std::sync::Arc<std::sync::Mutex<claude::ClaudeProvider>>),
    #[cfg(test)]
    Mock(std::sync::Arc<std::sync::Mutex<mock::MockProvider>>),
}

/// M5 (ADR-0022): multi-Agent registry — one runtime per character (pet).
/// `char-default` always exists (workflow ensures it); map is empty only
/// before characters are ensured.
pub struct AgentRegistry {
    pub runtimes: std::collections::HashMap<String, AgentRuntime>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            runtimes: std::collections::HashMap::new(),
        }
    }

    pub fn get(&self, character_id: &str) -> Option<&AgentRuntime> {
        self.runtimes.get(character_id)
    }

    pub fn insert(&mut self, character_id: String, rt: AgentRuntime) {
        self.runtimes.insert(character_id, rt);
    }

    pub fn get_or_try_insert_with(
        &mut self,
        character_id: &str,
        build: impl FnOnce() -> Result<AgentRuntime, String>,
    ) -> Result<AgentRuntime, String> {
        if let Some(runtime) = self.runtimes.get(character_id) {
            return Ok(runtime.shared_clone());
        }
        let runtime = build()?;
        self.runtimes
            .insert(character_id.to_string(), runtime.shared_clone());
        Ok(runtime)
    }
}

impl AgentRuntime {
    pub fn shared_clone(&self) -> Self {
        match self {
            AgentRuntime::Codex(provider) => AgentRuntime::Codex(provider.clone()),
            AgentRuntime::Claude(provider) => AgentRuntime::Claude(provider.clone()),
            #[cfg(test)]
            AgentRuntime::Mock(provider) => AgentRuntime::Mock(provider.clone()),
        }
    }

    pub fn has_active_turn(&self) -> bool {
        match self {
            AgentRuntime::Codex(provider) => provider.lock().unwrap().has_active_turn(),
            AgentRuntime::Claude(provider) => provider.lock().unwrap().has_active_turn(),
            #[cfg(test)]
            AgentRuntime::Mock(_) => false,
        }
    }

    pub fn kind(&self) -> AgentProviderKind {
        match self {
            AgentRuntime::Codex(_) => AgentProviderKind::Codex,
            AgentRuntime::Claude(_) => AgentProviderKind::Claude,
            #[cfg(test)]
            AgentRuntime::Mock(_) => AgentProviderKind::Mock,
        }
    }

    pub fn start_thread(
        &self,
        workspace_dir: &str,
        initial_message: &str,
        display: crate::workflow_engine::engine::AgentDisplay,
    ) -> Result<AgentThreadInfo, String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().start_thread(workspace_dir, initial_message, display),
            AgentRuntime::Claude(p) => p.lock().unwrap().start_thread(workspace_dir, initial_message, display),
            #[cfg(test)]
            AgentRuntime::Mock(p) => p.lock().unwrap().start_thread(workspace_dir, initial_message, display),
        }
    }

    pub fn resume_thread(&self, thread_id: &str) -> Result<AgentThreadInfo, String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().resume_thread(thread_id),
            AgentRuntime::Claude(p) => p.lock().unwrap().resume_thread(thread_id),
            #[cfg(test)]
            AgentRuntime::Mock(p) => p.lock().unwrap().resume_thread(thread_id),
        }
    }

    pub fn resume_and_send(
        &self,
        thread_id: &str,
        text: &str,
        display: crate::workflow_engine::engine::AgentDisplay,
    ) -> Result<AgentThreadInfo, String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().resume_and_send(thread_id, text, display),
            AgentRuntime::Claude(p) => p.lock().unwrap().resume_and_send(thread_id, text, display),
            #[cfg(test)]
            AgentRuntime::Mock(p) => p.lock().unwrap().resume_and_send(thread_id, text, display),
        }
    }

    pub fn list_threads(&self) -> Result<Vec<AgentThreadInfo>, String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().list_threads(),
            AgentRuntime::Claude(p) => p.lock().unwrap().list_threads(),
            #[cfg(test)]
            AgentRuntime::Mock(p) => p.lock().unwrap().list_threads(),
        }
    }

    pub fn send(&self, thread_id: &str, text: &str, display: crate::workflow_engine::engine::AgentDisplay) -> Result<(), String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().send(thread_id, text, display),
            AgentRuntime::Claude(p) => p.lock().unwrap().send(thread_id, text, display),
            #[cfg(test)]
            AgentRuntime::Mock(p) => p.lock().unwrap().send(thread_id, text, display),
        }
    }

    /// Subscribe to turn-completion signals (workflow agent nodes).
    pub fn subscribe_turn_done(&self) -> Option<tokio::sync::broadcast::Receiver<TurnDone>> {
        match self {
            AgentRuntime::Codex(p) => Some(p.lock().unwrap().subscribe_turn_done()),
            AgentRuntime::Claude(p) => Some(p.lock().unwrap().subscribe_turn_done()),
            #[cfg(test)]
            AgentRuntime::Mock(p) => Some(p.lock().unwrap().subscribe_turn_done()),
        }
    }

    pub fn interrupt(&self, thread_id: &str) -> Result<(), String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().interrupt(thread_id),
            AgentRuntime::Claude(p) => p.lock().unwrap().interrupt(thread_id),
            #[cfg(test)]
            AgentRuntime::Mock(p) => p.lock().unwrap().interrupt(thread_id),
        }
    }
}

/// Build a schema-valid AgentEvent v1 envelope.
pub(crate) fn envelope(agent_id: &str, session_id: &str, event: Value) -> Value {
    serde_json::json!({
        "schemaVersion": 1,
        "agentId": agent_id,
        "sessionId": session_id,
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "event": event,
    })
}

/// Lightweight, schema-driven structural validation (stand-in for a full JSON
/// Schema engine in Rust). Reads `$defs` enum + required fields from the
/// embedded `agent-event.schema.json`; full validation lives in
/// packages/event-schema (Ajv).
#[allow(dead_code)]
pub fn validate_envelope(value: &Value) -> Result<(), String> {
    let schema: Value = serde_json::from_str(SCHEMA_JSON).map_err(|e| e.to_string())?;
    let defs = schema.get("$defs").ok_or("schema has no $defs")?;

    if value.get("schemaVersion") != Some(&serde_json::json!(1)) {
        return Err("schemaVersion != 1".into());
    }
    for field in ["agentId", "sessionId", "timestamp"] {
        let ok = value
            .get(field)
            .and_then(Value::as_str)
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if !ok {
            return Err(format!("envelope missing {field}"));
        }
    }
    let event = value.get("event").ok_or("event missing")?;
    let ev_type = event.get("type").and_then(Value::as_str).ok_or("event.type missing")?;

    let states: Vec<&str> = defs
        .get("agentState")
        .and_then(|s| s.get("enum"))
        .and_then(Value::as_array)
        .ok_or("agentState enum missing")?
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    if let Some(state) = event.get("state").and_then(Value::as_str) {
        if !states.contains(&state) {
            return Err(format!("invalid state {state}"));
        }
    }
    if let Some(risk) = event.get("risk").and_then(Value::as_str) {
        if !["low", "medium", "high", "critical"].contains(&risk) {
            return Err(format!("invalid risk {risk}"));
        }
    }

    let kinds = defs
        .get("event")
        .and_then(|e| e.get("oneOf"))
        .and_then(Value::as_array)
        .ok_or("event oneOf missing")?;
    let ref_names: Vec<&str> = kinds
        .iter()
        .filter_map(|k| k.get("$ref").and_then(|r| r.as_str()))
        .filter_map(|r| r.rsplit('/').next())
        .collect();
    let kind = kind_name(ev_type);
    if !ref_names.contains(&kind.as_str()) {
        return Err(format!("event type {ev_type} not defined in schema"));
    }

    let required: Vec<&str> = defs
        .get(&kind)
        .and_then(|d| d.get("required"))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    for field in required {
        if event.get(field).is_none() {
            return Err(format!("{ev_type} missing required field {field}"));
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn kind_name(ev_type: &str) -> String {
    ev_type
        .split('.')
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                part.to_string()
            } else {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod runtime_registry_tests {
    use super::{AgentRegistry, AgentRuntime};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier, Mutex};

    #[test]
    fn concurrent_runtime_registry_builds_once_and_returns_one_shared_turn_guard() {
        let registry = Arc::new(Mutex::new(AgentRegistry::new()));
        let barrier = Arc::new(Barrier::new(3));
        let builds = Arc::new(AtomicUsize::new(0));
        let (tx, _) = tokio::sync::broadcast::channel(8);
        let mut workers = Vec::new();

        for _ in 0..2 {
            let registry = registry.clone();
            let barrier = barrier.clone();
            let builds = builds.clone();
            let tx = tx.clone();
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                let runtime = registry
                    .lock()
                    .unwrap()
                    .get_or_try_insert_with("char-race", || {
                        builds.fetch_add(1, Ordering::SeqCst);
                        Ok(AgentRuntime::Claude(Arc::new(Mutex::new(
                            super::claude::ClaudeProvider::new(
                                tx,
                                PathBuf::from("claude.exe"),
                                "char-race".into(),
                                r"C:\Focus-Agents\char-race".into(),
                            ),
                        ))))
                    })
                    .unwrap();
                match runtime {
                    AgentRuntime::Claude(provider) => provider,
                    _ => panic!("expected Claude runtime"),
                }
            }));
        }

        barrier.wait();
        let first = workers.remove(0).join().unwrap();
        let second = workers.remove(0).join().unwrap();
        assert_eq!(builds.load(Ordering::SeqCst), 1);
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(registry.lock().unwrap().runtimes.len(), 1);
    }
}
