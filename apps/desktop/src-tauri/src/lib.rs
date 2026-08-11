//! Focus Desktop spike (v1.2 visual & window-management round).
//! Windows: desktop (canvas), chat / stats / music / pet (12x8 grid floats,
//! frosted acrylic, collapsible to hidden), grid-overlay (drag preview),
//! topbar (focus status capsule). No AgentEvent protocol / event-name /
//! DB changes from the spike.

mod acrylic;
mod activity;
mod agents;
mod apps;
mod cli;
mod desktop_lock;
mod desktop_lock_escapes;
mod drag;
mod event_bus;
mod grid;
mod icons;
mod launch;
mod music;
mod pets;
mod settings;
mod shortcuts;
mod storage;
mod supervision;
mod wallpaper;
mod workflow;
mod workflow_engine;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Listener, Manager};
use tauri::{LogicalPosition, LogicalSize};

use event_bus::CoreEvent;
use grid::GridManager;
use settings::{GridRect, Settings, ShortcutType, Task};

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub data_dir: PathBuf,
    pub screen: Mutex<(f64, f64)>, // logical width/height
    pub active_drag: Mutex<Option<drag::ActiveDrag>>,
    /// Single-flight guard for shortcut launches (async, non-blocking).
    pub launch_lock: tokio::sync::Mutex<()>,
    pub focus_track: Mutex<supervision::FocusTrack>,
    pub focus_state: Mutex<String>,
    pub cli_pending: Mutex<HashMap<u64, std::sync::mpsc::Sender<serde_json::Value>>>,
    pub cli_next_id: AtomicU64,
    pub cli_token: Mutex<String>,
    /// v1.10: coalescer for raise_topbar (SetWindowPos churn, #31).
    pub last_topbar_raise: Mutex<std::time::Instant>,
    pub events_tx: tokio::sync::broadcast::Sender<CoreEvent>,
    /// M5 (ADR-0022): multi-Agent registry — one runtime per character.
    pub agents: Mutex<agents::AgentRegistry>,
    /// M4 workflow engine app layer (ADR-0012), initialized after the store.
    pub workflow: Mutex<Option<std::sync::Arc<workflow::WorkflowManager>>>,
    /// M5 (ADR-0022): the shared SQLite store (characters/session hashes).
    pub store: std::sync::Arc<std::sync::Mutex<storage::Store>>,
    /// v1.12.3: desktop-lock Drop guard kept alive for the process lifetime
    /// (a local in setup() would drop when setup returns, never restoring).
    pub _desktop_lock_guard: Mutex<Option<desktop_lock::DesktopLock>>,
}

const FLOAT_LABELS: [&str; 5] = ["chat", "stats", "music", "pet", "workflow"];

fn is_float_label(label: &str) -> bool {
    FLOAT_LABELS.contains(&label)
}

// ---------------------------------------------------------------------------
// window helpers
// ---------------------------------------------------------------------------

fn apply_acrylic_opt(w: &tauri::WebviewWindow, enabled: bool) {
    // Frosted glass via the SWCA acrylic API with our own low-alpha deep-green
    // tint. (window-vibrancy 0.8's apply_acrylic ignores the tint on Win11,
    // leaving the system's default light-gray backdrop.) Failure is
    // non-fatal; FOCUS_NO_ACRYLIC=1 skips it (CSS fallback) if WebView2 +
    // acrylic misbehaves.
    #[cfg(target_os = "windows")]
    {
        if !enabled || std::env::var_os("FOCUS_NO_ACRYLIC").is_some() {
            if let Ok(hwnd) = w.hwnd() {
                acrylic::clear(hwnd.0);
            }
            return;
        }
        if let Ok(hwnd) = w.hwnd() {
            acrylic::apply(hwnd.0, (14, 24, 18, 56));
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (w, enabled);
}

fn set_float_topmost_noactivate(w: &tauri::WebviewWindow, topmost: bool) {
    enforce_float_invariants(w);
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = w.hwnd() {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        let insert_after = if topmost {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        unsafe {
            let _ = SetWindowPos(
                HWND(hwnd.0 as *mut core::ffi::c_void),
                Some(insert_after),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        return;
    }
    #[cfg(not(target_os = "windows"))]
    let _ = w.set_always_on_top(topmost);
}

fn position_window(app: &tauri::AppHandle, label: &str, rect: &GridRect, gm: &GridManager) {
    if let Some(w) = app.get_webview_window(label) {
        if is_float_label(label) {
            enforce_float_invariants(&w);
        }
        let (x, y, wpx, hpx) = gm.rect_to_logical(rect);
        // v1.10: skip when already at the target (avoid Win32 churn under
        // rapid restore/collapse, #31). Getters are main-thread-only; all
        // callers run on the main thread.
        let scale = w.scale_factor().unwrap_or(1.0);
        let (px, py) = ((x * scale).round() as i32, (y * scale).round() as i32);
        let (pwp, php) = ((wpx * scale).round() as u32, (hpx * scale).round() as u32);
        // v1.10.3.1 (#48): position the *client* origin at the grid-cell
        // origin so the visible content is centered on the cell even when
        // the host window keeps a non-client frame (ncdelta ~15x9).
        let (ox, oy) = client_origin_offset(&w);
        let (cx, cy) = (px - ox, py - oy);
        let same = w.outer_position().map(|p| (p.x, p.y)).ok() == Some((cx, cy))
            && w.outer_size().map(|s| (s.width, s.height)).ok() == Some((pwp, php));
        if !same {
            // v1.10.2 (#35, ADR-0014): position changes move the native HWND
            // (no WebView2 SetBounds RPC per call); size changes still go
            // through the webview so the renderer relayouts.
            // v1.12.2: size path is ALSO native (SetWindowPos + SWP_NOACTIVATE)
            // — Tauri's set_size can activate the window and paint a caption
            // highlight (light-blue bar) while drag/resize preview is held.
            if !crate::drag::move_window_raw(&w, cx, cy) {
                #[cfg(not(target_os = "windows"))]
                let _ = w.set_position(LogicalPosition::new(
                    x - ox as f64 / scale,
                    y - oy as f64 / scale,
                ));
            }
            crate::drag::resize_window_raw(&w, pwp, php);
        }
    }
}

fn emit_visibility(app: &tauri::AppHandle, label: &str, visible: bool) {
    let _ = app.emit(
        "window:visibility",
        serde_json::json!({ "label": label, "visible": visible }),
    );
}

pub(crate) fn occupied_rects(settings: &Settings, except: Option<&str>) -> Vec<GridRect> {
    settings
        .grid
        .iter()
        .filter(|(k, _)| Some(k.as_str()) != except && !settings.collapsed.contains(*k))
        .map(|(_, r)| *r)
        .collect()
}

// ---------------------------------------------------------------------------
// commands
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    grid: HashMap<String, GridRect>,
    topmost: HashMap<String, bool>,
    collapsed: Vec<String>,
    wallpaper_path: Option<String>,
    shortcuts: Vec<storage::ShortcutRow>,
    acrylic_enabled: bool,
    focus_subtitle: String,
    tasks: Vec<Task>,
    current_task_id: Option<String>,
    focus_minutes: u32,
    rest_minutes: u32,
    distraction_apps: Vec<String>,
    allowed_apps: Vec<String>,
    supervision_enabled: bool,
    supervision_pause_until: Option<i64>,
    sound_enabled: bool,
    show_topbar: String,
    focus_mode: String,
    agent_workspace_dir: Option<String>,
    pet_bg_fade: bool,
}

#[tauri::command]
fn get_bootstrap(
    state: tauri::State<'_, AppState>,
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
) -> Bootstrap {
    let s = state.settings.lock().unwrap();
    let shortcuts = store
        .lock()
        .map(|st| st.list_shortcuts().unwrap_or_default())
        .unwrap_or_default();
    Bootstrap {
        grid: s.grid.clone(),
        topmost: s.topmost.clone(),
        collapsed: s.collapsed.clone(),
        wallpaper_path: s.wallpaper_path.clone(),
        shortcuts,
        acrylic_enabled: s.acrylic_enabled,
        focus_subtitle: s.focus_subtitle.clone(),
        tasks: s.tasks.clone(),
        current_task_id: s.current_task_id.clone(),
        focus_minutes: s.focus_minutes,
        rest_minutes: s.rest_minutes,
        distraction_apps: s.distraction_apps.clone(),
        allowed_apps: s.allowed_apps.clone(),
        supervision_enabled: s.supervision_enabled,
        supervision_pause_until: s.supervision_pause_until,
        sound_enabled: s.sound_enabled,
        show_topbar: s.show_topbar.clone(),
        focus_mode: s.focus_mode.clone(),
        agent_workspace_dir: s.agent_workspace_dir.clone(),
        pet_bg_fade: s.pet_bg_fade,
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentStatusView {
    character_id: String,
    provider: String,
    ready: bool,
    exe_path: Option<String>,
    workspace_dir: String,
}

fn user_home() -> String {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into())
}

fn select_status_character<'a>(
    characters: &'a [storage::CharacterRow],
    character_id: Option<&str>,
) -> Result<&'a storage::CharacterRow, String> {
    match character_id {
        Some(character_id) => characters
            .iter()
            .find(|character| character.id == character_id)
            .ok_or_else(|| format!("角色 {character_id} 不存在")),
        None => characters
            .first()
            .ok_or_else(|| "没有可用的 Agent 角色".to_string()),
    }
}

fn agent_status_for_character(
    character: &storage::CharacterRow,
    codex_path: &Option<String>,
    claude_path: &Option<String>,
) -> Result<AgentStatusView, String> {
    let provider = agents::AgentProviderKind::parse(&character.tool)
        .ok_or_else(|| format!("未知 Agent provider: {}", character.tool))?;
    let workspace_dir = character.workspace_dir.clone().unwrap_or_else(|| {
        PathBuf::from(user_home())
            .join("Focus-Agents")
            .join(&character.id)
            .to_string_lossy()
            .to_string()
    });
    let exe_path = match provider {
        agents::AgentProviderKind::Codex => codex_path.clone(),
        agents::AgentProviderKind::Claude => claude_path.clone(),
        #[cfg(test)]
        agents::AgentProviderKind::Mock => None,
    };
    Ok(AgentStatusView {
        character_id: character.id.clone(),
        provider: provider.as_str().to_string(),
        ready: provider_ready(provider, codex_path, claude_path),
        exe_path,
        workspace_dir,
    })
}

fn agent_status_view(
    app: &tauri::AppHandle,
    character_id: Option<&str>,
) -> Result<AgentStatusView, String> {
    let characters = app
        .state::<AppState>()
        .store
        .lock()
        .unwrap()
        .list_characters()
        .map_err(|error| error.to_string())?;
    let character = select_status_character(&characters, character_id)?;
    let codex_path = agents::codex::find_codex_exe().map(|p| p.to_string_lossy().to_string());
    let claude_path = agents::claude::find_claude_exe().map(|p| p.to_string_lossy().to_string());
    agent_status_for_character(character, &codex_path, &claude_path)
}

fn provider_ready(
    provider: agents::AgentProviderKind,
    codex_path: &Option<String>,
    claude_path: &Option<String>,
) -> bool {
    match provider {
        agents::AgentProviderKind::Codex => codex_path.is_some(),
        agents::AgentProviderKind::Claude => claude_path.is_some(),
        #[cfg(test)]
        agents::AgentProviderKind::Mock => false,
    }
}

fn emit_agent_status(app: &tauri::AppHandle, character_id: Option<&str>) {
    if let Ok(status) = agent_status_view(app, character_id) {
        let _ = app.emit("agent:status", status);
    }
}

/// Upgrade the one exact pre-provider Demo Pet once. The SQLite marker and
/// row update are committed together, so an unrelated settings write cannot
/// make a later launch repeat the migration.
fn bootstrap_existing_demo_pet_provider(store: &mut storage::Store) -> Result<bool, String> {
    store
        .upgrade_existing_demo_pet_to_claude_once()
        .map_err(|error| error.to_string())
}

fn bootstrap_existing_demo_pet_provider_durably(state: &AppState) -> Result<(), String> {
    let mut store = state.store.lock().unwrap();
    bootstrap_existing_demo_pet_provider(&mut store)?;
    Ok(())
}

/// M5 (ADR-0022): build (or reuse) the runtime for a character's Agent.
/// Lazily creates the per-Agent workspace + AGENTS.md when missing.
/// Returns the real Codex runtime. Mock runtimes exist only for Rust tests.
fn ensure_runtime_serialized(
    registry: &Mutex<agents::AgentRegistry>,
    character_id: &str,
    build: impl FnOnce() -> Result<agents::AgentRuntime, String>,
) -> Result<agents::AgentRuntime, String> {
    with_agent_runtime_serialized(
        registry,
        character_id,
        build,
        || {},
        |runtime| Ok(runtime.shared_clone()),
    )
}

fn with_agent_runtime_serialized<R>(
    registry: &Mutex<agents::AgentRegistry>,
    character_id: &str,
    build: impl FnOnce() -> Result<agents::AgentRuntime, String>,
    after_runtime_acquired: impl FnOnce(),
    action: impl FnOnce(&agents::AgentRuntime) -> Result<R, String>,
) -> Result<R, String> {
    let mut registry = registry.lock().unwrap();
    let runtime = registry.get_or_try_insert_with(character_id, build)?;
    after_runtime_acquired();
    let result = action(&runtime);
    if let Err(error) = &result {
        discard_runtime_after_provider_error(&mut registry, character_id, error);
    }
    result
}

pub fn ensure_agent_runtime(
    app: &tauri::AppHandle,
    character_id: &str,
) -> Result<agents::AgentRuntime, String> {
    let state = app.state::<AppState>();
    ensure_runtime_serialized(&state.agents, character_id, || {
        build_agent_runtime(&state, character_id)
    })
}

fn build_agent_runtime(
    state: &AppState,
    character_id: &str,
) -> Result<agents::AgentRuntime, String> {
    // Keep provider selection and runtime insertion serializable with
    // set_agent_provider. Lock order: registry -> store.
    let row = state
        .store
        .lock()
        .unwrap()
        .get_character(character_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("角色 {character_id} 不存在"))?;
    let workspace = ensure_agent_workspace(state, &row)?;
    let tx = state.events_tx.clone();
    match row.tool.as_str() {
        "codex" => {
            // M5 (ADR-0022): no fallback — if Codex is missing, the call
            // fails with a clear error; next use rebuilds lazily.
            let exe = agents::codex::find_codex_exe()
                .ok_or_else(|| "未找到 Codex（%LOCALAPPDATA%/OpenAI/Codex/bin）".to_string())?;
            let p = agents::codex::CodexProvider::new(tx, exe, character_id.to_string());
            Ok(agents::AgentRuntime::Codex(std::sync::Arc::new(
                std::sync::Mutex::new(p),
            )))
        }
        "claude" => {
            let exe = agents::claude::find_claude_exe().ok_or_else(|| {
                "未在 PATH 中找到 Claude CLI（claude.exe/claude.cmd）".to_string()
            })?;
            let p =
                agents::claude::ClaudeProvider::new(tx, exe, character_id.to_string(), workspace);
            Ok(agents::AgentRuntime::Claude(std::sync::Arc::new(
                std::sync::Mutex::new(p),
            )))
        }
        "mock" => Err("Mock provider is test-only; production requires a real provider".into()),
        other => Err(format!("未知 Agent provider: {other}")),
    }
}

/// M5 (ADR-0022): lazy-create `%USERPROFILE%/Focus-Agents/<agent-id>/AGENTS.md`.
/// AGENTS.md is the single identity source (persona is retired).
pub const AGENTS_MD_TEMPLATE: &str = "你是 Focus 桌宠 Agent「{name}」。请用简洁中文短句回答，句间用单个换行分隔；不要使用 Markdown、列表、代码块或长段落；总长度不超过约 200 字；只输出需要直接展示给用户看的内容。\n";

fn ensure_agent_workspace(state: &AppState, row: &storage::CharacterRow) -> Result<String, String> {
    let home = user_home();
    let dir = PathBuf::from(&home).join("Focus-Agents").join(&row.id);
    if !dir.is_dir() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建 Agent 工作区失败: {e}"))?;
        let md = AGENTS_MD_TEMPLATE.replace("{name}", &row.name);
        std::fs::write(dir.join("AGENTS.md"), md).map_err(|e| format!("写 AGENTS.md 失败: {e}"))?;
    }
    let ws = dir.to_string_lossy().to_string();
    let store = state.store.lock().unwrap();
    if row.workspace_dir.as_deref() != Some(ws.as_str()) {
        let _ = store.update_character_agent(
            &row.id,
            Some(&ws),
            row.current_session_hash.as_deref(),
            row.session_date.as_deref(),
        );
    }
    Ok(ws)
}

fn today_local() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn saved_session_for_today(
    row: Option<storage::ProviderSessionRow>,
    today: &str,
) -> Option<String> {
    row.filter(|session| session.session_date == today && !session.session_hash.trim().is_empty())
        .map(|session| session.session_hash)
}

fn provider_skills_dir(home: &Path, provider: agents::AgentProviderKind) -> PathBuf {
    match provider {
        agents::AgentProviderKind::Codex => home.join(".codex").join("skills"),
        agents::AgentProviderKind::Claude => home.join(".claude").join("skills"),
        #[cfg(test)]
        agents::AgentProviderKind::Mock => home.join(".focus-test").join("skills"),
    }
}

fn list_provider_skills(
    home: &Path,
    provider: agents::AgentProviderKind,
) -> Result<Vec<String>, String> {
    let dir = provider_skills_dir(home, provider);
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };
    let mut names = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with('.') && entry.path().join("SKILL.md").is_file() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn direct_user_message(message: &str) -> &str {
    message
}

/// Runs a provider call for a specific character's Agent. M5 (ADR-0022):
/// a dead process just drops this Agent's runtime — the next use rebuilds
/// it lazily (no fallback, no retry loop).
pub fn with_agent_for<R>(
    app: &tauri::AppHandle,
    character_id: &str,
    f: impl FnOnce(&agents::AgentRuntime) -> Result<R, String>,
) -> Result<R, String> {
    let state = app.state::<AppState>();
    with_agent_runtime_serialized(
        &state.agents,
        character_id,
        || build_agent_runtime(&state, character_id),
        || {},
        f,
    )
}

fn discard_runtime_after_provider_error(
    registry: &mut agents::AgentRegistry,
    character_id: &str,
    error: &str,
) {
    if !agents::is_busy_turn_error(error) {
        registry.runtimes.remove(character_id);
    }
}

#[tauri::command]
fn agent_status(
    app: tauri::AppHandle,
    character_id: Option<String>,
) -> Result<AgentStatusView, String> {
    agent_status_view(&app, character_id.as_deref())
}

#[tauri::command]
fn agent_start_thread(
    app: tauri::AppHandle,
    character_id: String,
    initial_message: String,
) -> Result<agents::AgentThreadInfo, String> {
    // ADR-0025: daily sessions are scoped by both character and provider.
    let state = app.state::<AppState>();
    let today = today_local();
    let prompt = direct_user_message(&initial_message);
    // M5 (ADR-0022): conversation = full display (stream + result both shown).
    let display = agents::agent_display_full();
    let (info, persistence) = with_agent_for(&app, &character_id, |runtime| {
        let provider = runtime.kind();
        let (saved_session, ws) = {
            let store = state.store.lock().unwrap();
            let row = store
                .load_provider_session(&character_id, provider.as_str())
                .map_err(|error| error.to_string())?;
            let ws = store
                .get_character(&character_id)
                .ok()
                .flatten()
                .and_then(|character| character.workspace_dir)
                .unwrap_or_else(user_home);
            (saved_session_for_today(row, &today), ws)
        };
        if let Some(session_id) = saved_session {
            let info = resume_with_initial_message(runtime, &session_id, &prompt, display)?;
            Ok((info, Ok(())))
        } else {
            let info = runtime.start_thread(&ws, &prompt, display)?;
            let persistence = (|| -> Result<(), String> {
                let store = state.store.lock().unwrap();
                store
                    .upsert_provider_session(&character_id, provider.as_str(), &info.id, &today)
                    .map_err(|error| error.to_string())?;
                if provider == agents::AgentProviderKind::Codex {
                    store
                        .update_character_agent(
                            &character_id,
                            Some(&ws),
                            Some(&info.id),
                            Some(&today),
                        )
                        .map_err(|error| error.to_string())?;
                }
                Ok(())
            })();
            Ok((info, persistence))
        }
    })?;
    persistence?;
    Ok(info)
}

fn resume_with_initial_message(
    rt: &agents::AgentRuntime,
    thread_id: &str,
    initial_message: &str,
    display: crate::workflow_engine::engine::AgentDisplay,
) -> Result<agents::AgentThreadInfo, String> {
    if initial_message.trim().is_empty() {
        with_agent_rt(rt, |r| r.resume_thread(thread_id))
    } else {
        with_agent_rt(rt, |r| {
            r.resume_and_send(thread_id, initial_message, display)
        })
    }
}

fn with_agent_rt<R>(
    rt: &agents::AgentRuntime,
    f: impl FnOnce(&agents::AgentRuntime) -> Result<R, String>,
) -> Result<R, String> {
    match rt {
        agents::AgentRuntime::Codex(p) => {
            let p2 = p.clone();
            let tmp = agents::AgentRuntime::Codex(p2);
            f(&tmp)
        }
        agents::AgentRuntime::Claude(p) => {
            let p2 = p.clone();
            let tmp = agents::AgentRuntime::Claude(p2);
            f(&tmp)
        }
        #[cfg(test)]
        agents::AgentRuntime::Mock(_) => f(rt),
    }
}

#[tauri::command]
fn agent_resume_thread(
    app: tauri::AppHandle,
    character_id: String,
    thread_id: String,
) -> Result<agents::AgentThreadInfo, String> {
    with_agent_for(&app, &character_id, |rt| rt.resume_thread(&thread_id))
}

#[tauri::command]
fn agent_list_threads(
    app: tauri::AppHandle,
    character_id: String,
) -> Result<Vec<agents::AgentThreadInfo>, String> {
    let mut threads = with_agent_for(&app, &character_id, |rt| rt.list_threads())?;
    // ADR-0012: hide cleaned automation threads and badge the rest.
    let hidden: std::collections::HashSet<String> = app
        .state::<AppState>()
        .workflow
        .lock()
        .unwrap()
        .as_ref()
        .map(|m| m.hidden_automation_thread_ids())
        .unwrap_or_default();
    threads.retain(|t| !hidden.contains(&t.id));
    let visible: std::collections::HashSet<String> = app
        .state::<AppState>()
        .workflow
        .lock()
        .unwrap()
        .as_ref()
        .map(|m| m.visible_automation_thread_ids())
        .unwrap_or_default();
    for t in &mut threads {
        t.automation = visible.contains(&t.id);
    }
    Ok(threads)
}

#[tauri::command]
fn agent_send(
    app: tauri::AppHandle,
    character_id: String,
    thread_id: String,
    text: String,
) -> Result<(), String> {
    // M5 (ADR-0022): conversation = full display.
    let display = agents::agent_display_full();
    let prompt = direct_user_message(&text);
    with_agent_for(&app, &character_id, |rt| {
        rt.send(&thread_id, &prompt, display)
    })
}

#[tauri::command]
fn agent_interrupt(
    app: tauri::AppHandle,
    character_id: String,
    thread_id: String,
) -> Result<(), String> {
    with_agent_for(&app, &character_id, |rt| rt.interrupt(&thread_id))
}

#[tauri::command]
fn agent_list_skills(
    app: tauri::AppHandle,
    character_id: Option<String>,
) -> Result<Vec<String>, String> {
    let characters = app
        .state::<AppState>()
        .store
        .lock()
        .unwrap()
        .list_characters()
        .map_err(|error| error.to_string())?;
    let character = select_status_character(&characters, character_id.as_deref())?;
    let provider = agents::AgentProviderKind::parse(&character.tool)
        .ok_or_else(|| format!("Unknown Agent provider: {}", character.tool))?;
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "USERPROFILE/HOME is not configured".to_string())?;
    list_provider_skills(Path::new(&home), provider)
    /*
    let home = std::env::var("USERPROFILE").map_err(|_| "USERPROFILE 未设置".to_string())?;
    let dir = std::path::PathBuf::from(home).join(".codex").join("skills");
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if entry.path().join("SKILL.md").is_file() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
    */
}

/// M5 (ADR-0022): delete an Agent — remove its workspace dir (AGENTS.md +
/// any session files) and clear the stored session hash.
#[tauri::command]
fn agent_delete(app: tauri::AppHandle, character_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let ws = {
        let store = state.store.clone();
        let s = store.lock().unwrap();
        s.get_character(&character_id)
            .ok()
            .flatten()
            .and_then(|c| c.workspace_dir)
    };
    if let Some(ws) = ws {
        let _ = std::fs::remove_dir_all(&ws);
    }
    {
        let store = state.store.clone();
        let s = store.lock().unwrap();
        let _ = s.update_character_agent(&character_id, None, None, None);
    }
    state.agents.lock().unwrap().runtimes.remove(&character_id);
    Ok(())
}

/// M5 (ADR-0022): open the Agent's workspace folder in explorer so the user
/// can edit AGENTS.md directly.
#[tauri::command]
fn agent_open_workspace(app: tauri::AppHandle, character_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let row = {
        let store = state.store.clone();
        let s = store.lock().unwrap();
        s.get_character(&character_id)
            .ok()
            .flatten()
            .ok_or_else(|| "角色不存在".to_string())?
    };
    let ws = ensure_agent_workspace(&state, &row)?;
    std::process::Command::new("explorer.exe")
        .arg(&ws)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// v1.12.2: UI-facing desktop lock (focus start). Fail-no-lock semantics live
/// in desktop_lock::lock_desktop.
#[tauri::command]
fn desktop_lock() -> Result<(), String> {
    crate::desktop_lock::lock_desktop()
}

/// v1.12.2: UI-facing desktop unlock (focus pause/skip/end).
#[tauri::command]
fn desktop_unlock() -> Result<(), String> {
    crate::desktop_lock::unlock_desktop()
}

/// UI-only focus lock: this intentionally differs from `desktop_lock`, which
/// remains the strict/full lock used by focus-cli.
#[tauri::command]
fn desktop_set_focus_lock(mode: String) -> Result<(), String> {
    crate::desktop_lock::set_focus_lock(&mode)
}

#[tauri::command]
fn agent_set_provider(
    app: tauri::AppHandle,
    character_id: String,
    provider: String,
) -> Result<AgentStatusView, String> {
    let state = app.state::<AppState>();
    let selected_id = agent_set_provider_serialized_with(
        &state.agents,
        &state.store,
        &character_id,
        &provider,
        || {},
    )?;
    let status = agent_status_view(&app, Some(&selected_id))?;
    let _ = app.emit("agent:status", status.clone());
    Ok(status)
}

fn agent_set_provider_serialized_with(
    registry: &Mutex<agents::AgentRegistry>,
    store: &Mutex<storage::Store>,
    character_id: &str,
    provider: &str,
    after_registry_lock: impl FnOnce(),
) -> Result<String, String> {
    let provider = agents::AgentProviderKind::parse(provider)
        .ok_or_else(|| "provider must be codex or claude".to_string())?;
    set_agent_provider_serialized_with(
        registry,
        store,
        Some(character_id),
        provider,
        after_registry_lock,
    )
}

fn set_agent_provider_serialized_with(
    registry: &Mutex<agents::AgentRegistry>,
    store: &Mutex<storage::Store>,
    character_id: Option<&str>,
    provider: agents::AgentProviderKind,
    after_registry_lock: impl FnOnce(),
) -> Result<String, String> {
    let mut registry = registry.lock().unwrap();
    after_registry_lock();
    let store = store.lock().unwrap();
    let characters = store.list_characters().map_err(|error| error.to_string())?;
    let selected_id = select_status_character(&characters, character_id)?
        .id
        .clone();
    if registry
        .get(&selected_id)
        .is_some_and(agents::AgentRuntime::has_active_turn)
    {
        return Err(agents::PROVIDER_SWITCH_BUSY_ERROR.to_string());
    }
    store
        .update_character_tool(&selected_id, provider.as_str())
        .map_err(|error| error.to_string())?;
    registry.runtimes.remove(&selected_id);
    Ok(selected_id)
}

#[tauri::command]
fn set_agent_workspace_dir(app: tauri::AppHandle, dir: String) -> Result<(), String> {
    let dir = dir.trim().to_string();
    if !dir.is_empty() {
        let p = std::path::PathBuf::from(&dir);
        if !p.is_dir() {
            return Err("目录不存在".into());
        }
    }
    let state = app.state::<AppState>();
    let mut s = state.settings.lock().unwrap();
    s.agent_workspace_dir = if dir.is_empty() { None } else { Some(dir) };
    let _ = s.save(&state.data_dir);
    Ok(())
}
#[tauri::command]
fn pet_import_pack(
    state: tauri::State<'_, AppState>,
    dir: String,
) -> Result<pets::PetInfo, String> {
    let info = pets::import(std::path::Path::new(&dir), &state.data_dir)?;
    {
        let mut s = state.settings.lock().unwrap();
        s.pet_pack_id = Some(info.id.clone());
        let _ = s.save(&state.data_dir);
    }
    Ok(info)
}

#[tauri::command]
fn pet_remove_pack(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let was_active = state.settings.lock().unwrap().pet_pack_id.as_deref() == Some(id.as_str());
    pets::remove(&state.data_dir, &id)?;
    if was_active {
        state.settings.lock().unwrap().pet_pack_id = None;
        let _ = state.settings.lock().unwrap().save(&state.data_dir);
    }
    Ok(())
}

#[tauri::command]
fn pet_list_packs(state: tauri::State<'_, AppState>) -> Result<Vec<pets::PetInfo>, String> {
    pets::list(&state.data_dir)
}

#[tauri::command]
fn pet_sheet_data(state: tauri::State<'_, AppState>, id: String) -> Result<String, String> {
    pets::sheet_base64(&state.data_dir, &id)
}

#[tauri::command]
fn pet_activate(state: tauri::State<'_, AppState>, id: String) -> Result<pets::PetInfo, String> {
    let info = pets::info_for(&state.data_dir, &id)?;
    {
        let mut s = state.settings.lock().unwrap();
        s.pet_pack_id = Some(id);
        let _ = s.save(&state.data_dir);
    }
    Ok(info)
}

#[tauri::command]
fn pet_active(state: tauri::State<'_, AppState>) -> Result<Option<pets::PetInfo>, String> {
    let id = state.settings.lock().unwrap().pet_pack_id.clone();
    match id {
        Some(id) => pets::info_for(&state.data_dir, &id).map(Some),
        None => Ok(None),
    }
}
#[tauri::command]
fn resize_preview(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
    visible: bool,
    cols: Option<usize>,
    rows: Option<usize>,
) -> Result<(), String> {
    let label = label.as_str();
    let Some(ov) = app.get_webview_window("grid-overlay") else {
        return Ok(());
    };
    if !visible {
        let _ = ov.hide();
        let _ = app.emit("grid:preview", serde_json::json!({ "visible": false }));
        return Ok(());
    }
    let _ = ov.set_ignore_cursor_events(true);
    let _ = ov.show();
    let cols = cols.unwrap_or(1);
    let rows = rows.unwrap_or(1);
    let settings = state.settings.lock().unwrap();
    let current = settings.grid.get(label).copied().unwrap_or(GridRect {
        col: 0,
        row: 0,
        cols,
        rows,
    });
    let occupied = occupied_rects(&settings, Some(label));
    let target = GridRect {
        col: current.col,
        row: current.row,
        cols,
        rows,
    };
    let conflict = occupied.iter().any(|o| crate::grid::overlap(&target, o));
    drop(settings);
    let _ = app.emit(
        "grid:preview",
        serde_json::json!({
            "visible": true,
            "label": label,
            "rect": target,
            "floatRect": {
                "x": target.col as f64,
                "y": target.row as f64,
                "w": target.cols as f64,
                "h": target.rows as f64,
            },
            "occupiedCells": occupied,
            "conflict": conflict,
        }),
    );
    Ok(())
}

#[tauri::command]
fn resize_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
    cols: usize,
    rows: usize,
) -> Result<GridRect, String> {
    let cols = cols.clamp(1, grid::GRID_COLS);
    let rows = rows.clamp(1, grid::GRID_ROWS);
    let (w, h) = *state.screen.lock().unwrap();
    let gm = GridManager {
        screen_w: w,
        screen_h: h,
    };
    let mut settings = state.settings.lock().unwrap();
    let current = settings.grid.get(&label).copied().unwrap_or(GridRect {
        col: 0,
        row: 0,
        cols,
        rows,
    });
    let rect = GridRect {
        col: current.col,
        row: current.row,
        cols,
        rows,
    };
    let occupied = occupied_rects(&settings, Some(&label));
    if occupied.iter().any(|o| crate::grid::overlap(&rect, o)) {
        // Reject conflicting resize: keep current size and window position.
        drop(settings);
        position_window(&app, &label, &current, &gm);
        return Err("目标尺寸与现有窗口重叠".into());
    }
    settings.grid.insert(label.clone(), rect);
    let _ = settings.save(&state.data_dir);
    drop(settings);
    position_window(&app, &label, &rect, &gm);
    Ok(rect)
}

#[tauri::command]
fn get_grid_metrics(state: tauri::State<'_, AppState>) -> grid::GridMetrics {
    let (w, h) = *state.screen.lock().unwrap();
    GridManager {
        screen_w: w,
        screen_h: h,
    }
    .metrics()
}

pub(crate) fn place_window_inner(
    app: &tauri::AppHandle,
    state: &AppState,
    label: &str,
    col: usize,
    row: usize,
) -> Result<GridRect, String> {
    let (w, h) = *state.screen.lock().unwrap();
    let gm = GridManager {
        screen_w: w,
        screen_h: h,
    };
    let mut settings = state.settings.lock().unwrap();
    let current = settings.grid.get(label).copied().unwrap_or(GridRect {
        col: 0,
        row: 0,
        cols: 2,
        rows: 2,
    });
    let occupied = occupied_rects(&settings, Some(label));
    match gm.place(label, &current, col, row, &occupied) {
        Ok(new_rect) => {
            settings.grid.insert(label.to_string(), new_rect);
            let _ = settings.save(&state.data_dir);
            position_window(app, label, &new_rect, &gm);
            raise_topbar(app);
            Ok(new_rect)
        }
        Err(()) => {
            // occupied: snap back to the current cell
            position_window(app, label, &current, &gm);
            raise_topbar(app);
            Ok(current)
        }
    }
}

#[tauri::command]
fn place_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
    col: usize,
    row: usize,
) -> Result<GridRect, String> {
    place_window_inner(&app, &state, &label, col, row)
}

#[tauri::command]
fn set_topmost(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
    topmost: bool,
) -> Result<(), String> {
    {
        let mut settings = state.settings.lock().unwrap();
        if settings.topmost.get(&label) == Some(&topmost) {
            return Ok(()); // v1.10: no-op when unchanged (#31)
        }
        settings.topmost.insert(label.clone(), topmost);
        let _ = settings.save(&state.data_dir);
    }
    if let Some(w) = app.get_webview_window(&label) {
        if is_float_label(&label) {
            set_float_topmost_noactivate(&w, topmost);
        } else {
            let _ = w.set_always_on_top(topmost);
        }
    }
    Ok(())
}

#[tauri::command]
fn collapse(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
) -> Result<(), String> {
    {
        let mut settings = state.settings.lock().unwrap();
        if settings.collapsed.contains(&label) {
            return Ok(()); // v1.10: no-op when already collapsed (#31)
        }
        settings.collapsed.push(label.clone());
        let _ = settings.save(&state.data_dir);
    }
    if let Some(w) = app.get_webview_window(&label) {
        hide_float_noactivate(&w);
    }
    emit_visibility(&app, &label, false);
    Ok(())
}

#[tauri::command]
fn restore(
    app: tauri::AppHandle,
    _state: tauri::State<'_, AppState>,
    label: String,
) -> Result<(), String> {
    restore_window(&app, &label)
}

/// Show + position a float window back on its grid slot (shared by the
/// restore command and the M4 `show_window` node).
pub(crate) fn restore_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    // v1.10: dedupe — restoring an already-visible window must not churn
    // show/position/topmost/raise (root cause of the freeze, #31).
    {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().unwrap();
        if !settings.collapsed.iter().any(|c| c == label) {
            if let Some(win) = app.get_webview_window(label) {
                if win.is_visible().unwrap_or(false) {
                    return Ok(());
                }
            }
        }
    }
    let state = app.state::<AppState>();
    let (w, h) = *state.screen.lock().unwrap();
    let gm = GridManager {
        screen_w: w,
        screen_h: h,
    };
    let default_rect = if label == "workflow" {
        GridRect {
            col: 4,
            row: 2,
            cols: 4,
            rows: 3,
        } // v1.10.2 (#36): 4x3
    } else {
        GridRect {
            col: 0,
            row: 0,
            cols: 2,
            rows: 2,
        }
    };
    // Resolve a free slot before changing persistent visibility.  A full grid
    // keeps this window collapsed, so it can never be shown on top of another.
    let (rect, topmost) = {
        let mut settings = state.settings.lock().unwrap();
        let desired = settings
            .grid
            .get(label)
            .copied()
            .unwrap_or(default_rect);
        let occupied = occupied_rects(&settings, Some(label));
        let rect = gm
            .restore_slot(label, &desired, &occupied)
            .map_err(|_| "No available grid position for this window".to_string())?;
        let mut changed = false;
        if rect != desired {
            settings.grid.insert(label.to_string(), rect);
            changed = true;
        }
        if settings.collapsed.iter().any(|c| c == label) {
            settings.collapsed.retain(|c| c != label);
            changed = true;
        }
        if changed {
            let _ = settings.save(&state.data_dir);
        }
        (rect, *settings.topmost.get(label).unwrap_or(&true))
    };
    if let Some(win) = app.get_webview_window(label) {
        set_float_topmost_noactivate(&win, topmost);
        show_float_noactivate(&win);
    }
    position_window(app, label, &rect, &gm);
    emit_visibility(app, label, true);
    raise_topbar(app);
    Ok(())
}

#[tauri::command]
fn get_wallpaper(state: tauri::State<'_, AppState>) -> Option<String> {
    state.settings.lock().unwrap().wallpaper_path.clone()
}

#[tauri::command]
fn persist_wallpaper(state: tauri::State<'_, AppState>, src: String) -> Result<String, String> {
    let path = wallpaper::import(&src, &state.data_dir)?;
    state.settings.lock().unwrap().wallpaper_path = Some(path.clone());
    let _ = state.settings.lock().unwrap().save(&state.data_dir);
    Ok(path)
}

#[tauri::command]
fn reset_wallpaper(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.settings.lock().unwrap().wallpaper_path = None;
    let _ = state.settings.lock().unwrap().save(&state.data_dir);
    Ok(())
}

/// First free icon cell (row-major from the top) that is not forbidden
/// (hero cols 3-9 x rows 0-3, dock row 7) and not already occupied.
fn free_cell_for(existing: &[storage::ShortcutRow]) -> (i64, i64) {
    let forbidden = |c: i64, r: i64| (c >= 3 && c <= 9 && r >= 0 && r <= 3) || r == 7;
    for row in 0i64..grid::GRID_ROWS as i64 {
        for col in 0i64..grid::GRID_COLS as i64 {
            if forbidden(col, row) {
                continue;
            }
            if !existing.iter().any(|e| e.col == col && e.row == row) {
                return (col, row);
            }
        }
    }
    (0, 4)
}

fn gen_shortcut_id(existing: &[storage::ShortcutRow]) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let mut i = 0u64;
    loop {
        let id = format!("sc-{ts}-{i}");
        if !existing.iter().any(|e| e.id == id) {
            return id;
        }
        i += 1;
    }
}

fn insert_new_shortcut(
    store: &std::sync::Arc<Mutex<storage::Store>>,
    name: String,
    kind: ShortcutType,
    target: String,
) -> Result<storage::ShortcutRow, String> {
    let st = store.lock().map_err(|e| e.to_string())?;
    let existing = st.list_shortcuts().map_err(|e| e.to_string())?;
    let (col, row) = free_cell_for(&existing);
    let row_ = storage::ShortcutRow {
        id: gen_shortcut_id(&existing),
        name,
        kind: kind.as_str().to_string(),
        target,
        col,
        row,
        fit_col: None,
        fit_row: None,
        fit_cols: None,
        fit_rows: None,
    };
    st.insert_shortcut(&row_).map_err(|e| e.to_string())?;
    Ok(row_)
}

#[tauri::command]
fn add_shortcut(
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    path: String,
) -> Result<storage::ShortcutRow, String> {
    // The Windows file dialog can hand back shell namespace paths (virtual
    // known folders). Keep only real filesystem paths so the launch engine
    // never pops a "???????" dialog.
    if path.starts_with("shell:::") || path.starts_with("::{") {
        return Err("??????????????? shell ??????".into());
    }
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("path not found: {path}"));
    }
    let name = shortcuts::display_name(&p);
    let kind = shortcuts::infer_type(&p);
    insert_new_shortcut(&store, name, kind, path)
}

#[tauri::command]
fn add_url_shortcut(
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    name: String,
    url: String,
) -> Result<storage::ShortcutRow, String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("URL 需以 http:// 或 https:// 开头".into());
    }
    let display = if name.trim().is_empty() {
        url.clone()
    } else {
        name.trim().to_string()
    };
    insert_new_shortcut(&store, display, ShortcutType::Url, url)
}

#[tauri::command]
fn add_internal_shortcut(
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    name: String,
    target: String,
) -> Result<storage::ShortcutRow, String> {
    if !matches!(target.as_str(), "chat" | "stats" | "music") {
        return Err("内部页 target 需为 chat|stats|music".into());
    }
    let display = if name.trim().is_empty() {
        match target.as_str() {
            "chat" => "对话",
            "stats" => "统计",
            _ => "音乐",
        }
        .to_string()
    } else {
        name.trim().to_string()
    };
    insert_new_shortcut(&store, display, ShortcutType::Internal, target)
}

#[tauri::command]
fn remove_shortcut(
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    id: String,
) -> Result<(), String> {
    store
        .lock()
        .map_err(|e| e.to_string())?
        .delete_shortcut(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn move_shortcut(
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    id: String,
    col: i64,
    row: i64,
) -> Result<(), String> {
    let col = col.clamp(0, (grid::GRID_COLS - 1) as i64);
    let row = row.clamp(0, (grid::GRID_ROWS - 1) as i64);
    store
        .lock()
        .map_err(|e| e.to_string())?
        .move_shortcut(&id, col, row)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_shortcut_fit(
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    id: String,
    col: i64,
    row: i64,
    cols: i64,
    rows: i64,
) -> Result<(), String> {
    let col = col.clamp(0, (grid::GRID_COLS - 1) as i64);
    let row = row.clamp(0, (grid::GRID_ROWS - 1) as i64);
    let cols = cols.clamp(1, grid::GRID_COLS as i64 - col);
    let rows = rows.clamp(1, grid::GRID_ROWS as i64 - row);
    store
        .lock()
        .map_err(|e| e.to_string())?
        .set_shortcut_fit(&id, col, row, cols, rows)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn launch_shortcut(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    id: String,
) -> Result<(), String> {
    // Keep the store guard inside a block so it is dropped before any await.
    let row = {
        let st = store.lock().map_err(|e| e.to_string())?;
        let rows = st.list_shortcuts().map_err(|e| e.to_string())?;
        rows.iter()
            .find(|r| r.id == id)
            .cloned()
            .ok_or("shortcut not found")?
    };

    // Single-flight: rapid clicks must not queue another blocking launch.
    // Async command runs off the UI thread; the blocking launch work is
    // moved to the tokio blocking pool so windows stay responsive.
    let _guard = state
        .launch_lock
        .try_lock()
        .map_err(|_| "另一个快捷方式正在启动，请稍候".to_string())?;

    // Internal shortcuts restore Focus windows; keep Tauri window APIs on the
    // main thread (run_on_main_thread posts, so this returns immediately).
    if ShortcutType::parse(&row.kind) == Some(ShortcutType::Internal) {
        let app2 = app.clone();
        let app3 = app2.clone();
        let row2 = row.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _ = app2.run_on_main_thread(move || {
                let _ = crate::restore(app3.clone(), app3.state::<AppState>(), row2.target.clone());
            });
        })
        .await
        .map_err(|e| e.to_string())?;
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || crate::launch::launch_shortcut(&app, &row))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn set_acrylic(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    {
        let mut s = state.settings.lock().unwrap();
        s.acrylic_enabled = enabled;
        let _ = s.save(&state.data_dir);
    }
    for label in ["chat", "stats", "music", "pet", "workflow"] {
        if let Some(w) = app.get_webview_window(label) {
            apply_acrylic_opt(&w, enabled);
        }
    }
    Ok(())
}

#[tauri::command]
fn save_task(state: tauri::State<'_, AppState>, task: Task) -> Result<Task, String> {
    let mut settings = state.settings.lock().unwrap();
    if task.id.is_empty() {
        return Err("task id required".into());
    }
    if let Some(existing) = settings.tasks.iter_mut().find(|t| t.id == task.id) {
        *existing = task.clone();
    } else {
        settings.tasks.push(task.clone());
    }
    let _ = settings.save(&state.data_dir);
    Ok(task)
}

#[tauri::command]
fn set_current_task(state: tauri::State<'_, AppState>, id: Option<String>) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.current_task_id = id;
    let _ = settings.save(&state.data_dir);
    Ok(())
}

#[tauri::command]
fn set_focus_durations(
    state: tauri::State<'_, AppState>,
    focus: u32,
    rest: u32,
) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.focus_minutes = focus.clamp(1, 240);
    settings.rest_minutes = rest.clamp(1, 120);
    let _ = settings.save(&state.data_dir);
    Ok(())
}

#[tauri::command]
fn set_focus_mode(state: tauri::State<'_, AppState>, mode: String) -> Result<(), String> {
    if !matches!(mode.as_str(), "light" | "standard" | "scholar") {
        return Err("invalid focus mode".into());
    }
    let mut settings = state.settings.lock().unwrap();
    settings.focus_mode = mode;
    settings.save(&state.data_dir)
}

#[tauri::command]
fn set_distraction_lists(
    state: tauri::State<'_, AppState>,
    black: Vec<String>,
    white: Vec<String>,
) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.distraction_apps = black;
    settings.allowed_apps = white;
    let _ = settings.save(&state.data_dir);
    Ok(())
}

#[tauri::command]
fn set_supervision_paused(app: tauri::AppHandle, minutes: i64) -> Result<(), String> {
    supervision::pause_for(&app, minutes)
}

#[tauri::command]
fn resume_supervision(app: tauri::AppHandle) -> Result<(), String> {
    supervision::resume(&app)
}

#[tauri::command]
fn set_supervision_enabled(state: tauri::State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.supervision_enabled = enabled;
    let _ = settings.save(&state.data_dir);
    Ok(())
}

#[tauri::command]
fn set_sound_enabled(state: tauri::State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.sound_enabled = enabled;
    let _ = settings.save(&state.data_dir);
    Ok(())
}

#[tauri::command]
fn set_pet_bg_fade(state: tauri::State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.pet_bg_fade = enabled;
    let _ = settings.save(&state.data_dir);
    Ok(())
}

#[tauri::command]
fn set_show_topbar(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    if !matches!(mode.as_str(), "auto" | "on" | "off") {
        return Err("showTopbar must be auto|on|off".into());
    }
    let app_state = app.state::<AppState>();
    let mut settings = app_state.settings.lock().unwrap();
    settings.show_topbar = mode;
    let _ = settings.save(&app_state.data_dir);
    drop(settings);
    apply_topbar_visibility(&app);
    Ok(())
}

/// Wall-clock seconds between two RFC3339 timestamps (v1.8.2): a focus
/// round records exact elapsed time even when skipped or when parts of it
/// were judged distraction/idle.
fn elapsed_sec(started_at: &str, ended_at: &str) -> Option<i64> {
    let start = chrono::DateTime::parse_from_rfc3339(started_at).ok()?;
    let end = chrono::DateTime::parse_from_rfc3339(ended_at).ok()?;
    Some(end.signed_duration_since(start).num_seconds())
}

#[tauri::command]
fn record_focus_session(
    app: tauri::AppHandle,
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    started_at: String,
    ended_at: String,
    duration_sec: i64,
    task_id: Option<String>,
) -> Result<(), String> {
    store
        .lock()
        .map_err(|e| e.to_string())?
        .record_focus_session(&started_at, &ended_at, duration_sec, task_id.as_deref())
        .map_err(|e| e.to_string())?;
    let _ = app.emit("stats:changed", ());
    Ok(())
}

#[tauri::command]
fn get_today_focus_summary(
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
) -> Result<(i64, i64), String> {
    store
        .lock()
        .unwrap()
        .today_focus_summary()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn stats_dashboard(
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
) -> Result<storage::DashboardPayload, String> {
    store
        .lock()
        .map_err(|e| e.to_string())?
        .dashboard()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn music_set_folder(app: tauri::AppHandle, dir: String) -> Result<Vec<music::Track>, String> {
    let path = std::path::PathBuf::from(&dir);
    if !path.is_dir() {
        return Err("音乐文件夹不存在".into());
    }
    {
        let app_state = app.state::<AppState>();
        let mut settings = app_state.settings.lock().unwrap();
        settings.music_folder = Some(dir.clone());
        settings.save(&app_state.data_dir)?;
    }
    app.asset_protocol_scope()
        .allow_directory(&path, true)
        .map_err(|e| e.to_string())?;
    Ok(music::list_tracks(&path))
}

#[tauri::command]
fn music_get_folder(state: tauri::State<'_, AppState>) -> Option<String> {
    state.settings.lock().unwrap().music_folder.clone()
}

#[tauri::command]
fn music_list(state: tauri::State<'_, AppState>) -> Result<Vec<music::Track>, String> {
    let folder = state.settings.lock().unwrap().music_folder.clone();
    match folder {
        Some(dir) => Ok(music::list_tracks(std::path::Path::new(&dir))),
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
fn music_cover(path: String) -> Result<Option<String>, String> {
    Ok(music::cover_data_uri(&path))
}

#[tauri::command]
fn get_shortcut_icon(path: String) -> Result<serde_json::Value, String> {
    match icons::extract_icon_rgba(&path) {
        Some(data) => Ok(serde_json::json!({
            "width": icons::ICON_SIZE,
            "height": icons::ICON_SIZE,
            "data": data,
        })),
        None => Err("no icon".into()),
    }
}

#[tauri::command]
fn list_running_apps() -> Vec<String> {
    apps::list_running_apps()
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    let _ = crate::desktop_lock::unlock_desktop();
    app.exit(0);
}

// ---------------------------------------------------------------------------
// window creation
// ---------------------------------------------------------------------------

/// v1.10.3.1 (#46): physical initial rect for a float at its saved grid slot.

const WM_NCCALCSIZE: u32 = 0x0083;
const FLOAT_SUBCLASS_ID: usize = 0x464F_4355;

fn float_nonclient_message_result(message: u32) -> Option<isize> {
    match message {
        WM_NCCALCSIZE => Some(0),
        _ => None,
    }
}

/// Uses comctl32's managed subclass chain instead of replacing GWLP_WNDPROC.
/// Windows otherwise retains an 15x8 non-client band after activation even
/// when the caption style bits have already been cleared.
#[cfg(target_os = "windows")]
unsafe extern "system" fn float_nonclient_subclass(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _subclass_id: usize,
    _reference_data: usize,
) -> windows::Win32::Foundation::LRESULT {
    if let Some(result) = float_nonclient_message_result(message) {
        return windows::Win32::Foundation::LRESULT(result);
    }
    unsafe {
        windows::Win32::UI::Shell::DefSubclassProc(hwnd, message, wparam, lparam)
    }
}

/// Strip the system frame from a float window so that outer == client and the
/// window exactly matches its grid rect (no white edge, #49).
fn strip_float_frame(w: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::SetWindowSubclass;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_BORDER, WS_DLGFRAME,
            WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
        };
        if let Ok(hwnd) = w.hwnd() {
            let hwnd_win = HWND(hwnd.0 as *mut core::ffi::c_void);
            unsafe {
                let style = GetWindowLongPtrW(hwnd_win, GWL_STYLE);
                let mask = !((WS_BORDER.0 as isize)
                    | (WS_DLGFRAME.0 as isize)
                    | (WS_SYSMENU.0 as isize)
                    | (WS_MINIMIZEBOX.0 as isize)
                    | (WS_MAXIMIZEBOX.0 as isize)
                    | (WS_THICKFRAME.0 as isize));
                let new_style = (style & mask) | (WS_POPUP.0 as isize);
                if new_style != style {
                    let _ = SetWindowLongPtrW(hwnd_win, GWL_STYLE, new_style);
                }
                let _ = SetWindowSubclass(
                    hwnd_win,
                    Some(float_nonclient_subclass),
                    FLOAT_SUBCLASS_ID,
                    0,
                );
                let _ = SetWindowPos(
                    hwnd_win,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
}

/// v1.10.3.1 (#48): physical offset of the client-area origin from the window
/// origin (non-client border). Used to position the *content* exactly at the
/// grid-cell origin even if the host window keeps a non-client frame.
pub(crate) fn client_origin_offset(w: &tauri::WebviewWindow) -> (i32, i32) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::{HWND, POINT, RECT};
        use windows::Win32::Graphics::Gdi::ClientToScreen;
        use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};
        if let Ok(hwnd) = w.hwnd() {
            let hwnd_win = HWND(hwnd.0 as *mut core::ffi::c_void);
            unsafe {
                let mut wr = RECT::default();
                let mut cr = RECT::default();
                if GetWindowRect(hwnd_win, &mut wr).is_ok()
                    && GetClientRect(hwnd_win, &mut cr).is_ok()
                {
                    let mut pt = POINT {
                        x: cr.left,
                        y: cr.top,
                    };
                    if ClientToScreen(hwnd_win, &mut pt).as_bool() {
                        return (pt.x - wr.left, pt.y - wr.top);
                    }
                }
            }
        }
    }
    (0, 0)
}

/// v1.10.4 (#50): client-area geometry (origin offset + size, physical px) for
/// the drag preview so the brightness center tracks the visible content.
pub(crate) fn client_geometry(w: &tauri::WebviewWindow) -> (i32, i32, u32, u32) {
    let (ox, oy) = client_origin_offset(w);
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
        if let Ok(hwnd) = w.hwnd() {
            let hwnd_win = windows::Win32::Foundation::HWND(hwnd.0 as *mut core::ffi::c_void);
            unsafe {
                let mut cr = RECT::default();
                if GetClientRect(hwnd_win, &mut cr).is_ok() {
                    return (ox, oy, cr.right as u32, cr.bottom as u32);
                }
            }
        }
    }
    (ox, oy, 0, 0)
}

fn initial_float_rect(
    grid: &std::collections::HashMap<String, GridRect>,
    collapsed: &[String],
    gm: &GridManager,
    label: &str,
    def: GridRect,
) -> (f64, f64, f64, f64, bool) {
    let rect = grid.get(label).copied().unwrap_or(def);
    let (x, y, w, h) = gm.rect_to_logical(&rect);
    (x, y, w, h, collapsed.iter().any(|c| c == label))
}

/// v1.12.3: floats must never become the active window — activation paints
/// the system caption highlight (the light-blue bar). Same treatment as the
/// grid-overlay (v1.7.2).
fn float_noactivate(w: &tauri::WebviewWindow) {
    if let Ok(hwnd) = w.hwnd() {
        acrylic::noactivate(hwnd.0);
    }
}

/// Re-assert the native invariants required by every floating window.
///
/// This is intentionally called only from main-thread lifecycle paths. The
/// drag poller must stay limited to asynchronous native movement so it never
/// dispatches synchronous window operations while the renderer is busy.
pub(crate) fn enforce_float_invariants(w: &tauri::WebviewWindow) {
    strip_float_frame(w);
    float_noactivate(w);
}

fn show_float_noactivate(w: &tauri::WebviewWindow) {
    enforce_float_invariants(w);
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = w.hwnd() {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
        unsafe {
            let _ = ShowWindow(HWND(hwnd.0 as *mut core::ffi::c_void), SW_SHOWNOACTIVATE);
        }
        return;
    }
    #[cfg(not(target_os = "windows"))]
    let _ = w.show();
}

fn hide_float_noactivate(w: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = w.hwnd() {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindowAsync, SW_HIDE};
        unsafe {
            let _ = ShowWindowAsync(HWND(hwnd.0 as *mut core::ffi::c_void), SW_HIDE);
        }
        return;
    }
    #[cfg(not(target_os = "windows"))]
    let _ = w.hide();
}

fn create_windows(app: &mut tauri::App) -> tauri::Result<()> {
    let url = tauri::WebviewUrl::App("index.html".into());
    // v1.10.3.1 (#46/#48): floats are born at their saved grid rect so they
    // never stack at the default size/position; collapsed windows stay hidden.
    let (sw, sh) = *app.state::<AppState>().screen.lock().unwrap();
    let gm = GridManager {
        screen_w: sw,
        screen_h: sh,
    };
    let grid = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .grid
        .clone();
    let collapsed = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .collapsed
        .clone();

    tauri::WebviewWindowBuilder::new(app, "desktop", url.clone())
        .title("Focus Desktop")
        .fullscreen(true)
        .decorations(false)
        .build()?;

    let (chat_px, chat_py, chat_pw, chat_ph, chat_collapsed) = initial_float_rect(
        &grid,
        &collapsed,
        &gm,
        "chat",
        GridRect {
            col: 0,
            row: 0,
            cols: 2,
            rows: 2,
        },
    );
    let chat = tauri::WebviewWindowBuilder::new(app, "chat", url.clone())
        .title("对话")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .background_color(tauri::window::Color::from((0, 0, 0, 0))) // v1.10.3.1 (#42/#48)
        .position(chat_px, chat_py)
        .inner_size(chat_pw, chat_ph)
        .visible(false)
        .build()?;
    enforce_float_invariants(&chat);

    let (stats_px, stats_py, stats_pw, stats_ph, stats_collapsed) = initial_float_rect(
        &grid,
        &collapsed,
        &gm,
        "stats",
        GridRect {
            col: 0,
            row: 0,
            cols: 2,
            rows: 2,
        },
    );
    let stats = tauri::WebviewWindowBuilder::new(app, "stats", url.clone())
        .title("统计")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .background_color(tauri::window::Color::from((0, 0, 0, 0))) // v1.10.3.1 (#42/#48)
        .position(stats_px, stats_py)
        .inner_size(stats_pw, stats_ph)
        .visible(false)
        .build()?;
    enforce_float_invariants(&stats);

    let (music_px, music_py, music_pw, music_ph, music_collapsed) = initial_float_rect(
        &grid,
        &collapsed,
        &gm,
        "music",
        GridRect {
            col: 0,
            row: 0,
            cols: 2,
            rows: 2,
        },
    );
    let music = tauri::WebviewWindowBuilder::new(app, "music", url.clone())
        .title("音乐")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .background_color(tauri::window::Color::from((0, 0, 0, 0))) // v1.10.3.1 (#42/#48)
        .position(music_px, music_py)
        .inner_size(music_pw, music_ph)
        .visible(false)
        .build()?;
    enforce_float_invariants(&music);

    let (pet_px, pet_py, pet_pw, pet_ph, pet_collapsed) = initial_float_rect(
        &grid,
        &collapsed,
        &gm,
        "pet",
        GridRect {
            col: 0,
            row: 0,
            cols: 2,
            rows: 2,
        },
    );
    let pet = tauri::WebviewWindowBuilder::new(app, "pet", url.clone())
        .title("桌宠")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .background_color(tauri::window::Color::from((0, 0, 0, 0))) // v1.10.3.1 (#42/#48)
        .position(pet_px, pet_py)
        .inner_size(pet_pw, pet_ph)
        .visible(false)
        .build()?;
    enforce_float_invariants(&pet);

    let (workflow_px, workflow_py, workflow_pw, workflow_ph, workflow_collapsed) =
        initial_float_rect(
            &grid,
            &collapsed,
            &gm,
            "workflow",
            GridRect {
                col: 0,
                row: 2,
                cols: 6,
                rows: 5,
            },
        ); // v1.10.4 (#51) default 6x5
    let workflow = tauri::WebviewWindowBuilder::new(app, "workflow", url.clone())
        .title("工作流")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .background_color(tauri::window::Color::from((0, 0, 0, 0))) // v1.10.3.1 (#42/#48)
        .position(workflow_px, workflow_py)
        .inner_size(workflow_pw, workflow_ph)
        .visible(false)
        .build()?;
    enforce_float_invariants(&workflow);

    for (label, collapsed) in [
        ("chat", chat_collapsed),
        ("stats", stats_collapsed),
        ("music", music_collapsed),
        ("pet", pet_collapsed),
        ("workflow", workflow_collapsed),
    ] {
        if !collapsed {
            if let Some(window) = app.get_webview_window(label) {
                show_float_noactivate(&window);
            }
        }
    }

    let overlay = tauri::WebviewWindowBuilder::new(app, "grid-overlay", url.clone())
        .title("Grid Overlay")
        .fullscreen(true)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
    overlay.set_ignore_cursor_events(true)?;
    if let Ok(hwnd) = overlay.hwnd() {
        acrylic::noactivate(hwnd.0);
    }

    let topbar = tauri::WebviewWindowBuilder::new(app, "topbar", url.clone())
        .title("状态")
        .inner_size(500.0, 44.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
    // informational only: never intercept mouse clicks on apps underneath
    topbar.set_ignore_cursor_events(true)?;

    Ok(())
}

/// Whether the floating status capsule (topbar window) should be visible.
pub fn topbar_visible(mode: &str, focus_state: &str) -> bool {
    mode == "on" || (mode == "auto" && focus_state != "idle")
}

fn apply_topbar_visibility(app: &tauri::AppHandle) {
    let app_state = app.state::<AppState>();
    let mode = app_state.settings.lock().unwrap().show_topbar.clone();
    let state = app_state.focus_state.lock().unwrap().clone();
    let visible = topbar_visible(&mode, &state);
    if let Some(w) = app.get_webview_window("topbar") {
        if visible {
            show_float_noactivate(&w);
            raise_topbar(app);
        } else {
            hide_float_noactivate(&w);
        }
    }
}

/// Re-assert the status capsule (topbar) above every always-on-top float:
/// a float shown/restored after the topbar would otherwise cover it.
///
/// `set_always_on_top(true)` alone does NOT reorder an already-topmost window
/// above its peers, so on Windows we raise the raw HWND with
/// `SetWindowPos(HWND_TOPMOST)` (verified: Tauri's re-assert leaves the float
/// on top, the native call fixes it).
pub(crate) fn raise_topbar(app: &tauri::AppHandle) {
    // v1.10: coalesce SetWindowPos churn (#31) — at most one raise per 150ms.
    {
        let state = app.state::<AppState>();
        let mut last = state.last_topbar_raise.lock().unwrap();
        if last.elapsed() < std::time::Duration::from_millis(150) {
            return;
        }
        *last = std::time::Instant::now();
    }
    let Some(w) = app.get_webview_window("topbar") else {
        return;
    };
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        if let Ok(hwnd) = w.hwnd() {
            // tauri links windows 0.61 while we depend on 0.62; convert via
            // the raw pointer (both HWNDs wrap *mut c_void).
            let hwnd_win = windows::Win32::Foundation::HWND(hwnd.0 as *mut core::ffi::c_void);
            unsafe {
                let _ = SetWindowPos(
                    hwnd_win,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            return;
        }
    }
    let _ = w.set_always_on_top(true);
}

/// Startup reconciliation: re-hide collapsed floats shortly after the event
/// loop starts, in case a webview showed one during the boot race. Visible
/// floats are left untouched (no re-position).
fn sync_collapsed(app: &tauri::AppHandle, state: &AppState) {
    let collapsed = state.settings.lock().unwrap().collapsed.clone();
    for label in ["chat", "stats", "music", "pet", "workflow"] {
        if collapsed.contains(&label.to_string()) {
            if let Some(w) = app.get_webview_window(label) {
                hide_float_noactivate(&w);
            }
        }
    }
}

fn apply_initial_layout(app: &tauri::App, state: &AppState) {
    let (w, h) = *state.screen.lock().unwrap();
    let gm = GridManager {
        screen_w: w,
        screen_h: h,
    };
    let (resolved, collapsed, topmost) = {
        let mut settings = state.settings.lock().unwrap();
        let saved = FLOAT_LABELS
            .iter()
            .map(|label| {
                let default_rect = if *label == "workflow" {
                    GridRect {
                        col: 4,
                        row: 2,
                        cols: 4,
                        rows: 4,
                    }
                } else {
                    GridRect {
                        col: 0,
                        row: 0,
                        cols: 2,
                        rows: 2,
                    }
                };
                (
                    (*label).to_string(),
                    settings.grid.get(*label).copied().unwrap_or(default_rect),
                )
            })
            .collect::<Vec<_>>();
        let (resolved, overflow) = gm.reconcile_visible_rects(&saved, &settings.collapsed);
        let mut changed = false;
        for (label, rect) in &resolved {
            if settings.grid.get(label) != Some(rect) {
                settings.grid.insert(label.clone(), *rect);
                changed = true;
            }
        }
        for label in overflow {
            if !settings.collapsed.contains(&label) {
                settings.collapsed.push(label);
                changed = true;
            }
        }
        if changed {
            let _ = settings.save(&state.data_dir);
        }
        (
            resolved,
            settings.collapsed.clone(),
            settings.topmost.clone(),
        )
    };

    for label in FLOAT_LABELS {
        if let Some(rect) = resolved.get(label) {
            position_window(&app.handle(), label, rect, &gm);
        }
        if let Some(win) = app.get_webview_window(label) {
            let top = *topmost.get(label).unwrap_or(&true);
            set_float_topmost_noactivate(&win, top);
            let visible = resolved.contains_key(label) && !collapsed.contains(&label.to_string());
            if !visible {
                hide_float_noactivate(&win);
            }
            emit_visibility(&app.handle(), label, visible);
        }
    }
}

// ---------------------------------------------------------------------------
// entry
// ---------------------------------------------------------------------------

pub fn run() {
    // v1.11.2/v1.12.1: VPN/proxy compatibility — WebView2 (Chromium) routes
    // even loopback hosts through the system proxy when a VPN/Clash is on,
    // breaking the local tauri:// protocol (blank windows). MERGE into any
    // pre-existing WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS (e.g. a debug port)
    // instead of skipping when one exists — the old `if is_none()` let an
    // existing env var disable the bypass entirely.
    let bypass = "--proxy-bypass-list=<-loopback>";
    match std::env::var_os("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS") {
        Some(existing) => {
            let mut val = existing.to_string_lossy().to_string();
            if !val.contains("proxy-bypass-list") {
                val.push(' ');
                val.push_str(bypass);
            }
            std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", val);
        }
        None => std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", bypass),
    }
    // v1.12 dev-only watchdog mode: restore the desktop if the parent dies.
    // The watchdog child re-launches this exe with --focus-watchdog <pid>.
    // v1.12.3: poll with WaitForSingleObject(h, 0) instead of blocking
    // forever — a process object only signals when the LAST handle closes,
    // and the watchdog's own handle would deadlock a blocking wait.
    let mut args = std::env::args();
    if args.nth(1).as_deref() == Some("--focus-watchdog") {
        let pid: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        if pid != 0 {
            unsafe {
                use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
                use windows::Win32::System::Threading::{
                    OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
                    PROCESS_SYNCHRONIZE,
                };
                // PROCESS_SYNCHRONIZE is required for WaitForSingleObject on a process
                // handle; PROCESS_QUERY_LIMITED_INFORMATION alone yields
                // WAIT_FAILED on modern Windows and the watchdog would exit
                // before the parent actually dies.
                let access = PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION;
                if let Ok(h) = OpenProcess(access, false, pid) {
                    loop {
                        // WAIT_OBJECT_0 = parent dead (signaled). Any other
                        // return (WAIT_TIMEOUT, WAIT_FAILED) means the parent
                        // is still around; poll again so the watchdog only
                        // restores the shell after the parent actually dies.
                        if WaitForSingleObject(h, 0) == WAIT_OBJECT_0 {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                    let _ = CloseHandle(h);
                }
            }
            let _ = desktop_lock::restore_desktop_after_process_exit();
            std::process::exit(0);
        }
    }
    // v1.8.1 single-instance guard: a second process must exit immediately so
    // two instances never share the same SQLite DB / settings (which could
    // silently drop writes). The mutex handle lives for the process lifetime
    // and is released by the OS on exit.
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{
            GetLastError, SetLastError, ERROR_ALREADY_EXISTS, WIN32_ERROR,
        };
        use windows::Win32::System::Threading::CreateMutexW;
        let name = windows::core::HSTRING::from("Local\\FocusDesktop_SingleInstance");
        unsafe {
            SetLastError(WIN32_ERROR(0));
        }
        match unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) } {
            Ok(handle) => {
                if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
                    std::process::exit(0);
                }
                // Keep the handle alive for the whole process.
                std::mem::forget(handle);
            }
            Err(e) => eprintln!("[focus] single-instance mutex failed: {e}"),
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // screen size (logical)
            let (sw, sh) = app
                .primary_monitor()
                .ok()
                .flatten()
                .map(|m| {
                    let size = m.size(); // physical
                    let scale = m.scale_factor();
                    (size.width as f64 / scale, size.height as f64 / scale)
                })
                .unwrap_or((1536.0, 960.0));

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let settings = Settings::load(&data_dir);
            // v1.9: re-allow the saved music folder in the asset protocol scope
            // (scope is per-process; it only covers $APPDATA/** by default).
            if let Some(music_folder) = settings.music_folder.clone() {
                let _ = app
                    .asset_protocol_scope()
                    .allow_directory(std::path::Path::new(&music_folder), true);
            }
            let legacy_shortcuts = settings.shortcuts.clone();
            let data_dir_clone = data_dir.clone();
            let (events_tx, _boot_rx) = tokio::sync::broadcast::channel::<CoreEvent>(256);
            // M5 (ADR-0022): open the store BEFORE manage() (AppState holds the
            // shared Arc; state() must not be called pre-manage).
            let store_arc = std::sync::Arc::new(Mutex::new(storage::Store::open(
                &data_dir.join("spike.db"),
            )?));
            let state = AppState {
                settings: Mutex::new(settings),
                data_dir,
                screen: Mutex::new((sw, sh)),
                active_drag: Mutex::new(None),
                launch_lock: tokio::sync::Mutex::new(()),
                focus_track: Mutex::new(supervision::FocusTrack::default()),
                focus_state: Mutex::new("idle".to_string()),
                cli_pending: Mutex::new(HashMap::new()),
                cli_next_id: AtomicU64::new(0),
                cli_token: Mutex::new(String::new()),
                last_topbar_raise: Mutex::new(std::time::Instant::now()),
                events_tx: events_tx.clone(),
                // M5 (ADR-0022): registry starts empty; runtimes are created
                // per character on first use (lazy).
                agents: Mutex::new(agents::AgentRegistry::new()),
                workflow: Mutex::new(None),
                store: store_arc,
                // v1.12.3: guard lives with AppState → dropped only at process exit.
                _desktop_lock_guard: Mutex::new(Some(desktop_lock::DesktopLock)),
            };
            app.manage(state);
            // M5 (ADR-0022): no upfront runtime — agents are built lazily per
            // character on first use (ensure_agent_runtime).

            // v1.5: DB store must be managed before the desktop webview calls
            // get_bootstrap on mount (otherwise: state not managed for field
            // `store`, observed as the v1.5 empty-shortcut regression).
            let store = app.state::<AppState>().store.clone();
            store.lock().unwrap().migrate()?;
            app.manage(store.clone());
            {
                let store_guard = store.lock().unwrap();
                let _ = store_guard.migrate_shortcuts_from_settings(&legacy_shortcuts);
            }

            let _ = desktop_lock::restore_desktop_after_process_exit();
            let app_handle = app.handle().clone();
            let wm = std::sync::Arc::new(workflow::WorkflowManager::new(
                app_handle.clone(),
                store.clone(),
            ));
            wm.purge_incompatible();
            let _ = wm.ensure_characters();
            if let Err(error) =
                bootstrap_existing_demo_pet_provider_durably(&app_handle.state::<AppState>())
            {
                eprintln!("[agent] Demo Pet provider bootstrap failed: {error}");
            }
            *app_handle.state::<AppState>().workflow.lock().unwrap() = Some(wm.clone());

            create_windows(app)?;

            // frosted glass on floating windows (respects settings toggle)
            let acrylic_enabled = app
                .state::<AppState>()
                .settings
                .lock()
                .unwrap()
                .acrylic_enabled;
            for label in ["chat", "stats", "music", "pet", "workflow", "topbar"] {
                if let Some(w) = app.get_webview_window(label) {
                    apply_acrylic_opt(&w, acrylic_enabled);
                }
            }

            let app_state = app.state::<AppState>();
            apply_initial_layout(app, &app_state);

            // always-on-top status capsule: top-center of the primary screen
            if let Some(tb) = app.get_webview_window("topbar") {
                let _ = tb.set_position(LogicalPosition::new(((sw - 500.0) / 2.0).max(0.0), 8.0));
            }
            apply_topbar_visibility(&app.handle());
            // defensive re-apply shortly after the event loop starts: the
            // first apply can race with window registration (observed once as
            // the capsule briefly showing in idle), this guarantees the
            // configured visibility wins.
            {
                let h = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
                    apply_topbar_visibility(&h);
                    let st = h.state::<AppState>();
                    sync_collapsed(&h, &st);
                });
            }

            // core event bus + relay
            let rx = app.state::<AppState>().events_tx.subscribe();
            let tx = app.state::<AppState>().events_tx.clone();
            tauri::async_runtime::spawn(event_bus::relay_task(app_handle.clone(), rx));

            emit_agent_status(&app_handle, None);

            // v1.5: local CLI control plane (focus-cli)
            cli::spawn(app_handle.clone(), store.clone(), data_dir_clone);

            // M4 workflow engine (ADR-0012): manager + scheduler + bus hooks
            {
                // v1.10.5 (#62): no backward compatibility — drop workflows
                // containing removed node kinds at startup.
                let wm_tick = wm.clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_secs(
                        workflow::SCHEDULER_TICK_SEC,
                    ));
                    wm_tick.scheduler_tick();
                });
                let wm_events = wm.clone();
                let mut rx_wf = tx.subscribe();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match rx_wf.recv().await {
                            Ok(ev) => wm_events.on_core_event(&ev),
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        }
                    }
                });
            }
            let tx_probe = tx.clone();
            activity::spawn_probe(tx_probe, store.clone());
            supervision::spawn(app_handle.clone(), store);

            // ---- frontend -> core listeners ----
            let h = app.handle().clone();

            let h5 = h.clone();
            h5.clone().listen("ui:toggle_chat", move |_event| {
                let state = h5.state::<AppState>();
                let collapsed = state
                    .settings
                    .lock()
                    .unwrap()
                    .collapsed
                    .contains(&"chat".to_string());
                if collapsed {
                    let _ = restore(h5.clone(), state.clone(), "chat".to_string());
                } else if let Some(w) = h5.get_webview_window("chat") {
                    let visible = w.is_visible().unwrap_or(true);
                    if visible {
                        hide_float_noactivate(&w);
                    } else {
                        show_float_noactivate(&w);
                    }
                    emit_visibility(&h5, "chat", !visible);
                    raise_topbar(&h5);
                }
            });

            let h6 = h.clone();
            h6.clone().listen("music:playback_tick", move |event| {
                let v: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or_default();
                let position_ms = v.get("positionMs").and_then(|x| x.as_u64()).unwrap_or(0);
                let duration_ms = v.get("durationMs").and_then(|x| x.as_u64()).unwrap_or(0);
                let _ = tx.send(CoreEvent::MusicTick {
                    position_ms,
                    duration_ms,
                });
            });

            // natural-release signal from the drag poller: finalize on the main
            // thread (window getters are main-thread-only)
            let hd = h.clone();
            hd.clone().listen("drag:released", move |event| {
                let v: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or_default();
                let label = v
                    .get("label")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                if !label.is_empty() {
                    drag::finalize(&hd, &label);
                }
            });

            // frontend focus timer -> supervision focus tracking + session record
            let hf = h.clone();
            hf.clone().listen("focus:state_changed", move |event| {
                let v: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or_default();
                let state = v
                    .get("state")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let paused = v.get("paused").and_then(|x| x.as_bool()).unwrap_or(false);
                let completed = v
                    .get("completed")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                let app_state = hf.state::<AppState>();
                *app_state.focus_state.lock().unwrap() = state.clone();
                // M4/ADR-0012: focus-end trigger via the core event bus
                let _ = app_state.events_tx.send(CoreEvent::FocusStateChanged {
                    state: state.clone(),
                    completed,
                });
                let mut ft = app_state.focus_track.lock().unwrap();
                match state.as_str() {
                    "focus" => {
                        if !ft.active {
                            let settings = app_state.settings.lock().unwrap();
                            ft.task_id = settings.current_task_id.clone();
                            ft.session_started_at = Some(chrono::Local::now().to_rfc3339());
                            ft.session_focus_sec = 0;
                        }
                        ft.active = true;
                        ft.paused = paused;
                    }
                    "rest" => {
                        // v1.8.2: any focus-round end (natural completion OR skip)
                        // records wall-clock elapsed time; distracted/idle periods
                        // inside the round still count as focus.
                        if ft.active {
                            let started = ft.session_started_at.clone().unwrap_or_default();
                            let ended = chrono::Local::now().to_rfc3339();
                            let dur = elapsed_sec(&started, &ended)
                                .unwrap_or_else(|| ft.session_focus_sec.max(1));
                            let tid = ft.task_id.clone();
                            let store_state = hf.state::<std::sync::Arc<Mutex<storage::Store>>>();
                            match store_state.lock() {
                                Ok(store) => {
                                    if let Err(e) = store.record_focus_session(
                                        &started,
                                        &ended,
                                        dur,
                                        tid.as_deref(),
                                    ) {
                                        eprintln!("[focus] record_focus_session failed: {e}");
                                    }
                                }
                                Err(_) => eprintln!("[focus] store lock poisoned"),
                            }
                            let _ = hf.emit("stats:changed", ());
                        }
                        ft.active = false;
                        ft.paused = paused;
                        ft.session_started_at = None;
                        ft.session_focus_sec = 0;
                    }
                    _ => {
                        ft.active = false;
                        ft.paused = false;
                    }
                }
                drop(ft);
                apply_topbar_visibility(&hf);
            });

            // CLI timer round-trip: desktop webview replies with live state
            let hc = h.clone();
            hc.clone().listen("cli:timer-done", move |event| {
                let v: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or_default();
                let id = v.get("id").and_then(|x| x.as_u64()).unwrap_or(u64::MAX);
                if let Some(tx) = hc
                    .state::<AppState>()
                    .cli_pending
                    .lock()
                    .unwrap()
                    .remove(&id)
                {
                    let _ = tx.send(v);
                }
            });

            // v1.12: desktop lock Drop guard lives in AppState (dropped only
            // at process exit — see _desktop_lock_guard). Dev-only crash
            // defenses (panic hook / watchdog / escape file) installed here;
            // removed after development — see desktop_lock_escapes.rs.
            desktop_lock_escapes::install_all();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_bootstrap,
            get_grid_metrics,
            place_window,
            drag::drag_start,
            drag::drag_end,
            set_topmost,
            collapse,
            restore,
            add_shortcut,
            add_url_shortcut,
            add_internal_shortcut,
            remove_shortcut,
            move_shortcut,
            set_shortcut_fit,
            launch_shortcut,
            set_acrylic,
            save_task,
            set_current_task,
            set_focus_durations,
            set_focus_mode,
            set_distraction_lists,
            set_supervision_paused,
            resume_supervision,
            set_supervision_enabled,
            set_sound_enabled,
            set_show_topbar,
            list_running_apps,
            record_focus_session,
            get_today_focus_summary,
            stats_dashboard,
            music_set_folder,
            music_get_folder,
            music_list,
            music_cover,
            get_shortcut_icon,
            get_wallpaper,
            persist_wallpaper,
            reset_wallpaper,
            agent_status,
            agent_start_thread,
            agent_resume_thread,
            agent_list_threads,
            agent_send,
            agent_interrupt,
            agent_list_skills,
            agent_delete,
            agent_open_workspace,
            desktop_lock,
            desktop_unlock,
            desktop_set_focus_lock,
            agent_set_provider,
            set_agent_workspace_dir,
            pet_import_pack,
            pet_remove_pack,
            pet_list_packs,
            pet_activate,
            pet_sheet_data,
            pet_active,
            resize_preview,
            set_pet_bg_fade,
            resize_window,
            workflow::characters_list,
            workflow::workflow_list,
            workflow::workflow_save,
            workflow::workflow_delete,
            workflow::workflow_run,
            workflow::workflow_cancel,
            workflow::workflow_copy,
            workflow::workflow_runs,
            workflow::workflow_cleanup_threads,
            workflow::workflow_automation_threads,
            workflow::workflow_runs_recent,
            workflow::workflow_runs_clear,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
#[cfg(test)]
mod tests {
    use super::{
        agent_set_provider_serialized_with, agent_status_for_character, agents,
        agents::AgentProviderKind, bootstrap_existing_demo_pet_provider, direct_user_message,
        discard_runtime_after_provider_error, elapsed_sec, ensure_runtime_serialized,
        float_nonclient_message_result, is_float_label, list_provider_skills, provider_ready, provider_skills_dir,
        resume_with_initial_message, saved_session_for_today, select_status_character,
        set_agent_provider_serialized_with, topbar_visible, with_agent_runtime_serialized,
    };

    fn status_character(id: &str, tool: &str) -> crate::storage::CharacterRow {
        crate::storage::CharacterRow {
            id: id.into(),
            name: id.into(),
            persona: String::new(),
            pet_pack_id: None,
            tool: tool.into(),
            workspace_dir: Some(format!(r"C:\Focus-Agents\{id}")),
            current_session_hash: None,
            session_date: None,
        }
    }

    #[test]
    fn direct_user_message_does_not_read_or_inject_selected_skill_content() {
        let root = std::env::temp_dir().join(format!(
            "focus-provider-skills-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let codex_skill = provider_skills_dir(&root, AgentProviderKind::Codex).join("focus-cli");
        let claude_skill = provider_skills_dir(&root, AgentProviderKind::Claude).join("focus-cli");
        std::fs::create_dir_all(&codex_skill).unwrap();
        std::fs::create_dir_all(&claude_skill).unwrap();
        std::fs::write(codex_skill.join("SKILL.md"), "CODEX SKILL").unwrap();
        std::fs::write(claude_skill.join("SKILL.md"), "CLAUDE SKILL").unwrap();

        assert_eq!(
            list_provider_skills(&root, AgentProviderKind::Codex).unwrap(),
            vec!["focus-cli"]
        );
        assert_eq!(
            list_provider_skills(&root, AgentProviderKind::Claude).unwrap(),
            vec!["focus-cli"]
        );
        assert_eq!(
            direct_user_message("$focus-cli  check status"),
            "$focus-cli  check status"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    fn provider_race_store(
        character_id: &str,
        tool: &str,
    ) -> (
        std::path::PathBuf,
        std::sync::Arc<std::sync::Mutex<crate::storage::Store>>,
    ) {
        let path = std::env::temp_dir().join(format!(
            "focus-provider-race-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = crate::storage::Store::open(&path).unwrap();
        store.migrate().unwrap();
        store
            .insert_character(&status_character(character_id, tool))
            .unwrap();
        (path, std::sync::Arc::new(std::sync::Mutex::new(store)))
    }

    fn existing_demo_pet_store() -> (
        std::path::PathBuf,
        std::sync::Arc<std::sync::Mutex<crate::storage::Store>>,
    ) {
        let path = std::env::temp_dir().join(format!(
            "focus-demo-pet-bootstrap-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = crate::storage::Store::open(&path).unwrap();
        store.migrate().unwrap();
        let mut row = status_character("focus-demo-pet", "codex");
        row.name = "Focus Demo Pet".into();
        row.pet_pack_id = Some("focus-demo-pet".into());
        store.insert_character(&row).unwrap();
        (path, std::sync::Arc::new(std::sync::Mutex::new(store)))
    }

    fn inert_runtime(kind: AgentProviderKind, character_id: &str) -> agents::AgentRuntime {
        let (tx, _) = tokio::sync::broadcast::channel(8);
        match kind {
            AgentProviderKind::Codex => agents::AgentRuntime::Codex(std::sync::Arc::new(
                std::sync::Mutex::new(agents::codex::CodexProvider::new(
                    tx,
                    std::path::PathBuf::from("codex.exe"),
                    character_id.into(),
                )),
            )),
            AgentProviderKind::Claude => agents::AgentRuntime::Claude(std::sync::Arc::new(
                std::sync::Mutex::new(agents::claude::ClaudeProvider::new(
                    tx,
                    std::path::PathBuf::from("claude.exe"),
                    character_id.into(),
                    format!(r"C:\Focus-Agents\{character_id}"),
                )),
            )),
            AgentProviderKind::Mock => panic!("production provider required"),
        }
    }

    #[cfg(windows)]
    fn long_running_claude_runtime(
        character_id: &str,
        label: &str,
    ) -> (std::path::PathBuf, agents::AgentRuntime) {
        let workspace = std::env::temp_dir().join(format!(
            "focus-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&workspace).unwrap();
        let shim = workspace.join("claude.cmd");
        std::fs::write(
            &shim,
            concat!(
                "@echo off\r\n",
                "echo {\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"guarded-session\"}\r\n",
                "for /L %%i in (1,1,10000000) do @rem\r\n",
            ),
        )
        .unwrap();
        let (tx, _) = tokio::sync::broadcast::channel(8);
        let runtime = agents::AgentRuntime::Claude(std::sync::Arc::new(std::sync::Mutex::new(
            agents::claude::ClaudeProvider::new(
                tx,
                shim,
                character_id.into(),
                workspace.to_string_lossy().into_owned(),
            ),
        )));
        (workspace, runtime)
    }

    #[cfg(windows)]
    #[test]
    fn direct_turn_claim_blocks_switch_and_persists_the_runtime_provider() {
        let character_id = "char-claim-switch";
        let (db_path, store) = provider_race_store(character_id, "claude");
        let (workspace, runtime) = long_running_claude_runtime(character_id, "claim-switch");
        let registry = std::sync::Arc::new(std::sync::Mutex::new(agents::AgentRegistry::new()));
        registry
            .lock()
            .unwrap()
            .insert(character_id.into(), runtime.shared_clone());
        let (acquired_tx, acquired_rx) = std::sync::mpsc::channel();
        let (claim_tx, claim_rx) = std::sync::mpsc::channel();

        let claim_registry = registry.clone();
        let claim_store = store.clone();
        let claim_workspace = workspace.clone();
        let claim = std::thread::spawn(move || {
            with_agent_runtime_serialized(
                &claim_registry,
                character_id,
                || panic!("existing runtime must be reused"),
                || {
                    acquired_tx.send(()).unwrap();
                    claim_rx.recv().unwrap();
                },
                |actual_runtime| {
                    let actual_provider = actual_runtime.kind();
                    let info = actual_runtime.start_thread(
                        &claim_workspace.to_string_lossy(),
                        "claim before switch",
                        agents::agent_display_full(),
                    )?;
                    claim_store
                        .lock()
                        .unwrap()
                        .upsert_provider_session(
                            character_id,
                            actual_provider.as_str(),
                            &info.id,
                            "2026-08-10",
                        )
                        .map_err(|error| error.to_string())?;
                    Ok((actual_provider, info))
                },
            )
        });
        acquired_rx.recv().unwrap();

        let (switch_locked_tx, switch_locked_rx) = std::sync::mpsc::channel();
        let switch_registry = registry.clone();
        let switch_store = store.clone();
        let switch = std::thread::spawn(move || {
            set_agent_provider_serialized_with(
                &switch_registry,
                &switch_store,
                Some(character_id),
                AgentProviderKind::Codex,
                || switch_locked_tx.send(()).unwrap(),
            )
        });
        assert!(
            switch_locked_rx
                .recv_timeout(std::time::Duration::from_millis(100))
                .is_err(),
            "switch must wait until the acquired runtime claims its turn"
        );

        claim_tx.send(()).unwrap();
        let (actual_provider, info) = claim.join().unwrap().unwrap();
        assert_eq!(actual_provider, AgentProviderKind::Claude);
        let switch_error = switch.join().unwrap().unwrap_err();
        switch_locked_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        assert_eq!(switch_error, agents::PROVIDER_SWITCH_BUSY_ERROR);
        let store_guard = store.lock().unwrap();
        assert_eq!(
            store_guard
                .get_character(character_id)
                .unwrap()
                .unwrap()
                .tool,
            "claude"
        );
        assert_eq!(
            store_guard
                .load_provider_session(character_id, "claude")
                .unwrap()
                .unwrap()
                .session_hash,
            info.id
        );
        assert!(store_guard
            .load_provider_session(character_id, "codex")
            .unwrap()
            .is_none());
        drop(store_guard);

        runtime.interrupt(&info.id).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while runtime.has_active_turn() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        drop(runtime);
        drop(registry);
        drop(store);
        std::fs::remove_dir_all(workspace).unwrap();
        std::fs::remove_file(db_path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn workflow_turn_start_reloads_after_switch_instead_of_using_escaped_arc() {
        let character_id = "char-workflow-switch";
        let (db_path, store) = provider_race_store(character_id, "codex");
        let registry = std::sync::Mutex::new(agents::AgentRegistry::new());
        let escaped_old_runtime = inert_runtime(AgentProviderKind::Codex, character_id);
        registry
            .lock()
            .unwrap()
            .insert(character_id.into(), escaped_old_runtime.shared_clone());

        set_agent_provider_serialized_with(
            &registry,
            &store,
            Some(character_id),
            AgentProviderKind::Claude,
            || {},
        )
        .unwrap();
        let (workspace, replacement) = long_running_claude_runtime(character_id, "workflow-switch");
        let replacement_for_build = replacement.shared_clone();
        let (actual_provider, info) = with_agent_runtime_serialized(
            &registry,
            character_id,
            || Ok(replacement_for_build),
            || {},
            |actual_runtime| {
                let _turn_done = actual_runtime
                    .subscribe_turn_done()
                    .ok_or_else(|| "runtime must expose turn completion".to_string())?;
                let info = actual_runtime.start_thread(
                    &workspace.to_string_lossy(),
                    "workflow guarded start",
                    agents::agent_display_full(),
                )?;
                Ok((actual_runtime.kind(), info))
            },
        )
        .unwrap();

        assert_eq!(escaped_old_runtime.kind(), AgentProviderKind::Codex);
        assert!(!escaped_old_runtime.has_active_turn());
        assert_eq!(actual_provider, AgentProviderKind::Claude);
        assert!(replacement.has_active_turn());
        assert_eq!(
            registry.lock().unwrap().get(character_id).unwrap().kind(),
            AgentProviderKind::Claude
        );

        replacement.interrupt(&info.id).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while replacement.has_active_turn() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        drop(replacement);
        drop(escaped_old_runtime);
        drop(registry);
        drop(store);
        std::fs::remove_dir_all(workspace).unwrap();
        std::fs::remove_file(db_path).unwrap();
    }

    #[test]
    fn provider_switch_serializes_against_ensure_that_observed_old_tool() {
        let character_id = "char-provider-race";
        let (db_path, store) = provider_race_store(character_id, "codex");
        let registry = std::sync::Arc::new(std::sync::Mutex::new(agents::AgentRegistry::new()));
        let (observed_tx, observed_rx) = std::sync::mpsc::channel();
        let (continue_tx, continue_rx) = std::sync::mpsc::channel();

        let ensure_registry = registry.clone();
        let ensure_store = store.clone();
        let ensure = std::thread::spawn(move || {
            ensure_runtime_serialized(&ensure_registry, character_id, || {
                let row = ensure_store
                    .lock()
                    .unwrap()
                    .get_character(character_id)
                    .unwrap()
                    .unwrap();
                observed_tx.send(row.tool.clone()).unwrap();
                continue_rx.recv().unwrap();
                Ok(inert_runtime(
                    AgentProviderKind::parse(&row.tool).unwrap(),
                    character_id,
                ))
            })
            .unwrap()
        });
        assert_eq!(observed_rx.recv().unwrap(), "codex");

        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (locked_tx, locked_rx) = std::sync::mpsc::channel();
        let switch_registry = registry.clone();
        let switch_store = store.clone();
        let switch = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            set_agent_provider_serialized_with(
                &switch_registry,
                &switch_store,
                Some(character_id),
                AgentProviderKind::Claude,
                || locked_tx.send(()).unwrap(),
            )
        });
        started_rx.recv().unwrap();
        assert!(
            locked_rx
                .recv_timeout(std::time::Duration::from_millis(100))
                .is_err(),
            "switch must wait behind the ensure serialization guard"
        );

        continue_tx.send(()).unwrap();
        assert_eq!(ensure.join().unwrap().kind(), AgentProviderKind::Codex);
        switch.join().unwrap().unwrap();
        locked_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();

        let final_runtime = ensure_runtime_serialized(&registry, character_id, || {
            let row = store
                .lock()
                .unwrap()
                .get_character(character_id)
                .unwrap()
                .unwrap();
            Ok(inert_runtime(
                AgentProviderKind::parse(&row.tool).unwrap(),
                character_id,
            ))
        })
        .unwrap();
        assert_eq!(
            store
                .lock()
                .unwrap()
                .get_character(character_id)
                .unwrap()
                .unwrap()
                .tool,
            "claude"
        );
        assert_eq!(final_runtime.kind(), AgentProviderKind::Claude);
        assert_eq!(
            registry.lock().unwrap().get(character_id).unwrap().kind(),
            AgentProviderKind::Claude
        );

        drop(final_runtime);
        drop(registry);
        drop(store);
        std::fs::remove_file(db_path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn provider_switch_rejects_active_turn_without_changing_tool_or_runtime() {
        let character_id = "char-active-switch";
        let (db_path, store) = provider_race_store(character_id, "claude");
        let workspace = std::env::temp_dir().join(format!(
            "focus-active-provider-switch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&workspace).unwrap();
        let shim = workspace.join("claude.cmd");
        std::fs::write(
            &shim,
            concat!(
                "@echo off\r\n",
                "echo {\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"active-switch-session\"}\r\n",
                "for /L %%i in (1,1,10000000) do @rem\r\n",
            ),
        )
        .unwrap();
        let (tx, _) = tokio::sync::broadcast::channel(8);
        let runtime = agents::AgentRuntime::Claude(std::sync::Arc::new(std::sync::Mutex::new(
            agents::claude::ClaudeProvider::new(
                tx,
                shim,
                character_id.into(),
                workspace.to_string_lossy().into_owned(),
            ),
        )));
        let registry = std::sync::Mutex::new(agents::AgentRegistry::new());
        registry
            .lock()
            .unwrap()
            .insert(character_id.into(), runtime.shared_clone());
        let thread = runtime
            .start_thread(
                &workspace.to_string_lossy(),
                "stay active",
                agents::agent_display_full(),
            )
            .unwrap();
        assert!(runtime.has_active_turn());

        let error = set_agent_provider_serialized_with(
            &registry,
            &store,
            Some(character_id),
            AgentProviderKind::Codex,
            || {},
        )
        .unwrap_err();
        assert_eq!(error, agents::PROVIDER_SWITCH_BUSY_ERROR);
        assert_eq!(
            store
                .lock()
                .unwrap()
                .get_character(character_id)
                .unwrap()
                .unwrap()
                .tool,
            "claude"
        );
        assert_eq!(
            registry.lock().unwrap().get(character_id).unwrap().kind(),
            AgentProviderKind::Claude
        );
        assert!(runtime.has_active_turn());

        runtime.interrupt(&thread.id).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while runtime.has_active_turn() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        drop(runtime);
        drop(registry);
        drop(store);
        std::fs::remove_dir_all(workspace).unwrap();
        std::fs::remove_file(db_path).unwrap();
    }

    #[test]
    fn agent_set_provider_rejects_unknown_provider() {
        let (db_path, store) = provider_race_store("char-a", "codex");
        let registry = std::sync::Mutex::new(agents::AgentRegistry::new());

        let error =
            agent_set_provider_serialized_with(&registry, &store, "char-a", "unknown", || {})
                .unwrap_err();

        assert_eq!(error, "provider must be codex or claude");
        assert_eq!(
            store
                .lock()
                .unwrap()
                .get_character("char-a")
                .unwrap()
                .unwrap()
                .tool,
            "codex"
        );
        drop(registry);
        drop(store);
        std::fs::remove_file(db_path).unwrap();
    }

    #[test]
    fn agent_set_provider_changes_only_the_selected_inactive_character() {
        let (db_path, store) = provider_race_store("char-a", "codex");
        store
            .lock()
            .unwrap()
            .insert_character(&status_character("char-b", "codex"))
            .unwrap();
        let registry = std::sync::Mutex::new(agents::AgentRegistry::new());

        agent_set_provider_serialized_with(&registry, &store, "char-b", "claude", || {}).unwrap();

        let store_guard = store.lock().unwrap();
        assert_eq!(
            store_guard.get_character("char-a").unwrap().unwrap().tool,
            "codex"
        );
        assert_eq!(
            store_guard.get_character("char-b").unwrap().unwrap().tool,
            "claude"
        );
        drop(store_guard);
        drop(registry);
        drop(store);
        std::fs::remove_file(db_path).unwrap();
    }

    #[test]
    fn existing_exact_demo_pet_bootstraps_to_claude_only_once() {
        let (db_path, store) = existing_demo_pet_store();

        assert!(bootstrap_existing_demo_pet_provider(&mut store.lock().unwrap()).unwrap());
        assert_eq!(
            store
                .lock()
                .unwrap()
                .get_character("focus-demo-pet")
                .unwrap()
                .unwrap()
                .tool,
            "claude"
        );
        store
            .lock()
            .unwrap()
            .update_character_tool("focus-demo-pet", "codex")
            .unwrap();

        assert!(!bootstrap_existing_demo_pet_provider(&mut store.lock().unwrap()).unwrap());
        assert_eq!(
            store
                .lock()
                .unwrap()
                .get_character("focus-demo-pet")
                .unwrap()
                .unwrap()
                .tool,
            "codex"
        );
        drop(store);
        std::fs::remove_file(db_path).unwrap();
    }

    #[test]
    fn character_provider_status_follows_the_supplied_target_not_global_precedence() {
        let characters = vec![
            status_character("char-codex", "codex"),
            status_character("char-claude", "claude"),
        ];
        let codex_path = Some(r"C:\Tools\codex.exe".to_string());
        let claude_path = None;

        let selected = select_status_character(&characters, Some("char-claude")).unwrap();
        let claude = agent_status_for_character(selected, &codex_path, &claude_path).unwrap();
        assert_eq!(claude.provider, "claude");
        assert!(
            !claude.ready,
            "Codex availability must not make Claude ready"
        );
        assert_eq!(claude.workspace_dir, r"C:\Focus-Agents\char-claude");

        let default = select_status_character(&characters, None).unwrap();
        let codex = agent_status_for_character(default, &codex_path, &claude_path).unwrap();
        assert_eq!(codex.provider, "codex");
        assert!(codex.ready);
        assert_eq!(
            default.id, "char-codex",
            "no-target status is deterministic"
        );
    }

    #[test]
    fn character_provider_status_rejects_unknown_character_tool() {
        let unknown = status_character("char-unknown", "legacy-global");
        let error = match agent_status_for_character(&unknown, &None, &None) {
            Ok(_) => panic!("unknown provider must be rejected"),
            Err(error) => error,
        };
        assert_eq!(error, "未知 Agent provider: legacy-global");
    }

    #[test]
    fn legacy_mock_provider_is_not_a_production_provider() {
        assert!(AgentProviderKind::parse("mock").is_none());
    }

    #[test]
    fn claude_is_a_real_runtime_kind_with_independent_readiness() {
        use std::path::PathBuf;
        use std::sync::{Arc, Mutex};

        assert_eq!(
            AgentProviderKind::parse("claude"),
            Some(AgentProviderKind::Claude)
        );
        assert_eq!(AgentProviderKind::Claude.as_str(), "claude");
        assert!(provider_ready(
            AgentProviderKind::Claude,
            &None,
            &Some(r"C:\Tools\claude.exe".into()),
        ));
        assert!(!provider_ready(
            AgentProviderKind::Claude,
            &Some("codex.exe".into()),
            &None
        ));

        let (tx, _) = tokio::sync::broadcast::channel(8);
        let runtime = agents::AgentRuntime::Claude(Arc::new(Mutex::new(
            agents::claude::ClaudeProvider::new(
                tx,
                PathBuf::from("claude.exe"),
                "char-claude".into(),
                r"C:\Focus-Agents\char-claude".into(),
            ),
        )));
        assert_eq!(runtime.kind(), AgentProviderKind::Claude);
    }

    #[test]
    fn provider_session_resumes_only_on_the_same_day() {
        let row = crate::storage::ProviderSessionRow {
            character_id: "char-claude".into(),
            provider: "claude".into(),
            session_hash: "claude-session".into(),
            session_date: "2026-08-10".into(),
        };
        assert_eq!(
            saved_session_for_today(Some(row.clone()), "2026-08-10").as_deref(),
            Some("claude-session")
        );
        assert_eq!(saved_session_for_today(Some(row), "2026-08-11"), None);
        assert_eq!(saved_session_for_today(None, "2026-08-10"), None);
    }

    #[test]
    fn codex_readiness_reflects_executable_availability() {
        assert!(provider_ready(
            AgentProviderKind::Codex,
            &Some(r"C:\\Codex\\codex.exe".into()),
            &None,
        ));
        assert!(!provider_ready(
            AgentProviderKind::Codex,
            &None,
            &Some("claude.exe".into())
        ));
    }

    #[test]
    fn busy_turn_error_preserves_runtime_while_other_errors_drop_it() {
        let (tx, _) = tokio::sync::broadcast::channel(8);
        let mut registry = agents::AgentRegistry::new();
        registry.insert(
            "char-test".into(),
            agents::AgentRuntime::Mock(std::sync::Arc::new(std::sync::Mutex::new(
                agents::mock::MockProvider::new(tx),
            ))),
        );

        discard_runtime_after_provider_error(&mut registry, "char-test", agents::ACTIVE_TURN_ERROR);
        assert!(registry.get("char-test").is_some());

        discard_runtime_after_provider_error(&mut registry, "char-test", "codex app-server exited");
        assert!(registry.get("char-test").is_none());
    }

    #[test]
    fn same_day_resume_and_send_delivers_the_initial_message() {
        use std::sync::Mutex;
        use std::time::Duration;

        let (tx, _) = tokio::sync::broadcast::channel(32);
        let mut events = tx.subscribe();
        let runtime = agents::AgentRuntime::Mock(std::sync::Arc::new(Mutex::new(
            agents::mock::MockProvider::new(tx),
        )));

        let info = resume_with_initial_message(
            &runtime,
            "today-thread",
            "resume this message",
            agents::agent_display_full(),
        )
        .expect("same-day resume should accept its initial message");
        assert_eq!(info.id, "today-thread");

        let mut saw_resumed_input = false;
        for _ in 0..8 {
            let Ok(Ok(crate::event_bus::CoreEvent::AgentEvent(event))) =
                tauri::async_runtime::block_on(async {
                    tokio::time::timeout(Duration::from_secs(1), events.recv()).await
                })
            else {
                continue;
            };
            if event["event"]["type"] == "message.delta"
                && event["event"]["text"]
                    .as_str()
                    .is_some_and(|text| text.contains("resume this message"))
            {
                saw_resumed_input = true;
                break;
            }
        }
        assert!(
            saw_resumed_input,
            "the resumed thread must receive the caller's initial message"
        );
    }

    #[test]
    fn elapsed_sec_wall_clock() {
        assert_eq!(
            elapsed_sec("2026-08-07T13:43:18+08:00", "2026-08-07T13:45:18+08:00"),
            Some(120)
        );
        assert_eq!(
            elapsed_sec("2026-08-07T13:43:18+08:00", "2026-08-07T13:43:48+08:00"),
            Some(30)
        );
        assert_eq!(elapsed_sec("", ""), None);
        assert_eq!(elapsed_sec("bad", "2026-08-07T13:45:18+08:00"), None);
    }

    #[test]
    fn topbar_visibility_modes() {
        assert!(topbar_visible("on", "idle"));
        assert!(topbar_visible("on", "focus"));
        assert!(topbar_visible("on", "rest"));
        assert!(topbar_visible("auto", "focus"));
        assert!(topbar_visible("auto", "rest"));
        assert!(!topbar_visible("auto", "idle"));
        assert!(!topbar_visible("off", "focus"));
        assert!(!topbar_visible("off", "rest"));
        assert!(!topbar_visible("off", "idle"));
    }

    #[test]
    fn float_labels_cover_every_internal_page() {
        for label in ["chat", "stats", "music", "pet", "workflow"] {
            assert!(
                is_float_label(label),
                "missing float lifecycle coverage for {label}"
            );
        }
        assert!(!is_float_label("desktop"));
        assert!(!is_float_label("topbar"));
    }

    #[test]
    fn float_nonclient_handler_preserves_client_area_without_suppressing_erase() {
        assert_eq!(float_nonclient_message_result(0x0083), Some(0));
        assert_eq!(float_nonclient_message_result(0x0014), None);
        assert_eq!(float_nonclient_message_result(0x000F), None);
    }

    #[test]
    fn free_cell_skips_forbidden_zones() {
        use super::free_cell_for;
        let (c0, r0) = free_cell_for(&[]);
        assert_eq!(
            (c0, r0),
            (0, 0),
            "top-left is free (hero only blocks cols 3-9 rows 0-3)"
        );
        let occupied = vec![crate::storage::ShortcutRow {
            id: "x".into(),
            name: "x".into(),
            kind: "file".into(),
            target: "x".into(),
            col: 0,
            row: 4,
            fit_col: None,
            fit_row: None,
            fit_cols: None,
            fit_rows: None,
        }];
        let (c1, r1) = free_cell_for(&occupied);
        assert_eq!(
            (c1, r1),
            (0, 0),
            "occupied (0,4) does not block the top-left"
        );
    }
}
