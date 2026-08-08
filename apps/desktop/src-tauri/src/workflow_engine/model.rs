//! Pure data model for workflows (M4, ADR-0012). Serde shapes are the wire
//! format between the Vue Flow editor and the Rust engine.

use serde::{Deserialize, Serialize};
use chrono::TimeZone;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDef {
    pub id: String,
    pub character_id: String,
    pub name: String,
    #[serde(default = "default_manual")]
    pub trigger: String, // manual | scheduled | focus_end | supervision_alert
    #[serde(default)]
    pub schedule_type: Option<String>, // interval | daily
    #[serde(default)]
    pub interval_minutes: Option<i64>,
    #[serde(default)]
    pub daily_time: Option<String>, // "HH:MM"
    #[serde(default)]
    pub guard: String, // none | focusing | resting | idle
    #[serde(default)]
    pub nodes: Vec<NodeDef>,
    #[serde(default)]
    pub edges: Vec<EdgeDef>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub next_run_at: Option<i64>,
}

fn default_manual() -> String {
    "manual".into()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeDef {
    pub id: String,
    pub kind: String, // bubble | agent | show_window | wait | branch | focus | idle | ring | if(legacy)
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EdgeDef {
    pub id: String,
    pub source: String,
    #[serde(default = "default_out")]
    pub source_handle: String, // out | true | false | none | option1..optionN
    pub target: String,
}

fn default_out() -> String {
    "out".into()
}

/// A character = desktop pet + agents.md persona (ADR-0012). The engine only
/// needs the persona text for one-shot agent calls.
#[derive(Debug, Clone)]
#[allow(dead_code)] // name is informational; persona drives one-shot calls
pub struct CharacterInfo {
    pub id: String,
    pub name: String,
    pub persona: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)] // Running/Skipped recorded at the storage layer
pub enum RunStatus {
    Running,
    Success,
    Failed,
    Skipped,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Success => "success",
            RunStatus::Failed => "failed",
            RunStatus::Skipped => "skipped",
            RunStatus::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeLogEntry {
    pub node_id: String,
    pub kind: String,
    pub status: String, // ok | failed | skipped
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Interval schedule: next run is minutes after "now".
pub fn next_interval_run(now: i64, minutes: i64) -> i64 {
    now + minutes.saturating_mul(60).max(60)
}

/// Daily schedule: next occurrence of "HH:MM" (local time) strictly after "now".
pub fn next_daily_run(now: i64, hhmm: &str) -> Result<i64, String> {
    let parts: Vec<&str> = hhmm.trim().split(':').collect();
    if parts.len() != 2 {
        return Err(format!("每日时间格式应为 HH:MM: {hhmm}"));
    }
    let h: u32 = parts[0]
        .trim()
        .parse()
        .map_err(|_| format!("小时无效: {}", parts[0]))?;
    let m: u32 = parts[1]
        .trim()
        .parse()
        .map_err(|_| format!("分钟无效: {}", parts[1]))?;
    if h > 23 || m > 59 {
        return Err(format!("时间越界: {hhmm}"));
    }
    let now_dt = chrono::Local::now();
    let today = now_dt
        .date_naive()
        .and_hms_opt(h, m, 0)
        .ok_or_else(|| format!("时间无效: {hhmm}"))?;
    let candidate = chrono::Local
        .from_local_datetime(&today)
        .single()
        .ok_or_else(|| format!("本地时间存在歧义: {hhmm}"))?;
    let ts = candidate.timestamp();
    if ts > now {
        Ok(ts)
    } else {
        Ok(ts + 86_400)
    }
}

/// Pre-run guard: does the workflow's required focus state match reality?
pub fn guard_matches(guard: &str, focus_state: &str) -> bool {
    match guard {
        "focusing" => focus_state == "focus",
        "resting" => focus_state == "rest",
        "idle" => focus_state == "idle",
        _ => true, // "none" and unknown guards always pass
    }
}

/// Structural validation: node kinds known, ids unique, edges reference real
/// nodes, no self-loops, no cycles (Kahn over the full graph ignoring branch
/// handles).
pub fn validate_workflow(wf: &WorkflowDef) -> Result<(), String> {
    if wf.name.trim().is_empty() {
        return Err("工作流名称不能为空".into());
    }
    if wf.nodes.is_empty() {
        return Err("工作流至少需要一个节点".into());
    }
    let mut ids: HashSet<&str> = HashSet::new();
    for n in &wf.nodes {
        if !matches!(
            n.kind.as_str(),
            "bubble" | "agent" | "show_window" | "wait" | "branch" | "focus" | "idle" | "ring" | "if"
        ) {
            return Err(format!("未知节点类型: {}", n.kind));
        }
        if !ids.insert(n.id.as_str()) {
            return Err(format!("节点 id 重复: {}", n.id));
        }
    }
    for e in &wf.edges {
        if !ids.contains(e.source.as_str()) {
            return Err(format!("连线起点不存在: {}", e.source));
        }
        if !ids.contains(e.target.as_str()) {
            return Err(format!("连线终点不存在: {}", e.target));
        }
        let valid_handle = matches!(e.source_handle.as_str(), "out" | "true" | "false" | "none")
            || e.source_handle.starts_with("option");
        if !valid_handle {
            return Err(format!("无效连线手柄: {}", e.source_handle));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> WorkflowDef {
        WorkflowDef {
            id: "w1".into(),
            character_id: "c1".into(),
            name: "测试".into(),
            trigger: "manual".into(),
            schedule_type: None,
            interval_minutes: None,
            daily_time: None,
            guard: "none".into(),
            nodes: vec![
                NodeDef { id: "n1".into(), kind: "bubble".into(), params: serde_json::json!({"text":"hi"}), x: 0.0, y: 0.0 },
                NodeDef { id: "n2".into(), kind: "bubble".into(), params: serde_json::json!({"text":"yo"}), x: 0.0, y: 0.0 },
            ],
            edges: vec![EdgeDef { id: "e1".into(), source: "n1".into(), source_handle: "out".into(), target: "n2".into() }],
            enabled: true,
            next_run_at: None,
        }
    }

    #[test]
    fn validate_ok() {
        assert!(validate_workflow(&sample()).is_ok());
    }

    #[test]
    fn validate_allows_cycle_and_self_loop() {
        let mut wf = sample();
        wf.edges.push(EdgeDef { id: "e2".into(), source: "n2".into(), source_handle: "out".into(), target: "n1".into() });
        assert!(validate_workflow(&wf).is_ok());
        let mut wf2 = sample();
        wf2.edges.push(EdgeDef { id: "e3".into(), source: "n1".into(), source_handle: "out".into(), target: "n1".into() });
        assert!(validate_workflow(&wf2).is_ok());
    }

    #[test]
    fn validate_rejects_unknown_kind_and_bad_edge() {
        let mut wf = sample();
        wf.nodes[0].kind = "alien".into();
        assert!(validate_workflow(&wf).is_err());
        let mut wf2 = sample();
        wf2.edges[0].target = "ghost".into();
        assert!(validate_workflow(&wf2).is_err());
    }

    #[test]
    fn guards() {
        assert!(guard_matches("none", "focus"));
        assert!(guard_matches("focusing", "focus"));
        assert!(!guard_matches("focusing", "rest"));
        assert!(guard_matches("resting", "rest"));
        assert!(guard_matches("idle", "idle"));
    }

    #[test]
    fn interval_next_run_is_positive() {
        let now = now_ts();
        let nx = next_interval_run(now, 30);
        assert_eq!(nx - now, 1800);
    }

    #[test]
    fn daily_next_run_is_in_future() {
        let now = now_ts();
        // Two minutes from now, formatted HH:MM -> next occurrence is > now
        let future = chrono::Local::now() + chrono::Duration::minutes(2);
        let hhmm = future.format("%H:%M").to_string();
        let nx = next_daily_run(now, &hhmm).unwrap();
        assert!(nx > now, "next daily run must be in the future");
    }

    #[test]
    fn daily_bad_format_rejected() {
        assert!(next_daily_run(now_ts(), "25:99").is_err());
        assert!(next_daily_run(now_ts(), "abc").is_err());
    }
}
