//! v1.5 Agent CLI control plane: a localhost TCP server (127.0.0.1, ephemeral
//! port + per-run token published to `app_data_dir/cli.json`) exposing app
//! capabilities to the future Agent (and humans) through the `focus-cli`
//! binary. Transport is std TCP (no new dependencies); every message is JSON
//! framed with a 4-byte little-endian length prefix. The plan called for a
//! named pipe; std `named_pipe` is not available in this toolchain and the
//! windows-crate pipe plumbing would add features + unsafe, so localhost TCP
//! with a token file was chosen instead (ADR-0006).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::storage::Store;
use crate::AppState;

pub const CLI_FILE: &str = "cli.json";
const FRAME_MAX: usize = 1 << 20;

fn read_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > FRAME_MAX {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad frame len"));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

fn write_frame(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    stream.write_all(&(data.len() as u32).to_le_bytes())?;
    stream.write_all(data)?;
    stream.flush()
}

/// Per-run token (xorshift-ish from time + pid; not cryptographic — it only
/// stops casual same-machine callers, the socket is bound to 127.0.0.1).
fn token() -> String {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
        ^ std::process::id() as u64;
    format!("{:016x}", seed.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Forward a timer action to the desktop webview and wait (<=3s) for its
/// `cli:timer-done` reply carrying the live timer state.
fn timer_roundtrip(app: &AppHandle, action: &str) -> Value {
    let state = app.state::<AppState>();
    let id = state.cli_next_id.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = std::sync::mpsc::channel::<Value>();
    state.cli_pending.lock().unwrap().insert(id, tx);
    let _ = app.emit("cli:timer", json!({ "id": id, "action": action }));
    match rx.recv_timeout(Duration::from_secs(3)) {
        Ok(v) => v,
        Err(_) => json!({ "error": "timer no response (desktop window not ready?)", "action": action }),
    }
}

fn shortcuts_json(_app: &AppHandle, store: &Arc<Mutex<Store>>) -> Vec<Value> {
    let Ok(s) = store.lock() else { return vec![] };
    let Ok(rows) = s.list_shortcuts() else { return vec![] };
    rows.iter()
        .map(|r| {
            json!({
                "id": r.id, "name": r.name, "type": r.kind, "target": r.target,
                "col": r.col, "row": r.row, "windowFit": "grid",
                "fitCol": r.fit_col, "fitRow": r.fit_row, "fitCols": r.fit_cols, "fitRows": r.fit_rows,
            })
        })
        .collect()
}

fn handle_request(app: &AppHandle, store: &Arc<Mutex<Store>>, req: &Value) -> Value {
    let token_ok = req
        .get("token")
        .and_then(|t| t.as_str())
        .map(|t| t == app.state::<AppState>().cli_token.lock().unwrap().as_str())
        .unwrap_or(false);
    if !token_ok {
        return json!({ "error": "bad token" });
    }
    let cmd = req.get("cmd").and_then(|c| c.as_str()).unwrap_or("").to_string();
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let agent_thread = req.get("agentThread").and_then(|v| v.as_str()).map(str::to_string);
    if agent_thread.is_some() && !agent_whitelisted(&parts) {
        let denied = json!({ "error": "agent CLI denied: command not in whitelist", "command": cmd });
        audit_agent_call(store, agent_thread.as_deref().unwrap_or(""), &cmd, false, &denied);
        return denied;
    }
    let resp = match parts.as_slice() {
        ["ping"] => json!({ "pong": true }),
        ["debug", "windows"] => {
            let app_state = app.state::<AppState>();
            let settings = app_state.settings.lock().unwrap();
            let collapsed = settings.collapsed.clone();
            let grid = settings.grid.clone();
            drop(settings);
            let mut wins = Vec::new();
            for label in ["chat", "stats", "music", "pet"] {
                let visible = app
                    .get_webview_window(label)
                    .and_then(|w| w.is_visible().ok())
                    .unwrap_or(false);
                wins.push(json!({
                    "label": label,
                    "visible": visible,
                    "collapsed": collapsed.contains(&label.to_string()),
                }));
            }
            let topbar_visible = app
                .get_webview_window("topbar")
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false);
            let active_drag = app_state
                .active_drag
                .lock()
                .unwrap()
                .as_ref()
                .map(|d| d.label.clone());
            json!({
                "windows": wins,
                "topbarVisible": topbar_visible,
                "activeDrag": active_drag,
                "grid": grid,
            })
        }
        ["timer", action] if ["start", "pause", "skip", "status"].contains(action) => {
            timer_roundtrip(app, action)
        }
        ["stats", "today"] => match store.lock() {
            Ok(s) => match s.today_focus_summary() {
                Ok((sec, rounds)) => json!({ "totalSec": sec, "rounds": rounds }),
                Err(e) => json!({ "error": e.to_string() }),
            },
            Err(_) => json!({ "error": "store locked" }),
        },
        ["stats", "week"] => match store.lock() {
            Ok(s) => match s.week_focus_summary() {
                Ok(days) => json!({
                    "days": days.into_iter().map(|(d, sec)| json!({ "date": d, "totalSec": sec })).collect::<Vec<_>>()
                }),
                Err(e) => json!({ "error": e.to_string() }),
            },
            Err(_) => json!({ "error": "store locked" }),
        },
        ["stats", "sessions"] => match store.lock() {
            Ok(s) => match s.recent_sessions(20) {
                Ok(rows) => json!({
                    "sessions": rows.iter().map(|r| json!({
                        "id": r.id, "startedAt": r.started_at, "endedAt": r.ended_at,
                        "durationSec": r.duration_sec, "taskId": r.task_id,
                    })).collect::<Vec<_>>()
                }),
                Err(e) => json!({ "error": e.to_string() }),
            },
            Err(_) => json!({ "error": "store locked" }),
        },
        ["stats", "dashboard"] => match store.lock() {
            Ok(s) => match s.dashboard() {
                Ok(d) => serde_json::to_value(d).unwrap_or_else(|e| json!({ "error": e.to_string() })),
                Err(e) => json!({ "error": e.to_string() }),
            },
            Err(_) => json!({ "error": "store locked" }),
        },
        ["desktop", "layout"] => {
            let app_state = app.state::<AppState>();
            let settings = app_state.settings.lock().unwrap();
            json!({
                "grid": settings.grid,
                "collapsed": settings.collapsed,
                "shortcuts": shortcuts_json(app, store),
            })
        }
        // v1.12: desktop lock/unlock (escape hatch — TCP cannot be blocked by
        // the keyboard hook).
        ["desktop", "lock"] => match crate::desktop_lock::lock_desktop() {
            Ok(()) => json!({ "locked": true }),
            Err(e) => json!({ "error": e }),
        },
        ["desktop", "unlock"] => match crate::desktop_lock::unlock_desktop() {
            Ok(()) => json!({ "locked": false }),
            Err(e) => json!({ "error": e }),
        },
        ["desktop", "status"] => json!({ "locked": crate::desktop_lock::is_locked() }),
        ["apps", "now"] => match crate::activity::probe_foreground() {
            Some(f) => json!({ "process": f.process, "title": f.title }),
            None => json!({ "process": null, "title": null }),
        },
        ["apps", "visible"] => json!({ "apps": crate::apps::list_running_apps() }),
        // M4 workflow engine (ADR-0012): local control only; workflow commands
        // are intentionally NOT in the agent whitelist (anti-loop rule).
        // v1.11 (ADR-0020): Agent is the Boss — workflow CRUD moved into the
        // whitelist so Agents can schedule themselves (whitelist below).
        ["workflow", ..] => match crate::workflow::cli_handle(&app, &parts, req.get("payload")) {
            Ok(v) => v,
            Err(e) => json!({ "error": e }),
        },
        // M5 (ADR-0022): Agent-facing session info — the Agent reads its own
        // current session hash to revisit history as context.
        ["agent", "session", agent_id] => {
            let state = app.state::<AppState>();
            let store = state.store.clone();
            let Ok(s) = store.lock() else { return json!({ "error": "store locked" }) };
            match s.get_character(agent_id) {
                Ok(Some(c)) => json!({
                    "agentId": c.id,
                    "sessionHash": c.current_session_hash,
                    "sessionDate": c.session_date,
                    "workspaceDir": c.workspace_dir,
                    "name": c.name,
                    "tool": c.tool,
                }),
                Ok(None) => json!({ "error": format!("角色 {agent_id} 不存在") }),
                Err(e) => json!({ "error": e.to_string() }),
            }
        }
        ["agent", "list"] => {
            let state = app.state::<AppState>();
            let store = state.store.clone();
            let Ok(s) = store.lock() else { return json!({ "error": "store locked" }) };
            match s.list_characters() {
                Ok(chars) => json!({
                    "agents": chars.into_iter().map(|c| serde_json::json!({
                        "agentId": c.id,
                        "name": c.name,
                        "tool": c.tool,
                        "workspaceDir": c.workspace_dir,
                        "sessionHash": c.current_session_hash,
                        "sessionDate": c.session_date,
                    })).collect::<Vec<_>>()
                }),
                Err(e) => json!({ "error": e.to_string() }),
            }
        }
        _ => json!({ "error": format!("unknown command: {cmd}") }),
    };
    if let Some(tid) = agent_thread {
        audit_agent_call(store, &tid, &cmd, true, &resp);
    }
    resp
}


/// Whitelist enforced only for agent-triggered calls (ADR-0007): exactly the
/// ADR-0006 command set. `debug` and any future/unknown command are denied.
/// v1.11 (ADR-0020): `workflow *` is allowed — the Agent manages its own
/// schedule board (list/read/create/update/delete/run/runs/cancel).
fn agent_whitelisted(parts: &[&str]) -> bool {
    match parts {
        ["ping"] => true,
        ["timer", a] if ["start", "pause", "skip", "status"].contains(a) => true,
        ["stats", "today"] | ["stats", "week"] | ["stats", "sessions"] | ["stats", "dashboard"] => true,
        ["desktop", "layout"] => true,
        // v1.12: desktop lock/unlock/status — Agent can also lock/unlock.
        ["desktop", "lock"] | ["desktop", "unlock"] | ["desktop", "status"] => true,
        ["apps", "now"] | ["apps", "visible"] => true,
        ["workflow", sub, ..] if ["list", "read", "create", "update", "delete", "run", "runs", "cancel"].contains(sub) => true,
        // M5 (ADR-0022): Agent reads its own session hash / agent list.
        ["agent", "session", _] | ["agent", "list"] => true,
        _ => false,
    }
}

fn audit_agent_call(
    store: &Arc<Mutex<Store>>,
    thread_id: &str,
    command: &str,
    allowed: bool,
    result: &Value,
) {
    if let Ok(s) = store.lock() {
        let _ = s.record_agent_cli_call(thread_id, command, allowed, &result.to_string());
    }
}

fn handle_conn(app: AppHandle, store: Arc<Mutex<Store>>, mut stream: TcpStream) {
    let result = (|| -> std::io::Result<()> {
        let buf = read_frame(&mut stream)?;
        let req: Value = serde_json::from_slice(&buf).unwrap_or(Value::Null);
        let resp = handle_request(&app, &store, &req);
        let data = serde_json::to_vec(&resp).unwrap_or_default();
        write_frame(&mut stream, &data)
    })();
    if let Err(e) = result {
        eprintln!("[cli] connection error: {e}");
    }
}

/// Bind the local control server and publish `cli.json` with port + token.
pub fn spawn(app: AppHandle, store: Arc<Mutex<Store>>, data_dir: PathBuf) {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[cli] bind failed: {e}");
            return;
        }
    };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    let tok = token();
    let _ = std::fs::write(
        data_dir.join(CLI_FILE),
        json!({ "port": port, "token": tok }).to_string(),
    );
    *app.state::<AppState>().cli_token.lock().unwrap() = tok;
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let app2 = app.clone();
            let store2 = store.clone();
            std::thread::spawn(move || handle_conn(app2, store2, stream));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn frame_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let buf = read_frame(&mut stream).unwrap();
            write_frame(&mut stream, &buf).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        write_frame(&mut client, b"hello-frame").unwrap();
        let mut len_buf = [0u8; 4];
        client.read_exact(&mut len_buf).unwrap();
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        client.read_exact(&mut buf).unwrap();
        assert_eq!(buf, b"hello-frame");
        server.join().unwrap();
    }

    #[test]
    fn agent_whitelist_rules() {
        assert!(agent_whitelisted(&["ping"]));
        assert!(agent_whitelisted(&["timer", "status"]));
        assert!(agent_whitelisted(&["timer", "start"]));
        assert!(agent_whitelisted(&["stats", "today"]));
        assert!(agent_whitelisted(&["stats", "dashboard"]));
        assert!(agent_whitelisted(&["desktop", "layout"]));
        assert!(agent_whitelisted(&["desktop", "lock"]));
        assert!(agent_whitelisted(&["desktop", "unlock"]));
        assert!(agent_whitelisted(&["desktop", "status"]));
        assert!(agent_whitelisted(&["apps", "visible"]));
        // v1.11 (ADR-0020): Agent is the Boss — workflow CRUD allowed.
        assert!(agent_whitelisted(&["workflow", "list"]));
        assert!(agent_whitelisted(&["workflow", "read", "w-1"]));
        assert!(agent_whitelisted(&["workflow", "create"]));
        assert!(agent_whitelisted(&["workflow", "update", "w-1"]));
        assert!(agent_whitelisted(&["workflow", "delete", "w-1"]));
        assert!(agent_whitelisted(&["workflow", "run", "w-1"]));
        assert!(agent_whitelisted(&["workflow", "runs", "w-1"]));
        assert!(agent_whitelisted(&["workflow", "cancel", "w-1"]));
        // M5 (ADR-0022): agent session/list allowed.
        assert!(agent_whitelisted(&["agent", "session", "char-1"]));
        assert!(agent_whitelisted(&["agent", "list"]));
        assert!(!agent_whitelisted(&["agent", "other"]));
        assert!(!agent_whitelisted(&["debug", "windows"]));
        assert!(!agent_whitelisted(&["timer", "reset"]));
        assert!(!agent_whitelisted(&["stats", "month"]));
        assert!(!agent_whitelisted(&["unknown"]));
    }

}
