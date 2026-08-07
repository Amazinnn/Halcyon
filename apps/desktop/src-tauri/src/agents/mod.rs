//! Agent providers (ADR-0007): Focus embeds a real agent CLI (Codex app-server)
//! and keeps the scripted mock as fallback. Both implement the same
//! `AgentProvider` trait; events are published as AgentEvent v1 envelopes over
//! the core event bus (`agent:event` / `pet:state_changed` /
//! `bubble:requested`).

pub mod codex;
pub mod mock;

use serde::Serialize;
use serde_json::Value;

pub const AGENT_ID: &str = "focus-codex";

/// Embedded AgentEvent schema (v1) so emitted samples can be validated in tests.
pub const SCHEMA_JSON: &str =
    include_str!("../../../../../packages/event-schema/agent-event.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentProviderKind {
    Codex,
    Mock,
}

impl AgentProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentProviderKind::Codex => "codex",
            AgentProviderKind::Mock => "mock",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "codex" => Some(Self::Codex),
            "mock" => Some(Self::Mock),
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
pub trait AgentProvider: Send + Sync {

    /// Create a new thread and start a turn (initial message may be empty).
    fn start_thread(
        &mut self,
        workspace_dir: &str,
        initial_message: &str,
    ) -> Result<AgentThreadInfo, String>;

    /// Load an existing thread so turns can be appended.
    fn resume_thread(&mut self, thread_id: &str) -> Result<AgentThreadInfo, String>;

    fn list_threads(&mut self) -> Result<Vec<AgentThreadInfo>, String>;

    /// Append a user turn to an active thread (streaming events follow).
    fn send(&mut self, thread_id: &str, text: &str) -> Result<(), String>;

    fn interrupt(&mut self, thread_id: &str) -> Result<(), String>;
}

/// The provider slot held in AppState. Codex is Arc<Mutex<..>> so a slow
/// in-flight request never blocks the whole agent state (e.g. interrupt).
pub enum AgentRuntime {
    Codex(std::sync::Arc<std::sync::Mutex<codex::CodexProvider>>),
    Mock(std::sync::Mutex<mock::MockProvider>),
}

impl AgentRuntime {
    pub fn kind(&self) -> AgentProviderKind {
        match self {
            AgentRuntime::Codex(_) => AgentProviderKind::Codex,
            AgentRuntime::Mock(_) => AgentProviderKind::Mock,
        }
    }

    pub fn start_thread(
        &self,
        workspace_dir: &str,
        initial_message: &str,
    ) -> Result<AgentThreadInfo, String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().start_thread(workspace_dir, initial_message),
            AgentRuntime::Mock(p) => p.lock().unwrap().start_thread(workspace_dir, initial_message),
        }
    }

    pub fn resume_thread(&self, thread_id: &str) -> Result<AgentThreadInfo, String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().resume_thread(thread_id),
            AgentRuntime::Mock(p) => p.lock().unwrap().resume_thread(thread_id),
        }
    }

    pub fn list_threads(&self) -> Result<Vec<AgentThreadInfo>, String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().list_threads(),
            AgentRuntime::Mock(p) => p.lock().unwrap().list_threads(),
        }
    }

    pub fn send(&self, thread_id: &str, text: &str) -> Result<(), String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().send(thread_id, text),
            AgentRuntime::Mock(p) => p.lock().unwrap().send(thread_id, text),
        }
    }

    /// Subscribe to turn-completion signals (workflow agent nodes).
    pub fn subscribe_turn_done(&self) -> Option<tokio::sync::broadcast::Receiver<TurnDone>> {
        match self {
            AgentRuntime::Codex(p) => Some(p.lock().unwrap().subscribe_turn_done()),
            AgentRuntime::Mock(p) => Some(p.lock().unwrap().subscribe_turn_done()),
        }
    }

    pub fn interrupt(&self, thread_id: &str) -> Result<(), String> {
        match self {
            AgentRuntime::Codex(p) => p.lock().unwrap().interrupt(thread_id),
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