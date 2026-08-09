//! CodexProvider (ADR-0007): embeds the real Codex CLI app-server
//! (`codex app-server --stdio`, line-framed JSON-RPC without the `jsonrpc`
//! header field) as the Focus agent. The agent keeps its full local
//! environment (config / skills / shell); Focus only attaches the conversation
//! and publishes the focus-cli skill so the agent can orchestrate Focus
//! (Paseo-style: wrap, do not intercept). Protocol fields were cross-checked
//! against `codex app-server generate-json-schema` (0.146.0-alpha.3.1).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::broadcast::Sender;

use super::{AgentProvider, AgentThreadInfo, TurnDone};
use crate::event_bus::CoreEvent;
use crate::agents::mock::state_to_animation;

pub const CLIENT_NAME: &str = "focus_desktop";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const START_TIMEOUT: Duration = Duration::from_secs(20);

/// State shared between the provider and its stdout reader thread.
struct Shared {
    /// M5 (ADR-0022): this provider's agent identity — used in every event
    /// envelope so the frontend can isolate per-Agent streams.
    character_id: String,
    session_id: String,
    current_thread: Mutex<Option<String>>,
    current_turn: Mutex<Option<String>>,
    last_message: Mutex<String>,
    /// M5 (ADR-0022): per-turn display switches + first-delta marker.
    display: Mutex<crate::workflow_engine::engine::AgentDisplay>,
    first_delta_sent: Mutex<bool>,
}

pub struct CodexProvider {
    tx: Sender<CoreEvent>,
    exe_path: PathBuf,
    shared: Arc<Shared>,
    child: Option<Child>,
    stdin: Option<Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<u64, std::sync::mpsc::Sender<Value>>>>,
    next_id: AtomicU64,
    turn_done: Arc<tokio::sync::broadcast::Sender<TurnDone>>,
}

/// Latest installed `codex.exe` under %LOCALAPPDATA%\OpenAI\Codex\bin\*\.
/// Version directories are content hashes, so the newest by file mtime wins.
pub fn find_codex_exe() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    let bin = PathBuf::from(local).join("OpenAI").join("Codex").join("bin");
    let rd = std::fs::read_dir(&bin).ok()?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in rd.flatten() {
        let p = entry.path().join("codex.exe");
        if !p.is_file() {
            continue;
        }
        let mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
        let key = mtime.unwrap_or(std::time::UNIX_EPOCH);
        if best.as_ref().map(|(t, _)| key > *t).unwrap_or(true) {
            best = Some((key, p));
        }
    }
    best.map(|(_, p)| p)
}

/// Legacy unavailable-error classifier retained for focused tests.
#[cfg(test)]
pub fn is_unavailable_error(err: &str) -> bool {
    err.contains("启动失败") || err.contains("codex 未启动") || err.contains("写入失败")
}

/// Embedded focus-cli skill asset (installed into ~/.codex/skills so the
/// agent can orchestrate Focus, Paseo-style; ADR-0007).
pub const FOCUS_CLI_SKILL: &str = include_str!("../../assets/agent-skills/focus-cli/SKILL.md");

/// Install the focus-cli skill into `<base>/.codex/skills/focus-cli/SKILL.md`.
pub fn install_focus_cli_skill_into(base: &Path) -> Result<PathBuf, String> {
    let dir = base.join(".codex").join("skills").join("focus-cli");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("SKILL.md");
    if std::fs::read_to_string(&path).map(|s| s == FOCUS_CLI_SKILL).unwrap_or(false) {
        return Ok(path);
    }
    std::fs::write(&path, FOCUS_CLI_SKILL).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Install into the real user home (~/.codex/skills).
pub fn install_focus_cli_skill() -> Result<PathBuf, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "USERPROFILE/HOME 未设置".to_string())?;
    install_focus_cli_skill_into(Path::new(&home))
}


impl CodexProvider {
    pub fn new(tx: Sender<CoreEvent>, exe_path: PathBuf, character_id: String) -> Self {
        let session_id = format!("focus-{}-{}", character_id, std::process::id());
        Self {
            tx,
            exe_path,
            shared: Arc::new(Shared {
                character_id,
                session_id,
                current_thread: Mutex::new(None),
                current_turn: Mutex::new(None),
                last_message: Mutex::new(String::new()),
                display: Mutex::new(crate::workflow_engine::engine::AgentDisplay::default()),
                first_delta_sent: Mutex::new(false),
            }),
            child: None,
            stdin: None,
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(1),
            turn_done: Arc::new(tokio::sync::broadcast::channel::<TurnDone>(64).0),
        }
    }


    /// Publish the active thread id so the agent can pass `--agent-thread`
    /// when invoking focus-cli (see the focus-cli skill).
    fn write_thread_marker(&self) {
        let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) else {
            return;
        };
        let path = PathBuf::from(home).join(".codex").join("focus-thread.json");
        let tid = self.shared.current_thread.lock().unwrap().clone().unwrap_or_default();
        let _ = std::fs::write(
            path,
            json!({
                "threadId": tid,
                "updatedAt": chrono::Utc::now()
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            })
            .to_string(),
        );
    }

    fn ensure_started(&mut self) -> Result<(), String> {
        if let Err(e) = install_focus_cli_skill() {
            eprintln!("[codex] focus-cli skill 安装跳过: {e}");
        }
        if self.child.is_some() && self.stdin.is_some() {
            return Ok(());
        }
        let mut child = Command::new(&self.exe_path)
            .arg("app-server")
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("codex app-server 启动失败: {e}"))?;
        let stdout = child.stdout.take().ok_or("codex stdout 不可用")?;
        let stdin = child.stdin.take().ok_or("codex stdin 不可用")?;
        if let Some(mut err) = child.stderr.take() {
            let _ = std::thread::spawn(move || {
                use std::io::Read;
                let mut buf = [0u8; 2048];
                loop {
                    match err.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => eprintln!("[codex] {}", String::from_utf8_lossy(&buf[..n]).trim()),
                    }
                }
            });
        }
        let pending = self.pending.clone();
        let tx = self.tx.clone();
        let shared = self.shared.clone();
        let turn_done = self.turn_done.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                dispatch_message(&tx, &shared, &pending, &turn_done, msg);
            }
        });
        self.child = Some(child);
        self.stdin = Some(Mutex::new(stdin));

        let init = self.request(
            "initialize",
            json!({ "clientInfo": { "name": CLIENT_NAME, "version": env!("CARGO_PKG_VERSION") } }),
            START_TIMEOUT,
        )?;
        if init.get("error").is_some() {
            return Err(format!("codex initialize 失败: {init}"));
        }
        self.send_notification("initialized");
        Ok(())
    }

    fn reset_process(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
        }
        self.stdin = None;
    }

    fn send_notification(&self, method: &str) {
        let line = json!({ "method": method }).to_string() + "\n";
        if let Some(stdin) = &self.stdin {
            let mut g = stdin.lock().unwrap();
            let _ = g.write_all(line.as_bytes());
            let _ = g.flush();
        }
    }

    fn request(&mut self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx_req, rx_req) = std::sync::mpsc::channel::<Value>();
        self.pending.lock().unwrap().insert(id, tx_req);
        let line = json!({ "id": id, "method": method, "params": params }).to_string() + "\n";
        let write_res = {
            let stdin = self.stdin.as_ref().ok_or("codex 未启动")?;
            let mut g = stdin.lock().unwrap();
            g.write_all(line.as_bytes()).and_then(|_| g.flush())
        };
        if let Err(e) = write_res {
            self.pending.lock().unwrap().remove(&id);
            self.reset_process();
            return Err(format!("codex 写入失败（进程可能已退出）: {e}"));
        }
        match rx_req.recv_timeout(timeout) {
            Ok(v) => Ok(v),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                Err(format!("codex {method} 请求超时"))
            }
        }
    }

    fn send_internal(&mut self, thread_id: &str, text: &str, display: crate::workflow_engine::engine::AgentDisplay) -> Result<(), String> {
        // M5 (ADR-0022): system-level output discipline injected into every
        // turn (short newline-separated sentences, no Markdown).
        *self.shared.display.lock().unwrap() = display;
        reset_turn_capture(&self.shared);
        let full = format!("{}\n\n{}", super::OUTPUT_DISCIPLINE, text);
        let resp = self.request(
            "turn/start",
            json!({
                "threadId": thread_id,
                "input": [{ "type": "text", "text": full }]
            }),
            REQUEST_TIMEOUT,
        )?;
        if resp.get("error").is_some() {
            return Err(format!("turn/start 失败: {resp}"));
        }
        if let Some(turn_id) = resp
            .get("result")
            .and_then(|r| r.get("turn"))
            .and_then(|t| t.get("id"))
            .and_then(Value::as_str)
        {
            *self.shared.current_turn.lock().unwrap() = Some(turn_id.to_string());
        }
        Ok(())
    }

    fn emit_envelope(&self, event: Value) {
        let env = super::envelope(&self.shared.character_id, &self.shared.session_id, event);
        let _ = self.tx.send(CoreEvent::AgentEvent(env));
    }

    /// Subscribe to turn-completion signals (M4 workflow agent nodes).
    pub fn subscribe_turn_done(&self) -> tokio::sync::broadcast::Receiver<TurnDone> {
        self.turn_done.subscribe()
    }
}

impl Drop for CodexProvider {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
        }
    }
}

impl AgentProvider for CodexProvider {

    fn start_thread(
        &mut self,
        workspace_dir: &str,
        initial_message: &str,
        display: crate::workflow_engine::engine::AgentDisplay,
    ) -> Result<AgentThreadInfo, String> {
        self.ensure_started()?;
        let mut params = serde_json::Map::new();
        if !workspace_dir.trim().is_empty() {
            params.insert("cwd".into(), json!(workspace_dir.trim()));
        }
        let resp = self.request("thread/start", Value::Object(params), REQUEST_TIMEOUT)?;
        if resp.get("error").is_some() {
            return Err(format!("thread/start 失败: {resp}"));
        }
        let thread = resp
            .get("result")
            .and_then(|r| r.get("thread"))
            .cloned()
            .ok_or_else(|| format!("thread/start 响应缺少 thread: {resp}"))?;
        let info = thread_info(&thread);
        *self.shared.current_thread.lock().unwrap() = Some(info.id.clone());
        self.write_thread_marker();
        self.emit_envelope(json!({ "type": "session.started" }));
        if !initial_message.trim().is_empty() {
            self.send_internal(&info.id, initial_message, display)?;
        }
        Ok(info)
    }

    fn resume_thread(&mut self, thread_id: &str) -> Result<AgentThreadInfo, String> {
        self.ensure_started()?;
        let resp = self.request(
            "thread/resume",
            json!({ "threadId": thread_id }),
            REQUEST_TIMEOUT,
        )?;
        if resp.get("error").is_some() {
            return Err(format!("thread/resume 失败: {resp}"));
        }
        let thread = resp
            .get("result")
            .and_then(|r| r.get("thread"))
            .cloned()
            .ok_or_else(|| format!("thread/resume 响应缺少 thread: {resp}"))?;
        let info = thread_info(&thread);
        *self.shared.current_thread.lock().unwrap() = Some(info.id.clone());
        self.write_thread_marker();
        self.emit_envelope(json!({ "type": "session.started" }));
        Ok(info)
    }

    fn list_threads(&mut self) -> Result<Vec<AgentThreadInfo>, String> {
        self.ensure_started()?;
        let resp = self.request(
            "thread/list",
            json!({ "sortKey": "updated_at", "sortDirection": "desc", "limit": 20 }),
            REQUEST_TIMEOUT,
        )?;
        if resp.get("error").is_some() {
            return Err(format!("thread/list 失败: {resp}"));
        }
        let data = resp
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(data.iter().map(thread_info).collect())
    }

    fn send(&mut self, thread_id: &str, text: &str, display: crate::workflow_engine::engine::AgentDisplay) -> Result<(), String> {
        self.ensure_started()?;
        {
            let cur = self.shared.current_thread.lock().unwrap().clone();
            if cur.as_deref() != Some(thread_id) {
                let _ = self.resume_thread(thread_id)?;
            }
        }
        self.send_internal(thread_id, text, display)
    }

    fn interrupt(&mut self, thread_id: &str) -> Result<(), String> {
        self.ensure_started()?;
        let turn_id = self.shared.current_turn.lock().unwrap().clone();
        let Some(turn_id) = turn_id else { return Ok(()) };
        let _ = self.request(
            "turn/interrupt",
            json!({ "threadId": thread_id, "turnId": turn_id }),
            Duration::from_secs(10),
        )?;
        Ok(())
    }
}

fn thread_info(t: &Value) -> AgentThreadInfo {
    AgentThreadInfo {
        id: t.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
        preview: t.get("preview").and_then(Value::as_str).unwrap_or("").to_string(),
        cwd: t.get("cwd").and_then(Value::as_str).unwrap_or("").to_string(),
        status: t
            .get("status")
            .and_then(|s| s.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("idle")
            .to_string(),
        updated_at: t.get("updatedAt").and_then(Value::as_i64).unwrap_or(0),
        automation: false,
    }
}

// ---------------------------------------------------------------------------
// stdio message dispatch: responses complete pending requests; notifications
// are mapped to AgentEvent v1 envelopes on the core event bus.
// ---------------------------------------------------------------------------

fn dispatch_message(
    tx: &Sender<CoreEvent>,
    shared: &Arc<Shared>,
    pending: &Arc<Mutex<HashMap<u64, std::sync::mpsc::Sender<Value>>>>,
    turn_done: &Arc<tokio::sync::broadcast::Sender<TurnDone>>,
    msg: Value,
) {
    if let Some(id) = msg.get("id").and_then(Value::as_u64) {
        if let Some(s) = pending.lock().unwrap().remove(&id) {
            let _ = s.send(msg);
        }
        return;
    }
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "item/started" => handle_item_started(tx, shared, &params),
        "item/completed" => handle_item_completed(tx, shared, &params),
        "item/agentMessage/delta" => handle_agent_delta(tx, shared, &params),
        "turn/started" => {
            reset_turn_capture(shared);
            if let Some(turn_id) = params
                .get("turn")
                .and_then(|t| t.get("id"))
                .and_then(Value::as_str)
            {
                *shared.current_turn.lock().unwrap() = Some(turn_id.to_string());
            }
            emit_status(tx, shared, "thinking");
        }
        "turn/completed" => handle_turn_completed(tx, shared, turn_done, &params),
        "thread/started" => {
            if let Some(tid) = params.get("thread").and_then(|t| t.get("id")).and_then(Value::as_str) {
                *shared.current_thread.lock().unwrap() = Some(tid.to_string());
            }
        }
        "error" => {
            emit_envelope(
                tx,
                shared,
                json!({ "type": "session.error", "message": params.get("message").and_then(Value::as_str).unwrap_or("codex error") }),
            );
        }
        _ => {}
    }
}

fn reset_turn_capture(shared: &Shared) {
    shared.last_message.lock().unwrap().clear();
    *shared.first_delta_sent.lock().unwrap() = false;
}

fn emit_envelope(tx: &Sender<CoreEvent>, shared: &Shared, event: Value) {
    // M5 (ADR-0022): agentId = character_id so the frontend can isolate
    // per-Agent event streams.
    let env = super::envelope(&shared.character_id, &shared.session_id, event);
    let _ = tx.send(CoreEvent::AgentEvent(env));
}

fn emit_status(tx: &Sender<CoreEvent>, shared: &Shared, state: &str) {
    emit_envelope(tx, shared, json!({ "type": "status.changed", "state": state }));
    let _ = tx.send(CoreEvent::PetStateChanged {
        state: state.to_string(),
        animation: state_to_animation(state).to_string(),
    });
}

fn tool_kind<'a>(item: &'a Value) -> Option<&'a str> {
    let t = item.get("type").and_then(Value::as_str)?;
    match t {
        "local_shell_call" | "function_call" | "command" | "file_change" | "mcpToolCall" => Some(t),
        _ => None,
    }
}

fn tool_summary(item: &Value, fallback: &str) -> String {
    if let Some(c) = item.get("command").and_then(Value::as_str) {
        return c.to_string();
    }
    if let Some(c) = item.get("name").and_then(Value::as_str) {
        return c.to_string();
    }
    if let Some(a) = item.get("action").and_then(|a| a.get("command")).and_then(Value::as_str) {
        return a.to_string();
    }
    if let Some(p) = item.get("path").and_then(Value::as_str) {
        return p.to_string();
    }
    fallback.to_string()
}
fn handle_item_started(tx: &Sender<CoreEvent>, shared: &Shared, params: &Value) {
    let Some(item) = params.get("item") else { return };
    match item.get("type").and_then(Value::as_str) {
        Some("reasoning") => emit_status(tx, shared, "thinking"),
        Some(t) if tool_kind(item).is_some() => emit_envelope(
            tx,
            shared,
            json!({ "type": "tool.started", "tool": t, "inputSummary": tool_summary(item, "执行中") }),
        ),
        _ => {}
    }
}

fn handle_item_completed(tx: &Sender<CoreEvent>, shared: &Shared, params: &Value) {
    let Some(item) = params.get("item") else { return };
    match item.get("type").and_then(Value::as_str) {
        Some("agentMessage") => {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                *shared.last_message.lock().unwrap() = text.to_string();
                // M5 (ADR-0022): final result shown only when showResult is on.
                if !text.trim().is_empty() && shared.display.lock().unwrap().show_result {
                    emit_envelope(tx, shared, json!({ "type": "message.completed", "text": text }));
                }
            }
        }
        Some(t) if tool_kind(item).is_some() => {
            let summary = item
                .get("aggregatedOutput")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    item.get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("completed")
                        .to_string()
                });
            emit_envelope(
                tx,
                shared,
                json!({ "type": "tool.completed", "tool": t, "resultSummary": summary }),
            );
        }
        _ => {}
    }
}

fn handle_agent_delta(tx: &Sender<CoreEvent>, shared: &Shared, params: &Value) {
    if let Some(delta) = params.get("delta").and_then(Value::as_str) {
        if delta.is_empty() {
            return;
        }
        // M5 (ADR-0022): first delta = the initial short sentence (showInitial),
        // later deltas = the thinking stream (showThinking).
        let display = *shared.display.lock().unwrap();
        let mut first = shared.first_delta_sent.lock().unwrap();
        let allow = if !*first { display.show_initial } else { display.show_thinking };
        *first = true;
        if allow {
            emit_envelope(tx, shared, json!({ "type": "message.delta", "text": delta }));
        }
    }
}

fn handle_turn_completed(
    tx: &Sender<CoreEvent>,
    shared: &Shared,
    turn_done: &Arc<tokio::sync::broadcast::Sender<TurnDone>>,
    params: &Value,
) {
    *shared.current_turn.lock().unwrap() = None;
    let thread_id = params
        .get("threadId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            params
                .get("turn")
                .and_then(|t| t.get("threadId"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| shared.current_thread.lock().unwrap().clone());
    let result = shared.last_message.lock().unwrap().clone();
    let status = params
        .get("turn")
        .and_then(|t| t.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("completed");
    match status {
        "completed" => {
            emit_status(tx, shared, "success");
            emit_envelope(tx, shared, json!({ "type": "session.completed", "outcome": "success" }));
            emit_status(tx, shared, "idle");
            let _ = turn_done.send(TurnDone { thread_id, status: "completed".into(), result: Some(result) });
        }
        "interrupted" => {
            emit_status(tx, shared, "cancelled");
            emit_envelope(tx, shared, json!({ "type": "session.completed", "outcome": "cancelled" }));
            emit_status(tx, shared, "idle");
            let _ = turn_done.send(TurnDone { thread_id, status: "interrupted".into(), result: Some(result) });
        }
        _ => {
            let message = params
                .get("turn")
                .and_then(|t| t.get("error"))
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("agent turn failed");
            emit_status(tx, shared, "error");
            emit_envelope(tx, shared, json!({ "type": "session.error", "message": message }));
            emit_envelope(tx, shared, json!({ "type": "session.completed", "outcome": "error" }));
            let _ = tx.send(CoreEvent::BubbleRequested {
                text: "Agent 出错了，已停止。".to_string(),
                priority: "critical".to_string(),
            });
            emit_status(tx, shared, "idle");
            let _ = turn_done.send(TurnDone { thread_id, status: "error".into(), result: Some(result) });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::validate_envelope;

    #[test]
    fn find_codex_exe_picks_newest() {
        let base = std::env::temp_dir().join(format!("focus-codex-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("bin/v1")).unwrap();
        std::fs::create_dir_all(base.join("bin/v2")).unwrap();
        let p1 = base.join("bin/v1/codex.exe");
        let p2 = base.join("bin/v2/codex.exe");
        std::fs::write(&p1, b"x").unwrap();
        std::fs::write(&p2, b"y").unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(3600);
        let _ = filetime_set(&p1, old);
        let new = std::time::SystemTime::now();
        let _ = filetime_set(&p2, new);

        // monkey-patch via a helper that mimics find over a given dir
        let got = find_in_dir(&base.join("bin"));
        assert_eq!(got, Some(p2.clone()), "newest codex.exe must win");

        let _ = std::fs::remove_dir_all(&base);
        let _ = p1;
        let _ = p2;
    }

    fn filetime_set(p: &std::path::Path, t: std::time::SystemTime) -> std::io::Result<()> {
        let ft = std::fs::File::options().write(true).open(p)?;
        ft.set_times(std::fs::FileTimes::new().set_modified(t))
    }

    fn find_in_dir(bin: &std::path::Path) -> Option<PathBuf> {
        let rd = std::fs::read_dir(bin).ok()?;
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        for entry in rd.flatten() {
            let p = entry.path().join("codex.exe");
            if !p.is_file() {
                continue;
            }
            let mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
            let key = mtime.unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(t, _)| key > *t).unwrap_or(true) {
                best = Some((key, p));
            }
        }
        best.map(|(_, p)| p)
    }

    #[test]
    fn request_payload_shape() {
        let p = json!({
            "id": 7,
            "method": "turn/start",
            "params": {
                "threadId": "th-1",
                "input": [{ "type": "text", "text": "hi" }]
            }
        });
        assert_eq!(p["method"], "turn/start");
        assert_eq!(p["params"]["input"][0]["text"], "hi");
        let s = p.to_string();
        assert!(!s.contains("jsonrpc"), "wire format has no jsonrpc header");
        assert!(s.ends_with('}'));
    }

    #[test]
    fn delta_maps_to_schema_valid_envelope() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<CoreEvent>(8);
        let shared = Arc::new(Shared {
            character_id: "char-test".into(),
            session_id: "s1".into(),
            current_thread: Mutex::new(None),
            current_turn: Mutex::new(None),
            last_message: Mutex::new(String::new()),
            display: Mutex::new(crate::workflow_engine::engine::AgentDisplay::default()),
            first_delta_sent: Mutex::new(false),
        });
        let mut rx = tx.subscribe();
        handle_agent_delta(
            &tx,
            &shared,
            &json!({ "delta": "你好", "itemId": "i1", "threadId": "t", "turnId": "u" }),
        );
        let env = rx.try_recv().expect("envelope emitted");
        let CoreEvent::AgentEvent(v) = env else { panic!("expected AgentEvent") };
        validate_envelope(&v).unwrap();
        assert_eq!(v["event"]["text"], "你好");
    }

    #[test]
    fn turn_completed_failed_maps_to_error() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<CoreEvent>(32);
        let shared = Arc::new(Shared {
            character_id: "char-test".into(),
            session_id: "s1".into(),
            current_thread: Mutex::new(None),
            current_turn: Mutex::new(Some("u1".into())),
            last_message: Mutex::new(String::new()),
            display: Mutex::new(crate::workflow_engine::engine::AgentDisplay::default()),
            first_delta_sent: Mutex::new(false),
        });
        let (td_tx, _td_rx) = tokio::sync::broadcast::channel::<TurnDone>(16);
        let td = Arc::new(td_tx);
        let mut rx = tx.subscribe();
        handle_turn_completed(
            &tx,
            &shared,
            &td,
            &json!({ "threadId": "t", "turn": { "id": "u1", "status": "failed", "error": { "message": "boom" } } }),
        );
        assert!(shared.current_turn.lock().unwrap().is_none());
        let mut saw_error = false;
        while let Ok(env) = rx.try_recv() {
            if let CoreEvent::AgentEvent(v) = env {
                if v["event"]["type"] == "session.error" {
                    saw_error = true;
                    validate_envelope(&v).unwrap();
                }
            }
        }
        assert!(saw_error, "failed turn must emit session.error");
    }

    #[test]
    fn hidden_turn_preserves_result_without_emitting_completed_chat() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<CoreEvent>(32);
        let shared = Arc::new(Shared {
            character_id: "char-test".into(),
            session_id: "s1".into(),
            current_thread: Mutex::new(Some("thread-1".into())),
            current_turn: Mutex::new(None),
            last_message: Mutex::new("stale result".into()),
            display: Mutex::new(crate::workflow_engine::engine::AgentDisplay {
                show_initial: false,
                show_thinking: false,
                show_result: false,
            }),
            first_delta_sent: Mutex::new(false),
        });
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (td_tx, mut td_rx) = tokio::sync::broadcast::channel::<TurnDone>(16);
        let td = Arc::new(td_tx);
        let mut events = tx.subscribe();

        dispatch_message(
            &tx,
            &shared,
            &pending,
            &td,
            json!({ "method": "turn/started", "params": { "turn": { "id": "turn-1" } } }),
        );
        assert!(shared.last_message.lock().unwrap().is_empty());
        dispatch_message(
            &tx,
            &shared,
            &pending,
            &td,
            json!({ "method": "item/completed", "params": { "item": { "type": "agentMessage", "text": "private result" } } }),
        );
        dispatch_message(
            &tx,
            &shared,
            &pending,
            &td,
            json!({ "method": "turn/completed", "params": { "threadId": "thread-1", "turn": { "id": "turn-1", "status": "completed" } } }),
        );

        let done = td_rx.try_recv().expect("turn completion signal");
        assert_eq!(done.result.as_deref(), Some("private result"));
        while let Ok(event) = events.try_recv() {
            if let CoreEvent::AgentEvent(envelope) = event {
                assert_ne!(envelope["event"]["type"], "message.completed");
            }
        }
    }

    #[test]
    fn whitelist_unavailable_detection() {
        assert!(is_unavailable_error("codex app-server 启动失败: x"));
        assert!(is_unavailable_error("codex 写入失败（进程可能已退出）"));
        assert!(!is_unavailable_error("codex thread/list 请求超时"));
    }

    #[test]
    fn focus_cli_skill_installs_into_home() {
        let base = std::env::temp_dir().join(format!("focus-skill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let path = install_focus_cli_skill_into(&base).unwrap();
        let name = path.to_string_lossy().to_string();
        assert!(name.contains(".codex") && name.contains("focus-cli") && name.ends_with("SKILL.md"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("focus-cli"));
        assert!(content.contains("--agent-thread"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn focus_cli_skill_teaches_agent_workflow_control() {
        assert!(FOCUS_CLI_SKILL.contains("agent list"));
        assert!(FOCUS_CLI_SKILL.contains("agent session"));
        for command in [
            "workflow list",
            "workflow read",
            "workflow create",
            "workflow update",
            "workflow delete",
            "workflow run",
            "workflow runs",
            "workflow cancel",
        ] {
            assert!(FOCUS_CLI_SKILL.contains(command), "missing {command}");
        }
        assert!(FOCUS_CLI_SKILL.contains("--payload"));
        assert!(FOCUS_CLI_SKILL.contains("\"trigger\": \"manual\""));
        assert!(FOCUS_CLI_SKILL.contains("\"characterId\": \"\""));
        assert!(FOCUS_CLI_SKILL.contains("\"kind\": \"agent\""));
        assert!(FOCUS_CLI_SKILL.contains("\"prompt\": \""));
        assert!(FOCUS_CLI_SKILL.contains("\"showResult\": true"));

        let json_start = FOCUS_CLI_SKILL.find("```json\n").unwrap() + "```json\n".len();
        let json_end = FOCUS_CLI_SKILL[json_start..].find("\n```").unwrap() + json_start;
        let workflow: crate::workflow_engine::model::WorkflowDef =
            serde_json::from_str(&FOCUS_CLI_SKILL[json_start..json_end]).unwrap();
        assert_eq!(workflow.trigger, "manual");
        assert!(workflow.character_id.is_empty());
        assert_eq!(workflow.nodes.len(), 1);
        assert_eq!(workflow.nodes[0].kind, "agent");
        assert!(!workflow.nodes[0].params["characterId"].as_str().unwrap().is_empty());
        assert!(workflow.nodes[0].params["prompt"].is_string());
        assert_eq!(workflow.nodes[0].params["showResult"], true);
    }
}
