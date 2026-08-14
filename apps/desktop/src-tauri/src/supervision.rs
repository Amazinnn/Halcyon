//! Blacklist-only foreground reminders. A non-empty blacklist is monitored
//! during active, unpaused focus; a match shows a pet message, plays the
//! existing frontend sound, and preserves the workflow alert event.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::activity;
use crate::event_bus::CoreEvent;
use crate::storage::Store;
use crate::AppState;

pub const TICK_MS: u64 = 2000;
const FIRST_DISTRACTION_SEC: i64 = 120;
const GRACE_SEC: i64 = 30;
const ESCALATION_SEC: [i64; 4] = [300, 180, 60, 30];
const HOURLY_CAP: usize = 4;

#[derive(Default)]
pub struct FocusTrack {
    pub active: bool,
    pub paused: bool,
    pub session_started_at: Option<String>,
    pub session_focus_sec: i64,
}

#[derive(Default)]
struct DistractionState {
    since: Option<Instant>,
    last_alert: Option<Instant>,
    alerts: usize,
    last_clear: Option<Instant>,
}

#[derive(Default)]
struct RuleState {
    dist: DistractionState,
    alerts: Vec<Instant>,
}

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Ok,
    Drift,
}

fn in_list(list: &[String], process: &str) -> bool {
    list.iter().any(|name| name.eq_ignore_ascii_case(process))
}

fn cooldown_sec(alerts: usize) -> i64 {
    if alerts == 0 { 0 } else { ESCALATION_SEC[(alerts - 1).min(ESCALATION_SEC.len() - 1)] }
}

fn distraction_text(alerts: usize) -> String {
    match alerts {
        0 => "It looks like a distraction app is open. Return when ready.".into(),
        1 => "Focus is still waiting for you.".into(),
        2 => "Time to return to the current focus round.".into(),
        _ => "Come back to Focus when you are ready.".into(),
    }
}

fn fire(
    app: &AppHandle,
    store: &Arc<Mutex<Store>>,
    st: &mut RuleState,
    app_name: Option<&str>,
    level: i64,
    text: &str,
) -> bool {
    let now = Instant::now();
    st.alerts.retain(|time| now.duration_since(*time).as_secs() < 3600);
    if st.alerts.len() >= HOURLY_CAP { return false }
    st.alerts.push(now);
    let _ = app.emit(
        "supervision:alert",
        serde_json::json!({ "rule": "distraction", "app": app_name, "level": level, "text": text }),
    );
    let _ = app.state::<AppState>().events_tx.send(CoreEvent::SupervisionAlert {
        rule: "distraction".to_string(),
        app: app_name.map(str::to_string),
        level,
        text: text.to_string(),
    });
    let agent_id = app.state::<AppState>().settings.lock().unwrap().current_agent_id.clone();
    let _ = app.state::<AppState>().events_tx.send(CoreEvent::BubbleRequested {
        text: text.to_string(),
        priority: "high".to_string(),
        agent_id,
        delivery_id: None,
        reliable_delivery: false,
    });
    if let Ok(store) = store.lock() {
        let _ = store.record_supervision_event("distraction", app_name, level);
    }
    true
}

fn tick(app: &AppHandle, store: &Arc<Mutex<Store>>, st: &mut RuleState) -> Status {
    let app_state = app.state::<AppState>();
    let (active, paused) = {
        let mut track = app_state.focus_track.lock().unwrap();
        if track.active && !track.paused { track.session_focus_sec += (TICK_MS / 1000) as i64; }
        (track.active, track.paused)
    };
    if !active || paused { return Status::Ok }

    let blacklist = app_state.settings.lock().unwrap().distraction_apps.clone();
    if blacklist.is_empty() { return Status::Ok }
    let foreground = activity::probe_foreground();
    let process = foreground.as_ref().map(|entry| entry.process.as_str()).unwrap_or("");
    if !in_list(&blacklist, process) {
        if st.dist.since.is_some() {
            match st.dist.last_clear {
                None => st.dist.last_clear = Some(Instant::now()),
                Some(clear) if clear.elapsed().as_secs() as i64 >= GRACE_SEC => st.dist = DistractionState::default(),
                Some(_) => return Status::Drift,
            }
        }
        return Status::Ok;
    }

    st.dist.last_clear = None;
    if st.dist.since.is_none() { st.dist.since = Some(Instant::now()); }
    let since = st.dist.since.expect("set above");
    if since.elapsed().as_secs() as i64 >= FIRST_DISTRACTION_SEC {
        let due = st.dist.last_alert.map(|time| time.elapsed().as_secs() as i64 >= cooldown_sec(st.dist.alerts)).unwrap_or(true);
        if due {
            let level = (st.dist.alerts as i64 + 1).min(4);
            let text = distraction_text(st.dist.alerts);
            if fire(app, store, st, Some(process), level, &text) {
                st.dist.alerts += 1;
                st.dist.last_alert = Some(Instant::now());
            }
        }
    }
    Status::Drift
}

pub fn spawn(app: AppHandle, store: Arc<Mutex<Store>>) {
    std::thread::spawn(move || {
        let mut state = RuleState::default();
        let mut previous = Status::Ok;
        loop {
            std::thread::sleep(Duration::from_millis(TICK_MS));
            let status = tick(&app, &store, &mut state);
            if status != previous {
                previous = status;
                let label = match status { Status::Ok => "ok", Status::Drift => "drift" };
                let _ = app.emit("supervision:status", serde_json::json!({ "status": label }));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blacklist_matching_is_exact_and_case_insensitive() {
        assert!(in_list(&["Chrome.EXE".to_string()], "chrome.exe"));
        assert!(!in_list(&["Chrome.EXE".to_string()], "chrome-helper.exe"));
    }

    #[test]
    fn cooldown_ladder_is_preserved_for_blacklist_alerts() {
        assert_eq!(cooldown_sec(0), 0);
        assert_eq!(cooldown_sec(1), 300);
        assert_eq!(cooldown_sec(4), 30);
    }
}
