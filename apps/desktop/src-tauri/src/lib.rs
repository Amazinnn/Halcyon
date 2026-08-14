//! Focus Desktop spike (v1.2 visual & window-management round).
//! Windows: desktop (canvas), chat / stats / music / pet (12x8 grid floats,
//! frosted acrylic, collapsible to hidden), grid-overlay (drag preview),
//! topbar (focus status capsule). No AgentEvent protocol / event-name /
//! DB changes from the spike.

mod acrylic;
mod activity;
mod agents;
mod apps;
pub mod cli; // focus-cli bin derives help from the command registry (C2)
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
mod window_spec;
mod workflow;
mod workflow_engine;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use base64::Engine;
use serde::Serialize;
use tauri::{Emitter, Listener, Manager};
use tauri::LogicalPosition;

use event_bus::CoreEvent;
use grid::GridManager;
use settings::{GridRect, Settings, ShortcutType};
use window_spec::{float_labels, is_float_label, WindowKind, WINDOW_SPECS};

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub data_dir: PathBuf,
    pub screen: Mutex<(f64, f64)>, // logical width/height
    pub active_drag: Mutex<Option<drag::ActiveDrag>>,
    pub drag_diagnostics: drag::DragDiagnosticRecorder,
    /// Single-flight guard for shortcut launches (async, non-blocking).
    pub launch_lock: tokio::sync::Mutex<()>,
    /// A float visibility transition includes native show/hide/position/topmost
    /// work. It must never overlap another such transition.
    float_visibility_gate: FloatVisibilityGate,
    pub focus_track: Mutex<supervision::FocusTrack>,
    pub focus_state: Mutex<String>,
    pub active_focus_mode: Mutex<Option<String>>,
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
    bubble_controller: Mutex<BubbleController>,
    bubble_next_id: AtomicU64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingBubble {
    delivery_id: String,
    agent_id: String,
    text: String,
    priority: String,
    created_at_ms: u64,
}

#[derive(Debug, Default)]
struct BubbleController {
    pending: Option<PendingBubble>,
    ready_agent_id: Option<String>,
    ready_generation: u64,
    last_stage: &'static str,
    last_delivery_id: Option<String>,
}

impl BubbleController {
    fn expire(&mut self, now_ms: u64) {
        if self.pending.as_ref().is_some_and(|bubble| now_ms.saturating_sub(bubble.created_at_ms) > PENDING_BUBBLE_TTL_MS) {
            self.pending = None;
            self.last_stage = "expired";
        }
    }

    fn ready(&mut self, agent_id: &str, generation: u64, now_ms: u64) -> Option<PendingBubble> {
        self.expire(now_ms);
        self.ready_agent_id = Some(agent_id.to_string());
        self.ready_generation = generation;
        self.last_stage = "host_ready";
        let pending = self.pending.as_ref().filter(|bubble| bubble.agent_id == agent_id).cloned();
        if let Some(bubble) = &pending { self.last_delivery_id = Some(bubble.delivery_id.clone()); }
        pending
    }

    fn rendered(&mut self, agent_id: &str, generation: u64, delivery_id: &str, shown: bool, now_ms: u64) -> bool {
        self.expire(now_ms);
        if self.ready_agent_id.as_deref() != Some(agent_id) || self.ready_generation != generation {
            self.last_stage = "stale_render_ack";
            return false;
        }
        let matches = self.pending.as_ref().is_some_and(|bubble| bubble.agent_id == agent_id && bubble.delivery_id == delivery_id);
        if !matches {
            self.last_stage = "unknown_render_ack";
            return false;
        }
        self.last_delivery_id = Some(delivery_id.to_string());
        self.last_stage = if shown { "shown" } else { "placement_unavailable" };
        if shown { self.pending = None; }
        true
    }

    fn clear_for_agent_change(&mut self) {
        self.pending = None;
        self.ready_agent_id = None;
        self.last_stage = "cleared_agent_change";
    }
}

const PENDING_BUBBLE_TTL_MS: u64 = 30_000;


#[derive(Default)]
struct FloatVisibilityGate {
    active: AtomicBool,
}

struct FloatVisibilityOperation<'a> {
    gate: &'a FloatVisibilityGate,
}

impl FloatVisibilityGate {
    fn try_enter(&self) -> Result<FloatVisibilityOperation<'_>, String> {
        self.active
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| "窗口操作正在进行，请稍候".to_string())?;
        Ok(FloatVisibilityOperation { gate: self })
    }
}

impl Drop for FloatVisibilityOperation<'_> {
    fn drop(&mut self) {
        self.gate.active.store(false, Ordering::Release);
    }
}


// ---------------------------------------------------------------------------
// window helpers
// ---------------------------------------------------------------------------

/// Glass alpha for one layer under the global opacity (requirement #123):
/// alpha = round(base_alpha x opacity/22), clamped to 8..=255, so opacity 22
/// reproduces the historical visuals exactly and the slider never degrades
/// the SWCA path to plain transparency.
pub(crate) fn glass_alpha(base_alpha: u8, opacity: u8) -> u8 {
    ((base_alpha as u32)
        .saturating_mul(opacity.clamp(5, 100) as u32)
        .div_ceil(22)
        .clamp(8, 255)) as u8
}

fn glass_opacity(settings: &Settings) -> u8 {
    settings.acrylic_opacity.clamp(5, 100)
}

fn apply_acrylic_opt(w: &tauri::WebviewWindow, enabled: bool, opacity: u8) {
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
            acrylic::apply(hwnd.0, (14, 24, 18, crate::glass_alpha(56, opacity)));
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (w, enabled, opacity);
}

fn parse_rgb_hex(value: &str) -> Option<(u8, u8, u8)> {
    let value = value.strip_prefix('#')?;
    if value.len() != 6 { return None; }
    Some((
        u8::from_str_radix(&value[0..2], 16).ok()?,
        u8::from_str_radix(&value[2..4], 16).ok()?,
        u8::from_str_radix(&value[4..6], 16).ok()?,
    ))
}

fn current_pet_host_tint(state: &AppState) -> Option<(u8, u8, u8)> {
    let character_id = state.settings.lock().unwrap().current_agent_id.clone()?;
    let row = state.store.lock().unwrap().get_character(&character_id).ok().flatten()?;
    let workspace = ensure_agent_workspace(state, &row).ok()?;
    let info = pets::info_for_agent(Path::new(&workspace)).ok()?;
    parse_rgb_hex(&info.host_tint)
}

fn apply_current_pet_acrylic(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let (enabled, opacity) = {
        let s = state.settings.lock().unwrap();
        (s.acrylic_enabled, glass_opacity(&s))
    };
    let tint = current_pet_host_tint(&state).unwrap_or((14, 24, 18));
    let Some(window) = app.get_webview_window("pet") else { return };
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        if enabled && std::env::var_os("FOCUS_NO_ACRYLIC").is_none() {
            acrylic::apply(hwnd.0, (tint.0, tint.1, tint.2, crate::glass_alpha(64, opacity)));
        } else {
            acrylic::clear(hwnd.0);
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (window, enabled, tint);
}

fn set_float_topmost_noactivate(w: &tauri::WebviewWindow, topmost: bool) {
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
        let (x, y, wpx, hpx) = gm.rect_to_logical(rect);
        // v1.10: skip when already at the target (avoid Win32 churn under
        // rapid restore/collapse, #31). Getters are main-thread-only; all
        // callers run on the main thread.
        let scale = w.scale_factor().unwrap_or(1.0);
        let (px, py) = ((x * scale).round() as i32, (y * scale).round() as i32);
        let (pwp, php) = ((wpx * scale).round() as u32, (hpx * scale).round() as u32);
        // Grid coordinates are client-area coordinates. Query the live frame
        // instead of assuming that a platform host has no non-client extent.
        let geometry = client_geometry_snapshot(&w);
        let (outer_x, outer_y, outer_w, outer_h) =
            geometry.outer_rect_for_client(px, py, pwp, php);
        let same = w.outer_position().map(|p| (p.x, p.y)).ok() == Some((outer_x, outer_y))
            && w.outer_size().map(|s| (s.width, s.height)).ok() == Some((outer_w, outer_h));
        if !same {
            // v1.10.2 (#35, ADR-0014): position changes move the native HWND
            // (no WebView2 SetBounds RPC per call); size changes still go
            // through the webview so the renderer relayouts.
            // v1.12.2: size path is ALSO native (SetWindowPos + SWP_NOACTIVATE)
            // — Tauri's set_size can activate the window and paint a caption
            // highlight (light-blue bar) while drag/resize preview is held.
            if !crate::drag::move_window_raw(&w, outer_x, outer_y) {
                #[cfg(not(target_os = "windows"))]
                let _ = w.set_position(LogicalPosition::new(
                    outer_x as f64 / scale,
                    outer_y as f64 / scale,
                ));
            }
            crate::drag::resize_window_raw(&w, outer_w, outer_h);
        }
    }
}

fn emit_visibility(app: &tauri::AppHandle, label: &str, visible: bool) {
    let _ = app.emit(
        "window:visibility",
        serde_json::json!({ "label": label, "visible": visible }),
    );
}

fn pet_window_should_be_visible(has_valid_package: bool, collapsed: bool) -> bool {
    has_valid_package && !collapsed
}

fn current_agent_has_valid_pet(state: &AppState) -> bool {
    let current = state.settings.lock().unwrap().current_agent_id.clone();
    let Some(character_id) = current else { return false };
    let row = match state.store.lock().unwrap().get_character(&character_id) {
        Ok(Some(row)) if row.pet_pack_id.is_some() => row,
        _ => return false,
    };
    let Ok(workspace) = ensure_agent_workspace(state, &row) else { return false };
    pets::info_for_agent(Path::new(&workspace)).is_ok()
}

/// The pet host must not be a visible empty or transparent window when the
/// current Agent has no readable package. This is native-window visibility,
/// not merely a Vue empty state, so it cannot start a drag lifecycle.
fn sync_pet_host_visibility(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let collapsed = state.settings.lock().unwrap().collapsed.contains(&"pet".to_string());
    let visible = pet_window_should_be_visible(current_agent_has_valid_pet(&state), collapsed);
    if let Some(pet) = app.get_webview_window("pet") {
        if visible {
            show_window_noactivate(&pet);
        } else {
            hide_window_noactivate(&pet);
        }
    }
    if !visible {
        if let Some(bubble) = app.get_webview_window("pet-bubble") {
            hide_window_noactivate(&bubble);
        }
    }
    emit_visibility(app, "pet", visible);
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    grid: HashMap<String, GridRect>,
    topmost: HashMap<String, bool>,
    collapsed: Vec<String>,
    wallpaper_path: Option<String>,
    shortcuts: Vec<storage::ShortcutRow>,
    acrylic_enabled: bool,
    focus_subtitle: String,
    focus_minutes: u32,
    rest_minutes: u32,
    distraction_apps: Vec<String>,
    sound_enabled: bool,
    show_topbar: String,
    focus_mode: String,
    agent_workspace_dir: Option<String>,
    pet_bg_fade: bool,
    current_agent_id: Option<String>,
    chat_streaming_enabled: bool,
    acrylic_opacity: u8,
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
        focus_minutes: s.focus_minutes,
        rest_minutes: s.rest_minutes,
        distraction_apps: s.distraction_apps.clone(),
        sound_enabled: s.sound_enabled,
        show_topbar: s.show_topbar.clone(),
        focus_mode: s.focus_mode.clone(),
        agent_workspace_dir: s.agent_workspace_dir.clone(),
        pet_bg_fade: s.pet_bg_fade,
        current_agent_id: s.current_agent_id.clone(),
        chat_streaming_enabled: s.chat_streaming_enabled,
        acrylic_opacity: s.acrylic_opacity.clamp(5, 100),
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
    let display = agents::agent_display_full(state.settings.lock().unwrap().chat_streaming_enabled);
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
    let display = agents::agent_display_full(
        app.state::<AppState>().settings.lock().unwrap().chat_streaming_enabled,
    );
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

#[tauri::command]
fn agent_delete(app: tauri::AppHandle, character_id: String) -> Result<usize, String> {
    let state = app.state::<AppState>();
    let row = state.store.lock().unwrap().get_character(&character_id)
        .map_err(|e| e.to_string())?.ok_or_else(|| "Agent 不存在".to_string())?;
    let workspace = ensure_agent_workspace(&state, &row)?;
    let pet_dir = Path::new(&workspace).join(pets::AGENT_PET_DIR);
    let backup = Path::new(&workspace).with_extension(format!("delete-backup-{}", std::process::id()));
    let had_pet = pet_dir.is_dir();
    if had_pet {
        std::fs::rename(&pet_dir, &backup).map_err(|e| format!("桌宠包删除准备失败: {e}"))?;
    }
    let removed = {
        let store = state.store.clone();
        let s = store.lock().unwrap();
        match s.delete_agent_and_workflows(&character_id) {
            Ok(removed) => removed,
            Err(error) => {
                if had_pet { let _ = std::fs::rename(&backup, &pet_dir); }
                return Err(error.to_string());
            }
        }
    };
    if had_pet { let _ = std::fs::remove_dir_all(&backup); }
    state.agents.lock().unwrap().runtimes.remove(&character_id);
    let removed_current = {
        let mut settings = state.settings.lock().unwrap();
        let is_current = settings.current_agent_id.as_deref() == Some(character_id.as_str());
        if is_current {
            settings.current_agent_id = None;
            let _ = settings.save(&state.data_dir);
        }
        is_current
    };
    if removed_current {
        state.bubble_controller.lock().unwrap().clear_for_agent_change();
        sync_pet_host_visibility(&app);
    }
    Ok(removed)
}

#[tauri::command]
fn agent_workflow_reference_count(app: tauri::AppHandle, character_id: String) -> Result<usize, String> {
    app.state::<AppState>().store.lock().unwrap()
        .count_agent_workflow_references(&character_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn agent_create(app: tauri::AppHandle, name: String, provider: String) -> Result<storage::CharacterRow, String> {
    let name = name.trim();
    if name.is_empty() { return Err("Agent 名称不能为空".into()); }
    if !matches!(provider.as_str(), "codex" | "claude") { return Err("未知 Provider".into()); }
    let state = app.state::<AppState>();
    let row = state.store.lock().unwrap().create_agent(name, &provider).map_err(|e| e.to_string())?;
    ensure_agent_workspace(&state, &row)?;
    Ok(row)
}

#[tauri::command]
fn agent_set_current(app: tauri::AppHandle, character_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.store.lock().unwrap().get_character(&character_id).map_err(|e| e.to_string())?.is_none() {
        return Err("Agent 不存在".into());
    }
    let changed = {
        let mut settings = state.settings.lock().unwrap();
        let changed = settings.current_agent_id.as_deref() != Some(character_id.as_str());
        settings.current_agent_id = Some(character_id);
        settings.save(&state.data_dir)?;
        changed
    };
    if changed {
        state.bubble_controller.lock().unwrap().clear_for_agent_change();
    }
    sync_pet_host_visibility(&app);
    apply_current_pet_acrylic(&app);
    Ok(())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingBubbleView {
    delivery_id: String,
    text: String,
    priority: String,
    agent_id: String,
}

pub(crate) fn queue_bubble_for_agent(app: &tauri::AppHandle, agent_id: &str, text: String, priority: String) -> String {
    let state = app.state::<AppState>();
    let id = format!("bubble-{}", state.bubble_next_id.fetch_add(1, Ordering::Relaxed) + 1);
    if state.settings.lock().unwrap().current_agent_id.as_deref() == Some(agent_id) {
        let mut controller = state.bubble_controller.lock().unwrap();
        controller.pending = Some(PendingBubble {
            delivery_id: id.clone(),
            agent_id: agent_id.to_string(),
            text,
            priority,
            created_at_ms: now_millis(),
        });
        controller.last_stage = "queued";
        controller.last_delivery_id = Some(id.clone());
    }
    id
}

fn now_millis() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

fn bubble_view(bubble: PendingBubble) -> PendingBubbleView {
    PendingBubbleView {
        delivery_id: bubble.delivery_id,
        text: bubble.text,
        priority: bubble.priority,
        agent_id: bubble.agent_id,
    }
}

fn dispatch_bubble_to_ready_host(app: &tauri::AppHandle) {
    let pending = {
        let state = app.state::<AppState>();
        let mut controller = state.bubble_controller.lock().unwrap();
        let Some(agent_id) = controller.ready_agent_id.clone() else { return };
        let generation = controller.ready_generation;
        controller.ready(&agent_id, generation, now_millis())
    };
    if let Some(bubble) = pending {
        let _ = app.emit_to("pet-bubble", "bubble:deliver", bubble_view(bubble));
    }
}

#[tauri::command]
fn pet_bubble_ready(app: tauri::AppHandle, character_id: String, generation: u64) -> Option<PendingBubbleView> {
    let state = app.state::<AppState>();
    let pending = state.bubble_controller.lock().unwrap().ready(&character_id, generation, now_millis());
    pending.map(bubble_view)
}

#[tauri::command]
fn pet_bubble_rendered(app: tauri::AppHandle, character_id: String, generation: u64, delivery_id: String) -> bool {
    let placement = pet_bubble_placement(app.state::<AppState>()).ok().flatten();
    let shown = placement.is_some() && pet_bubble_show(app.clone(), None, None).is_some();
    app.state::<AppState>().bubble_controller.lock().unwrap().rendered(
        &character_id, generation, &delivery_id, shown, now_millis(),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BubbleDiagnosticsView {
    stage: String,
    delivery_id: Option<String>,
    pending: bool,
    ready_agent_id: Option<String>,
    ready_generation: u64,
}

#[tauri::command]
fn pet_bubble_diagnostics(state: tauri::State<'_, AppState>) -> BubbleDiagnosticsView {
    let mut controller = state.bubble_controller.lock().unwrap();
    controller.expire(now_millis());
    BubbleDiagnosticsView {
        stage: controller.last_stage.to_string(),
        delivery_id: controller.last_delivery_id.clone(),
        pending: controller.pending.is_some(),
        ready_agent_id: controller.ready_agent_id.clone(),
        ready_generation: controller.ready_generation,
    }
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
    app: tauri::AppHandle,
    dir: String,
    character_id: String,
) -> Result<pets::PetInfo, String> {
    let state = app.state::<AppState>();
    let row = state.store.lock().unwrap().get_character(&character_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Agent 不存在".to_string())?;
    let workspace = ensure_agent_workspace(&state, &row)?;
    let pending = pets::prepare_import_for_agent(Path::new(&dir), Path::new(&workspace))?;
    let info = pending.info().clone();
    let persistence = {
        let store = state.store.lock().unwrap();
        let existing = store.load_pet_state_mapping(&character_id).map_err(|e| e.to_string())?;
        let mapping = pets::reconcile_state_mapping(&existing, &info.animations);
        store.replace_character_pet_and_state_mapping(&character_id, &info.id, &mapping)
            .map_err(|e| e.to_string())
    };
    if let Err(error) = persistence {
        pending.rollback();
        return Err(error);
    }
    pending.commit();
    let is_current = state.settings.lock().unwrap().current_agent_id.as_deref() == Some(character_id.as_str());
    if is_current {
        sync_pet_host_visibility(&app);
        apply_current_pet_acrylic(&app);
    }
    Ok(info)
}

#[tauri::command]
fn pet_remove_pack(app: tauri::AppHandle, character_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let row = state.store.lock().unwrap().get_character(&character_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Agent 不存在".to_string())?;
    let workspace = ensure_agent_workspace(&state, &row)?;
    pets::remove_for_agent(Path::new(&workspace))?;
    state.store.lock().unwrap().set_character_pet(&character_id, None).map_err(|e| e.to_string())?;
    let is_current = state.settings.lock().unwrap().current_agent_id.as_deref() == Some(character_id.as_str());
    if is_current {
        sync_pet_host_visibility(&app);
    }
    Ok(())
}

#[tauri::command]
fn pet_list_packs(state: tauri::State<'_, AppState>) -> Result<Vec<pets::PetInfo>, String> {
    let characters = state.store.lock().unwrap().list_characters().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in characters {
        let workspace = ensure_agent_workspace(&state, &row)?;
        if let Ok(info) = pets::info_for_agent(Path::new(&workspace)) { out.push(info); }
    }
    Ok(out)
}

#[tauri::command]
fn pet_sheet_data(state: tauri::State<'_, AppState>, character_id: String) -> Result<String, String> {
    let row = state.store.lock().unwrap().get_character(&character_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Agent 不存在".to_string())?;
    if row.pet_pack_id.is_none() { return Err("Agent 未导入桌宠包".into()); }
    let workspace = ensure_agent_workspace(&state, &row)?;
    let info = pets::info_for_agent(Path::new(&workspace))?;
    let bytes = std::fs::read(&info.spritesheet_path).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PetAnimationData {
    animation: pets::PetAnimation,
    source_rect: pets::PetSourceRect,
    horizontal_correction: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PetBubblePlacement {
    anchor_x: f32,
    anchor_y: f32,
    accent: String,
}

#[tauri::command]
fn pet_bubble_placement(state: tauri::State<'_, AppState>) -> Result<Option<PetBubblePlacement>, String> {
    let current = state.settings.lock().unwrap().current_agent_id.clone();
    let Some(character_id) = current else { return Ok(None) };
    let row = state.store.lock().unwrap().get_character(&character_id).map_err(|e| e.to_string())?;
    let Some(row) = row else { return Ok(None) };
    if row.pet_pack_id.is_none() { return Ok(None) }
    let workspace = ensure_agent_workspace(&state, &row)?;
    let info = pets::info_for_agent(Path::new(&workspace))?;
    let anchor = info.bubble_anchor.unwrap_or(pets::PetAnchor { x: 0.5, y: 0.05 });
    Ok(Some(PetBubblePlacement {
        anchor_x: anchor.x.clamp(0.0, 1.0),
        anchor_y: anchor.y.clamp(0.0, 1.0),
        accent: info.bubble_accent,
    }))
}

#[tauri::command]
fn pet_animation_data(
    state: tauri::State<'_, AppState>,
    character_id: String,
    pet_state: String,
) -> Result<PetAnimationData, String> {
    if !pets::FOCUS_PET_STATES.contains(&pet_state.as_str()) {
        return Err("未知桌宠状态".into());
    }
    let row = state.store.lock().unwrap().get_character(&character_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Agent 不存在".to_string())?;
    let workspace = ensure_agent_workspace(&state, &row)?;
    let package = pets::info_for_agent(Path::new(&workspace))?;
    let mapping = state.store.lock().unwrap().load_pet_state_mapping(&character_id).map_err(|e| e.to_string())?;
    let animation = pets::resolve_state_animation(&pet_state, &mapping, &package.animations)
        .ok_or_else(|| "宠物包未发现可播放动画".to_string())?
        .clone();
    let source_rect = package.analyses.get(&animation.id)
        .map(|analysis| analysis.source_rect)
        .unwrap_or(pets::PetSourceRect {
            x: 0,
            y: 0,
            width: animation.cell_width,
            height: animation.cell_height,
        });
    Ok(PetAnimationData {
        animation,
        source_rect,
        horizontal_correction: package.horizontal_correction,
    })
}

#[tauri::command]
fn pet_set_horizontal_correction(
    app: tauri::AppHandle,
    character_id: String,
    horizontal_correction: f32,
) -> Result<pets::PetInfo, String> {
    let state = app.state::<AppState>();
    let row = state.store.lock().unwrap().get_character(&character_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Agent 不存在".to_string())?;
    let workspace = ensure_agent_workspace(&state, &row)?;
    pets::set_horizontal_correction_for_agent(Path::new(&workspace), horizontal_correction)?;
    let info = pets::info_for_agent(Path::new(&workspace))?;
    if state.settings.lock().unwrap().current_agent_id.as_deref() == Some(character_id.as_str()) {
        apply_current_pet_acrylic(&app);
    }
    let _ = app.emit("pet:changed", serde_json::json!({ "characterId": character_id }));
    Ok(info)
}

fn migrate_legacy_pet_packs(state: &AppState) -> Result<(), String> {
    let legacy = state.data_dir.join(pets::PETS_DIR);
    if !legacy.is_dir() { return Ok(()); }
    let mut first_agent_id = None;
    for pack in pets::list(&state.data_dir)? {
        let row = {
            let store = state.store.lock().unwrap();
            let existing = store.list_characters().map_err(|e| e.to_string())?
                .into_iter().find(|c| c.pet_pack_id.as_deref() == Some(pack.id.as_str()));
            match existing {
                Some(row) => row,
                None => store.create_agent(&pack.display_name, if pack.id == "focus-demo-pet" { "claude" } else { "codex" })
                    .map_err(|e| e.to_string())?,
            }
        };
        let workspace = ensure_agent_workspace(state, &row)?;
        let target = Path::new(&workspace).join(pets::AGENT_PET_DIR);
        if !target.is_dir() {
            pets::prepare_legacy_import_for_agent(&legacy.join(&pack.id), Path::new(&workspace))?
                .commit();
        }
        state.store.lock().unwrap().set_character_pet(&row.id, Some(&pack.id)).map_err(|e| e.to_string())?;
        if first_agent_id.is_none() { first_agent_id = Some(row.id); }
    }
    if let Some(id) = first_agent_id {
        let mut settings = state.settings.lock().unwrap();
        if settings.current_agent_id.is_none() {
            settings.current_agent_id = Some(id);
            settings.save(&state.data_dir)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn pet_activate(state: tauri::State<'_, AppState>, id: String) -> Result<pets::PetInfo, String> {
    let chars = state.store.lock().unwrap().list_characters().map_err(|e| e.to_string())?;
    let row = chars.into_iter().find(|c| c.pet_pack_id.as_deref() == Some(id.as_str()))
        .ok_or_else(|| "桌宠包不存在".to_string())?;
    let workspace = ensure_agent_workspace(&state, &row)?;
    pets::info_for_agent(Path::new(&workspace))
}

#[tauri::command]
fn pet_active(state: tauri::State<'_, AppState>) -> Result<Option<pets::PetInfo>, String> {
    let current = state.settings.lock().unwrap().current_agent_id.clone();
    let Some(id) = current else { return Ok(None) };
    let row = state.store.lock().unwrap().get_character(&id).map_err(|e| e.to_string())?;
    let Some(row) = row else { return Ok(None) };
    if row.pet_pack_id.is_none() { return Ok(None); }
    let workspace = ensure_agent_workspace(&state, &row)?;
    pets::info_for_agent(Path::new(&workspace)).map(Some)
}

#[tauri::command]
fn pet_get_state_mapping(
    state: tauri::State<'_, AppState>,
    character_id: String,
) -> Result<pets::StateMapping, String> {
    state.store.lock().unwrap().load_pet_state_mapping(&character_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn pet_save_state_mapping(
    app: tauri::AppHandle,
    character_id: String,
    mapping: pets::StateMapping,
) -> Result<(), String> {
    if mapping.keys().any(|state| !pets::FOCUS_PET_STATES.contains(&state.as_str())) {
        return Err("包含未知桌宠状态".into());
    }
    let state = app.state::<AppState>();
    let row = state.store.lock().unwrap().get_character(&character_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "Agent 不存在".to_string())?;
    let workspace = ensure_agent_workspace(&state, &row)?;
    let package = pets::info_for_agent(Path::new(&workspace))?;
    if let Some(invalid) = mapping.values().flatten().find(|id| !package.animations.iter().any(|animation| animation.id == **id)) {
        return Err(format!("动画不存在: {invalid}"));
    }
    let result = state.store.lock().unwrap()
        .save_pet_state_mapping(&character_id, &mapping)
        .map_err(|e| e.to_string());
    result
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
    if label == "pet" { position_pet_bubble_for_current_pet(&app); }
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

fn resolve_window_placement(
    settings: &Mutex<Settings>,
    data_dir: &Path,
    gm: &GridManager,
    label: &str,
    col: usize,
    row: usize,
) -> GridRect {
    let mut settings = settings.lock().unwrap();
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
            let _ = settings.save(data_dir);
            new_rect
        }
        Err(()) => current,
    }
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
    let rect = resolve_window_placement(
        &state.settings,
        &state.data_dir,
        &gm,
        label,
        col,
        row,
    );

    position_window(app, label, &rect, &gm);
    if label == "pet" {
        position_pet_bubble_for_current_pet(app);
    }
    raise_topbar(app);
    Ok(rect)
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
    let _operation = state.float_visibility_gate.try_enter()?;
    {
        let mut settings = state.settings.lock().unwrap();
        if settings.collapsed.contains(&label) {
            return Ok(()); // v1.10: no-op when already collapsed (#31)
        }
        settings.collapsed.push(label.clone());
        let _ = settings.save(&state.data_dir);
    }
    if let Some(w) = app.get_webview_window(&label) {
        hide_window_noactivate(&w);
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
    let state = app.state::<AppState>();
    if label == "pet" && !current_agent_has_valid_pet(&state) {
        sync_pet_host_visibility(app);
        return Err("The current Agent has no valid pet package".into());
    }
    let _operation = state.float_visibility_gate.try_enter()?;
    // v1.10: dedupe — restoring an already-visible window must not churn
    // show/position/topmost/raise (root cause of the freeze, #31).
    {
        let settings = state.settings.lock().unwrap();
        if !settings.collapsed.iter().any(|c| c == label) {
            if let Some(win) = app.get_webview_window(label) {
                if win.is_visible().unwrap_or(false) {
                    return Ok(());
                }
            }
        }
    }
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
        show_window_noactivate(&win);
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
    let opacity = {
        let mut s = state.settings.lock().unwrap();
        s.acrylic_enabled = enabled;
        let _ = s.save(&state.data_dir);
        glass_opacity(&s)
    };
    for label in ["chat", "stats", "music", "workflow"] {
        if let Some(w) = app.get_webview_window(label) {
            apply_acrylic_opt(&w, enabled, opacity);
        }
    }
    let _ = app.emit("settings:acrylic-changed", serde_json::json!({ "enabled": enabled, "opacity": opacity }));
    apply_current_pet_acrylic(&app);
    Ok(())
}

/// Global glass opacity (requirement #123): persists 5..100, re-applies the
/// native acrylic on every float and the pet, then fans the value out to all
/// WebView surfaces through the existing settings event.
#[tauri::command]
fn set_acrylic_opacity(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    opacity: u8,
) -> Result<(), String> {
    let (enabled, clamped) = {
        let mut s = state.settings.lock().unwrap();
        s.acrylic_opacity = opacity.clamp(5, 100);
        let _ = s.save(&state.data_dir);
        (s.acrylic_enabled, s.acrylic_opacity)
    };
    for label in ["chat", "stats", "music", "workflow"] {
        if let Some(w) = app.get_webview_window(label) {
            apply_acrylic_opt(&w, enabled, clamped);
        }
    }
    apply_current_pet_acrylic(&app);
    let _ = app.emit("settings:acrylic-changed", serde_json::json!({ "enabled": enabled, "opacity": clamped }));
    Ok(())
}

#[tauri::command]
fn set_chat_streaming_enabled(
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let mut s = state.settings.lock().unwrap();
    s.chat_streaming_enabled = enabled;
    s.save(&state.data_dir)
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
) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.distraction_apps = black;
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
) -> Result<(), String> {
    store
        .lock()
        .map_err(|e| e.to_string())?
        .record_focus_session(&started_at, &ended_at, duration_sec, None)
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
fn list_apps_catalog() -> Vec<String> {
    apps::list_apps_catalog()
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let focus_state = state.focus_state.lock().unwrap().clone();
    let active_focus_mode = state.active_focus_mode.lock().unwrap().clone();
    let is_restricted = should_reject_quit(&focus_state, active_focus_mode.as_deref());
    if is_restricted {
        return Err("当前专注模式不允许退出 Focus".into());
    }
    let _ = crate::desktop_lock::unlock_desktop();
    app.exit(0);
    Ok(())
}

fn should_reject_quit(focus_state: &str, active_focus_mode: Option<&str>) -> bool {
    focus_state == "focus"
        && matches!(active_focus_mode, Some("standard" | "scholar"))
}

// ---------------------------------------------------------------------------
// window creation
// ---------------------------------------------------------------------------

/// v1.10.3.1 (#46): physical initial rect for a float at its saved grid slot.

const FLOAT_NONCLIENT_STYLE_BITS: isize = 0x00cf_0000u32 as isize;
const WS_POPUP_STYLE: isize = 0x8000_0000u32 as isize;
const WM_NCCALCSIZE: u32 = 0x0083;
const WM_NCACTIVATE: u32 = 0x0086;
const WM_ERASEBKGND: u32 = 0x0014;

pub(crate) const fn float_corner_preference_attribute() -> u32 {
    33 // DWMWA_WINDOW_CORNER_PREFERENCE
}

pub(crate) const fn float_corner_preference_value() -> i32 {
    2 // DWMWCP_ROUND
}

/// The topbar is a transparent host: its exact visible pill is wholly owned by
/// the WebView, so it never asks Windows for rectangular acrylic composition.
pub(crate) const fn topbar_uses_native_composition() -> bool { false }

/// Topbar pill geometry (requirement #121): the 500x44 pill keeps its size and
/// position, while the transparent host reserves shadow margins so the
/// WebView-owned box-shadow (`0 6px 18px`) renders fully inside the window
/// and follows the pill curve exactly instead of being clipped by the
/// rectangular window bounds.
pub const TOPBAR_PILL_WIDTH: f64 = 500.0;
pub const TOPBAR_PILL_HEIGHT: f64 = 44.0;
pub const TOPBAR_SHADOW_LEFT: f64 = 20.0;
pub const TOPBAR_SHADOW_RIGHT: f64 = 20.0;
pub const TOPBAR_SHADOW_TOP: f64 = 14.0;
pub const TOPBAR_SHADOW_BOTTOM: f64 = 26.0;
pub const TOPBAR_WINDOW_WIDTH: f64 = TOPBAR_PILL_WIDTH + TOPBAR_SHADOW_LEFT + TOPBAR_SHADOW_RIGHT;
pub const TOPBAR_WINDOW_HEIGHT: f64 = TOPBAR_PILL_HEIGHT + TOPBAR_SHADOW_TOP + TOPBAR_SHADOW_BOTTOM;

static FLOAT_HOST_ORIGINAL_WNDPROCS:
    std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<isize, isize>>> =
    std::sync::OnceLock::new();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClientFrame {
    pub origin_x: i32,
    pub origin_y: i32,
    pub extra_width: u32,
    pub extra_height: u32,
}

/// A single snapshot of the visible client rectangle for a floating HWND.
/// Every drag coordinate is derived from this geometry: Windows moves the
/// outer HWND, while Focus renders and snaps the client content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClientGeometry {
    frame: ClientFrame,
    client_width: u32,
    client_height: u32,
}

impl ClientGeometry {
    pub(crate) fn from_native_rects(
        outer_x: i32,
        outer_y: i32,
        outer_width: u32,
        outer_height: u32,
        client_x: i32,
        client_y: i32,
        client_width: u32,
        client_height: u32,
    ) -> Self {
        Self {
            frame: ClientFrame {
                origin_x: client_x - outer_x,
                origin_y: client_y - outer_y,
                extra_width: outer_width.saturating_sub(client_width),
                extra_height: outer_height.saturating_sub(client_height),
            },
            client_width,
            client_height,
        }
    }

    pub(crate) fn client_rect_for_outer(&self, outer_x: i32, outer_y: i32) -> (i32, i32, u32, u32) {
        (
            outer_x + self.frame.origin_x,
            outer_y + self.frame.origin_y,
            self.client_width,
            self.client_height,
        )
    }

    pub(crate) fn outer_rect_for_client(
        &self,
        client_x: i32,
        client_y: i32,
        client_width: u32,
        client_height: u32,
    ) -> (i32, i32, u32, u32) {
        outer_rect_for_client(client_x, client_y, client_width, client_height, self.frame)
    }
}

pub(crate) fn float_host_style(style: isize) -> isize {
    (style & !FLOAT_NONCLIENT_STYLE_BITS) | WS_POPUP_STYLE
}

pub(crate) fn frame_change_required(previous: isize, configured: isize) -> bool {
    previous != configured
}

pub(crate) fn float_nonclient_message_result(message: u32) -> Option<isize> {
    match message {
        WM_NCCALCSIZE => Some(0),
        // Do not delegate activation to the default non-client renderer:
        // during a native drag it repaints the host title/caption even though
        // this popup's complete visible surface is client content.
        WM_NCACTIVATE => Some(1),
        WM_ERASEBKGND => Some(1),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn float_host_wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::{CallWindowProcW, DefWindowProcW, WNDPROC};

    if let Some(result) = float_nonclient_message_result(message) {
        return LRESULT(result);
    }

    let original: WNDPROC = FLOAT_HOST_ORIGINAL_WNDPROCS
        .get()
        .and_then(|procs| procs.lock().ok())
        .and_then(|procs| procs.get(&(hwnd.0 as isize)).copied())
        .map(|proc| unsafe { std::mem::transmute::<isize, WNDPROC>(proc) })
        .unwrap_or(None);
    if let Some(proc) = original {
        unsafe { CallWindowProcW(Some(proc), hwnd, message, wparam, lparam) }
    } else {
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }
}

pub(crate) fn outer_rect_for_client(
    client_x: i32,
    client_y: i32,
    client_width: u32,
    client_height: u32,
    frame: ClientFrame,
) -> (i32, i32, u32, u32) {
    (
        client_x - frame.origin_x,
        client_y - frame.origin_y,
        client_width.saturating_add(frame.extra_width),
        client_height.saturating_add(frame.extra_height),
    )
}

pub(crate) fn client_geometry_snapshot(w: &tauri::WebviewWindow) -> ClientGeometry {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::{HWND, POINT, RECT};
        use windows::Win32::Graphics::Gdi::ClientToScreen;
        use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};

        if let Ok(hwnd) = w.hwnd() {
            let hwnd_win = HWND(hwnd.0 as *mut core::ffi::c_void);
            unsafe {
                let mut outer = RECT::default();
                let mut client = RECT::default();
                if GetWindowRect(hwnd_win, &mut outer).is_ok()
                    && GetClientRect(hwnd_win, &mut client).is_ok()
                {
                    let mut client_origin = POINT {
                        x: client.left,
                        y: client.top,
                    };
                    if ClientToScreen(hwnd_win, &mut client_origin).as_bool() {
                    return ClientGeometry::from_native_rects(
                        outer.left,
                        outer.top,
                        (outer.right - outer.left).max(0) as u32,
                        (outer.bottom - outer.top).max(0) as u32,
                        client_origin.x,
                        client_origin.y,
                        (client.right - client.left).max(0) as u32,
                        (client.bottom - client.top).max(0) as u32,
                    );
                    }
                }
            }
        }
    }
    ClientGeometry::default()
}

/// Configure each float host exactly once, while it is still hidden. No later
/// lifecycle path may change the host style or send `SWP_FRAMECHANGED`.
fn configure_float_host(w: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::{HWND, POINT, RECT};
        use windows::Win32::Graphics::Gdi::ClientToScreen;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetClientRect, GetWindowLongPtrW, GetWindowRect, SetWindowLongPtrW, SetWindowPos,
            GWL_EXSTYLE, GWL_STYLE, GWLP_WNDPROC, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
            SWP_NOSIZE, SWP_NOZORDER, WS_EX_NOACTIVATE,
        };

        if let Ok(hwnd) = w.hwnd() {
            let hwnd_win = HWND(hwnd.0 as *mut core::ffi::c_void);
            unsafe {
                let mut outer = RECT::default();
                let mut client = RECT::default();
                let has_geometry = GetWindowRect(hwnd_win, &mut outer).is_ok()
                    && GetClientRect(hwnd_win, &mut client).is_ok();
                let style = GetWindowLongPtrW(hwnd_win, GWL_STYLE);
                let configured_style = float_host_style(style);
                let ex_style = GetWindowLongPtrW(hwnd_win, GWL_EXSTYLE);
                let configured_ex_style = ex_style | WS_EX_NOACTIVATE.0 as isize;

                if frame_change_required(style, configured_style) {
                    let _ = SetWindowLongPtrW(hwnd_win, GWL_STYLE, configured_style);
                }
                if configured_ex_style != ex_style {
                    let _ = SetWindowLongPtrW(hwnd_win, GWL_EXSTYLE, configured_ex_style);
                }

                // Native acrylic belongs to the HWND, not the WebView. Ask
                // DWM to clip that composition to system-rounded corners once
                // while this hidden host is configured.
                let corner_preference = float_corner_preference_value();
                let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
                    hwnd_win,
                    windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(
                        float_corner_preference_attribute() as i32,
                    ),
                    &corner_preference as *const i32 as *const core::ffi::c_void,
                    std::mem::size_of_val(&corner_preference) as u32,
                );

                let proc_ptr: unsafe extern "system" fn(
                    windows::Win32::Foundation::HWND,
                    u32,
                    windows::Win32::Foundation::WPARAM,
                    windows::Win32::Foundation::LPARAM,
                ) -> windows::Win32::Foundation::LRESULT = float_host_wnd_proc;
                let current_proc = GetWindowLongPtrW(hwnd_win, GWLP_WNDPROC);
                if current_proc != proc_ptr as isize {
                    let original = SetWindowLongPtrW(hwnd_win, GWLP_WNDPROC, proc_ptr as isize);
                    if original != 0 {
                        FLOAT_HOST_ORIGINAL_WNDPROCS
                            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
                            .lock()
                            .unwrap()
                            .insert(hwnd_win.0 as isize, original);
                    }
                }

                // This is the sole frame recalculation for this HWND, applying
                // the one-time WS_POPUP/WS_EX_NOACTIVATE configuration and the
                // full-client window procedure.
                if has_geometry {
                    let mut client_origin = POINT {
                        x: client.left,
                        y: client.top,
                    };
                    let (x, y) = if ClientToScreen(hwnd_win, &mut client_origin).as_bool() {
                        (client_origin.x, client_origin.y)
                    } else {
                        (outer.left, outer.top)
                    };
                    let _ = SetWindowPos(
                        hwnd_win,
                        None,
                        x,
                        y,
                        (client.right - client.left).max(0),
                        (client.bottom - client.top).max(0),
                        SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                } else {
                    let _ = SetWindowPos(
                        hwnd_win,
                        None,
                        0,
                        0,
                        0,
                        0,
                        SWP_FRAMECHANGED
                            | SWP_NOMOVE
                            | SWP_NOSIZE
                            | SWP_NOZORDER
                            | SWP_NOACTIVATE,
                    );
                }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = w;
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
pub(crate) fn show_window_noactivate(w: &tauri::WebviewWindow) {
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

pub(crate) fn hide_window_noactivate(w: &tauri::WebviewWindow) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScreenRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl ScreenRect {
    const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    fn right(self) -> i32 { self.x.saturating_add(self.width as i32) }
    fn bottom(self) -> i32 { self.y.saturating_add(self.height as i32) }
    fn intersection_area(self, other: Self) -> u64 {
        let width = (self.right().min(other.right()) - self.x.max(other.x)).max(0) as u64;
        let height = (self.bottom().min(other.bottom()) - self.y.max(other.y)).max(0) as u64;
        width * height
    }
    fn contains(self, other: Self) -> bool {
        other.x >= self.x && other.y >= self.y && other.right() <= self.right() && other.bottom() <= self.bottom()
    }
    fn clamp_inside(self, work: Self) -> Self {
        let max_x = work.right().saturating_sub(self.width as i32).max(work.x);
        let max_y = work.bottom().saturating_sub(self.height as i32).max(work.y);
        Self { x: self.x.clamp(work.x, max_x), y: self.y.clamp(work.y, max_y), ..self }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum BubbleDirection { Above, AboveLeft, AboveRight, Right, Left, Below }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BubblePosition {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    direction: BubbleDirection,
}

impl BubblePosition {
    fn rect(self) -> ScreenRect { ScreenRect::new(self.x, self.y, self.width, self.height) }
}

fn pet_client_rect_or_outer(
    outer_x: i32,
    outer_y: i32,
    outer_width: u32,
    outer_height: u32,
    client_x: i32,
    client_y: i32,
    client_width: u32,
    client_height: u32,
) -> ScreenRect {
    if client_width > 0 && client_height > 0 {
        ScreenRect::new(client_x, client_y, client_width, client_height)
    } else {
        ScreenRect::new(outer_x, outer_y, outer_width, outer_height)
    }
}

fn choose_bubble_placement(
    pet: ScreenRect,
    work: ScreenRect,
    chat: Option<ScreenRect>,
    bubble_width: u32,
    bubble_height: u32,
) -> Option<BubblePosition> {
    const GAP: i32 = 10;
    let bw = bubble_width as i32;
    let bh = bubble_height as i32;
    let center_x = pet.x + pet.width as i32 / 2 - bw / 2;
    let candidates = [
        (BubbleDirection::Above, center_x, pet.y - bh - GAP),
        (BubbleDirection::AboveLeft, pet.x - bw + pet.width as i32 / 3, pet.y - bh - GAP),
        (BubbleDirection::AboveRight, pet.right() - pet.width as i32 / 3, pet.y - bh - GAP),
        (BubbleDirection::Right, pet.right() + GAP, pet.y + pet.height as i32 / 2 - bh / 2),
        (BubbleDirection::Left, pet.x - bw - GAP, pet.y + pet.height as i32 / 2 - bh / 2),
        (BubbleDirection::Below, center_x, pet.bottom() + GAP),
    ];

    candidates.into_iter().enumerate()
        .map(|(index, (direction, x, y))| {
            let rect = ScreenRect::new(x, y, bubble_width, bubble_height).clamp_inside(work);
            let pet_overlap = rect.intersection_area(pet);
            let chat_overlap = chat.map(|chat| rect.intersection_area(chat)).unwrap_or(0);
            (pet_overlap, chat_overlap, index, BubblePosition {
                x: rect.x, y: rect.y, width: bubble_width, height: bubble_height, direction,
            })
        })
        .filter(|(pet_overlap, _, _, _)| *pet_overlap == 0)
        .min_by_key(|(_, chat_overlap, index, _)| (*chat_overlap, *index))
        .map(|(_, _, _, placement)| placement)
}

fn window_screen_rect(window: &tauri::WebviewWindow) -> Option<ScreenRect> {
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(ScreenRect::new(position.x, position.y, size.width, size.height))
}

fn desktop_work_area(fallback: ScreenRect) -> ScreenRect {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};
        let mut rect = RECT::default();
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&mut rect as *mut RECT).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        ).is_ok() {
            return ScreenRect::new(
                rect.left,
                rect.top,
                (rect.right - rect.left).max(0) as u32,
                (rect.bottom - rect.top).max(0) as u32,
            );
        }
    }
    fallback
}

fn position_pet_bubble(app: &tauri::AppHandle, _anchor_x: f32, _anchor_y: f32) -> Option<BubbleDirection> {
    let pet = app.get_webview_window("pet")?;
    let bubble = app.get_webview_window("pet-bubble")?;
    let geometry = client_geometry_snapshot(&pet);
    let pet_outer = pet.outer_position().unwrap_or_default();
    let pet_outer_size = pet.outer_size().unwrap_or(tauri::PhysicalSize::new(1, 1));
    let (client_x, client_y, client_w, client_h) = geometry.client_rect_for_outer(pet_outer.x, pet_outer.y);
    let bubble_size = bubble.outer_size().unwrap_or(tauri::PhysicalSize::new(248, 82));
    let monitor_rect = pet.current_monitor().ok().flatten()
        .map(|monitor| {
            let pos = monitor.position();
            let size = monitor.size();
            ScreenRect::new(pos.x, pos.y, size.width, size.height)
        })
        .unwrap_or(ScreenRect::new(0, 0, 1920, 1080));
    let work = desktop_work_area(monitor_rect);
    let chat = app.get_webview_window("chat")
        .filter(|window| window.is_visible().unwrap_or(false))
        .and_then(|window| window_screen_rect(&window));
    let placement = choose_bubble_placement(
        pet_client_rect_or_outer(
            pet_outer.x, pet_outer.y, pet_outer_size.width, pet_outer_size.height,
            client_x, client_y, client_w, client_h,
        ),
        work,
        chat,
        bubble_size.width,
        bubble_size.height,
    )?;
    let _ = crate::drag::move_window_raw(&bubble, placement.x, placement.y);
    Some(placement.direction)
}

fn position_pet_bubble_for_current_pet(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let current = state.settings.lock().unwrap().current_agent_id.clone();
    let Some(character_id) = current else { return };
    let row = state.store.lock().unwrap().get_character(&character_id).ok().flatten();
    let Some(row) = row else { return };
    let Ok(workspace) = ensure_agent_workspace(&state, &row) else { return };
    let anchor = pets::info_for_agent(Path::new(&workspace))
        .ok()
        .and_then(|info| info.bubble_anchor)
        .unwrap_or(pets::PetAnchor { x: 0.5, y: 0.05 });
    let _ = position_pet_bubble(app, anchor.x.clamp(0.0, 1.0), anchor.y.clamp(0.0, 1.0));
}

#[tauri::command]
fn pet_bubble_show(app: tauri::AppHandle, anchor_x: Option<f32>, anchor_y: Option<f32>) -> Option<BubbleDirection> {
    let direction = position_pet_bubble(&app, anchor_x.unwrap_or(0.5), anchor_y.unwrap_or(0.05));
    if let Some(bubble) = app.get_webview_window("pet-bubble") {
        if direction.is_some() {
            show_window_noactivate(&bubble);
        } else {
            hide_window_noactivate(&bubble);
        }
    }
    direction
}

#[tauri::command]
fn pet_bubble_hide(app: tauri::AppHandle) {
    if let Some(bubble) = app.get_webview_window("pet-bubble") {
        hide_window_noactivate(&bubble);
    }
}

/// Content-sized bubble host (requirement #124): resize through the native
/// no-activate path, then re-run placement with the new outer size so the
/// bubble still avoids the pet and the visible chat window.
#[tauri::command]
fn pet_bubble_resize(app: tauri::AppHandle, width: u32, height: u32) -> Option<BubbleDirection> {
    let bubble = app.get_webview_window("pet-bubble")?;
    // The WebView measures in CSS pixels; the native path uses physical
    // pixels (SetWindowPos), so convert with the window scale factor.
    let scale = bubble.scale_factor().unwrap_or(1.0);
    crate::drag::resize_window_raw(
        &bubble,
        ((width.max(120) as f64) * scale).round() as u32,
        ((height.max(40) as f64) * scale).round() as u32,
    );
    position_pet_bubble(&app, 0.5, 0.05)
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
    let state = app.state::<AppState>();
    let (grid, collapsed) = {
        let settings = state.settings.lock().unwrap();
        (settings.grid.clone(), settings.collapsed.clone())
    };

    // ADR-0037: the declarative window registry drives creation; every
    // builder flag mirrors the historical per-window builders exactly.
    for spec in WINDOW_SPECS {
        let mut builder = tauri::WebviewWindowBuilder::new(app, spec.label, url.clone())
            .title(spec.title)
            .decorations(false);
        if spec.transparent {
            builder = builder.transparent(true);
        }
        if spec.transparent_background {
            builder = builder.background_color(tauri::window::Color::from((0, 0, 0, 0))); // v1.10.3.1 (#42/#48)
        }
        if spec.always_on_top {
            builder = builder.always_on_top(true);
        }
        if spec.skip_taskbar {
            builder = builder.skip_taskbar(true);
        }
        if !spec.resizable {
            builder = builder.resizable(false);
        }
        if spec.fullscreen {
            builder = builder.fullscreen(true);
        }
        if spec.hidden_at_start {
            builder = builder.visible(false);
        }
        match spec.kind {
            WindowKind::Float => {
                let def = spec
                    .birth_rect
                    .unwrap_or(GridRect { col: 0, row: 0, cols: 2, rows: 2 });
                let (x, y, w, h, _collapsed) =
                    initial_float_rect(&grid, &collapsed, &gm, spec.label, def);
                builder = builder.position(x, y).inner_size(w, h);
            }
            WindowKind::Bubble | WindowKind::Topbar => {
                if let Some((w, h)) = spec.fixed_size {
                    builder = builder.inner_size(w, h);
                }
            }
            WindowKind::Desktop | WindowKind::Overlay => {}
        }
        let window = builder.build()?;
        if spec.float_host {
            configure_float_host(&window);
        }
        if spec.ignore_cursor_events {
            // informational surfaces only: never intercept mouse clicks on
            // apps underneath
            window.set_ignore_cursor_events(true)?;
            if let Ok(hwnd) = window.hwnd() {
                acrylic::noactivate(hwnd.0);
            }
        }
    }

    // Initial visibility: floats that are not collapsed show at startup;
    // the pet additionally requires a valid package for the current Agent.
    for label in float_labels() {
        if !collapsed.iter().any(|c| c == label)
            && (label != "pet" || current_agent_has_valid_pet(&app.state::<AppState>()))
        {
            if let Some(window) = app.get_webview_window(label) {
                show_window_noactivate(&window);
            }
        }
    }

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
            show_window_noactivate(&w);
            raise_topbar(app);
        } else {
            hide_window_noactivate(&w);
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
                hide_window_noactivate(&w);
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
        let saved = float_labels()
            .map(|label| {
                let default_rect = window_spec::spec(label)
                    .and_then(|s| s.default_rect)
                    .unwrap_or(GridRect {
                        col: 0,
                        row: 0,
                        cols: 2,
                        rows: 2,
                    });
                (label.to_string(), settings.grid.get(label).copied().unwrap_or(default_rect))
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

    for label in float_labels() {
        if let Some(rect) = resolved.get(label) {
            position_window(&app.handle(), label, rect, &gm);
        }
        if let Some(win) = app.get_webview_window(label) {
            let top = *topmost.get(label).unwrap_or(&true);
            set_float_topmost_noactivate(&win, top);
            let visible = resolved.contains_key(label)
                && !collapsed.contains(&label.to_string())
                && (label != "pet" || current_agent_has_valid_pet(state));
            if !visible {
                hide_window_noactivate(&win);
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
                drag_diagnostics: drag::DragDiagnosticRecorder::from_environment(data_dir_clone.clone()),
                launch_lock: tokio::sync::Mutex::new(()),
                float_visibility_gate: FloatVisibilityGate::default(),
                focus_track: Mutex::new(supervision::FocusTrack::default()),
                focus_state: Mutex::new("idle".to_string()),
                active_focus_mode: Mutex::new(None),
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
                bubble_controller: Mutex::new(BubbleController::default()),
                bubble_next_id: AtomicU64::new(0),
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
            migrate_legacy_pet_packs(&app.state::<AppState>())?;
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
            let glass_opacity_value = {
                let app_state = app.state::<AppState>();
                let s = app_state.settings.lock().unwrap();
                glass_opacity(&s)
            };
            // ADR-0037: glass setup derives from the registry; pet's glass is
            // applied separately via its derived-tint path below.
            for spec in WINDOW_SPECS.iter().filter(|s| s.setup_acrylic) {
                if let Some(w) = app.get_webview_window(spec.label) {
                    apply_acrylic_opt(&w, acrylic_enabled, glass_opacity_value);
                }
            }
            apply_current_pet_acrylic(&app.handle());

            let app_state = app.state::<AppState>();
            apply_initial_layout(app, &app_state);

            // always-on-top status capsule: top-center of the primary screen
            if let Some(tb) = app.get_webview_window("topbar") {
                let _ = tb.set_position(LogicalPosition::new(
                    ((sw - TOPBAR_WINDOW_WIDTH) / 2.0).max(0.0),
                    (8.0 - TOPBAR_SHADOW_TOP).max(0.0),
                ));
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
                    let Ok(_operation) = state.float_visibility_gate.try_enter() else {
                        return;
                    };
                    let visible = w.is_visible().unwrap_or(true);
                    if visible {
                        hide_window_noactivate(&w);
                    } else {
                        show_window_noactivate(&w);
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
                    let state = hd.state::<AppState>();
                    if let Some(finished) = drag::finish_drag(&state, &label) {
                        state.drag_diagnostics.record(finished.sequence, &label, "rust", "release:claimed", false);
                        state.drag_diagnostics.arm_post_release_click();
                        drag::finalize(&hd, &label, finished.sequence);
                    }
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
                let mode = v.get("mode").and_then(|x| x.as_str()).map(str::to_string);
                let completed = v
                    .get("completed")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                let app_state = hf.state::<AppState>();
                *app_state.focus_state.lock().unwrap() = state.clone();
                *app_state.active_focus_mode.lock().unwrap() = if state == "focus" {
                    mode
                } else {
                    None
                };
                // M4/ADR-0012: focus-end trigger via the core event bus
                let _ = app_state.events_tx.send(CoreEvent::FocusStateChanged {
                    state: state.clone(),
                    completed,
                });
                let mut ft = app_state.focus_track.lock().unwrap();
                match state.as_str() {
                    "focus" => {
                        if !ft.active {
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
                            let store_state = hf.state::<std::sync::Arc<Mutex<storage::Store>>>();
                            match store_state.lock() {
                                Ok(store) => {
                                    if let Err(e) = store.record_focus_session(
                                        &started,
                                        &ended,
                                        dur,
                                        None,
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
            drag::drag_diagnostic_browser_event,
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
            set_acrylic_opacity,
            set_chat_streaming_enabled,
            set_focus_durations,
            set_focus_mode,
            set_distraction_lists,
            set_sound_enabled,
            set_show_topbar,
            list_running_apps,
            list_apps_catalog,
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
            agent_workflow_reference_count,
            agent_create,
            agent_set_current,
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
            pet_animation_data,
            pet_set_horizontal_correction,
            pet_active,
            pet_bubble_placement,
            pet_bubble_show,
            pet_bubble_hide,
            pet_bubble_resize,
            pet_bubble_ready,
            pet_bubble_rendered,
            pet_bubble_diagnostics,
            pet_get_state_mapping,
            pet_save_state_mapping,
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
        float_corner_preference_attribute, float_corner_preference_value, float_host_style,
        float_nonclient_message_result, frame_change_required, is_float_label, list_provider_skills,
        outer_rect_for_client, pet_client_rect_or_outer, pet_window_should_be_visible, provider_ready, provider_skills_dir,
        resolve_window_placement, ClientFrame, ClientGeometry, ScreenRect,
        FloatVisibilityGate,
        resume_with_initial_message, saved_session_for_today, select_status_character,
        set_agent_provider_serialized_with, topbar_uses_native_composition, topbar_visible, with_agent_runtime_serialized,
        BubbleController, PendingBubble, PENDING_BUBBLE_TTL_MS,
    };

    #[test]
    fn bubble_controller_keeps_delivery_until_matching_render_ack_and_expiry() {
        let mut controller = BubbleController::default();
        controller.pending = Some(PendingBubble {
            delivery_id: "bubble-1".into(),
            agent_id: "char-a".into(),
            text: "hello".into(),
            priority: "normal".into(),
            created_at_ms: 1_000,
        });
        assert_eq!(controller.ready("char-b", 1, 1_001), None);
        assert!(controller.pending.is_some());
        assert_eq!(controller.ready("char-a", 1, 1_001).unwrap().delivery_id, "bubble-1");
        assert!(controller.pending.is_some(), "ready must not consume before render acknowledgement");
        assert!(!controller.rendered("char-a", 0, "bubble-1", true, 1_001));
        assert!(controller.pending.is_some(), "stale generation must not consume");
        assert!(controller.rendered("char-a", 1, "bubble-1", true, 1_001));
        assert!(controller.pending.is_none());

        controller.pending = Some(PendingBubble {
            delivery_id: "bubble-2".into(),
            agent_id: "char-a".into(),
            text: "expired".into(),
            priority: "normal".into(),
            created_at_ms: 1_000,
        });
        assert_eq!(controller.ready("char-a", 2, 1_000 + PENDING_BUBBLE_TTL_MS + 1), None);
        assert!(controller.pending.is_none());
    }

    #[test]
    fn successful_placement_releases_settings_before_post_placement_work() {
        let data_dir = std::env::temp_dir().join(format!(
            "focus-placement-lock-success-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut initial = crate::settings::Settings::default();
        initial.grid.clear();
        initial.grid.insert(
            "pet".into(),
            crate::settings::GridRect { col: 0, row: 0, cols: 1, rows: 1 },
        );
        let settings = std::sync::Mutex::new(initial);
        let gm = crate::grid::GridManager { screen_w: 1200.0, screen_h: 800.0 };

        let rect = resolve_window_placement(&settings, &data_dir, &gm, "pet", 2, 2);

        assert_eq!((rect.col, rect.row), (2, 2));
        assert!(settings.try_lock().is_ok());
        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn occupied_placement_releases_settings_before_snap_back_work() {
        let data_dir = std::env::temp_dir().join(format!(
            "focus-placement-lock-occupied-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut initial = crate::settings::Settings::default();
        initial.grid.clear();
        initial.grid.insert(
            "pet".into(),
            crate::settings::GridRect { col: 0, row: 0, cols: 1, rows: 1 },
        );
        initial.grid.insert(
            "music".into(),
            crate::settings::GridRect { col: 2, row: 2, cols: 1, rows: 1 },
        );
        let settings = std::sync::Mutex::new(initial);
        let gm = crate::grid::GridManager { screen_w: 1200.0, screen_h: 800.0 };

        let rect = resolve_window_placement(&settings, &data_dir, &gm, "pet", 2, 2);

        assert_eq!((rect.col, rect.row), (0, 0));
        assert!(settings.try_lock().is_ok());
        std::fs::remove_dir_all(data_dir).unwrap();
    }

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
    fn pet_window_requires_a_valid_current_agent_package() {
        assert!(pet_window_should_be_visible(true, false));
        assert!(!pet_window_should_be_visible(false, false));
        assert!(!pet_window_should_be_visible(true, true));
    }

    #[test]
    fn pet_host_tint_accepts_only_a_complete_rgb_hex_value() {
        assert_eq!(super::parse_rgb_hex("#1a80ff"), Some((0x1a, 0x80, 0xff)));
        assert_eq!(super::parse_rgb_hex("1a80ff"), None);
        assert_eq!(super::parse_rgb_hex("#fff"), None);
        assert_eq!(super::parse_rgb_hex("#zz80ff"), None);
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
                        agents::agent_display_full(false),
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
                    agents::agent_display_full(false),
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
                agents::agent_display_full(false),
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
            agents::agent_display_full(false),
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
        assert!(
            !is_float_label("pet-bubble"),
            "the mouse-through companion must never enter the grid or tray lifecycle"
        );
    }

    #[test]
    fn float_visibility_gate_rejects_an_overlapping_operation_then_reopens() {
        let gate = FloatVisibilityGate::default();
        let first = gate.try_enter().expect("first operation enters");
        assert!(gate.try_enter().is_err());
        drop(first);
        assert!(gate.try_enter().is_ok());
    }

    #[test]
    fn float_host_style_is_popup_without_nonclient_frame() {
        let decorated = 0x10cf_0000u32 as isize;
        let configured = float_host_style(decorated);
        let nonclient = 0x00cf_0000u32 as isize;

        assert_eq!(configured & nonclient, 0);
        assert_ne!(configured & 0x8000_0000u32 as isize, 0);
        assert!(frame_change_required(decorated, configured));
        assert!(!frame_change_required(configured, configured));
    }

    #[test]
    fn float_host_keeps_a_full_client_rect_without_default_background_erase() {
        assert_eq!(float_nonclient_message_result(0x0083), Some(0));
        assert_eq!(float_nonclient_message_result(0x0014), Some(1));
        assert_eq!(float_nonclient_message_result(0x0086), Some(1));
        assert_eq!(float_nonclient_message_result(0x000f), None);
    }

    #[test]
    fn float_hosts_prefer_dwm_rounded_corners_for_native_acrylic() {
        assert_eq!(float_corner_preference_attribute(), 33);
        assert_eq!(float_corner_preference_value(), 2);
    }

    #[test]
    fn glass_alpha_preserves_default_and_clamps_extremes() {
        // opacity 22 (the default) reproduces historical alphas exactly.
        assert_eq!(crate::glass_alpha(56, 22), 56);
        assert_eq!(crate::glass_alpha(64, 22), 64);
        // The most solid slider value saturates at 255.
        assert_eq!(crate::glass_alpha(56, 100), 255);
        assert_eq!(crate::glass_alpha(64, 100), 255);
        // The most transparent value never degrades below the floor.
        assert_eq!(crate::glass_alpha(56, 5), 13);
        assert_eq!(crate::glass_alpha(64, 5), 15);
        assert!(crate::glass_alpha(56, 1) >= 8, "out-of-range input still floors");
        // Monotonic in opacity.
        let a = crate::glass_alpha(56, 10);
        let b = crate::glass_alpha(56, 30);
        let c = crate::glass_alpha(56, 80);
        assert!(a < b && b < c, "alpha must grow with opacity: {a} {b} {c}");
    }

    #[test]
    fn topbar_host_reserves_shadow_margins_around_the_pill() {
        assert_eq!(crate::TOPBAR_WINDOW_WIDTH, crate::TOPBAR_PILL_WIDTH + crate::TOPBAR_SHADOW_LEFT + crate::TOPBAR_SHADOW_RIGHT);
        assert_eq!(crate::TOPBAR_WINDOW_HEIGHT, crate::TOPBAR_PILL_HEIGHT + crate::TOPBAR_SHADOW_TOP + crate::TOPBAR_SHADOW_BOTTOM);
        assert_eq!(crate::TOPBAR_PILL_WIDTH, 500.0);
        assert_eq!(crate::TOPBAR_PILL_HEIGHT, 44.0);
        // Every side's margin must cover the shadow extent so the WebView
        // pill shadow is never clipped by the host bounds (#121): blur 18px,
        // vertical offset 6px down.
        assert!(crate::TOPBAR_SHADOW_LEFT >= 18.0 && crate::TOPBAR_SHADOW_RIGHT >= 18.0);
        assert!(crate::TOPBAR_SHADOW_TOP >= 12.0 && crate::TOPBAR_SHADOW_BOTTOM >= 24.0);
    }

    #[test]
    fn topbar_has_no_native_composition_or_float_lifecycle() {
        assert!(!topbar_uses_native_composition());
        assert!(!is_float_label("topbar"), "topbar remains outside grid/tray lifecycle");
    }

    #[test]
    fn bubble_positioning_uses_pet_outer_rect_when_hidden_client_geometry_is_zero() {
        assert_eq!(
            pet_client_rect_or_outer(120, 800, 96, 96, 0, 0, 0, 0),
            ScreenRect::new(120, 800, 96, 96),
        );
        assert_eq!(
            pet_client_rect_or_outer(120, 800, 96, 96, 122, 802, 92, 92),
            ScreenRect::new(122, 802, 92, 92),
        );
    }

    #[test]
    fn client_grid_rect_converts_to_outer_rect_from_live_frame_geometry() {
        let frame = ClientFrame {
            origin_x: 13,
            origin_y: 8,
            extra_width: 26,
            extra_height: 16,
        };

        assert_eq!(
            outer_rect_for_client(320, 180, 1024, 768, frame),
            (307, 172, 1050, 784)
        );
    }

    #[test]
    fn client_geometry_keeps_drag_preview_snap_and_final_placement_aligned() {
        let geometry = ClientGeometry::from_native_rects(
            307,
            172,
            1050,
            784,
            320,
            180,
            1024,
            768,
        );

        // A drag moves the outer HWND; the preview must use the corresponding
        // visible client rect, and restoring that client rect must yield the
        // same outer rect used by final placement.
        let preview = geometry.client_rect_for_outer(507, 372);
        assert_eq!(preview, (520, 380, 1024, 768));
        assert_eq!(geometry.outer_rect_for_client(preview.0, preview.1, preview.2, preview.3), (507, 372, 1050, 784));
    }

    #[test]
    fn bubble_placement_uses_client_geometry_and_normalized_package_anchor() {
        let placement = super::choose_bubble_placement(
            super::ScreenRect::new(100, 200, 192, 208),
            super::ScreenRect::new(0, 0, 1200, 800),
            None,
            248,
            82,
        ).expect("a pet-safe candidate should exist");
        assert_eq!(placement.direction, super::BubbleDirection::Above);
        assert_eq!((placement.x, placement.y), (72, 108));
        assert_eq!(placement.rect().intersection_area(super::ScreenRect::new(100, 200, 192, 208)), 0);
    }

    #[test]
    fn bubble_placement_uses_all_six_directions_and_never_covers_the_pet() {
        let work = super::ScreenRect::new(0, 0, 640, 480);
        for pet in [
            super::ScreenRect::new(220, 180, 120, 140),
            super::ScreenRect::new(0, 0, 120, 140),
            super::ScreenRect::new(520, 0, 120, 140),
            super::ScreenRect::new(0, 340, 120, 140),
            super::ScreenRect::new(520, 340, 120, 140),
        ] {
            let placement = super::choose_bubble_placement(pet, work, None, 248, 82)
                .expect("a pet-safe candidate should exist");
            assert_eq!(placement.rect().intersection_area(pet), 0, "{placement:?}");
            assert!(work.contains(placement.rect()), "{placement:?}");
        }
    }

    #[test]
    fn bubble_placement_prefers_a_later_candidate_when_chat_blocks_above() {
        let pet = super::ScreenRect::new(300, 260, 120, 140);
        let work = super::ScreenRect::new(0, 0, 900, 700);
        let chat = super::ScreenRect::new(260, 120, 360, 130);
        let placement = super::choose_bubble_placement(pet, work, Some(chat), 248, 82)
            .expect("a pet-safe candidate should exist");

        assert_eq!(placement.rect().intersection_area(pet), 0);
        assert_eq!(placement.rect().intersection_area(chat), 0);
        assert_ne!(placement.direction, super::BubbleDirection::Above);
    }

    #[test]
    fn bubble_placement_returns_none_when_no_candidate_is_pet_safe() {
        let work = super::ScreenRect::new(0, 0, 120, 120);
        let pet = super::ScreenRect::new(0, 0, 120, 120);
        assert!(super::choose_bubble_placement(pet, work, None, 100, 80).is_none());
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
    #[test]
    fn quit_restriction_uses_the_frozen_round_mode() {
        assert!(super::should_reject_quit("focus", Some("standard")));
        assert!(super::should_reject_quit("focus", Some("scholar")));
        assert!(!super::should_reject_quit("focus", Some("light")));
        assert!(!super::should_reject_quit("rest", Some("scholar")));
        assert!(!super::should_reject_quit("focus", None));
    }
}
