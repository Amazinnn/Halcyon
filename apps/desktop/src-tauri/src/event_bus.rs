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
    BubbleRequested {
        text: String,
        priority: String,
        agent_id: Option<String>,
    },
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
    /// Workflow node system action (focus/idle/ring) for the frontend timer/sound (v1.10.4/ADR-0017).
    WorkflowSystemAction { action: String, seconds: i64 },
    /// A workflow run changed (workflow window refresh, M4/ADR-0012).
    WorkflowRunChanged {
        workflow_id: String,
        run_id: String,
        status: String,
        error: Option<String>,
    },
    /// v1.11 (ADR-0020): a workflow was created/updated/deleted (via UI or
    /// Agent CLI). Frontend reloads the workflow list; M5 listens.
    WorkflowChanged {
        action: String,
        workflow_id: String,
    },
    /// Internal workflow result channel. This is deliberately separate from
    /// the externally versioned AgentEvent schema.
    WorkflowAgentResult {
        workflow_id: String,
        workflow_name: String,
        agent_id: String,
        text: String,
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
            CoreEvent::WorkflowSystemAction { .. } => "workflow:system-action",
            CoreEvent::WorkflowRunChanged { .. } => "workflow:runs_changed",
            CoreEvent::WorkflowChanged { .. } => "workflow:changed",
            CoreEvent::WorkflowAgentResult { .. } => "workflow:agent_result",
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            CoreEvent::AgentEvent(v) => v.clone(),
            CoreEvent::PetStateChanged { state, animation } => {
                json!({ "state": state, "animation": animation })
            }
            CoreEvent::BubbleRequested {
                text,
                priority,
                agent_id,
            } => {
                let mut payload = json!({ "text": text, "priority": priority });
                if let Some(agent_id) = agent_id {
                    payload["agentId"] = json!(agent_id);
                }
                payload
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
            CoreEvent::WorkflowSystemAction { action, seconds } => {
                json!({ "action": action, "seconds": seconds })
            }
            CoreEvent::WorkflowRunChanged { workflow_id, run_id, status, error } => {
                json!({ "workflowId": workflow_id, "runId": run_id, "status": status, "error": error })
            }
            CoreEvent::WorkflowChanged { action, workflow_id } => {
                json!({ "action": action, "workflowId": workflow_id })
            }
            CoreEvent::WorkflowAgentResult {
                workflow_id,
                workflow_name,
                agent_id,
                text,
            } => json!({
                "workflowId": workflow_id,
                "workflowName": workflow_name,
                "agentId": agent_id,
                "text": text,
            }),
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

#[cfg(test)]
mod tests {
    use super::CoreEvent;
    use serde_json::json;

    #[test]
    fn workflow_agent_result_has_internal_name_and_target_agent_payload() {
        let event = CoreEvent::WorkflowAgentResult {
            workflow_id: "wf-1".into(),
            workflow_name: "Morning".into(),
            agent_id: "char-b".into(),
            text: "done".into(),
        };

        assert_eq!(event.event_name(), "workflow:agent_result");
        assert_eq!(
            event.payload(),
            json!({
                "workflowId": "wf-1",
                "workflowName": "Morning",
                "agentId": "char-b",
                "text": "done",
            })
        );
    }

    #[test]
    fn targeted_bubble_keeps_its_agent_id() {
        let event = CoreEvent::BubbleRequested {
            text: "done".into(),
            priority: "normal".into(),
            agent_id: Some("char-b".into()),
        };

        assert_eq!(
            event.payload(),
            json!({ "text": "done", "priority": "normal", "agentId": "char-b" })
        );
    }
}
