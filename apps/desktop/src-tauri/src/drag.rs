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
//! - On Windows the poller moves the native HWND directly with
//!   SetWindowPos(SWP_ASYNCWINDOWPOS) (v1.10.1, #34): it never waits on the
//!   window thread and skips the WebView2 controller SetBounds synchronous
//!   COM RPC per tick (root cause of the drag freeze). set_position remains
//!   only as a fallback and posts asynchronously to the main loop, so it is
//!   safe to call from the poller thread.
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
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetWindowPos, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER};

use crate::grid::{GridManager, GRID_COLS, GRID_ROWS};
use crate::settings::GridRect;
use crate::{occupied_rects, place_window_inner, AppState};

const POLL_MS: u64 = 24;
const PREVIEW_INTERVAL_MS: u64 = 50;
const GRID_LABELS: [&str; 5] = ["chat", "stats", "music", "pet", "workflow"]; // v1.10.3.1 (#47)

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

/// v1.10.1 (#34): move the window by its native HWND so the poller never
/// triggers the WebView2 controller SetBounds synchronous COM RPC every tick
/// (root cause of the drag freeze). Returns false when the HWND is not
/// available, in which case the caller falls back to `set_position`.
pub(crate) fn move_window_raw(w: &tauri::WebviewWindow, x: i32, y: i32) -> bool {
    #[cfg(target_os = "windows")]
    {
        if let Ok(hwnd) = w.hwnd() {
            // tauri links windows 0.61 while we depend on 0.62; convert via
            // the raw pointer (both HWNDs wrap *mut c_void).
            let hwnd_win = HWND(hwnd.0 as *mut core::ffi::c_void);
            let res = unsafe {
                SetWindowPos(
                    hwnd_win,
                    None,
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
                )
            };
            return res.is_ok();
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (w, x, y);
        false
    }
}

/// v1.12.2: native resize (SetWindowPos + SWP_NOACTIVATE + SWP_ASYNCWINDOWPOS).
/// Tauri's `set_size` can activate the window and paint a caption highlight
/// while a drag/resize preview is held (the light-blue bar).
pub(crate) fn resize_window_raw(w: &tauri::WebviewWindow, width: u32, height: u32) {
    #[cfg(target_os = "windows")]
    {
        if let Ok(hwnd) = w.hwnd() {
            let hwnd_win = HWND(hwnd.0 as *mut core::ffi::c_void);
            let _ = unsafe {
                SetWindowPos(
                    hwnd_win,
                    None,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
                )
            };
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (w, width, height);
    }
}

/// v1.10.1 (#34): grid preview throttling (>=50ms) keeps the full-screen
/// gradient overlay from being rebuilt on every poll tick.
fn should_emit_preview(
    last: std::time::Instant,
    now: std::time::Instant,
    interval: Duration,
) -> bool {
    now.duration_since(last) >= interval
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

fn emit_preview(
    app: &AppHandle,
    label: &str,
    fx: f64,
    fy: f64,
    fw: f64,
    fh: f64,
    col: usize,
    row: usize,
) {
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
            // Actual floating rect of the dragged window in continuous grid
            // units. The frontend uses this as the brightness-gradient
            // center, NOT the snapped placement rect, so the glow follows
            // the real position during the drag instead of jumping between
            // cells. (fx, fy, fw, fh are precomputed by the caller: logical
            // px / logical cell size.)
            "floatRect": { "x": fx, "y": fy, "w": fw, "h": fh },
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
    let Some(w) = app.get_webview_window(label) else { return };
    crate::enforce_float_invariants(&w);
    let pos = w.outer_position().unwrap_or_default();
    let scale = w.scale_factor().unwrap_or(1.0);
    // v1.10.4 (#50): snap from the client origin so the content lands on the
    // cell even when the outer rect carries a non-client band.
    let (co_x, co_y, _, _) = crate::client_geometry(&w);
    let (sw, sh) = *state.screen.lock().unwrap();
    let gm = GridManager { screen_w: sw, screen_h: sh };
    let m = gm.metrics();
    let col = (((pos.x as f64 + co_x as f64) / scale) / m.cell_w).round() as usize;
    let row = (((pos.y as f64 + co_y as f64) / scale) / m.cell_h).round() as usize;
    let _ = place_window_inner(app, &state, label, col, row);
    // the float was just raised above the topbar; put the capsule back on top
    crate::raise_topbar(app);
}

fn poller(
    app: AppHandle,
    label: String,
    off_x: i32,
    off_y: i32,
    scale: f64,
    win_w: u32,
    win_h: u32,
    co_x: i32,
    co_y: i32,
    cw: u32,
    ch: u32,
    stop: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
) {
    // Background thread: only async-safe calls (native SetWindowPos with
    // SWP_ASYNCWINDOWPOS, or the set_position fallback which posts to the main
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
    let mut last_preview = std::time::Instant::now();
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
        let now = std::time::Instant::now();
        if !move_window_raw(&w, x, y) {
            let _ = w.set_position(PhysicalPosition::new(x, y));
        }
        if is_grid
            && should_emit_preview(last_preview, now, Duration::from_millis(PREVIEW_INTERVAL_MS))
        {
            last_preview = now;
            let m = GridManager { screen_w: sw, screen_h: sh }.metrics();
            let fx = ((x as f64 + co_x as f64) / scale) / m.cell_w;
            let fy = ((y as f64 + co_y as f64) / scale) / m.cell_h;
            let fw = (cw as f64 / scale) / m.cell_w;
            let fh = (ch as f64 / scale) / m.cell_h;
            let col = fx.round() as usize;
            let row = fy.round() as usize;
            emit_preview(&app, &label, fx, fy, fw, fh, col, row);
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
    // Never start a drag on a hidden/collapsed window: the poller would fight
    // an invisible window and could leave a zombie thread behind.
    if !w.is_visible().unwrap_or(true) {
        return Ok(());
    }
    crate::enforce_float_invariants(&w);
    // getters on the main thread only
    let pos = w.outer_position().map_err(|e| format!("outer_position: {e}"))?;
    let scale = w.scale_factor().unwrap_or(1.0);
    let size = w.outer_size().unwrap_or_default();
    // v1.10.4 (#50): client-area geometry so the brightness gradient tracks
    // the visible content, not the outer rect.
    let (co_x, co_y, cw, ch) = crate::client_geometry(&w);
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
        let (sw, sh) = *s.screen.lock().unwrap();
        let cell_w = sw / GRID_COLS as f64;
        let cell_h = sh / GRID_ROWS as f64;
        let settings = s.settings.lock().unwrap();
        let current = settings.grid.get(&label).copied();
        let occupied = occupied_rects(&settings, Some(&label));
        let _ = app.emit(
            "grid:preview",
            serde_json::json!({
                "visible": true,
                "label": label,
                "rect": current,
                "floatRect": {
                    "x": (pos.x as f64 + co_x as f64) / scale / cell_w,
                    "y": (pos.y as f64 + co_y as f64) / scale / cell_h,
                    "w": cw as f64 / scale / cell_w,
                    "h": ch as f64 / scale / cell_h,
                },
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
        poller(app2, label2, off_x, off_y, scale, size.width, size.height, co_x, co_y, cw, ch, stop2, fin2)
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

    #[test]
    fn preview_throttle_blocks_early_tick() {
        let t0 = std::time::Instant::now();
        assert!(!should_emit_preview(
            t0,
            t0 + Duration::from_millis(10),
            Duration::from_millis(50)
        ));
    }

    #[test]
    fn grid_labels_cover_all_floats() {
        for lbl in ["chat", "stats", "music", "pet", "workflow"] {
            assert!(GRID_LABELS.contains(&lbl), "missing grid preview for {lbl}");
        }
    }

    #[test]
    fn preview_throttle_passes_after_interval() {
        let t0 = std::time::Instant::now();
        assert!(should_emit_preview(
            t0,
            t0 + Duration::from_millis(50),
            Duration::from_millis(50)
        ));
    }
}
