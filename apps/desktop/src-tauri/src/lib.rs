//! Focus Desktop spike (v1.2 visual & window-management round).
//! Windows: desktop (canvas), chat / stats / music / pet (12x8 grid floats,
//! frosted acrylic, collapsible to logos), grid-overlay (drag preview),
//! logos (collapsed capsule strip). No AgentEvent protocol / event-name /
//! DB changes from the spike.

mod activity;
mod agents;
mod event_bus;
mod grid;
mod settings;
mod storage;
mod wallpaper;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Listener, Manager};
use tauri::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};

use event_bus::CoreEvent;
use grid::GridManager;
use settings::GridRect;
use settings::Settings;

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub data_dir: PathBuf,
    pub screen: Mutex<(f64, f64)>, // logical width/height
}

// ---------------------------------------------------------------------------
// window helpers
// ---------------------------------------------------------------------------

fn apply_acrylic(w: &tauri::WebviewWindow) {
    // True frosted glass (Win10 1809+ / Win11). Failure is non-fatal:
    // windows fall back to translucent fake-glass content styling.
    #[cfg(target_os = "windows")]
    {
        let tint: window_vibrancy::Color = (14, 24, 18, 150);
        if let Err(e) = window_vibrancy::apply_acrylic(w, Some(tint)) {
            eprintln!("[acrylic] {label} failed: {e}", label = w.label());
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = w;
}

fn position_window(app: &tauri::AppHandle, label: &str, rect: &GridRect, gm: &GridManager) {
    if let Some(w) = app.get_webview_window(label) {
        let (x, y, wpx, hpx) = gm.rect_to_logical(rect);
        let _ = w.set_position(LogicalPosition::new(x, y));
        let _ = w.set_size(LogicalSize::new(wpx, hpx));
    }
}

fn position_logos(app: &tauri::AppHandle, state: &AppState) {
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

fn occupied_rects(settings: &Settings, except: Option<&str>) -> Vec<GridRect> {
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
}

#[tauri::command]
fn get_bootstrap(state: tauri::State<'_, AppState>) -> Bootstrap {
    let s = state.settings.lock().unwrap();
    Bootstrap {
        grid: s.grid.clone(),
        topmost: s.topmost.clone(),
        collapsed: s.collapsed.clone(),
        wallpaper_path: s.wallpaper_path.clone(),
    }
}

#[tauri::command]
fn get_grid_metrics(state: tauri::State<'_, AppState>) -> grid::GridMetrics {
    let (w, h) = *state.screen.lock().unwrap();
    GridManager { screen_w: w, screen_h: h }.metrics()
}

#[tauri::command]
fn place_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
    col: usize,
    row: usize,
) -> Result<GridRect, String> {
    let (w, h) = *state.screen.lock().unwrap();
    let gm = GridManager { screen_w: w, screen_h: h };
    let mut settings = state.settings.lock().unwrap();
    let current = settings.grid.get(&label).copied().unwrap_or(GridRect { col: 0, row: 0, cols: 2, rows: 2 });
    let occupied = occupied_rects(&settings, Some(&label));
    match gm.place(&label, &current, col, row, &occupied) {
        Ok(new_rect) => {
            settings.grid.insert(label.clone(), new_rect);
            let _ = settings.save(&state.data_dir);
            position_window(&app, &label, &new_rect, &gm);
            Ok(new_rect)
        }
        Err(()) => {
            // occupied: snap back to the current cell
            position_window(&app, &label, &current, &gm);
            Ok(current)
        }
    }
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

#[tauri::command]
fn dock_logos(app: tauri::AppHandle, state: tauri::State<'_, AppState>, edge: String) -> Result<(), String> {
    state.settings.lock().unwrap().logos_edge = edge;
    let _ = state.settings.lock().unwrap().save(&state.data_dir);
    position_logos(&app, &state);
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

    Ok(())
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
            let state = AppState {
                settings: Mutex::new(settings),
                data_dir,
                screen: Mutex::new((sw, sh)),
            };
            app.manage(state);

            create_windows(app)?;

            // frosted glass on floating windows
            for label in ["chat", "stats", "music", "pet", "logos"] {
                if let Some(w) = app.get_webview_window(label) {
                    apply_acrylic(&w);
                }
            }

            let app_state = app.state::<AppState>();
            apply_initial_layout(app, &app_state);

            // core event bus + relay
            let (tx, rx) = tokio::sync::broadcast::channel::<CoreEvent>(256);
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(event_bus::relay_task(app_handle, rx));

            agents::mock::spawn(tx.clone());

            let store = storage::Store::open(&app_state.data_dir.join("spike.db"))?;
            store.migrate()?;
            let store = std::sync::Arc::new(Mutex::new(store));
            app.manage(store.clone());
            let tx_probe = tx.clone();
            activity::spawn_probe(tx_probe, store);

            // ---- frontend -> core listeners ----
            let h = app.handle().clone();

            let h2 = h.clone();
            h2.clone().listen("grid:drag_start", move |event| {
                let v: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or_default();
                let label = v.get("label").and_then(|x| x.as_str()).unwrap_or("").to_string();
                if let Some(overlay) = h2.get_webview_window("grid-overlay") {
                    // ensure the preview layer never intercepts mouse input
                    let _ = overlay.set_ignore_cursor_events(true);
                    let _ = overlay.show();
                }
                let state = h2.state::<AppState>();
                let settings = state.settings.lock().unwrap();
                let occupied = occupied_rects(&settings, None);
                let current = settings.grid.get(&label).copied();
                let _ = h2.emit(
                    "grid:preview",
                    serde_json::json!({
                        "visible": true,
                        "label": label,
                        "rect": current,
                        "occupiedCells": occupied,
                        "conflict": false,
                    }),
                );
            });

            let h3 = h.clone();
            h3.clone().listen("grid:drag_move", move |event| {
                let v: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or_default();
                let label = v.get("label").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let col = v.get("col").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                let row = v.get("row").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                let state = h3.state::<AppState>();
                let (wpx, hpx) = *state.screen.lock().unwrap();
                let gm = GridManager { screen_w: wpx, screen_h: hpx };
                let settings = state.settings.lock().unwrap();
                let current = settings.grid.get(&label).copied().unwrap_or(GridRect { col: 0, row: 0, cols: 2, rows: 2 });
                let occupied = occupied_rects(&settings, Some(&label));
                let (rect, conflict) = match gm.place(&label, &current, col, row, &occupied) {
                    Ok(r) => (r, false),
                    Err(()) => (current, true),
                };
                let _ = h3.emit(
                    "grid:preview",
                    serde_json::json!({
                        "visible": true,
                        "label": label,
                        "rect": rect,
                        "occupiedCells": occupied,
                        "conflict": conflict,
                    }),
                );
            });

            let h4 = h.clone();
            h4.clone().listen("grid:drag_end", move |_event| {
                if let Some(overlay) = h4.get_webview_window("grid-overlay") {
                    let _ = overlay.hide();
                }
                let _ = h4.emit("grid:preview", serde_json::json!({ "visible": false }));
            });

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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_bootstrap,
            get_grid_metrics,
            place_window,
            set_topmost,
            collapse,
            restore,
            dock_logos,
            get_wallpaper,
            persist_wallpaper,
            reset_wallpaper,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}