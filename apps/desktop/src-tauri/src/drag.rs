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

#[cfg(not(test))]
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetWindowPos, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER};

use crate::grid::{GridManager, GRID_COLS, GRID_ROWS};
use crate::settings::GridRect;
use crate::{client_geometry_snapshot, occupied_rects, place_window_inner, AppState, ClientGeometry};

const POLL_MS: u64 = 24;
const PREVIEW_INTERVAL_MS: u64 = 50;
const GRID_LABELS: [&str; 5] = ["chat", "stats", "music", "pet", "workflow"]; // v1.10.3.1 (#47)

const DIAGNOSTICS_ENV: &str = "FOCUS_DRAG_DIAGNOSTICS";
#[cfg(not(test))]
const DIAGNOSTICS_FILE: &str = "pet-drag.jsonl";

#[derive(Clone)]
pub struct DragDiagnosticRecorder {
    inner: Arc<DragDiagnosticInner>,
}

struct DragDiagnosticInner {
    enabled: bool,
    #[cfg_attr(test, allow(dead_code))]
    data_dir: PathBuf,
    next_sequence: AtomicU64,
    latest_sequence: AtomicU64,
    post_release_click_pending: AtomicBool,
    #[cfg(test)]
    entries: Mutex<Vec<String>>,
}

impl DragDiagnosticRecorder {
    pub fn from_environment(data_dir: PathBuf) -> Self {
        let enabled = std::env::var(DIAGNOSTICS_ENV)
            .map(|value| value == "1")
            .unwrap_or(false);
        let recorder = Self {
            inner: Arc::new(DragDiagnosticInner {
                enabled,
                data_dir,
                next_sequence: AtomicU64::new(1),
                latest_sequence: AtomicU64::new(0),
                post_release_click_pending: AtomicBool::new(false),
                #[cfg(test)]
                entries: Mutex::new(Vec::new()),
            }),
        };
        recorder.record(Some(0), "pet", "rust", "diagnostics:enabled", false);
        recorder
    }

    pub fn start_sequence(&self) -> Option<u64> {
        let sequence = self.inner
            .enabled
            .then(|| self.inner.next_sequence.fetch_add(1, Ordering::Relaxed));
        if let Some(sequence) = sequence {
            self.inner.latest_sequence.store(sequence, Ordering::Relaxed);
        }
        sequence
    }

    pub fn latest_sequence(&self) -> Option<u64> {
        let sequence = self.inner.latest_sequence.load(Ordering::Relaxed);
        (sequence != 0).then_some(sequence)
    }

    pub fn arm_post_release_click(&self) {
        if self.inner.enabled {
            self.inner.post_release_click_pending.store(true, Ordering::Release);
        }
    }

    pub fn claim_post_release_click(&self) -> bool {
        self.inner.enabled
            && self
                .inner
                .post_release_click_pending
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
    }

    pub fn record(&self, sequence: Option<u64>, label: &str, source: &str, stage: &str, active_drag: bool) {
        if !self.inner.enabled {
            return;
        }
        #[cfg(test)]
        {
            let _ = (sequence, label, source, active_drag);
            self.inner.entries.lock().unwrap().push(stage.to_string());
            return;
        }

        #[cfg(not(test))]
        {
            let Some(sequence) = sequence else { return };
            let dir = self.inner.data_dir.join("diagnostics");
            let _ = std::fs::create_dir_all(&dir);
            let record = serde_json::json!({
                "sequence": sequence,
                "label": label,
                "source": source,
                "stage": stage,
                "timestampMs": chrono::Utc::now().timestamp_millis(),
                "activeDrag": active_drag,
            });
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join(DIAGNOSTICS_FILE))
            {
                let _ = writeln!(file, "{}", record);
            }
        }
    }

    #[cfg(test)]
    fn disabled_for_test() -> Self {
        Self {
            inner: Arc::new(DragDiagnosticInner {
                enabled: false,
                data_dir: std::env::temp_dir(),
                next_sequence: AtomicU64::new(1),
                latest_sequence: AtomicU64::new(0),
                post_release_click_pending: AtomicBool::new(false),
                entries: Mutex::new(Vec::new()),
            }),
        }
    }

    #[cfg(test)]
    fn enabled_for_test() -> Self {
        Self {
            inner: Arc::new(DragDiagnosticInner {
                enabled: true,
                data_dir: std::env::temp_dir(),
                next_sequence: AtomicU64::new(1),
                latest_sequence: AtomicU64::new(0),
                post_release_click_pending: AtomicBool::new(false),
                entries: Mutex::new(Vec::new()),
            }),
        }
    }

    #[cfg(test)]
    fn entries_for_test(&self) -> Vec<String> {
        self.inner.entries.lock().unwrap().clone()
    }
}

/// The in-flight drag (serialized in `AppState.active_drag`). Only one drag
/// runs at a time; a repeated `drag_start` first terminates the previous one.
pub struct ActiveDrag {
    pub label: String,
    pub sequence: Option<u64>,
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

pub struct FinishedDrag {
    pub sequence: Option<u64>,
}

/// Claim a drag exactly once. Browser pointer release and the native poller
/// can report the same release; only the claimant may finalize the window.
fn take_active_drag(active: &mut Option<ActiveDrag>, label: &str) -> Option<ActiveDrag> {
    if active.as_ref().is_some_and(|drag| drag.label == label) {
        active.take()
    } else {
        None
    }
}

/// Stop and remove the active drag before any main-thread window getters run.
/// Returns false when the same release was already consumed by the other path.
pub fn finish_drag(state: &AppState, label: &str) -> Option<FinishedDrag> {
    let active = {
        let mut current = state.active_drag.lock().unwrap();
        take_active_drag(&mut current, label)
    };
    let Some(active) = active else { return None };
    active.stop.store(true, Ordering::Relaxed);
    wait_finished(&active.finished, Duration::from_millis(250));
    Some(FinishedDrag { sequence: active.sequence })
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
pub fn finalize(app: &AppHandle, label: &str, sequence: Option<u64>) {
    let state = app.state::<AppState>();
    state.drag_diagnostics.record(sequence, label, "rust", "finalize:overlay-hide:start", false);
    if let Some(ov) = app.get_webview_window("grid-overlay") {
        let _ = ov.hide();
    }
    state.drag_diagnostics.record(sequence, label, "rust", "finalize:overlay-hide:complete", false);
    let _ = app.emit("grid:preview", serde_json::json!({ "visible": false }));

    let Some(w) = app.get_webview_window(label) else { return };
    state.drag_diagnostics.record(sequence, label, "rust", "finalize:geometry:start", false);
    let pos = w.outer_position().unwrap_or_default();
    let scale = w.scale_factor().unwrap_or(1.0);
    // v1.10.4 (#50): snap from the client origin so the content lands on the
    // cell even when the outer rect carries a non-client band.
    let geometry = client_geometry_snapshot(&w);
    let (client_x, client_y, _, _) = geometry.client_rect_for_outer(pos.x, pos.y);
    let (sw, sh) = *state.screen.lock().unwrap();
    let gm = GridManager { screen_w: sw, screen_h: sh };
    let m = gm.metrics();
    let col = ((client_x as f64 / scale) / m.cell_w).round() as usize;
    let row = ((client_y as f64 / scale) / m.cell_h).round() as usize;
    state.drag_diagnostics.record(sequence, label, "rust", "finalize:snap:start", false);
    let _ = place_window_inner(app, &state, label, col, row);
    state.drag_diagnostics.record(sequence, label, "rust", "finalize:snap:complete", false);
    if label == "pet" {
        let _ = app.emit("pet:drag-ended", ());
    }
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
    geometry: ClientGeometry,
    sequence: Option<u64>,
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
            let (client_x, client_y, client_w, client_h) = geometry.client_rect_for_outer(x, y);
            let fx = (client_x as f64 / scale) / m.cell_w;
            let fy = (client_y as f64 / scale) / m.cell_h;
            let fw = (client_w as f64 / scale) / m.cell_w;
            let fh = (client_h as f64 / scale) / m.cell_h;
            let col = fx.round() as usize;
            let row = fy.round() as usize;
            emit_preview(&app, &label, fx, fy, fw, fh, col, row);
        }
        if !lbutton_down() {
            break; // released -> ask the main thread to finalize below
        }
        std::thread::sleep(Duration::from_millis(POLL_MS));
    }
    let released_naturally = !stop.load(Ordering::Relaxed);
    finished.store(true, Ordering::Relaxed);
    app.state::<AppState>().drag_diagnostics.record(
        sequence,
        &label,
        "poller",
        if released_naturally { "poller:released" } else { "poller:stopped" },
        true,
    );
    if released_naturally {
        // Natural release (button up) detected by the poller: the main thread
        // finalizes (getters are main-thread-only). Safe even if the frontend
        // pointerup is swallowed by the overlay.
        let _ = app.emit("drag:released", serde_json::json!({ "label": label }));
    }
}

/// Start a drag for `label`: record grab offset, show the overlay, poll the
/// cursor on a background thread.
#[tauri::command]
pub fn drag_start(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
) -> Result<Option<u64>, String> {
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
        return Ok(None);
    }
    if label == "pet" {
        let _ = app.emit("pet:drag-started", ());
        if let Some(bubble) = app.get_webview_window("pet-bubble") {
            crate::hide_window_noactivate(&bubble);
        }
    }
    // getters on the main thread only
    let pos = w.outer_position().map_err(|e| format!("outer_position: {e}"))?;
    let scale = w.scale_factor().unwrap_or(1.0);
    let size = w.outer_size().unwrap_or_default();
    // v1.10.4 (#50): client-area geometry so the brightness gradient tracks
    // the visible content, not the outer rect.
    let geometry = client_geometry_snapshot(&w);
    let (client_x, client_y, cw, ch) = geometry.client_rect_for_outer(pos.x, pos.y);
    let Some((cx, cy)) = cursor_phys() else {
        return Err("drag_start: GetCursorPos failed".into());
    };
    let off_x = cx - pos.x;
    let off_y = cy - pos.y;

    // Overlay preview: re-assert input transparency before showing so the
    // layer can never swallow the mouse during the drag.
    if let Some(ov) = app.get_webview_window("grid-overlay") {
        crate::show_window_noactivate(&ov);
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
                    "x": client_x as f64 / scale / cell_w,
                    "y": client_y as f64 / scale / cell_h,
                    "w": cw as f64 / scale / cell_w,
                    "h": ch as f64 / scale / cell_h,
                },
                "occupiedCells": occupied,
                "conflict": false,
            }),
        );
    }

    let sequence = state.drag_diagnostics.start_sequence();
    state.drag_diagnostics.record(sequence, &label, "rust", "drag:start", true);
    let stop = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let fin2 = finished.clone();
    let app2 = app.clone();
    let label2 = label.clone();
    let handle = std::thread::spawn(move || {
        poller(app2, label2, off_x, off_y, scale, size.width, size.height, geometry, sequence, stop2, fin2)
    });
    *state.active_drag.lock().unwrap() = Some(ActiveDrag {
        label,
        sequence,
        stop,
        finished,
        handle: Some(handle),
    });
    Ok(sequence)
}

/// Stop the drag for `label` (frontend pointerup fallback; the poller also
/// self-detects release) and finalize placement on the main thread.
#[tauri::command]
pub fn drag_end(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
) -> Result<(), String> {
    if let Some(finished) = finish_drag(&state, &label) {
        state.drag_diagnostics.record(finished.sequence, &label, "rust", "release:claimed", false);
        state.drag_diagnostics.arm_post_release_click();
        finalize(&app, &label, finished.sequence);
    }
    Ok(())
}

/// Browser-side boundary report used only when FOCUS_DRAG_DIAGNOSTICS=1.
/// The native recorder owns the sequence because browser events can arrive
/// after an async release has already claimed and removed ActiveDrag.
#[tauri::command]
pub fn drag_diagnostic_browser_event(
    state: tauri::State<'_, AppState>,
    label: String,
    stage: String,
    sequence: Option<u64>,
) {
    let active_drag = state.active_drag.lock().unwrap().is_some();
    if stage.starts_with("browser:post-release-first-click") && !state.drag_diagnostics.claim_post_release_click() {
        return;
    }
    state.drag_diagnostics.record(
        sequence.or_else(|| state.drag_diagnostics.latest_sequence()),
        &label,
        "browser",
        &stage,
        active_drag,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_are_disabled_without_the_enablement_flag() {
        let recorder = DragDiagnosticRecorder::disabled_for_test();

        recorder.record(Some(7), "pet", "browser", "browser:pointerdown", false);

        assert!(recorder.entries_for_test().is_empty());
    }

    #[test]
    fn diagnostics_keep_drag_boundaries_in_recorded_order() {
        let recorder = DragDiagnosticRecorder::enabled_for_test();

        recorder.record(Some(12), "pet", "browser", "browser:pointerdown", true);
        recorder.record(Some(12), "pet", "rust", "release:claimed", false);
        recorder.record(Some(12), "pet", "rust", "finalize:complete", false);

        assert_eq!(
            recorder.entries_for_test(),
            vec![
                "browser:pointerdown",
                "release:claimed",
                "finalize:complete",
            ],
        );
    }

    #[test]
    fn only_the_first_release_claims_an_active_drag() {
        let stop = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(true));
        let mut active = Some(ActiveDrag {
            label: "pet".into(),
            sequence: None,
            stop,
            finished,
            handle: None,
        });

        assert!(take_active_drag(&mut active, "pet").is_some());
        assert!(take_active_drag(&mut active, "pet").is_none());
    }

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
