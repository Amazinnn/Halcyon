//! Focus Desktop spike (v1.2 visual & window-management round).
//! Windows: desktop (canvas), chat / stats / music / pet (12x8 grid floats,
//! frosted acrylic, collapsible to logos), grid-overlay (drag preview),
//! logos (collapsed capsule strip). No AgentEvent protocol / event-name /
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
mod settings;
mod shortcuts;
mod storage;
mod supervision;
mod wallpaper;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Listener, Manager};
use tauri::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};

use event_bus::CoreEvent;
use grid::GridManager;
use settings::{GridRect, Settings, ShortcutType, Task};

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub data_dir: PathBuf,
    pub screen: Mutex<(f64, f64)>, // logical width/height
    pub active_drag: Mutex<Option<drag::ActiveDrag>>,
    pub focus_track: Mutex<supervision::FocusTrack>,
    pub focus_state: Mutex<String>,
    pub cli_pending: Mutex<HashMap<u64, std::sync::mpsc::Sender<serde_json::Value>>>,
    pub cli_next_id: AtomicU64,
    pub cli_token: Mutex<String>,
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

pub(crate) fn position_logos(app: &tauri::AppHandle, state: &AppState) {
    let Some(w) = app.get_webview_window("logos") else { return };
    let scale = w.scale_factor().unwrap_or(1.0);
    let pos = w.outer_position().unwrap_or(PhysicalPosition::new(0, 0));
    let size = w.outer_size().unwrap_or(PhysicalSize::new(200, 110));
    let (lx, ly) = (pos.x as f64 / scale, pos.y as f64 / scale);
    let (lw, lh) = (size.width as f64 / scale, size.height as f64 / scale);
    let (sw, sh) = *state.screen.lock().unwrap();
    let edge = state.settings.lock().unwrap().logos_edge.clone();
    let (x, y) = match edge.as_str() {
        "top" => (lx.clamp(8.0, (sw - lw - 8.0).max(8.0)), 12.0),
        "bottom" => (lx.clamp(8.0, (sw - lw - 8.0).max(8.0)), (sh - lh - 12.0).max(8.0)),
        "left" => (12.0, ly.clamp(8.0, (sh - lh - 8.0).max(8.0))),
        _ => ((sw - lw - 12.0).max(8.0), ly.clamp(8.0, (sh - lh - 8.0).max(8.0))),
    };
    let _ = w.set_position(LogicalPosition::new(x, y));
}

fn emit_visibility(app: &tauri::AppHandle, label: &str, visible: bool) {
    let _ = app.emit("window:visibility", serde_json::json!({ "label": label, "visible": visible }));
}

fn update_logos(app: &tauri::AppHandle, state: &AppState) {
    let collapsed = state.settings.lock().unwrap().collapsed.clone();
    let _ = app.emit("logos:update", serde_json::json!({ "collapsed": collapsed }));
    if let Some(w) = app.get_webview_window("logos") {
        if collapsed.is_empty() {
            let _ = w.hide();
        } else {
            position_logos(app, state);
            let _ = w.show();
        }
    }
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
    }
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
            Ok(new_rect)
        }
        Err(()) => {
            // occupied: snap back to the current cell
            position_window(app, label, &current, &gm);
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
    update_logos(&app, &state);
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
    update_logos(&app, &state);
    Ok(())
}

pub(crate) fn dock_logos_inner(app: &tauri::AppHandle, state: &AppState, edge: String) -> Result<(), String> {
    state.settings.lock().unwrap().logos_edge = edge;
    let _ = state.settings.lock().unwrap().save(&state.data_dir);
    position_logos(app, state);
    Ok(())
}

#[tauri::command]
fn dock_logos(app: tauri::AppHandle, state: tauri::State<'_, AppState>, edge: String) -> Result<(), String> {
    dock_logos_inner(&app, &state, edge)
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
fn launch_shortcut(
    app: tauri::AppHandle,
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    id: String,
) -> Result<(), String> {
    let st = store.lock().map_err(|e| e.to_string())?;
    let rows = st.list_shortcuts().map_err(|e| e.to_string())?;
    let row = rows.iter().find(|r| r.id == id).cloned().ok_or("shortcut not found")?;
    drop(st);
    crate::launch::launch_shortcut(&app, &row)
}

#[tauri::command]
fn set_acrylic(app: tauri::AppHandle, state: tauri::State<'_, AppState>, enabled: bool) -> Result<(), String> {
    {
        let mut s = state.settings.lock().unwrap();
        s.acrylic_enabled = enabled;
        let _ = s.save(&state.data_dir);
    }
    for label in ["chat", "stats", "music", "pet", "logos"] {
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

#[tauri::command]
fn record_focus_session(
    store: tauri::State<'_, std::sync::Arc<Mutex<storage::Store>>>,
    started_at: String,
    ended_at: String,
    duration_sec: i64,
    task_id: Option<String>,
) -> Result<(), String> {
    let _ = store
        .lock()
        .unwrap()
        .record_focus_session(&started_at, &ended_at, duration_sec, task_id.as_deref())
        .map_err(|e| e.to_string())?;
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
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "stats", url.clone())
        .title("统计")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "music", url.clone())
        .title("音乐")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()?;

    tauri::WebviewWindowBuilder::new(app, "pet", url.clone())
        .title("桌宠")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
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

    tauri::WebviewWindowBuilder::new(app, "logos", url.clone())
        .title("Logos")
        .inner_size(200.0, 112.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()?;

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
        } else {
            let _ = w.hide();
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
    update_logos(&app.handle(), state);
}

// ---------------------------------------------------------------------------
// entry
// ---------------------------------------------------------------------------

pub fn run() {
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
            let legacy_shortcuts = settings.shortcuts.clone();
            let data_dir_clone = data_dir.clone();
            let state = AppState {
                settings: Mutex::new(settings),
                data_dir,
                screen: Mutex::new((sw, sh)),
                active_drag: Mutex::new(None),
                focus_track: Mutex::new(supervision::FocusTrack::default()),
                focus_state: Mutex::new("idle".to_string()),
                cli_pending: Mutex::new(HashMap::new()),
                cli_next_id: AtomicU64::new(0),
                cli_token: Mutex::new(String::new()),
            };
            app.manage(state);

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
            for label in ["chat", "stats", "music", "pet", "logos", "topbar"] {
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
                });
            }

            // core event bus + relay
            let (tx, rx) = tokio::sync::broadcast::channel::<CoreEvent>(256);
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(event_bus::relay_task(app_handle.clone(), rx));

            agents::mock::spawn(tx.clone());

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
                let completed = v.get("completed").and_then(|x| x.as_bool()).unwrap_or(false);
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
                        if ft.active && completed {
                            let started = ft.session_started_at.clone().unwrap_or_default();
                            let ended = chrono::Local::now().to_rfc3339();
                            let dur = ft.session_focus_sec.max(1);
                            let tid = ft.task_id.clone();
                            let store_state = hf.state::<std::sync::Arc<Mutex<storage::Store>>>();
                            let _ = store_state
                                .lock()
                                .unwrap()
                                .record_focus_session(&started, &ended, dur, tid.as_deref());
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
            dock_logos,
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
            get_shortcut_icon,
            get_wallpaper,
            persist_wallpaper,
            reset_wallpaper,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
#[cfg(test)]
mod tests {
    use super::topbar_visible;

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
