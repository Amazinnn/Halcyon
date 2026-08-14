//! ClaudeProvider (ADR-0025): starts one native Claude Code CLI process for
//! each turn and adapts its line-delimited stream-json output to Focus's
//! provider-neutral AgentEvent and TurnDone contracts.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::broadcast::Sender;

use super::{AgentProvider, AgentThreadInfo, TurnDone, ACTIVE_TURN_ERROR};
use crate::agents::mock::state_to_animation;
use crate::event_bus::CoreEvent;
use crate::workflow_engine::engine::AgentDisplay;

const START_TIMEOUT: Duration = Duration::from_secs(20);

#[cfg(windows)]
const CLAUDE_CHILD_CREATION_FLAGS: u32 =
    windows::Win32::System::Threading::CREATE_NO_WINDOW.0;

#[cfg(windows)]
fn configure_claude_child(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CLAUDE_CHILD_CREATION_FLAGS);
}

#[cfg(not(windows))]
fn configure_claude_child(_command: &mut Command) {}

pub const FOCUS_CLI_SKILL: &str = include_str!("../assets/agent-skills/focus-cli/SKILL.md");

pub fn find_claude_exe() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for name in ["claude.exe", "claude.cmd"] {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn install_focus_cli_skill_into(base: &Path) -> Result<PathBuf, String> {
    let dir = base.join(".claude").join("skills").join("focus-cli");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("SKILL.md");
    if std::fs::read_to_string(&path)
        .map(|content| content == FOCUS_CLI_SKILL)
        .unwrap_or(false)
    {
        return Ok(path);
    }
    std::fs::write(&path, FOCUS_CLI_SKILL).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn install_focus_cli_skill() -> Result<PathBuf, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "USERPROFILE/HOME 未设置".to_string())?;
    install_focus_cli_skill_into(Path::new(&home))
}

fn claude_path_with_focus_cli(focus_exe: &Path, existing_path: Option<OsString>) -> OsString {
    let mut paths = focus_exe
        .parent()
        .map(|parent| vec![parent.to_path_buf()])
        .unwrap_or_default();
    if let Some(existing_path) = existing_path.as_ref() {
        paths.extend(std::env::split_paths(existing_path));
    }
    std::env::join_paths(paths).unwrap_or_else(|_| existing_path.unwrap_or_default())
}

fn claude_resident_args(resume_session: Option<&str>) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-p"),
        OsString::from("--input-format"),
        OsString::from("stream-json"),
        OsString::from("--output-format"),
        OsString::from("stream-json"),
        OsString::from("--include-partial-messages"),
        OsString::from("--verbose"),
    ];
    if let Some(session_id) = resume_session.filter(|id| !id.trim().is_empty()) {
        args.push(OsString::from("--resume"));
        args.push(OsString::from(session_id));
    }
    args
}

fn claude_input_line(prompt: &str) -> String {
    json!({
        "type": "user",
        "message": { "role": "user", "content": prompt },
    })
    .to_string()
}

fn claude_turn_prompt(prompt: &str) -> String {
    format!("{}\n\n{}", super::OUTPUT_DISCIPLINE, prompt)
}

fn cmd_quote(value: &std::ffi::OsStr) -> String {
    format!("\"{}\"", value.to_string_lossy().replace('"', "\"\""))
}

fn command_for(executable: &Path, args: &[OsString]) -> Command {
    let is_cmd = executable
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("cmd"));
    let command = if is_cmd {
        let mut command =
            Command::new(std::env::var_os("COMSPEC").unwrap_or_else(|| OsString::from("cmd.exe")));
        let invocation = std::iter::once(executable.as_os_str())
            .chain(args.iter().map(OsString::as_os_str))
            .map(cmd_quote)
            .collect::<Vec<_>>()
            .join(" ");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.raw_arg(format!("/D /S /V:OFF /C \"{invocation}\""));
        }
        #[cfg(not(windows))]
        command.arg("/D").arg("/S").arg("/C").arg(invocation);
        command
    } else {
        let mut command = Command::new(executable);
        command.args(args);
        command
    };
    command
}

fn kill_and_reap(child: &mut Child) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn resident_is_current<T>(current: Option<&Arc<T>>, reader: &Arc<T>) -> bool {
    current.is_some_and(|current| Arc::ptr_eq(current, reader))
}

struct TurnState {
    sequence: u64,
    character_id: String,
    session_id: Mutex<String>,
    display: AgentDisplay,
    first_delta_sent: Mutex<bool>,
    last_message: Mutex<String>,
    active_tools: Mutex<HashMap<String, String>>,
    terminal_sent: AtomicBool,
    interrupted: AtomicBool,
}

impl TurnState {
    fn new(sequence: u64, character_id: String, session_id: String, display: AgentDisplay) -> Self {
        Self {
            sequence,
            character_id,
            session_id: Mutex::new(session_id),
            display,
            first_delta_sent: Mutex::new(false),
            last_message: Mutex::new(String::new()),
            active_tools: Mutex::new(HashMap::new()),
            terminal_sent: AtomicBool::new(false),
            interrupted: AtomicBool::new(false),
        }
    }

    fn session_id(&self) -> String {
        self.session_id.lock().unwrap().clone()
    }

    fn set_session_id(&self, session_id: &str) {
        if !session_id.trim().is_empty() {
            *self.session_id.lock().unwrap() = session_id.to_string();
        }
    }

    fn mark_interrupted(&self) {
        self.interrupted.store(true, Ordering::Relaxed);
    }
}

struct Shared {
    character_id: String,
    workspace_dir: Mutex<String>,
    current_thread: Mutex<Option<String>>,
    active_turn: Mutex<Option<Arc<TurnState>>>,
    next_sequence: AtomicU64,
}

impl Shared {
    fn new(character_id: String, workspace_dir: String) -> Self {
        Self {
            character_id,
            workspace_dir: Mutex::new(workspace_dir),
            current_thread: Mutex::new(None),
            active_turn: Mutex::new(None),
            next_sequence: AtomicU64::new(1),
        }
    }

    fn claim_turn(&self, display: AgentDisplay) -> Result<Arc<TurnState>, String> {
        let mut active = self.active_turn.lock().unwrap();
        if active.is_some() {
            return Err(ACTIVE_TURN_ERROR.to_string());
        }
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        let session_id = self
            .current_thread
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                format!(
                    "focus-claude-{}-{}-{}",
                    self.character_id,
                    std::process::id(),
                    sequence
                )
            });
        let turn = Arc::new(TurnState::new(
            sequence,
            self.character_id.clone(),
            session_id,
            display,
        ));
        *active = Some(turn.clone());
        Ok(turn)
    }

    fn release_turn(&self, sequence: u64) {
        let mut active = self.active_turn.lock().unwrap();
        if active
            .as_ref()
            .is_some_and(|turn| turn.sequence == sequence)
        {
            *active = None;
        }
    }
}

pub struct ClaudeProvider {
    tx: Sender<CoreEvent>,
    exe_path: PathBuf,
    shared: Arc<Shared>,
    resident: Arc<Mutex<Option<Arc<ResidentProcess>>>>,
    turn_done: Arc<tokio::sync::broadcast::Sender<TurnDone>>,
}

struct ResidentProcess {
    child: Mutex<Child>,
    stdin: Mutex<Option<std::process::ChildStdin>>,
}

impl ClaudeProvider {
    pub fn new(
        tx: Sender<CoreEvent>,
        exe_path: PathBuf,
        character_id: String,
        workspace_dir: String,
    ) -> Self {
        Self {
            tx,
            exe_path,
            shared: Arc::new(Shared::new(character_id, workspace_dir)),
            resident: Arc::new(Mutex::new(None)),
            turn_done: Arc::new(tokio::sync::broadcast::channel::<TurnDone>(64).0),
        }
    }

    pub fn subscribe_turn_done(&self) -> tokio::sync::broadcast::Receiver<TurnDone> {
        self.turn_done.subscribe()
    }

    pub fn has_active_turn(&self) -> bool {
        self.shared.active_turn.lock().unwrap().is_some()
    }

    fn resident_turn_info(&self, turn: &TurnState) -> AgentThreadInfo {
        AgentThreadInfo {
            id: self
                .shared
                .current_thread
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| turn.session_id()),
            preview: String::new(),
            cwd: self.shared.workspace_dir.lock().unwrap().clone(),
            status: "running".to_string(),
            updated_at: chrono::Utc::now().timestamp(),
            automation: false,
        }
    }

    fn write_resident_turn(&self, resident: &ResidentProcess, prompt: &str) -> Result<(), String> {
        let input = claude_input_line(&claude_turn_prompt(prompt));
        let mut stdin = resident.stdin.lock().unwrap();
        let stdin = stdin
            .as_mut()
            .ok_or_else(|| "Claude stdin is closed".to_string())?;
        writeln!(stdin, "{input}")
            .and_then(|_| stdin.flush())
            .map_err(|error| format!("Claude prompt write failed: {error}"))
    }

    fn stop_resident(resident: &ResidentProcess) {
        let _ = resident.stdin.lock().unwrap().take();
        let mut child = resident.child.lock().unwrap();
        kill_and_reap(&mut child);
    }

    fn start_resident_turn(
        &mut self,
        workspace_dir: &str,
        prompt: &str,
        resume_session: Option<&str>,
        display: AgentDisplay,
    ) -> Result<AgentThreadInfo, String> {
        let turn = self.shared.claim_turn(display)?;
        if !workspace_dir.trim().is_empty() {
            *self.shared.workspace_dir.lock().unwrap() = workspace_dir.trim().to_string();
        }

        let existing_resident = self.resident.lock().unwrap().clone();
        if let Some(resident) = existing_resident {
            if let Err(error) = self.write_resident_turn(&resident, prompt) {
                Self::stop_resident(&resident);
                *self.resident.lock().unwrap() = None;
                self.shared.release_turn(turn.sequence);
                return Err(error);
            }
            return Ok(self.resident_turn_info(&turn));
        }

        if let Err(error) = install_focus_cli_skill() {
            eprintln!("[claude] focus-cli skill install skipped: {error}");
        }
        let args = claude_resident_args(resume_session);
        let workspace = self.shared.workspace_dir.lock().unwrap().clone();
        let focus_thread = resume_session
            .filter(|session_id| !session_id.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| turn.session_id());
        let mut command = command_for(&self.exe_path, &args);
        configure_claude_child(&mut command);
        command.env("FOCUS_AGENT_THREAD", focus_thread);
        if !workspace.trim().is_empty() {
            command.current_dir(&workspace);
        }
        if let Ok(focus_exe) = std::env::current_exe() {
            command.env(
                "PATH",
                claude_path_with_focus_cli(&focus_exe, std::env::var_os("PATH")),
            );
        }
        let mut child = match command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                self.shared.release_turn(turn.sequence);
                return Err(format!("Claude CLI launch failed: {error}"));
            }
        };
        let Some(stdin) = child.stdin.take() else {
            kill_and_reap(&mut child);
            self.shared.release_turn(turn.sequence);
            return Err("Claude stdin is unavailable".to_string());
        };
        let Some(stdout) = child.stdout.take() else {
            kill_and_reap(&mut child);
            self.shared.release_turn(turn.sequence);
            return Err("Claude stdout is unavailable".to_string());
        };
        let Some(stderr) = child.stderr.take() else {
            kill_and_reap(&mut child);
            self.shared.release_turn(turn.sequence);
            return Err("Claude stderr is unavailable".to_string());
        };

        let resident = Arc::new(ResidentProcess {
            child: Mutex::new(child),
            stdin: Mutex::new(Some(stdin)),
        });
        *self.resident.lock().unwrap() = Some(resident.clone());
        let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<String, String>>();
        let stderr_capture = Arc::new(Mutex::new(String::new()));
        let stderr_for_reader = stderr_capture.clone();
        let _stderr_reader = std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut text = String::new();
            if reader.read_to_string(&mut text).is_ok() && !text.trim().is_empty() {
                eprintln!("[claude] {}", text.trim());
                *stderr_for_reader.lock().unwrap() = text;
            }
        });

        let tx = self.tx.clone();
        let done = self.turn_done.clone();
        let shared = self.shared.clone();
        let residents = self.resident.clone();
        let reader_resident = resident.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            let mut init_tx = Some(init_tx);
            for line in reader.lines() {
                if !resident_is_current(residents.lock().unwrap().as_ref(), &reader_resident) {
                    return;
                }
                let Ok(line) = line else { break };
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let active = shared.active_turn.lock().unwrap().clone();
                let Some(active) = active else { continue };
                let initialized = dispatch_stream_message(&tx, &active, &done, message.clone());
                if let Some(session_id) = initialized {
                    *shared.current_thread.lock().unwrap() = Some(session_id.clone());
                    if let Some(sender) = init_tx.take() {
                        let _ = sender.send(Ok(session_id));
                    }
                }
                if message.get("type").and_then(Value::as_str) == Some("result") {
                    shared.release_turn(active.sequence);
                }
            }
            let stderr = stderr_capture.lock().unwrap().trim().to_string();
            let is_current_resident =
                resident_is_current(residents.lock().unwrap().as_ref(), &reader_resident);
            if !is_current_resident {
                return;
            }
            let active_after_eof = shared.active_turn.lock().unwrap().clone();
            if let Some(active) = active_after_eof {
                finish_after_eof(&tx, &active, &done, &stderr);
                shared.release_turn(active.sequence);
            }
            *residents.lock().unwrap() = None;
            let _ = reader_resident.child.lock().unwrap().wait();
            if let Some(sender) = init_tx.take() {
                let message = if stderr.is_empty() {
                    "Claude CLI exited before session initialization".to_string()
                } else {
                    stderr
                };
                let _ = sender.send(Err(message));
            }
        });

        if let Err(error) = self.write_resident_turn(&resident, prompt) {
            Self::stop_resident(&resident);
            *self.resident.lock().unwrap() = None;
            self.shared.release_turn(turn.sequence);
            return Err(error);
        }
        match init_rx.recv_timeout(START_TIMEOUT) {
            Ok(Ok(_)) => Ok(self.resident_turn_info(&turn)),
            Ok(Err(error)) => Err(error),
            Err(_) => {
                turn.mark_interrupted();
                Self::stop_resident(&resident);
                self.shared.release_turn(turn.sequence);
                Err("Claude CLI initialization timed out".to_string())
            }
        }
    }
}

impl Drop for ClaudeProvider {
    fn drop(&mut self) {
        if let Some(resident) = self.resident.lock().unwrap().take() {
            Self::stop_resident(&resident);
        }
    }
}

impl AgentProvider for ClaudeProvider {
    fn start_thread(
        &mut self,
        workspace_dir: &str,
        initial_message: &str,
        display: AgentDisplay,
    ) -> Result<AgentThreadInfo, String> {
        self.start_resident_turn(workspace_dir, initial_message, None, display)
    }

    fn resume_thread(&mut self, thread_id: &str) -> Result<AgentThreadInfo, String> {
        if self.shared.active_turn.lock().unwrap().is_some() {
            return Err(ACTIVE_TURN_ERROR.to_string());
        }
        *self.shared.current_thread.lock().unwrap() = Some(thread_id.to_string());
        Ok(AgentThreadInfo {
            id: thread_id.to_string(),
            preview: String::new(),
            cwd: self.shared.workspace_dir.lock().unwrap().clone(),
            status: "idle".to_string(),
            updated_at: chrono::Utc::now().timestamp(),
            automation: false,
        })
    }

    fn resume_and_send(
        &mut self,
        thread_id: &str,
        text: &str,
        display: AgentDisplay,
    ) -> Result<AgentThreadInfo, String> {
        if self.shared.active_turn.lock().unwrap().is_some() {
            return Err(ACTIVE_TURN_ERROR.to_string());
        }
        *self.shared.current_thread.lock().unwrap() = Some(thread_id.to_string());
        let workspace = self.shared.workspace_dir.lock().unwrap().clone();
        let resume = if self.resident.lock().unwrap().is_some() {
            None
        } else {
            Some(thread_id)
        };
        self.start_resident_turn(&workspace, text, resume, display)
    }

    fn list_threads(&mut self) -> Result<Vec<AgentThreadInfo>, String> {
        let Some(thread_id) = self.shared.current_thread.lock().unwrap().clone() else {
            return Ok(Vec::new());
        };
        Ok(vec![AgentThreadInfo {
            id: thread_id,
            preview: String::new(),
            cwd: self.shared.workspace_dir.lock().unwrap().clone(),
            status: if self.shared.active_turn.lock().unwrap().is_some() {
                "running"
            } else {
                "idle"
            }
            .to_string(),
            updated_at: chrono::Utc::now().timestamp(),
            automation: false,
        }])
    }

    fn send(&mut self, thread_id: &str, text: &str, display: AgentDisplay) -> Result<(), String> {
        if self.shared.active_turn.lock().unwrap().is_some() {
            return Err(ACTIVE_TURN_ERROR.to_string());
        }
        *self.shared.current_thread.lock().unwrap() = Some(thread_id.to_string());
        let workspace = self.shared.workspace_dir.lock().unwrap().clone();
        let resume = if self.resident.lock().unwrap().is_some() {
            None
        } else {
            Some(thread_id)
        };
        self.start_resident_turn(&workspace, text, resume, display)?;
        Ok(())
    }

    fn interrupt(&mut self, thread_id: &str) -> Result<(), String> {
        let active = self.shared.active_turn.lock().unwrap().clone();
        let Some(turn) = active else {
            return Ok(());
        };
        let _ = thread_id;
        turn.mark_interrupted();
        let existing_resident = self.resident.lock().unwrap().clone();
        if let Some(resident) = existing_resident {
            Self::stop_resident(&resident);
            *self.resident.lock().unwrap() = None;
        }
        finish_after_eof(&self.tx, &turn, &self.turn_done, "");
        self.shared.release_turn(turn.sequence);
        Ok(())
    }
}

fn events_visible(turn: &TurnState) -> bool {
    turn.display.show_initial || turn.display.show_thinking || turn.display.show_result
}

fn emit_envelope(tx: &Sender<CoreEvent>, turn: &TurnState, event: Value) {
    if !events_visible(turn) {
        return;
    }
    let envelope = super::envelope(&turn.character_id, &turn.session_id(), event);
    let _ = tx.send(CoreEvent::AgentEvent(envelope));
}

fn emit_status(tx: &Sender<CoreEvent>, turn: &TurnState, state: &str) {
    if !events_visible(turn) {
        return;
    }
    emit_envelope(
        tx,
        turn,
        json!({ "type": "status.changed", "state": state }),
    );
    let _ = tx.send(CoreEvent::PetStateChanged {
        state: state.to_string(),
        animation: state_to_animation(state).to_string(),
    });
}

fn dispatch_stream_message(
    tx: &Sender<CoreEvent>,
    turn: &Arc<TurnState>,
    turn_done: &Sender<TurnDone>,
    message: Value,
) -> Option<String> {
    let session_id = message
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .map(str::to_string);
    if let Some(session_id) = session_id.as_deref() {
        turn.set_session_id(session_id);
    }

    match message.get("type").and_then(Value::as_str).unwrap_or("") {
        "system" if message.get("subtype").and_then(Value::as_str) == Some("init") => {
            emit_envelope(tx, turn, json!({ "type": "session.started" }));
            emit_status(tx, turn, "thinking");
            session_id
        }
        "stream_event" => {
            handle_stream_event(tx, turn, message.get("event").unwrap_or(&Value::Null));
            None
        }
        "user" => {
            handle_user_message(tx, turn, message.get("message").unwrap_or(&Value::Null));
            None
        }
        "result" => {
            let result = result_text(&message);
            *turn.last_message.lock().unwrap() = result.clone();
            let success = !message
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && message.get("subtype").and_then(Value::as_str) == Some("success");
            if turn.interrupted.load(Ordering::Relaxed) {
                finish_cancelled(tx, turn, turn_done);
            } else if success {
                finish_success(tx, turn, turn_done, &result);
            } else {
                finish_error(tx, turn, turn_done, &result);
            }
            None
        }
        _ => None,
    }
}

fn handle_stream_event(tx: &Sender<CoreEvent>, turn: &TurnState, event: &Value) {
    match event.get("type").and_then(Value::as_str).unwrap_or("") {
        "content_block_delta"
            if event
                .get("delta")
                .and_then(|delta| delta.get("type"))
                .and_then(Value::as_str)
                == Some("text_delta") =>
        {
            let text = event
                .get("delta")
                .and_then(|delta| delta.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if text.is_empty() {
                return;
            }
            if turn.display.shows_public_text_deltas() {
                emit_envelope(tx, turn, json!({ "type": "message.delta", "text": text }));
            }
        }
        "content_block_start" => {
            let block = event.get("content_block").unwrap_or(&Value::Null);
            if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                return;
            }
            let name = block
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string();
            let tool_id = block
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string();
            turn.active_tools
                .lock()
                .unwrap()
                .insert(tool_id, name.clone());
            let summary = block
                .get("input")
                .and_then(|input| input.get("command"))
                .and_then(Value::as_str)
                .unwrap_or("执行中");
            emit_envelope(
                tx,
                turn,
                json!({ "type": "tool.started", "tool": name, "inputSummary": summary }),
            );
        }
        "content_block_stop" => {}
        _ => {}
    }
}

fn handle_user_message(tx: &Sender<CoreEvent>, turn: &TurnState, message: &Value) {
    let Some(content) = message.get("content").and_then(Value::as_array) else {
        return;
    };
    for block in content {
        if block.get("type").and_then(Value::as_str) != Some("tool_result") {
            continue;
        }
        let tool_id = block
            .get("tool_use_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        let Some(name) = turn.active_tools.lock().unwrap().remove(tool_id) else {
            continue;
        };
        let summary = block
            .get("content")
            .and_then(Value::as_str)
            .filter(|content| !content.trim().is_empty())
            .unwrap_or_else(|| {
                if block
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "error"
                } else {
                    "completed"
                }
            });
        emit_envelope(
            tx,
            turn,
            json!({ "type": "tool.completed", "tool": name, "resultSummary": summary }),
        );
    }
}

fn result_text(message: &Value) -> String {
    message
        .get("result")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|result| !result.is_empty())
        .or_else(|| {
            message
                .get("errors")
                .and_then(Value::as_array)
                .and_then(|errors| {
                    errors
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .find(|error| !error.is_empty())
                })
        })
        .or_else(|| {
            message
                .get("subtype")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|subtype| !subtype.is_empty())
        })
        .unwrap_or("Claude CLI 执行失败")
        .to_string()
}

fn begin_terminal(turn: &TurnState) -> bool {
    !turn.terminal_sent.swap(true, Ordering::SeqCst)
}

fn finish_success(
    tx: &Sender<CoreEvent>,
    turn: &TurnState,
    turn_done: &Sender<TurnDone>,
    result: &str,
) {
    if !begin_terminal(turn) {
        return;
    }
    if !result.trim().is_empty() && turn.display.show_result {
        emit_envelope(
            tx,
            turn,
            json!({ "type": "message.completed", "text": result }),
        );
        let _ = tx.send(CoreEvent::BubbleRequested {
            text: result.to_string(),
            priority: "normal".to_string(),
            agent_id: Some(turn.character_id.clone()),
            delivery_id: None,
            reliable_delivery: true,
        });
    }
    emit_status(tx, turn, "success");
    emit_envelope(
        tx,
        turn,
        json!({ "type": "session.completed", "outcome": "success" }),
    );
    emit_status(tx, turn, "idle");
    let _ = turn_done.send(TurnDone {
        thread_id: Some(turn.session_id()),
        status: "completed".to_string(),
        result: Some(result.to_string()),
    });
}

fn finish_cancelled(tx: &Sender<CoreEvent>, turn: &TurnState, turn_done: &Sender<TurnDone>) {
    if !begin_terminal(turn) {
        return;
    }
    emit_status(tx, turn, "cancelled");
    emit_envelope(
        tx,
        turn,
        json!({ "type": "session.completed", "outcome": "cancelled" }),
    );
    emit_status(tx, turn, "idle");
    let _ = turn_done.send(TurnDone {
        thread_id: Some(turn.session_id()),
        status: "interrupted".to_string(),
        result: Some(turn.last_message.lock().unwrap().clone()),
    });
}

fn finish_error(
    tx: &Sender<CoreEvent>,
    turn: &TurnState,
    turn_done: &Sender<TurnDone>,
    message: &str,
) {
    if !begin_terminal(turn) {
        return;
    }
    let message = if message.trim().is_empty() {
        "Claude CLI 执行失败"
    } else {
        message.trim()
    };
    emit_status(tx, turn, "error");
    emit_envelope(
        tx,
        turn,
        json!({ "type": "session.error", "message": message }),
    );
    emit_envelope(
        tx,
        turn,
        json!({ "type": "session.completed", "outcome": "error" }),
    );
    emit_status(tx, turn, "idle");
    let result = turn.last_message.lock().unwrap().clone();
    let _ = turn_done.send(TurnDone {
        thread_id: Some(turn.session_id()),
        status: "error".to_string(),
        result: Some(if result.is_empty() {
            message.to_string()
        } else {
            result
        }),
    });
}

fn finish_after_eof(
    tx: &Sender<CoreEvent>,
    turn: &TurnState,
    turn_done: &Sender<TurnDone>,
    stderr: &str,
) {
    if turn.terminal_sent.load(Ordering::Relaxed) {
        return;
    }
    if turn.interrupted.load(Ordering::Relaxed) {
        finish_cancelled(tx, turn, turn_done);
    } else {
        let message = if stderr.trim().is_empty() {
            "Claude CLI 在返回最终结果前退出"
        } else {
            stderr.trim()
        };
        finish_error(tx, turn, turn_done, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::validate_envelope;
    use crate::event_bus::CoreEvent;
    use crate::workflow_engine::engine::AgentDisplay;
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    #[cfg(windows)]
    #[test]
    fn claude_child_uses_no_console_creation_flag() {
        assert_eq!(
            CLAUDE_CHILD_CREATION_FLAGS,
            windows::Win32::System::Threading::CREATE_NO_WINDOW.0,
        );
    }

    fn full_display() -> AgentDisplay {
        AgentDisplay {
            show_initial: true,
            show_thinking: true,
            show_result: true,
        }
    }

    fn hidden_display() -> AgentDisplay {
        AgentDisplay {
            show_initial: false,
            show_thinking: false,
            show_result: false,
        }
    }

    fn recorded_success_stream() -> Vec<Value> {
        vec![
            json!({
                "type": "system",
                "subtype": "init",
                "session_id": "claude-session-1",
                "cwd": r"C:\Focus-Agents\char-claude"
            }),
            json!({
                "type": "stream_event",
                "session_id": "claude-session-1",
                "event": {
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": { "type": "text_delta", "text": "开始处理。" }
                }
            }),
            json!({
                "type": "stream_event",
                "session_id": "claude-session-1",
                "event": {
                    "type": "content_block_start",
                    "index": 1,
                    "content_block": {
                        "type": "tool_use",
                        "id": "toolu_1",
                        "name": "Bash",
                        "input": { "command": "focus-cli ping" }
                    }
                }
            }),
            json!({
                "type": "stream_event",
                "session_id": "claude-session-1",
                "event": { "type": "content_block_stop", "index": 1 }
            }),
            json!({
                "type": "user",
                "session_id": "claude-session-1",
                "message": {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": "toolu_1",
                        "content": "pong",
                        "is_error": false
                    }]
                }
            }),
            json!({
                "type": "result",
                "subtype": "success",
                "is_error": false,
                "session_id": "claude-session-1",
                "result": "处理完成。"
            }),
        ]
    }

    fn dispatch_recorded(
        display: AgentDisplay,
        messages: Vec<Value>,
    ) -> (Vec<CoreEvent>, crate::agents::TurnDone) {
        let (tx, mut events) = tokio::sync::broadcast::channel::<CoreEvent>(64);
        let (done_tx, mut done_rx) = tokio::sync::broadcast::channel(8);
        let turn = Arc::new(TurnState::new(
            1,
            "char-claude".into(),
            "focus-pending".into(),
            display,
        ));

        for message in messages {
            dispatch_stream_message(&tx, &turn, &done_tx, message);
        }

        let mut emitted = Vec::new();
        while let Ok(event) = events.try_recv() {
            emitted.push(event);
        }
        let done = done_rx.try_recv().expect("terminal TurnDone");
        (emitted, done)
    }

    #[cfg(windows)]
    fn claude_cmd_shim() -> (PathBuf, PathBuf) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "focus claude cmd shim-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let shim = dir.join("claude.cmd");
        std::fs::write(
            dir.join("claude.ps1"),
            concat!(
                "$promptPath = Join-Path $PSScriptRoot 'prompt.txt'\r\n",
                "$argsPath = Join-Path $PSScriptRoot 'args.txt'\r\n",
                "[IO.File]::AppendAllText($argsPath, (($args -join ' ') + [Environment]::NewLine))\r\n",
                "[Console]::Out.WriteLine('{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"claude-shim-session\"}')\r\n",
                "[Console]::Out.Flush()\r\n",
                "while (($line = [Console]::In.ReadLine()) -ne $null) {\r\n",
                "  [IO.File]::AppendAllText($promptPath, $line + [Environment]::NewLine)\r\n",
                "  if ($line -match 'hold-open') { continue }\r\n",
                "  [Console]::Out.WriteLine('{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"session_id\":\"claude-shim-session\",\"result\":\"done\"}')\r\n",
                "  [Console]::Out.Flush()\r\n",
                "}\r\n",
            ),
        )
        .unwrap();
        std::fs::write(
            &shim,
            concat!(
                "@echo off\r\n",
                "echo %FOCUS_AGENT_THREAD%> \"%~dp0thread.txt\"\r\n",
                "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"%~dp0claude.ps1\" %*\r\n",
            ),
        )
        .unwrap();
        (dir, shim)
    }

    #[cfg(windows)]
    fn wait_for_turn_done(receiver: &mut tokio::sync::broadcast::Receiver<TurnDone>) -> TurnDone {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            match receiver.try_recv() {
                Ok(done) => return done,
                Err(tokio::sync::broadcast::error::TryRecvError::Empty)
                    if std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("turn did not finish: {error}"),
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn claude_cmd_receives_prompt_over_stdin_without_shell_expansion() {
        let (dir, shim) = claude_cmd_shim();
        let (tx, _) = tokio::sync::broadcast::channel::<CoreEvent>(32);
        let mut provider = ClaudeProvider::new(
            tx,
            shim,
            "char-claude".into(),
            dir.to_string_lossy().into_owned(),
        );
        let prompt = r#"literal \"quotes\" %PATH% & | < > ^ !"#;
        let mut done = provider.subscribe_turn_done();
        let _thread = provider
            .start_thread(&dir.to_string_lossy(), prompt, full_display())
            .unwrap();
        assert_eq!(wait_for_turn_done(&mut done).status, "completed");

        let captured = std::fs::read_to_string(dir.join("prompt.txt")).unwrap();
        let input: Value = serde_json::from_str(captured.lines().next().unwrap()).unwrap();
        assert!(
            input["message"]["content"]
                .as_str()
                .is_some_and(|content| content.contains(prompt)),
            "the literal ASCII prompt was not preserved"
        );
        drop(provider);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(windows)]
    #[test]
    fn claude_guard_remains_busy_until_terminal_child_is_reaped() {
        let (dir, shim) = claude_cmd_shim();
        let (tx, _) = tokio::sync::broadcast::channel::<CoreEvent>(32);
        let mut provider = ClaudeProvider::new(
            tx,
            shim,
            "char-claude".into(),
            dir.to_string_lossy().into_owned(),
        );
        let mut done = provider.subscribe_turn_done();
        let thread = provider
            .start_thread(&dir.to_string_lossy(), "first", full_display())
            .unwrap();
        assert_eq!(wait_for_turn_done(&mut done).status, "completed");

        let resumed = provider
            .resume_and_send(&thread.id, "second", full_display())
            .unwrap();
        assert_eq!(resumed.id, thread.id);
        assert_eq!(wait_for_turn_done(&mut done).status, "completed");
        assert_eq!(
            std::fs::read_to_string(dir.join("prompt.txt"))
                .unwrap()
                .lines()
                .count(),
            2,
            "two turns must share one resident stdin stream"
        );
        drop(provider);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(windows)]
    #[test]
    fn claude_new_provider_resumes_the_saved_session_on_its_first_turn() {
        let (dir, shim) = claude_cmd_shim();
        let (tx, _) = tokio::sync::broadcast::channel::<CoreEvent>(32);
        let mut first_provider = ClaudeProvider::new(
            tx.clone(),
            shim.clone(),
            "char-claude".into(),
            dir.to_string_lossy().into_owned(),
        );
        let mut first_done = first_provider.subscribe_turn_done();
        let first_thread = first_provider
            .start_thread(&dir.to_string_lossy(), "first turn", full_display())
            .unwrap();
        assert_eq!(wait_for_turn_done(&mut first_done).status, "completed");
        drop(first_provider);

        let mut restarted_provider = ClaudeProvider::new(
            tx,
            shim,
            "char-claude".into(),
            dir.to_string_lossy().into_owned(),
        );
        let mut restarted_done = restarted_provider.subscribe_turn_done();
        restarted_provider
            .resume_and_send(&first_thread.id, "after Focus restart", full_display())
            .unwrap();
        assert_eq!(wait_for_turn_done(&mut restarted_done).status, "completed");

        let invocations: Vec<_> = std::fs::read_to_string(dir.join("args.txt"))
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        assert_eq!(invocations.len(), 2);
        assert!(!invocations[0].contains("--resume"));
        assert!(
            invocations[1].contains("--resume claude-shim-session"),
            "the restarted provider must resume the saved provider session: {}",
            invocations[1]
        );

        drop(restarted_provider);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(windows)]
    #[test]
    fn claude_interruption_restarts_with_saved_session_and_accepts_the_next_turn() {
        let (dir, shim) = claude_cmd_shim();
        let (tx, _) = tokio::sync::broadcast::channel::<CoreEvent>(32);
        let mut provider = ClaudeProvider::new(
            tx,
            shim,
            "char-claude".into(),
            dir.to_string_lossy().into_owned(),
        );
        let mut done = provider.subscribe_turn_done();
        let thread = provider
            .start_thread(&dir.to_string_lossy(), "hold-open", full_display())
            .unwrap();

        provider.interrupt(&thread.id).unwrap();
        assert_eq!(wait_for_turn_done(&mut done).status, "interrupted");

        provider
            .send(&thread.id, "continue after cancel", full_display())
            .unwrap();
        assert_eq!(wait_for_turn_done(&mut done).status, "completed");

        let invocations: Vec<_> = std::fs::read_to_string(dir.join("args.txt"))
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        assert_eq!(invocations.len(), 2, "cancel must replace the resident CLI");
        assert!(!invocations[0].contains("--resume"));
        assert!(
            invocations[1].contains("--resume claude-shim-session"),
            "the replacement CLI must restore the saved provider session: {}",
            invocations[1]
        );

        drop(provider);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(windows)]
    #[test]
    fn claude_child_thread_environment_is_distinct_and_never_mutates_codex_marker() {
        const PROBE_ENV: &str = "FOCUS_CLAUDE_THREAD_ENV_PROBE";
        if std::env::var(PROBE_ENV).as_deref() != Ok("1") {
            let base = std::env::temp_dir().join(format!(
                "focus-claude-thread-env-parent-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&base).unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("agents::claude::tests::claude_child_thread_environment_is_distinct_and_never_mutates_codex_marker")
                .arg("--nocapture")
                .env(PROBE_ENV, "1")
                .env("USERPROFILE", &base)
                .env("HOME", &base)
                .env("TEMP", &base)
                .env("TMP", &base)
                .output()
                .unwrap();
            let _ = std::fs::remove_dir_all(&base);
            assert!(
                output.status.success(),
                "isolated probe failed\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        let home = PathBuf::from(std::env::var("USERPROFILE").unwrap());
        let marker = home.join(".codex").join("focus-thread.json");
        std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
        std::fs::write(&marker, b"codex-sentinel").unwrap();

        let (first_dir, first_shim) = claude_cmd_shim();
        let (second_dir, second_shim) = claude_cmd_shim();
        let (tx, _) = tokio::sync::broadcast::channel::<CoreEvent>(32);
        let mut first = ClaudeProvider::new(
            tx.clone(),
            first_shim,
            "char-first".into(),
            first_dir.to_string_lossy().into_owned(),
        );
        let mut second = ClaudeProvider::new(
            tx,
            second_shim,
            "char-second".into(),
            second_dir.to_string_lossy().into_owned(),
        );
        let _first_thread = first
            .start_thread(&first_dir.to_string_lossy(), "first", full_display())
            .unwrap();
        let _second_thread = second
            .start_thread(&second_dir.to_string_lossy(), "second", full_display())
            .unwrap();
        let first_env = std::fs::read_to_string(first_dir.join("thread.txt"))
            .unwrap()
            .trim()
            .to_string();
        let second_env = std::fs::read_to_string(second_dir.join("thread.txt"))
            .unwrap()
            .trim()
            .to_string();

        assert!(first_env.starts_with("focus-claude-char-first-"));
        assert!(second_env.starts_with("focus-claude-char-second-"));
        assert_ne!(first_env, second_env);
        assert_eq!(std::fs::read(&marker).unwrap(), b"codex-sentinel");

        drop(first);
        drop(second);
        let _ = std::fs::remove_dir_all(first_dir);
        let _ = std::fs::remove_dir_all(second_dir);
    }

    #[test]
    fn focus_cli_skill_prefers_child_thread_environment_before_codex_fallback() {
        let env_at = FOCUS_CLI_SKILL.find("FOCUS_AGENT_THREAD").unwrap();
        let marker_at = FOCUS_CLI_SKILL.find("~/.codex/focus-thread.json").unwrap();
        assert!(env_at < marker_at);
        assert!(FOCUS_CLI_SKILL.contains("仅当该环境变量为空或不存在"));
        assert_eq!(FOCUS_CLI_SKILL, crate::agents::codex::FOCUS_CLI_SKILL);
    }

    #[test]
    fn claude_resident_protocol_uses_realtime_input_and_one_json_line_per_turn() {
        let args = claude_resident_args(Some("claude-session-1"));
        let args: Vec<_> = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--input-format", "stream-json"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--output-format", "stream-json"]));
        assert_eq!(
            args[args.iter().position(|arg| arg == "--resume").unwrap() + 1],
            "claude-session-1"
        );

        let input: Value = serde_json::from_str(&claude_input_line("next question")).unwrap();
        assert_eq!(input["type"], "user");
        assert_eq!(input["message"]["role"], "user");
        assert_eq!(input["message"]["content"], "next question");
    }

    #[test]
    fn stale_resident_reader_never_owns_a_replacement_process() {
        let old = Arc::new(());
        let replacement = Arc::new(());

        assert!(resident_is_current(Some(&old), &old));
        assert!(!resident_is_current(Some(&replacement), &old));
        assert!(!resident_is_current(None, &old));
    }

    #[test]
    fn claude_command_uses_stream_json_and_resumes_only_saved_sessions() {
        let new_args = claude_resident_args(None);
        let new_args: Vec<_> = new_args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            &new_args[..6],
            [
                "-p",
                "--input-format",
                "stream-json",
                "--output-format",
                "stream-json",
                "--include-partial-messages"
            ]
        );
        assert!(new_args.contains(&"--verbose".to_string()));
        assert!(!new_args.contains(&"--resume".to_string()));

        let resumed_args = claude_resident_args(Some("claude-session-1"));
        let resumed_args: Vec<_> = resumed_args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let resume_at = resumed_args
            .iter()
            .position(|arg| arg == "--resume")
            .unwrap();
        assert_eq!(resumed_args[resume_at + 1], "claude-session-1");
        for forbidden in [
            "--dangerously-skip-permissions",
            "--allow-dangerously-skip-permissions",
            "--permission-mode",
            "--model",
            "--settings",
        ] {
            assert!(!resumed_args.iter().any(|arg| arg == forbidden));
        }
    }

    #[test]
    fn claude_direct_stream_emits_schema_valid_events_and_final_result() {
        let (events, done) = dispatch_recorded(full_display(), recorded_success_stream());
        let mut event_types = Vec::new();
        let mut bubbles = Vec::new();
        for event in events {
            match event {
                CoreEvent::AgentEvent(envelope) => {
                    validate_envelope(&envelope).unwrap();
                    assert_eq!(envelope["agentId"], "char-claude");
                    assert_eq!(envelope["sessionId"], "claude-session-1");
                    event_types.push(envelope["event"]["type"].as_str().unwrap().to_string());
                }
                CoreEvent::BubbleRequested { text, agent_id, .. } => bubbles.push((text, agent_id)),
                _ => {}
            }
        }
        for expected in [
            "session.started",
            "message.delta",
            "tool.started",
            "tool.completed",
            "message.completed",
            "session.completed",
        ] {
            assert!(
                event_types.iter().any(|actual| actual == expected),
                "missing {expected}"
            );
        }
        assert_eq!(done.thread_id.as_deref(), Some("claude-session-1"));
        assert_eq!(done.status, "completed");
        assert_eq!(done.result.as_deref(), Some("处理完成。"));
        assert_eq!(bubbles, vec![("处理完成。".into(), Some("char-claude".into()))]);
    }

    #[test]
    fn claude_hidden_stream_suppresses_raw_events_but_keeps_turn_result() {
        let (events, done) = dispatch_recorded(hidden_display(), recorded_success_stream());
        assert!(
            events.iter().all(|event| !matches!(
                event,
                CoreEvent::AgentEvent(_)
                    | CoreEvent::PetStateChanged { .. }
                    | CoreEvent::BubbleRequested { .. }
            )),
            "hidden workflow turns must not leak raw provider events"
        );
        assert_eq!(done.status, "completed");
        assert_eq!(done.result.as_deref(), Some("处理完成。"));
    }

    #[test]
    fn claude_failure_and_cancellation_have_distinct_terminal_statuses() {
        let failure = vec![
            json!({ "type": "system", "subtype": "init", "session_id": "failed-session" }),
            json!({
                "type": "result",
                "subtype": "error_during_execution",
                "is_error": true,
                "session_id": "failed-session",
                "result": "permission denied"
            }),
        ];
        let (events, failed) = dispatch_recorded(full_display(), failure);
        assert_eq!(failed.status, "error");
        assert_eq!(failed.result.as_deref(), Some("permission denied"));
        assert!(events.iter().any(|event| matches!(
            event,
            CoreEvent::AgentEvent(value) if value["event"]["type"] == "session.error"
        )));
        assert!(events.iter().all(|event| !matches!(event, CoreEvent::BubbleRequested { .. })));

        let (tx, mut events) = tokio::sync::broadcast::channel::<CoreEvent>(16);
        let (done_tx, mut done_rx) = tokio::sync::broadcast::channel(8);
        let turn = Arc::new(TurnState::new(
            2,
            "char-claude".into(),
            "claude-session-cancel".into(),
            full_display(),
        ));
        turn.mark_interrupted();
        finish_after_eof(&tx, &turn, &done_tx, "");

        let cancelled = done_rx.try_recv().expect("cancel TurnDone");
        assert_eq!(cancelled.status, "interrupted");
        assert_eq!(
            cancelled.thread_id.as_deref(),
            Some("claude-session-cancel")
        );
        let mut completed_count = 0;
        while let Ok(event) = events.try_recv() {
            if let CoreEvent::AgentEvent(value) = event {
                validate_envelope(&value).unwrap();
                if value["event"]["type"] == "session.completed" {
                    assert_eq!(value["event"]["outcome"], "cancelled");
                    completed_count += 1;
                }
            }
        }
        assert_eq!(completed_count, 1);
    }

    #[test]
    fn claude_empty_result_uses_nonempty_native_errors() {
        let (events, done) = dispatch_recorded(
            full_display(),
            vec![
                json!({
                    "type": "system",
                    "subtype": "init",
                    "session_id": "failed-empty-result"
                }),
                json!({
                    "type": "result",
                    "subtype": "error_during_execution",
                    "is_error": true,
                    "session_id": "failed-empty-result",
                    "result": "  ",
                    "errors": ["native provider permission failure"]
                }),
            ],
        );

        assert_eq!(done.status, "error");
        assert_eq!(
            done.result.as_deref(),
            Some("native provider permission failure")
        );
        assert!(events.iter().any(|event| matches!(
            event,
            CoreEvent::AgentEvent(value)
                if value["event"]["type"] == "session.error"
                    && value["event"]["message"] == "native provider permission failure"
        )));
    }

    #[test]
    fn claude_skill_install_and_path_match_the_codex_asset() {
        let base = std::env::temp_dir().join(format!("focus-claude-skill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let installed = install_focus_cli_skill_into(&base).unwrap();
        assert!(installed.ends_with(Path::new(".claude/skills/focus-cli/SKILL.md")));
        assert_eq!(
            std::fs::read_to_string(&installed).unwrap(),
            FOCUS_CLI_SKILL
        );
        assert_eq!(FOCUS_CLI_SKILL, crate::agents::codex::FOCUS_CLI_SKILL);

        let existing = std::ffi::OsString::from(r"C:\Windows\System32;C:\Tools");
        let path = claude_path_with_focus_cli(
            Path::new(r"C:\Focus\focus-desktop.exe"),
            Some(existing.clone()),
        );
        let entries: Vec<_> = std::env::split_paths(&path).collect();
        assert_eq!(entries.first(), Some(&PathBuf::from(r"C:\Focus")));
        assert_eq!(
            entries[1..],
            std::env::split_paths(&existing).collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn claude_tool_completes_only_after_the_native_tool_result() {
        let (tx, mut events) = tokio::sync::broadcast::channel::<CoreEvent>(32);
        let (done_tx, _) = tokio::sync::broadcast::channel(8);
        let turn = Arc::new(TurnState::new(
            3,
            "char-claude".into(),
            "claude-session-tool".into(),
            full_display(),
        ));
        dispatch_stream_message(
            &tx,
            &turn,
            &done_tx,
            json!({
                "type": "stream_event",
                "session_id": "claude-session-tool",
                "event": {
                    "type": "content_block_start",
                    "index": 0,
                    "content_block": {
                        "type": "tool_use",
                        "id": "toolu_native",
                        "name": "Bash",
                        "input": { "command": "focus-cli ping" }
                    }
                }
            }),
        );
        dispatch_stream_message(
            &tx,
            &turn,
            &done_tx,
            json!({
                "type": "stream_event",
                "session_id": "claude-session-tool",
                "event": { "type": "content_block_stop", "index": 0 }
            }),
        );

        let before_result: Vec<_> = std::iter::from_fn(|| events.try_recv().ok()).collect();
        assert!(!before_result.iter().any(|event| matches!(
            event,
            CoreEvent::AgentEvent(value) if value["event"]["type"] == "tool.completed"
        )));

        dispatch_stream_message(
            &tx,
            &turn,
            &done_tx,
            json!({
                "type": "user",
                "session_id": "claude-session-tool",
                "message": {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": "toolu_native",
                        "content": "pong",
                        "is_error": false
                    }]
                }
            }),
        );
        let completed = std::iter::from_fn(|| events.try_recv().ok())
            .filter(|event| {
                matches!(
                    event,
                    CoreEvent::AgentEvent(value) if value["event"]["type"] == "tool.completed"
                )
            })
            .count();
        assert_eq!(completed, 1);
    }
}
