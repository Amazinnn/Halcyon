//! v1.5 launch engine: open a shortcut's target and (for application / file /
//! folder / url) move + resize the resulting top-level window into a 12x8 grid
//! slot ("window fit"). `.exe/.bat/.cmd` are spawned directly (we know the
//! PID); `.lnk` / files / folders / urls go through ShellExecuteW plus a
//! before/after visible-window snapshot diff to find the new window. `internal`
//! targets just restore the matching Focus float window (no fit).

use std::collections::HashSet;
use std::process::Command;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use windows::core::{w, PCWSTR};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, SetWindowPos,
    SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOWNORMAL,
};

use crate::grid::{overlap, GridManager};
use crate::settings::{GridRect, ShortcutType};
use crate::storage::ShortcutRow;
use crate::AppState;

/// Time budget to find the launched window.
const FIND_TIMEOUT: Duration = Duration::from_secs(6);
/// Minimum window size (physical px) to consider a detected window "real".
const MIN_WIN_W: i32 = 80;
const MIN_WIN_H: i32 = 60;

fn own_pid() -> u32 {
    std::process::id()
}

fn window_pid(hwnd: HWND) -> Option<u32> {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    (pid != 0).then_some(pid)
}

fn window_rect(hwnd: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut r).ok()? };
    Some(r)
}

fn is_fullscreen(app: &AppHandle, hwnd: HWND) -> bool {
    let Some(r) = window_rect(hwnd) else { return false };
    let (sw, sh) = *app.state::<AppState>().screen.lock().unwrap();
    let scale = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let sw_p = sw * scale;
    let sh_p = sh * scale;
    (r.right - r.left) as f64 >= sw_p - 8.0 && (r.bottom - r.top) as f64 >= sh_p - 8.0
}

// ---- window discovery ----

unsafe extern "system" fn enum_pid_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let (pid, out) = unsafe { &mut *(lparam.0 as *mut (u32, &mut HWND)) };
    if unsafe { IsWindowVisible(hwnd) }.as_bool() && window_pid(hwnd) == Some(*pid) && *pid != own_pid() {
        **out = hwnd;
        return BOOL(0);
    }
    BOOL(1)
}

fn find_window_for_pid_once(pid: u32) -> Option<HWND> {
    let mut found = HWND::default();
    let mut ctx = (pid, &mut found);
    unsafe {
        let _ = EnumWindows(Some(enum_pid_proc), LPARAM(&mut ctx as *mut _ as isize));
    }
    (!found.is_invalid()).then_some(found)
}

fn find_window_for_pid(pid: u32, timeout: Duration) -> Option<HWND> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(h) = find_window_for_pid_once(pid) {
            return Some(h);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(120));
    }
}

unsafe extern "system" fn enum_snap_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let set = unsafe { &mut *(lparam.0 as *mut HashSet<isize>) };
    if unsafe { IsWindowVisible(hwnd) }.as_bool() && window_pid(hwnd) != Some(own_pid()) {
        set.insert(hwnd.0 as isize);
    }
    BOOL(1)
}

fn snapshot_visible_windows() -> HashSet<isize> {
    let mut set = HashSet::new();
    unsafe {
        let _ = EnumWindows(Some(enum_snap_proc), LPARAM(&mut set as *mut _ as isize));
    }
    set
}

fn detect_new_window(before: &HashSet<isize>, timeout: Duration) -> Option<HWND> {
    let deadline = Instant::now() + timeout;
    loop {
        for h in snapshot_visible_windows() {
            if before.contains(&h) {
                continue;
            }
            let hwnd = HWND(h as *mut std::ffi::c_void);
            if let Some(r) = window_rect(hwnd) {
                if r.right - r.left >= MIN_WIN_W && r.bottom - r.top >= MIN_WIN_H {
                    return Some(hwnd);
                }
            }
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

// ---- fit ----

/// Default fit slot: the right-side reserved area (cols 5-11 x rows 1-6).
pub fn reserved_slot() -> GridRect {
    GridRect { col: 5, row: 1, cols: 7, rows: 6 }
}

fn preferred_slot(row: &ShortcutRow) -> Option<GridRect> {
    match (row.fit_col, row.fit_row, row.fit_cols, row.fit_rows) {
        (Some(c), Some(r), Some(cs), Some(rs)) if cs > 0 && rs > 0 => {
            Some(GridRect { col: c as usize, row: r as usize, cols: cs as usize, rows: rs as usize })
        }
        _ => None,
    }
}

/// Nearest non-overlapping slot: prefer the remembered/reserved slot, else
/// scan row-major from it for a slot that does not overlap visible floats.
pub fn find_free_slot(app: &AppHandle, row: &ShortcutRow) -> GridRect {
    let preferred = preferred_slot(row).unwrap_or_else(reserved_slot);
    let app_state = app.state::<AppState>();
    let settings = app_state.settings.lock().unwrap();
    let occupied = crate::occupied_rects(&settings, None);
    drop(settings);
    if !occupied.iter().any(|o| overlap(&preferred, o)) {
        return preferred;
    }
    let (cols, rows) = (preferred.cols, preferred.rows);
    let max_col = crate::grid::GRID_COLS - cols;
    let max_row = crate::grid::GRID_ROWS - rows;
    let mut best: Option<GridRect> = None;
    let mut best_dist = i64::MAX;
    for r in 0..=max_row {
        for c in 0..=max_col {
            let cand = GridRect { col: c, row: r, cols, rows };
            if occupied.iter().any(|o| overlap(&cand, o)) {
                continue;
            }
            let d = ((c as i64 - preferred.col as i64).pow(2) + (r as i64 - preferred.row as i64).pow(2)) as i64;
            if d < best_dist {
                best_dist = d;
                best = Some(cand);
            }
        }
    }
    best.unwrap_or(preferred)
}

fn fit_window(app: &AppHandle, hwnd: HWND, slot: GridRect) {
    if is_fullscreen(app, hwnd) {
        return;
    }
    let (sw, sh) = *app.state::<AppState>().screen.lock().unwrap();
    let scale = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let gm = GridManager { screen_w: sw, screen_h: sh };
    let (x, y, w, h) = gm.rect_to_logical(&slot);
    let flags = SET_WINDOW_POS_FLAGS(SWP_NOZORDER.0 | SWP_NOACTIVATE.0);
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            (x * scale) as i32,
            (y * scale) as i32,
            ((w * scale) as i32).max(1),
            ((h * scale) as i32).max(1),
            flags,
        );
    }
}

// ---- open ----

fn shell_open_and_fit(app: &AppHandle, row: &ShortcutRow, file: &str) -> Result<(), String> {
    let before = snapshot_visible_windows();
    let mut file_w: Vec<u16> = file.encode_utf16().collect();
    file_w.push(0);
    let rc = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(w!("open").as_ptr()),
            PCWSTR(file_w.as_ptr()),
            PCWSTR(std::ptr::null()),
            PCWSTR(std::ptr::null()),
            SW_SHOWNORMAL,
        )
    };
    if rc.0 as isize <= 32 {
        return Err(format!("open failed (code {})", rc.0 as isize));
    }
    if let Some(h) = detect_new_window(&before, FIND_TIMEOUT) {
        fit_window(app, h, find_free_slot(app, row));
    }
    Ok(())
}

/// Launch one shortcut and (except `internal`) fit its window into the grid.
///
/// State guard: only `internal` shortcuts may touch the float windows (they
/// restore the matching view). Application/file/folder/url launches MUST NOT
/// modify `settings.collapsed` ? a regression here would "resurrect" collapsed
/// floats whenever the user opens an app (observed v1.5 regression report).
pub fn launch_shortcut(app: &AppHandle, row: &ShortcutRow) -> Result<(), String> {
    let kind = ShortcutType::parse(&row.kind).ok_or_else(|| format!("unknown shortcut type {}", row.kind))?;
    match kind {
        ShortcutType::Internal => {
            crate::restore(app.clone(), app.state::<AppState>(), row.target.clone())
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        ShortcutType::Application => {
            let p = std::path::Path::new(&row.target);
            let direct = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| matches!(e.to_ascii_lowercase().as_str(), "exe" | "bat" | "cmd"))
                .unwrap_or(false);
            if direct {
                // Note: some apps (e.g. Win11 notepad) are launcher stubs that
                // spawn the real window under a different PID, so the PID poll
                // falls back to the visible-window snapshot diff.
                let before = snapshot_visible_windows();
                let child = Command::new(&row.target)
                    .spawn()
                    .map_err(|e| format!("spawn failed: {e}"))?;
                let pid = child.id();
                let window = find_window_for_pid(pid, Duration::from_secs(2))
                    .or_else(|| detect_new_window(&before, FIND_TIMEOUT));
                if let Some(h) = window {
                    fit_window(app, h, find_free_slot(app, row));
                }
                Ok(())
            } else {
                shell_open_and_fit(app, row, &row.target)
            }
        }
        _ => shell_open_and_fit(app, row, &row.target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_slot_is_right_side() {
        let s = reserved_slot();
        assert_eq!((s.col, s.row, s.cols, s.rows), (5, 1, 7, 6));
    }

    #[test]
    fn preferred_slot_from_row() {
        let row = ShortcutRow {
            id: "a".into(),
            name: "a".into(),
            kind: "application".into(),
            target: "x".into(),
            col: 0,
            row: 0,
            fit_col: Some(2),
            fit_row: Some(3),
            fit_cols: Some(4),
            fit_rows: Some(2),
        };
        let s = preferred_slot(&row).unwrap();
        assert_eq!((s.col, s.row, s.cols, s.rows), (2, 3, 4, 2));
        let row2 = ShortcutRow { fit_col: None, ..row };
        assert!(preferred_slot(&row2).is_none());
    }
}
