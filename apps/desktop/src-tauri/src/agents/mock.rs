//! MockProvider: a scripted agent that, on start_thread/send, publishes a
//! schema-valid AgentEvent v1 sequence (thinking -> reading -> editing ->
//! success, with an error path on a cadence) plus derived pet/bubble events.
//! Used when `agentProvider=mock` or as fallback when Codex is unavailable
//! (ADR-0007).

use crate::agents::{AgentProvider, AgentThreadInfo};
use crate::event_bus::CoreEvent;
use serde_json::{json, Value};
use std::sync::Mutex;
use tokio::sync::broadcast::Sender;

pub const AGENT_ID: &str = "mock-opencode";
pub const SESSION_ID: &str = "sess-001";

pub fn state_to_animation(state: &str) -> &'static str {
    match state {
        "thinking" | "reading" | "searching" => "thinking",
        "editing" | "running" | "testing" => "editing",
        "waiting_permission" | "waiting_user" => "waiting",
        "success" => "success",
        "error" | "warning" => "error",
        "cancelled" | "offline" | "idle" => "idle",
        _ => "idle",
    }
}

#[cfg(test)]
fn envelope(event: Value) -> Value {
    envelope_session(SESSION_ID, event)
}

fn envelope_session(session_id: &str, event: Value) -> Value {
    json!({
        "schemaVersion": 1,
        "agentId": AGENT_ID,
        "sessionId": session_id,
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "event": event,
    })
}

struct Step {
    event: Value,
    state: Option<&'static str>,
    bubble: Option<(&'static str, &'static str)>, // (text, priority)
}

fn success_cycle() -> Vec<Step> {
    vec![
        Step { event: json!({"type":"session.started"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"thinking"}), state: Some("thinking"), bubble: None },
        Step { event: json!({"type":"tool.started","tool":"read_file","inputSummary":"读取 tasks/today.md"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"reading"}), state: Some("reading"), bubble: None },
        Step { event: json!({"type":"tool.completed","tool":"read_file","resultSummary":"3 个任务项"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"editing"}), state: Some("editing"), bubble: None },
        Step { event: json!({"type":"message.delta","text":"正在修改今天的任务文件…"}), state: None, bubble: None },
        Step { event: json!({"type":"message.delta","text":"还剩一个待办项。"}), state: None, bubble: None },
        Step { event: json!({"type":"message.completed","text":"已完成修改，等待确认。"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"success"}), state: Some("success"), bubble: Some(("修改完成，去看看 Diff 吧。","normal")) },
        Step { event: json!({"type":"message.completed","text":"修改完成，去看看 Diff 吧。"}), state: None, bubble: None },
        Step { event: json!({"type":"session.completed","outcome":"success"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"idle"}), state: Some("idle"), bubble: None },
    ]
}

#[cfg(test)]
fn error_cycle() -> Vec<Step> {
    vec![
        Step { event: json!({"type":"session.started"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"thinking"}), state: Some("thinking"), bubble: None },
        Step { event: json!({"type":"status.changed","state":"error"}), state: Some("error"), bubble: Some(("Agent 出错了，已停止。","critical")) },
        Step { event: json!({"type":"session.error","message":"provider timeout"}), state: None, bubble: None },
        Step { event: json!({"type":"session.completed","outcome":"error"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"idle"}), state: Some("idle"), bubble: None },
    ]
}

fn reply_cycle(text: &str) -> Vec<Step> {
    vec![
        Step { event: json!({"type":"status.changed","state":"thinking"}), state: Some("thinking"), bubble: None },
        Step { event: json!({"type":"message.delta","text": format!("（Mock 回话）收到：{text}\n")}), state: None, bubble: None },
        Step { event: json!({"type":"message.delta","text":"这是脚本化演示回复，不会真正执行任务。"}), state: None, bubble: None },
        Step { event: json!({"type":"message.completed","text":"这是脚本化演示回复，不会真正执行任务。"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"success"}), state: Some("success"), bubble: None },
        Step { event: json!({"type":"session.completed","outcome":"success"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"idle"}), state: Some("idle"), bubble: None },
    ]
}

/// Scripted fallback provider implementing the AgentProvider contract.
pub struct MockProvider {
    tx: Sender<CoreEvent>,
    session_id: Mutex<String>,
}

impl MockProvider {
    pub fn new(tx: Sender<CoreEvent>) -> Self {
        Self { tx, session_id: Mutex::new(SESSION_ID.to_string()) }
    }

    fn emit(&self, steps: Vec<Step>) {
        let tx = self.tx.clone();
        let session_id = self.session_id.lock().unwrap().clone();
        tauri::async_runtime::spawn(async move {
            for step in steps {
                let _ = tx.send(CoreEvent::AgentEvent(envelope_session(&session_id, step.event)));
                if let Some(state) = step.state {
                    let _ = tx.send(CoreEvent::PetStateChanged {
                        state: state.to_string(),
                        animation: state_to_animation(state).to_string(),
                    });
                }
                if let Some((text, priority)) = step.bubble {
                    let _ = tx.send(CoreEvent::BubbleRequested {
                        text: text.to_string(),
                        priority: priority.to_string(),
                    });
                }
                tokio::time::sleep(std::time::Duration::from_millis(600)).await;
            }
        });
    }

    fn current_info(&self, workspace_dir: &str) -> AgentThreadInfo {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        AgentThreadInfo {
            id: self.session_id.lock().unwrap().clone(),
            preview: "Mock 会话".to_string(),
            cwd: workspace_dir.to_string(),
            status: "idle".to_string(),
            updated_at: now,
        }
    }
}

impl AgentProvider for MockProvider {
    fn start_thread(
        &mut self,
        workspace_dir: &str,
        initial_message: &str,
    ) -> Result<AgentThreadInfo, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        *self.session_id.lock().unwrap() = format!("mock-{now}");
        self.emit(if initial_message.trim().is_empty() {
            success_cycle()
        } else {
            reply_cycle(initial_message)
        });
        Ok(self.current_info(workspace_dir))
    }

    fn resume_thread(&mut self, thread_id: &str) -> Result<AgentThreadInfo, String> {
        *self.session_id.lock().unwrap() = thread_id.to_string();
        self.emit(success_cycle());
        Ok(self.current_info(""))
    }

    fn list_threads(&mut self) -> Result<Vec<AgentThreadInfo>, String> {
        Ok(Vec::new())
    }

    fn send(&mut self, _thread_id: &str, text: &str) -> Result<(), String> {
        self.emit(reply_cycle(text));
        Ok(())
    }

    fn interrupt(&mut self, _thread_id: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::validate_envelope;

    #[test]
    fn emitted_samples_are_schema_valid() {
        let mut steps = success_cycle();
        steps.extend(error_cycle());
        assert!(!steps.is_empty());
        for step in &steps {
            validate_envelope(&envelope(step.event.clone()))
                .unwrap_or_else(|e| panic!("mock event not schema-valid: {e}"));
        }
    }

    #[test]
    fn all_eleven_event_kinds_validate() {
        let kinds = vec![
            json!({"type":"session.started"}),
            json!({"type":"message.delta","text":"x"}),
            json!({"type":"message.completed","text":"x"}),
            json!({"type":"tool.started","tool":"t","inputSummary":"s"}),
            json!({"type":"tool.completed","tool":"t","resultSummary":"s"}),
            json!({"type":"file.read","path":"p"}),
            json!({"type":"file.changed","path":"p","diffId":"d"}),
            json!({"type":"permission.requested","requestId":"r","risk":"medium"}),
            json!({"type":"status.changed","state":"thinking"}),
            json!({"type":"session.completed","outcome":"ok"}),
            json!({"type":"session.error","message":"m"}),
        ];
        for ev in kinds {
            validate_envelope(&envelope(ev)).unwrap_or_else(|e| panic!("event not schema-valid: {e}"));
        }
    }

    #[test]
    fn invalid_envelope_rejected() {
        let bad = json!({
            "schemaVersion": 1,
            "agentId": "a",
            "sessionId": "s",
            "timestamp": "2026-01-01T00:00:00Z",
            "event": { "type": "status.changed", "state": "sleeping" }
        });
        assert!(validate_envelope(&bad).is_err(), "invalid state must be rejected");
    }
}