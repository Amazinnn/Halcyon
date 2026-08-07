//! v1.4 supervision engine (V1 soft limits): local rules detect drift and
//! emit pet-bubble alerts. Runs on a ~2s heartbeat thread; reads settings from
//! AppState, foreground from activity::probe_foreground(), idle from
//! GetLastInputInfo, and writes every alert to the DB.
//!
//! Rules (design doc §11.2/§12.1): distraction timeout (blacklist hit, not
//! whitelisted, >=2min first alert then escalating cooldown 5->3->1->0.5min,
//! 30s switch-away grace), idle (no input >=3min, distraction has priority),
//! task overdue (accumulated task focus > estimate, once per task). Throttle:
//! sliding 60min window <=4 alerts; pause 30min; rest/idle/pause = silent.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::System::SystemInformation::GetTickCount;

use crate::activity;
use crate::event_bus::CoreEvent;
use crate::storage::Store;
use crate::AppState;

pub const TICK_MS: u64 = 2000;
const FIRST_DISTRACTION_SEC: i64 = 120;
const GRACE_SEC: i64 = 30;
const IDLE_SEC: i64 = 180;
const ESCALATION_SEC: [i64; 4] = [300, 180, 60, 30];
const HOURLY_CAP: usize = 4;

/// Frontend focus tracking, shared with the main thread (focus:state_changed
/// listener sets flags; this engine accumulates focus seconds each tick).
#[derive(Default)]
pub struct FocusTrack {
    pub active: bool,
    pub paused: bool,
    pub session_started_at: Option<String>,
    pub session_focus_sec: i64,
    pub task_focus_sec: i64,
    pub task_id: Option<String>,
}

#[derive(Default)]
struct DistractionState {
    since: Option<Instant>,
    last_alert: Option<Instant>,
    alerts: usize,
    last_allowed: Option<Instant>,
}

#[derive(Default)]
struct RuleState {
    dist: DistractionState,
    idle_alerted: bool,
    task_overdue_alerted: bool,
    last_task_id: Option<String>,
    alerts: Vec<Instant>,
}

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Ok,
    Drift,
    Paused,
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Minimal '*'/'?' glob matcher (no regex dependency).
pub fn wildcard_match(pat: &str, text: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(sp) = star {
            pi = sp + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn in_list(list: &[String], process: &str) -> bool {
    let p = process.to_lowercase();
    list.iter().any(|pat| wildcard_match(&pat.to_lowercase(), &p))
}

fn cooldown_sec(alerts: usize) -> i64 {
    if alerts == 0 {
        0
    } else {
        ESCALATION_SEC[(alerts - 1).min(ESCALATION_SEC.len() - 1)]
    }
}

fn distraction_text(alerts: usize) -> String {
    match alerts {
        0 => "好像有点走神了，回到任务上吧".into(),
        1 => "还在分心应用上，记得回来专注".into(),
        2 => "该回任务了，别让分心溜走时间".into(),
        _ => "任务还在等你，先回来完成它".into(),
    }
}

fn idle_seconds() -> i64 {
    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut info).as_bool() {
            (GetTickCount().wrapping_sub(info.dwTime) / 1000) as i64
        } else {
            0
        }
    }
}

/// Emit one alert if the sliding hourly cap allows; returns whether it fired.
fn fire(
    app: &AppHandle,
    store: &Arc<Mutex<Store>>,
    st: &mut RuleState,
    rule: &str,
    app_name: Option<&str>,
    level: i64,
    text: &str,
) -> bool {
    let now = Instant::now();
    st.alerts.retain(|t| now.duration_since(*t).as_secs() < 3600);
    if st.alerts.len() >= HOURLY_CAP {
        return false;
    }
    st.alerts.push(now);
    let _ = app.emit(
        "supervision:alert",
        serde_json::json!({ "rule": rule, "app": app_name, "level": level, "text": text }),
    );
    // M4/ADR-0012: also route through the core event bus so the workflow
    // engine can trigger `supervision_alert` workflows (frontend behavior
    // unchanged — the relay re-emits the same window event).
    let _ = app
        .state::<AppState>()
        .events_tx
        .send(CoreEvent::SupervisionAlert {
            rule: rule.to_string(),
            app: app_name.map(str::to_string),
            level,
            text: text.to_string(),
        });
    if let Ok(s) = store.lock() {
        let _ = s.record_supervision_event(rule, app_name, level);
    }
    true
}

fn tick(app: &AppHandle, store: &Arc<Mutex<Store>>, st: &mut RuleState) -> Status {
    let app_state = app.state::<AppState>();

    // pause / enabled gate
    {
        let mut settings = app_state.settings.lock().unwrap();
        if let Some(pu) = settings.supervision_pause_until {
            if pu > now_unix() {
                return Status::Paused;
            }
            settings.supervision_pause_until = None;
            let _ = settings.save(&app_state.data_dir);
        }
        if !settings.supervision_enabled {
            return Status::Ok;
        }
    }

    // focus accumulation + snapshot
    let (active, paused, task_focus_sec, task_id) = {
        let mut ft = app_state.focus_track.lock().unwrap();
        if ft.active && !ft.paused {
            let add = (TICK_MS / 1000) as i64;
            ft.session_focus_sec += add;
            ft.task_focus_sec += add;
        }
        (ft.active, ft.paused, ft.task_focus_sec, ft.task_id.clone())
    };
    if !active || paused {
        return Status::Ok;
    }

    let settings = app_state.settings.lock().unwrap();
    let foreground = activity::probe_foreground();
    let process = foreground.as_ref().map(|f| f.process.as_str()).unwrap_or("");
    let is_distraction =
        in_list(&settings.distraction_apps, process) && !in_list(&settings.allowed_apps, process);
    let mut status = Status::Ok;

    // ---- distraction ----
    if is_distraction {
        st.dist.last_allowed = None;
        if st.dist.since.is_none() {
            st.dist.since = Some(Instant::now());
            st.dist.alerts = 0;
            st.dist.last_alert = None;
        }
        status = Status::Drift;
    } else if st.dist.since.is_some() {
        match st.dist.last_allowed {
            None => st.dist.last_allowed = Some(Instant::now()),
            Some(t) if t.elapsed().as_secs() as i64 >= GRACE_SEC => {
                st.dist.since = None;
                st.dist.last_alert = None;
                st.dist.alerts = 0;
                st.dist.last_allowed = None;
            }
            Some(_) => status = Status::Drift,
        }
    }
    if let Some(since) = st.dist.since {
        if since.elapsed().as_secs() as i64 >= FIRST_DISTRACTION_SEC {
            let cooldown = cooldown_sec(st.dist.alerts);
            let due = match st.dist.last_alert {
                None => true,
                Some(t) => t.elapsed().as_secs() as i64 >= cooldown,
            };
            if due {
                let level = (st.dist.alerts as i64 + 1).min(4);
                let text = distraction_text(st.dist.alerts);
                if fire(app, store, st, "distraction", Some(process), level, &text) {
                    st.dist.alerts += 1;
                    st.dist.last_alert = Some(Instant::now());
                }
            }
        }
        status = Status::Drift;
    }

    // ---- idle (only when not distracted) ----
    if !is_distraction {
        let idle_sec = idle_seconds();
        if idle_sec >= IDLE_SEC {
            if !st.idle_alerted {
                st.idle_alerted = true;
                fire(
                    app,
                    store,
                    st,
                    "idle",
                    Some(process),
                    1,
                    "似乎离开了一会儿，回来继续专注吧",
                );
            }
        } else {
            st.idle_alerted = false;
        }
    }

    // ---- task overdue ----
    if st.last_task_id.as_deref() != task_id.as_deref() {
        st.last_task_id = task_id.clone();
        st.task_overdue_alerted = false;
        if task_id.is_some() {
            app_state.focus_track.lock().unwrap().task_focus_sec = 0;
        }
    }
    if let Some(t) = settings
        .tasks
        .iter()
        .find(|t| Some(t.id.as_str()) == task_id.as_deref())
    {
        if let Some(est) = t.estimated_minutes {
            if task_focus_sec > (est as i64) * 60 && !st.task_overdue_alerted {
                st.task_overdue_alerted = true;
                fire(
                    app,
                    store,
                    st,
                    "task_overdue",
                    Some(process),
                    1,
                    "当前任务已超过预计时长",
                );
            }
        }
    }

    status
}

/// Spawn the supervision heartbeat thread.
pub fn spawn(app: AppHandle, store: Arc<Mutex<Store>>) {
    std::thread::spawn(move || {
        let mut st = RuleState::default();
        let mut last_status = Status::Ok;
        loop {
            std::thread::sleep(Duration::from_millis(TICK_MS));
            let status = tick(&app, &store, &mut st);
            if status != last_status {
                last_status = status;
                let label = match status {
                    Status::Ok => "ok",
                    Status::Drift => "drift",
                    Status::Paused => "paused",
                };
                let _ = app.emit("supervision:status", serde_json::json!({ "status": label }));
            }
        }
    });
}

/// Pause supervision for `minutes` from now (persisted in settings).
pub fn pause_for(app: &AppHandle, minutes: i64) -> Result<(), String> {
    let app_state = app.state::<AppState>();
    let mut settings = app_state.settings.lock().unwrap();
    settings.supervision_pause_until = Some(now_unix() + minutes.saturating_mul(60));
    let _ = settings.save(&app_state.data_dir);
    Ok(())
}

/// Resume supervision immediately (clear pause).
pub fn resume(app: &AppHandle) -> Result<(), String> {
    let app_state = app.state::<AppState>();
    let mut settings = app_state.settings.lock().unwrap();
    settings.supervision_pause_until = None;
    let _ = settings.save(&app_state.data_dir);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_matches_star_patterns() {
        assert!(wildcard_match("*wechat*", "wechat.exe"));
        assert!(wildcard_match("*.exe", "chrome.exe"));
        assert!(!wildcard_match("chrome", "chrome.exe"));
        assert!(wildcard_match("", ""));
        assert!(wildcard_match("notepad*", "notepad.exe"));
        assert!(!wildcard_match("*game*", "code.exe"));
    }

    #[test]
    fn in_list_case_insensitive() {
        let list = vec!["Chrome.EXE".to_string(), "*game*".to_string()];
        assert!(in_list(&list, "chrome.exe"));
        assert!(in_list(&list, "minesweeper-game.exe"));
        assert!(!in_list(&list, "code.exe"));
    }

    #[test]
    fn escalation_cooldown_ladder() {
        assert_eq!(cooldown_sec(0), 0);
        assert_eq!(cooldown_sec(1), 300);
        assert_eq!(cooldown_sec(2), 180);
        assert_eq!(cooldown_sec(3), 60);
        assert_eq!(cooldown_sec(4), 30);
        assert_eq!(cooldown_sec(99), 30);
    }

    #[test]
    fn distraction_text_has_levels() {
        assert!(!distraction_text(0).is_empty());
        assert!(!distraction_text(3).is_empty());
    }

    #[test]
    fn now_unix_is_positive() {
        assert!(now_unix() > 1_700_000_000);
    }
}
