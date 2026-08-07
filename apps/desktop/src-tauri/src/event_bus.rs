//! Core event bus: typed events produced inside the Rust core and relayed to
//! every window through Tauri's event system. Windows never talk to each other
//! directly (ADR-0002).

use serde_json::{json, Value};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum CoreEvent {
    /// A raw AgentEvent envelope (schema v1) from an agent adapter.
    AgentEvent(Value),
    /// Pet state mapping (status -> animation), per design doc §5.2.
    PetStateChanged { state: String, animation: String },
    /// Short pet bubble request, per design doc §5.3.
    BubbleRequested { text: String, priority: String },
    /// Panel mode changed (chat / statistics / ...).
    PanelModeChanged { mode: String },
    /// Music playback tick (fake progress in the spike).
    MusicTick { position_ms: u64, duration_ms: u64 },
    /// Foreground window probe sample.
    ProbeRecorded { process: String, title: String },
    /// Focus round state changed (frontend timer -> engine, M4/ADR-0012).
    FocusStateChanged { state: String, completed: bool },
    /// Supervision alert fired (M4 workflow trigger source, ADR-0012).
    SupervisionAlert { rule: String, app: Option<String>, level: i64, text: String },
    /// A workflow run changed (workflow window refresh, M4/ADR-0012).
    WorkflowRunChanged {
        workflow_id: String,
        run_id: String,
        status: String,
        error: Option<String>,
    },
}

impl CoreEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            CoreEvent::AgentEvent(_) => "agent:event",
            CoreEvent::PetStateChanged { .. } => "pet:state_changed",
            CoreEvent::BubbleRequested { .. } => "bubble:requested",
            CoreEvent::PanelModeChanged { .. } => "panel:mode_changed",
            CoreEvent::MusicTick { .. } => "music:playback_tick",
            CoreEvent::ProbeRecorded { .. } => "probe:recorded",
            CoreEvent::FocusStateChanged { .. } => "focus:core_state",
            CoreEvent::SupervisionAlert { .. } => "supervision:core_alert",
            CoreEvent::WorkflowRunChanged { .. } => "workflow:runs_changed",
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            CoreEvent::AgentEvent(v) => v.clone(),
            CoreEvent::PetStateChanged { state, animation } => {
                json!({ "state": state, "animation": animation })
            }
            CoreEvent::BubbleRequested { text, priority } => {
                json!({ "text": text, "priority": priority })
            }
            CoreEvent::PanelModeChanged { mode } => json!({ "mode": mode }),
            CoreEvent::MusicTick { position_ms, duration_ms } => {
                json!({ "positionMs": position_ms, "durationMs": duration_ms })
            }
            CoreEvent::ProbeRecorded { process, title } => {
                json!({ "process": process, "title": title })
            }
            CoreEvent::FocusStateChanged { state, completed } => {
                json!({ "state": state, "completed": completed })
            }
            CoreEvent::SupervisionAlert { rule, app, level, text } => {
                json!({ "rule": rule, "app": app, "level": level, "text": text })
            }
            CoreEvent::WorkflowRunChanged { workflow_id, run_id, status, error } => {
                json!({ "workflowId": workflow_id, "runId": run_id, "status": status, "error": error })
            }
        }
    }
}


/// Forwards every core event to all windows through Tauri's event system.
pub async fn relay_task(
    app: tauri::AppHandle,
    mut rx: tokio::sync::broadcast::Receiver<CoreEvent>,
) {
    use tauri::Emitter;
    loop {
        match rx.recv().await {
            Ok(event) => {
                let _ = app.emit(event.event_name(), event.payload());
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}