//! MockAgentAdapter: a scripted agent that every 2s publishes schema-valid
//! AgentEvent envelopes (packages/event-schema) plus derived pet/bubble events.
//! Sequence: thinking -> reading -> editing -> waiting_permission -> success,
//! with an error path every third cycle (per the v0.2 plan task B).

use crate::event_bus::CoreEvent;
use serde_json::{json, Value};
use tokio::sync::broadcast::Sender;

pub const AGENT_ID: &str = "mock-opencode";
pub const SESSION_ID: &str = "sess-001";

/// Embedded AgentEvent schema (v1) so emitted samples can be validated in tests.
#[allow(dead_code)]
pub const SCHEMA_JSON: &str =
    include_str!("../../../../../packages/event-schema/agent-event.schema.json");

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

fn envelope(event: Value) -> Value {
    json!({
        "schemaVersion": 1,
        "agentId": AGENT_ID,
        "sessionId": SESSION_ID,
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
        Step { event: json!({"type":"status.changed","state":"waiting_permission"}), state: Some("waiting_permission"), bubble: Some(("这里需要你确认。","high")) },
        Step { event: json!({"type":"permission.requested","requestId":"req-0001","risk":"medium"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"success"}), state: Some("success"), bubble: Some(("修改完成，去看看 Diff 吧。","normal")) },
        Step { event: json!({"type":"message.completed","text":"修改完成，去看看 Diff 吧。"}), state: None, bubble: None },
        Step { event: json!({"type":"session.completed","outcome":"success"}), state: None, bubble: None },
        Step { event: json!({"type":"status.changed","state":"idle"}), state: Some("idle"), bubble: None },
    ]
}

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

pub fn spawn(tx: Sender<CoreEvent>) {
    tauri::async_runtime::spawn(async move {
        let mut cycle: u32 = 0;
        loop {
            let steps = if cycle >= 2 { error_cycle() } else { success_cycle() };
            for step in steps {
                let _ = tx.send(CoreEvent::AgentEvent(envelope(step.event)));
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
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            cycle = (cycle + 1) % 3;
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lightweight, schema-driven structural validation (stand-in for a full
    /// JSON Schema engine in Rust). Reads `$defs` enum + required fields from
    /// the embedded `agent-event.schema.json` and checks each emitted envelope.
    /// Full validation lives in packages/event-schema (Ajv).
    fn validate_envelope(value: &Value) -> Result<(), String> {
        let schema: Value = serde_json::from_str(SCHEMA_JSON).map_err(|e| e.to_string())?;
        let defs = schema.get("$defs").ok_or("schema has no $defs")?;

        if value.get("schemaVersion") != Some(&json!(1)) {
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

        // Derive allowed states from the embedded schema.
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

        // The event type must map to a $defs kind defined by the schema.
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

        // Required fields per kind, read from the schema.
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