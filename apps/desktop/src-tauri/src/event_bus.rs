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