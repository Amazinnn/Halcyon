//! v1.2.1: Rust-side cursor-polling drag.
//!
//! The previous drag mode ("webview pointer events + per-frame IPC setPosition
//! + coordinate conversion + fullscreen overlay") was architecturally
//! unstable: trajectory oscillation, windows left off-grid, drops landing at
//! (0,0), and live position diverging from settings.json.
//!
//! This module moves the drag into Rust: `drag_start` records the grab offset
//! (GetCursorPos - window physical origin), shows the grid overlay (always
//! input-transparent), and spawns a ~15 ms poller that repositions the window
//! using raw physical coordinates (no scaling / no conversion). The poller
//! stops on left-button release (GetAsyncKeyState) or an explicit `drag_end`.
//! Final placement reuses the same code path as `place_window` (occupied
//! snap-back, out-of-bounds clamp) and is persisted to settings.json.
//!
//! Threading notes (deadlock-safe):
//! - `WebviewWindow::set_position` posts asynchronously to the main loop, so
//!   it is safe to call from the poller thread.
//! - `outer_position` / `outer_size` / `scale_factor` are synchronous
//!   window_getter! calls that dispatch to the main thread and WAIT. They must
//!   only run on the main thread. The poller never calls them (scale/size are
//!   captured in `drag_start` on the main thread); `finalize` runs only on the
//!   main thread (via the `drag:released` listener or the `drag_end` command).
//! - The main thread never `join()`s the poller (that would deadlock if the
//!   poller were blocked on a getter); it waits on a `finished` atomic instead.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::grid::GridManager;
use crate::settings::GridRect;
use crate::{dock_logos_inner, occupied_rects, place_window_inner, AppState};

const POLL_MS: u64 = 15;
const GRID_LABELS: [&str; 4] = ["chat", "stats", "music", "pet"];

/// The in-flight drag (serialized in `AppState.active_drag`). Only one drag
/// runs at a time; a repeated `drag_start` first terminates the previous one.
pub struct ActiveDrag {
    pub label: String,
    pub stop: Arc<AtomicBool>,
    pub finished: Arc<AtomicBool>,
    pub handle: Option<JoinHandle<()>>,
}

fn cursor_phys() -> Option<(i32, i32)> {
    let mut pt = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut pt).ok()? };
    Some((pt.x, pt.y))
}

fn lbutton_down() -> bool {
    let vk = VK_LBUTTON.0 as i32;
    let pressed = unsafe { GetAsyncKeyState(vk) };
    (pressed as u16 & 0x8000u16) != 0
}

/// Physical-axis clamp against the screen bounds (insurance against landing
/// off-screen / at (0,0) on release).
fn clamp_axis(v: i32, max: i32) -> i32 {
    v.clamp(0, max.max(0))
}

/// Wait (bounded) for the poller thread to finish. Never blocks forever: the
/// poller exits within a poll tick of `stop` being set.
fn wait_finished(finished: &AtomicBool, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    while !finished.load(Ordering::Relaxed) && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(4));
    }
}

fn stop_drag(ad: ActiveDrag) {
    ad.stop.store(true, Ordering::Relaxed);
    wait_finished(&ad.finished, Duration::from_millis(250));
    // handle is dropped (detached); the thread is short-lived
}

fn emit_preview(app: &AppHandle, label: &str, col: usize, row: usize) {
    let state = app.state::<AppState>();
    let (sw, sh) = *state.screen.lock().unwrap();
    let gm = GridManager { screen_w: sw, screen_h: sh };
    let settings = state.settings.lock().unwrap();
    let current = settings
        .grid
        .get(label)
        .copied()
        .unwrap_or(GridRect { col: 0, row: 0, cols: 2, rows: 2 });
    let occupied = occupied_rects(&settings, Some(label));
    let (rect, conflict) = match gm.place(label, &current, col, row, &occupied) {
        Ok(r) => (r, false),
        Err(()) => (current, true),
    };
    let _ = app.emit(
        "grid:preview",
        serde_json::json!({
            "visible": true,
            "label": label,
            "rect": rect,
            "occupiedCells": occupied,
            "conflict": conflict,
        }),
    );
}

/// Finalize a drag: hide the overlay, then snap + persist. Runs ONLY on the
/// main thread (called from the `drag:released` listener or `drag_end`),
/// because it uses window getters that dispatch to the main thread.
pub fn finalize(app: &AppHandle, label: &str) {
    if let Some(ov) = app.get_webview_window("grid-overlay") {
        let _ = ov.hide();
    }
    let _ = app.emit("grid:preview", serde_json::json!({ "visible": false }));

    let state = app.state::<AppState>();
    if label == "logos" {
        let Some(w) = app.get_webview_window("logos") else { return };
        let pos = w.outer_position().unwrap_or_default();
        let size = w.outer_size().unwrap_or_default();
        let scale = w.scale_factor().unwrap_or(1.0);
        let (sw, sh) = *state.screen.lock().unwrap();
        let cx = (pos.x + size.width as i32 / 2) as f64 / scale;
        let cy = (pos.y + size.height as i32 / 2) as f64 / scale;
        let edges = [("top", cy), ("bottom", sh - cy), ("left", cx), ("right", sw - cx)];
        let edge = edges
            .iter()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|e| e.0)
            .unwrap_or("top")
            .to_string();
        let _ = dock_logos_inner(app, &state, edge);
        return;
    }

    let Some(w) = app.get_webview_window(label) else { return };
    let pos = w.outer_position().unwrap_or_default();
    let scale = w.scale_factor().unwrap_or(1.0);
    let (sw, sh) = *state.screen.lock().unwrap();
    let gm = GridManager { screen_w: sw, screen_h: sh };
    let m = gm.metrics();
    let col = ((pos.x as f64 / scale) / m.cell_w).round() as usize;
    let row = ((pos.y as f64 / scale) / m.cell_h).round() as usize;
    let _ = place_window_inner(app, &state, label, col, row);
}

fn poller(
    app: AppHandle,
    label: String,
    off_x: i32,
    off_y: i32,
    scale: f64,
    win_w: u32,
    win_h: u32,
    stop: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
) {
    // Background thread: only async-safe calls (set_position posts to the main
    // loop; app.emit and the settings/screen Mutexes are thread-safe). No
    // window getters here.
    let (sw, sh) = *app.state::<AppState>().screen.lock().unwrap();
    let (psw, psh) = ((sw * scale) as i32, (sh * scale) as i32);
    let max_x = (psw - win_w as i32).max(0);
    let max_y = (psh - win_h as i32).max(0);
    let is_grid = GRID_LABELS.contains(&label.as_str());
    let Some(w) = app.get_webview_window(&label) else {
        finished.store(true, Ordering::Relaxed);
        return;
    };

    let mut last = (i32::MAX, i32::MAX);
    let mut moved = false;
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let Some((cx, cy)) = cursor_phys() else {
            std::thread::sleep(Duration::from_millis(POLL_MS));
            continue;
        };
        // Skip when the cursor has not moved since the last tick (keeps the
        // compositor pressure low and avoids pointless IPC).
        if moved && cx == last.0 && cy == last.1 {
            std::thread::sleep(Duration::from_millis(POLL_MS));
            continue;
        }
        last = (cx, cy);
        moved = true;
        let x = clamp_axis(cx - off_x, max_x);
        let y = clamp_axis(cy - off_y, max_y);
        let _ = w.set_position(PhysicalPosition::new(x, y));
        if is_grid {
            let m = GridManager { screen_w: sw, screen_h: sh }.metrics();
            let col = ((x as f64 / scale) / m.cell_w).round() as usize;
            let row = ((y as f64 / scale) / m.cell_h).round() as usize;
            emit_preview(&app, &label, col, row);
        }
        if !lbutton_down() {
            break; // released -> ask the main thread to finalize below
        }
        std::thread::sleep(Duration::from_millis(POLL_MS));
    }
    if !stop.load(Ordering::Relaxed) {
        // Natural release (button up) detected by the poller: the main thread
        // finalizes (getters are main-thread-only). Safe even if the frontend
        // pointerup is swallowed by the overlay.
        let _ = app.emit("drag:released", serde_json::json!({ "label": label }));
    }
    finished.store(true, Ordering::Relaxed);
}

/// Start a drag for `label`: record grab offset, show the overlay, poll the
/// cursor on a background thread.
#[tauri::command]
pub fn drag_start(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
) -> Result<(), String> {
    // Serialize: only one drag at a time; end any previous drag first.
    {
        let mut ad = state.active_drag.lock().unwrap();
        if let Some(prev) = ad.take() {
            stop_drag(prev);
        }
    }

    let Some(w) = app.get_webview_window(&label) else {
        return Err(format!("drag_start: unknown window '{label}'"));
    };
    // getters on the main thread only
    let pos = w.outer_position().map_err(|e| format!("outer_position: {e}"))?;
    let scale = w.scale_factor().unwrap_or(1.0);
    let size = w.outer_size().unwrap_or_default();
    let Some((cx, cy)) = cursor_phys() else {
        return Err("drag_start: GetCursorPos failed".into());
    };
    let off_x = cx - pos.x;
    let off_y = cy - pos.y;

    // Overlay preview: re-assert input transparency before showing so the
    // layer can never swallow the mouse during the drag.
    if let Some(ov) = app.get_webview_window("grid-overlay") {
        let _ = ov.set_ignore_cursor_events(true);
        let _ = ov.show();
    }

    // Initial preview with the window's current rect.
    if GRID_LABELS.contains(&label.as_str()) {
        let s = app.state::<AppState>();
        let settings = s.settings.lock().unwrap();
        let current = settings.grid.get(&label).copied();
        let occupied = occupied_rects(&settings, Some(&label));
        let _ = app.emit(
            "grid:preview",
            serde_json::json!({
                "visible": true,
                "label": label,
                "rect": current,
                "occupiedCells": occupied,
                "conflict": false,
            }),
        );
    }

    let stop = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let fin2 = finished.clone();
    let app2 = app.clone();
    let label2 = label.clone();
    let handle = std::thread::spawn(move || {
        poller(app2, label2, off_x, off_y, scale, size.width, size.height, stop2, fin2)
    });
    *state.active_drag.lock().unwrap() = Some(ActiveDrag {
        label,
        stop,
        finished,
        handle: Some(handle),
    });
    Ok(())
}

/// Stop the drag for `label` (frontend pointerup fallback; the poller also
/// self-detects release) and finalize placement on the main thread.
#[tauri::command]
pub fn drag_end(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
) -> Result<(), String> {
    let mut stopped = false;
    {
        let mut ad = state.active_drag.lock().unwrap();
        if let Some(prev) = ad.as_ref() {
            if prev.label == label {
                prev.stop.store(true, Ordering::Relaxed);
                stopped = true;
            }
        }
        if stopped {
            if let Some(prev) = ad.take() {
                wait_finished(&prev.finished, Duration::from_millis(250));
            }
        }
    }
    if stopped {
        finalize(&app, &label);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_negative_to_zero() {
        assert_eq!(clamp_axis(-40, 1000), 0);
    }

    #[test]
    fn clamp_overflow_to_max() {
        assert_eq!(clamp_axis(5000, 1000), 1000);
    }

    #[test]
    fn clamp_zero_max_ok() {
        assert_eq!(clamp_axis(0, 0), 0);
        assert_eq!(clamp_axis(-5, 0), 0);
    }

    #[test]
    fn clamp_keeps_inside() {
        assert_eq!(clamp_axis(42, 1000), 42);
    }
}
