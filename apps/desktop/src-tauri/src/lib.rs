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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
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
    pub events_tx: tokio::sync::broadcast::Sender<CoreEvent>,
    pub agent: Mutex<agents::AgentRuntime>,
    pub agent_fallback: AtomicBool,
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


fn position_window(app: &tauri::AppHandle, label: &str, rect: &GridRect, gm: &GridManager) {
    if let Some(w) = app.get_webview_window(label) {
        let (x, y, wpx, hpx) = gm.rect_to_logical(rect);
        let _ = w.set_position(LogicalPosition::new(x, y));
        let _ = w.set_size(LogicalSize::new(wpx, hpx));
    }
}

fn emit_visibility(app: &tauri::AppHandle, label: &str, visible: bool) {
    let _ = app.emit("window:visibility", serde_json::json!({ "label": label, "visible": visible }));
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
    agent_provider: String,
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
        agent_provider: s.agent_provider.clone(),
        agent_workspace_dir: s.agent_workspace_dir.clone(),
        pet_bg_fade: s.pet_bg_fade,
    }
}

#[derive(Clone)]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentStatusView {
    provider: String,
    fallback: bool,
    ready: bool,
    exe_path: Option<String>,
    workspace_dir: String,
}

fn user_home() -> String {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into())
}

fn current_workspace_dir(state: &AppState) -> String {
    let s = state.settings.lock().unwrap();
    s.agent_workspace_dir.clone().unwrap_or_else(user_home)
}

fn agent_status_view(app: &tauri::AppHandle) -> AgentStatusView {
    let state = app.state::<AppState>();
    let fallback = state.agent_fallback.load(std::sync::atomic::Ordering::Relaxed);
    let kind = state.agent.lock().unwrap().kind();
    let ws = current_workspace_dir(&state);
    let exe_path = if kind == agents::AgentProviderKind::Codex {
        agents::codex::find_codex_exe().map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };
    AgentStatusView {
        provider: kind.as_str().to_string(),
        fallback,
        ready: !fallback,
        exe_path,
        workspace_dir: ws,
    }
}

fn emit_agent_status(app: &tauri::AppHandle) {
    let _ = app.emit("agent:status", agent_status_view(app));
}

fn rebuild_agent_runtime(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let provider = {
        let s = state.settings.lock().unwrap();
        s.agent_provider.clone()
    };
    let tx = state.events_tx.clone();
    let mut slot = state.agent.lock().unwrap();
    match provider.as_str() {
        "mock" => {
            *slot = agents::AgentRuntime::Mock(std::sync::Mutex::new(agents::mock::MockProvider::new(tx)));
            state.agent_fallback.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        _ => match agents::codex::find_codex_exe() {
            Some(exe) => {
                *slot = agents::AgentRuntime::Codex(std::sync::Arc::new(std::sync::Mutex::new(
                    agents::codex::CodexProvider::new(tx, exe),
                )));
                state.agent_fallback.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            None => {
                *slot = agents::AgentRuntime::Mock(std::sync::Mutex::new(agents::mock::MockProvider::new(tx)));
                state.agent_fallback.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        },
    }
}

/// Runs a provider call; when the Codex binary is unavailable at runtime,
/// swaps the slot to Mock (fallback badge) and retries once (ADR-0007).
fn with_agent<R>(
    app: &tauri::AppHandle,
    mut f: impl FnMut(&agents::AgentRuntime) -> Result<R, String>,
) -> Result<R, String> {
    let state = app.state::<AppState>();
    let mut swapped = false;
    loop {
        let r = {
            let slot = state.agent.lock().unwrap();
            match &*slot {
                agents::AgentRuntime::Codex(p) => {
                    let p2 = p.clone();
                    drop(slot);
                    let tmp = agents::AgentRuntime::Codex(p2);
                    f(&tmp)
                }
                agents::AgentRuntime::Mock(_) => f(&slot),
            }
        };
        match r {
            Ok(v) => return Ok(v),
            Err(e) if !swapped => {
                let kind = state.agent.lock().unwrap().kind();
                if kind == agents::AgentProviderKind::Codex
                    && agents::codex::is_unavailable_error(&e)
                {
                    let tx = state.events_tx.clone();
                    *state.agent.lock().unwrap() =
                        agents::AgentRuntime::Mock(std::sync::Mutex::new(
                            agents::mock::MockProvider::new(tx),
                        ));
                    state.agent_fallback.store(true, std::sync::atomic::Ordering::Relaxed);
                    swapped = true;
                    emit_agent_status(app);
                } else {
                    return Err(e);
                }
            }
            Err(e) => return Err(e),
        }
    }
}

#[tauri::command]
fn agent_status(app: tauri::AppHandle) -> AgentStatusView {
    agent_status_view(&app)
}

#[tauri::command]
fn agent_start_thread(
    app: tauri::AppHandle,
    initial_message: String,
) -> Result<agents::AgentThreadInfo, String> {
    let state = app.state::<AppState>();
    let ws = current_workspace_dir(&state);
    with_agent(&app, |rt| rt.start_thread(&ws, &initial_message))
}

#[tauri::command]
fn agent_resume_thread(
    app: tauri::AppHandle,
    thread_id: String,
) -> Result<agents::AgentThreadInfo, String> {
    with_agent(&app, |rt| rt.resume_thread(&thread_id))
}

#[tauri::command]
fn agent_list_threads(app: tauri::AppHandle) -> Result<Vec<agents::AgentThreadInfo>, String> {
    with_agent(&app, |rt| rt.list_threads())
}

#[tauri::command]
fn agent_send(app: tauri::AppHandle, thread_id: String, text: String) -> Result<(), String> {
    with_agent(&app, |rt| rt.send(&thread_id, &text))
}

#[tauri::command]
fn agent_interrupt(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    with_agent(&app, |rt| rt.interrupt(&thread_id))
}

#[tauri::command]
fn agent_list_skills() -> Result<Vec<String>, String> {
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
}

#[tauri::command]
fn set_agent_provider(app: tauri::AppHandle, provider: String) -> Result<(), String> {
    let kind = agents::AgentProviderKind::parse(&provider).ok_or("provider 需为 codex 或 mock")?;
    {
        let state = app.state::<AppState>();
        let mut s = state.settings.lock().unwrap();
        s.agent_provider = kind.as_str().to_string();
        let _ = s.save(&state.data_dir);
    }
    rebuild_agent_runtime(&app);
    emit_agent_status(&app);
    Ok(())
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
fn pet_import_pack(state: tauri::State<'_, AppState>, dir: String) -> Result<pets::PetInfo, String> {
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
    let current = settings.grid.get(label).copied().unwrap_or(GridRect { col: 0, row: 0, cols, rows });
    let occupied = occupied_rects(&settings, Some(label));
    let target = GridRect { col: current.col, row: current.row, cols, rows };
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
    let gm = GridManager { screen_w: w, screen_h: h };
    let mut settings = state.settings.lock().unwrap();
    let current = settings
        .grid
        .get(&label)
        .copied()
        .unwrap_or(GridRect { col: 0, row: 0, cols, rows });
    let rect = GridRect { col: current.col, row: current.row, cols, rows };
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
    GridManager { screen_w: w, screen_h: h }.metrics()
}

pub(crate) fn place_window_inner(
    app: &tauri::AppHandle,
    state: &AppState,
    label: &str,
    col: usize,
    row: usize,
) -> Result<GridRect, String> {
    let (w, h) = *state.screen.lock().unwrap();
    let gm = GridManager { screen_w: w, screen_h: h };
    let mut settings = state.settings.lock().unwrap();
    let current = settings.grid.get(label).copied().unwrap_or(GridRect { col: 0, row: 0, cols: 2, rows: 2 });
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
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.set_always_on_top(topmost);
    }
    state.settings.lock().unwrap().topmost.insert(label, topmost);
    let _ = state.settings.lock().unwrap().save(&state.data_dir);
    Ok(())
}

#[tauri::command]
fn collapse(app: tauri::AppHandle, state: tauri::State<'_, AppState>, label: String) -> Result<(), String> {
    {
        let mut settings = state.settings.lock().unwrap();
        if !settings.collapsed.contains(&label) {
            settings.collapsed.push(label.clone());
        }
        let _ = settings.save(&state.data_dir);
    }
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.hide();
    }
    emit_visibility(&app, &label, false);
    Ok(())
}

#[tauri::command]
fn restore(app: tauri::AppHandle, state: tauri::State<'_, AppState>, label: String) -> Result<(), String> {
    {
        let mut settings = state.settings.lock().unwrap();
        settings.collapsed.retain(|c| c != &label);
        let _ = settings.save(&state.data_dir);
    }
    let (w, h) = *state.screen.lock().unwrap();
    let gm = GridManager { screen_w: w, screen_h: h };
    let rect = state.settings.lock().unwrap().grid.get(&label).copied().unwrap_or(GridRect { col: 0, row: 0, cols: 2, rows: 2 });
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_always_on_top(*state.settings.lock().unwrap().topmost.get(&label).unwrap_or(&true));
        let _ = win.show();
    }
    position_window(&app, &label, &rect, &gm);
    emit_visibility(&app, &label, true);
    raise_topbar(&app);
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
    let display = if name.trim().is_empty() { url.clone() } else { name.trim().to_string() };
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
        rows.iter().find(|r| r.id == id).cloned().ok_or("shortcut not found")?
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
fn set_acrylic(app: tauri::AppHandle, state: tauri::State<'_, AppState>, enabled: bool) -> Result<(), String> {
    {
        let mut s = state.settings.lock().unwrap();
        s.acrylic_enabled = enabled;
        let _ = s.save(&state.data_dir);
    }
    for label in ["chat", "stats", "music", "pet"] {
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
fn set_focus_durations(state: tauri::State<'_, AppState>, focus: u32, rest: u32) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.focus_minutes = focus.clamp(1, 240);
    settings.rest_minutes = rest.clamp(1, 120);
    let _ = settings.save(&state.data_dir);
    Ok(())
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
    app.exit(0);
}

// ---------------------------------------------------------------------------
// window creation
// ---------------------------------------------------------------------------

fn create_windows(app: &mut tauri::App) -> tauri::Result<()> {
    let url = tauri::WebviewUrl::App("index.html".into());

    tauri::WebviewWindowBuilder::new(app, "desktop", url.clone())
        .title("Focus Desktop")
        .fullscreen(true)
        .decorations(false)
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "chat", url.clone())
        .title("对话")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "stats", url.clone())
        .title("统计")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "music", url.clone())
        .title("音乐")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "pet", url.clone())
        .title("桌宠")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .build()?;

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
            let _ = w.show();
            raise_topbar(app);
        } else {
            let _ = w.hide();
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
    let Some(w) = app.get_webview_window("topbar") else { return };
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
    for label in ["chat", "stats", "music", "pet"] {
        if collapsed.contains(&label.to_string()) {
            if let Some(w) = app.get_webview_window(label) {
                let _ = w.hide();
            }
        }
    }
}

fn apply_initial_layout(app: &tauri::App, state: &AppState) {
    let (w, h) = *state.screen.lock().unwrap();
    let gm = GridManager { screen_w: w, screen_h: h };
    let settings = state.settings.lock().unwrap();

    for label in ["chat", "stats", "music", "pet"] {
        if let Some(rect) = settings.grid.get(label) {
            position_window(&app.handle(), label, rect, &gm);
        }
        if let Some(win) = app.get_webview_window(label) {
            let top = *settings.topmost.get(label).unwrap_or(&true);
            let _ = win.set_always_on_top(top);
            let collapsed = settings.collapsed.contains(&label.to_string());
            if collapsed {
                let _ = win.hide();
            }
            emit_visibility(&app.handle(), label, !collapsed);
        }
    }
    drop(settings);
}

// ---------------------------------------------------------------------------
// entry
// ---------------------------------------------------------------------------

pub fn run() {
    // v1.8.1 single-instance guard: a second process must exit immediately so
    // two instances never share the same SQLite DB / settings (which could
    // silently drop writes). The mutex handle lives for the process lifetime
    // and is released by the OS on exit.
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{GetLastError, SetLastError, ERROR_ALREADY_EXISTS, WIN32_ERROR};
        use windows::Win32::System::Threading::CreateMutexW;
        let name = windows::core::HSTRING::from("Local\\FocusDesktop_SingleInstance");
        unsafe { SetLastError(WIN32_ERROR(0)); }
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
                let _ = app.asset_protocol_scope().allow_directory(std::path::Path::new(&music_folder), true);
            }
            let legacy_shortcuts = settings.shortcuts.clone();
            let data_dir_clone = data_dir.clone();
            let (events_tx, _boot_rx) = tokio::sync::broadcast::channel::<CoreEvent>(256);
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
                events_tx: events_tx.clone(),
                agent: Mutex::new(agents::AgentRuntime::Mock(std::sync::Mutex::new(agents::mock::MockProvider::new(
                    events_tx.clone(),
                )))),
                agent_fallback: AtomicBool::new(false),
            };
            app.manage(state);
            rebuild_agent_runtime(&app.handle());

            // v1.5: DB store must be managed before the desktop webview calls
            // get_bootstrap on mount (otherwise: state not managed for field
            // `store`, observed as the v1.5 empty-shortcut regression).
            let store = storage::Store::open(&app.state::<AppState>().data_dir.join("spike.db"))?;
            store.migrate()?;
            let store = std::sync::Arc::new(Mutex::new(store));
            app.manage(store.clone());
            {
                let store_guard = store.lock().unwrap();
                let _ = store_guard.migrate_shortcuts_from_settings(&legacy_shortcuts);
            }

            create_windows(app)?;

            // frosted glass on floating windows (respects settings toggle)
            let acrylic_enabled = app.state::<AppState>().settings.lock().unwrap().acrylic_enabled;
            for label in ["chat", "stats", "music", "pet", "topbar"] {
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
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(event_bus::relay_task(app_handle.clone(), rx));

            emit_agent_status(&app_handle);

            // v1.5: local CLI control plane (focus-cli)
            cli::spawn(app_handle.clone(), store.clone(), data_dir_clone);
            let tx_probe = tx.clone();
            activity::spawn_probe(tx_probe, store.clone());
            supervision::spawn(app_handle.clone(), store);

            // ---- frontend -> core listeners ----
            let h = app.handle().clone();

            let h5 = h.clone();
            h5.clone().listen("ui:toggle_chat", move |_event| {
                let state = h5.state::<AppState>();
                let collapsed = state.settings.lock().unwrap().collapsed.contains(&"chat".to_string());
                if collapsed {
                    let _ = restore(h5.clone(), state.clone(), "chat".to_string());
                } else if let Some(w) = h5.get_webview_window("chat") {
                    let visible = w.is_visible().unwrap_or(true);
                    if visible {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
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
                let _ = tx.send(CoreEvent::MusicTick { position_ms, duration_ms });
            });

            // natural-release signal from the drag poller: finalize on the main
            // thread (window getters are main-thread-only)
            let hd = h.clone();
            hd.clone().listen("drag:released", move |event| {
                let v: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or_default();
                let label = v.get("label").and_then(|x| x.as_str()).unwrap_or("").to_string();
                if !label.is_empty() {
                    drag::finalize(&hd, &label);
                }
            });

            // frontend focus timer -> supervision focus tracking + session record
            let hf = h.clone();
            hf.clone().listen("focus:state_changed", move |event| {
                let v: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or_default();
                let state = v.get("state").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let paused = v.get("paused").and_then(|x| x.as_bool()).unwrap_or(false);
                let app_state = hf.state::<AppState>();
                *app_state.focus_state.lock().unwrap() = state.clone();
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
                            let dur = elapsed_sec(&started, &ended).unwrap_or_else(|| ft.session_focus_sec.max(1));
                            let tid = ft.task_id.clone();
                            let store_state = hf.state::<std::sync::Arc<Mutex<storage::Store>>>();
                            match store_state.lock() {
                                Ok(store) => {
                                    if let Err(e) = store.record_focus_session(&started, &ended, dur, tid.as_deref()) {
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
                let v: serde_json::Value = serde_json::from_str(event.payload()).unwrap_or_default();
                let id = v.get("id").and_then(|x| x.as_u64()).unwrap_or(u64::MAX);
                if let Some(tx) = hc.state::<AppState>().cli_pending.lock().unwrap().remove(&id) {
                    let _ = tx.send(v);
                }
            });

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
            set_agent_provider,
            set_agent_workspace_dir,
            pet_import_pack,
            pet_remove_pack,
            pet_list_packs,
            pet_activate,
            pet_active,
            resize_preview,
            set_pet_bg_fade,
            resize_window,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
#[cfg(test)]
mod tests {
    use super::{elapsed_sec, topbar_visible};

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
    fn free_cell_skips_forbidden_zones() {
        use super::free_cell_for;
        let (c0, r0) = free_cell_for(&[]);
        assert_eq!((c0, r0), (0, 0), "top-left is free (hero only blocks cols 3-9 rows 0-3)");
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
        assert_eq!((c1, r1), (0, 0), "occupied (0,4) does not block the top-left");
    }
}
