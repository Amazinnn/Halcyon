//! v1.12: focus-mode desktop lock (backend only; UI trigger comes with the
//! future focus modes). Lock = hide taskbar (Shell_TrayWnd) + hide desktop
//! icons (Progman) + block Win/Alt+Tab/Alt+F4 via a low-level keyboard hook.
//! Escape: Drop impl restores on normal exit; development-only defenses
//! (panic hook / watchdog / escape file) live in desktop_lock_escapes.rs
//! and are removed after development (they only call our public API).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, FindWindowW, ShowWindow, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, SetWindowsHookExW,
    SW_HIDE, SW_SHOW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN, HHOOK,
};

/// HWND is a nullable pointer wrapper; check non-null via `.is_invalid()`.
fn hwnd_is_null(h: HWND) -> bool {
    h.is_invalid()
}

static LOCKED: AtomicBool = AtomicBool::new(false);
// HHOOK is !Send; wrap in a raw usize (handle value) for the static.
static HOOK: Mutex<Option<isize>> = Mutex::new(None);
static TRANSITION: Mutex<()> = Mutex::new(());

pub fn is_locked() -> bool {
    LOCKED.load(Ordering::Relaxed)
}

/// Drop: restore on normal exit.
pub struct DesktopLock;
impl Drop for DesktopLock {
    fn drop(&mut self) {
        if LOCKED.load(Ordering::Relaxed) {
            let _ = unlock_desktop();
        }
    }
}

/// Low-level keyboard hook: block Win key / Alt+Tab / Alt+F4 / Ctrl+Esc.
unsafe extern "system" fn kb_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kbd = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        // Win key (single press)
        if (kbd.vkCode == 0x5B || kbd.vkCode == 0x5C)
            && (wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize)
        {
            return LRESULT(1);
        }
        // Alt+Tab / Alt+F4 / Ctrl+Esc
        let alt = (kbd.flags.0 & LLKHF_ALTDOWN.0) != 0;
        let ctrl = unsafe {
            windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x11) < 0
        };
        if (kbd.vkCode == 0x09 && alt) // Alt+Tab
            || (kbd.vkCode == 0x73 && alt) // Alt+F4
            || (kbd.vkCode == 0x1B && ctrl) // Ctrl+Esc
        {
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(Some(HHOOK::default()), code, wparam, lparam) }
}

fn lock_desktop_inner() -> Result<(), String> {
    if LOCKED.load(Ordering::Relaxed) {
        return Ok(());
    }
    let tray = unsafe { FindWindowW(windows::core::w!("Shell_TrayWnd"), None).unwrap_or_default() };
    if hwnd_is_null(tray) {
        return Err("找不到任务栏窗口".into());
    }
    let progman = unsafe { FindWindowW(windows::core::w!("Progman"), None).unwrap_or_default() };
    if hwnd_is_null(progman) {
        return Err("找不到桌面窗口".into());
    }
    unsafe {
        let _ = ShowWindow(tray, SW_HIDE);
        let _ = ShowWindow(progman, SW_HIDE);
    }
    let hook = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(kb_hook as unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT),
            None,
            0,
        )
    };
    let Ok(hook) = hook else {
        // Fail-no-lock: restore windows, do NOT enter locked state.
        unsafe {
            let _ = ShowWindow(tray, SW_SHOW);
            let _ = ShowWindow(progman, SW_SHOW);
        }
        return Err("键盘钩子安装失败".into());
    };
    if hook.is_invalid() {
        unsafe {
            let _ = ShowWindow(tray, SW_SHOW);
            let _ = ShowWindow(progman, SW_SHOW);
        }
        return Err("键盘钩子安装失败".into());
    }
    *HOOK.lock().unwrap() = Some(hook.0 as isize);
    LOCKED.store(true, Ordering::Relaxed);
    Ok(())
}

/// Restore the Windows shell windows without relying on this process's lock
/// state. A crash watchdog runs in a separate process, where `LOCKED` always
/// starts false even if its parent hid the taskbar and desktop.
pub fn restore_desktop_after_process_exit() -> Result<(), String> {
    let tray = unsafe { FindWindowW(windows::core::w!("Shell_TrayWnd"), None).unwrap_or_default() };
    let progman = unsafe { FindWindowW(windows::core::w!("Progman"), None).unwrap_or_default() };
    unsafe {
        if !hwnd_is_null(tray) {
            let _ = ShowWindow(tray, SW_SHOW);
        }
        if !hwnd_is_null(progman) {
            let _ = ShowWindow(progman, SW_SHOW);
        }
    }
    Ok(())
}

fn unlock_desktop_inner() -> Result<(), String> {
    // Keep shell restoration idempotent so an explicit unlock can also repair
    // a stale hidden desktop after an earlier process has exited.
    restore_desktop_after_process_exit()?;
    if !LOCKED.load(Ordering::Relaxed) {
        return Ok(());
    }
    if let Some(h) = HOOK.lock().unwrap().take() {
        unsafe {
            let _ = UnhookWindowsHookEx(HHOOK(h as *mut _));
        }
    }
    LOCKED.store(false, Ordering::Relaxed);
    Ok(())
}

/// The only entry point that changes shell visibility or the keyboard hook.
/// Tauri commands can arrive from multiple WebViews, so this mutex preserves
/// the user's request order across the whole hide/show operation.
pub fn set_desktop_locked(locked: bool) -> Result<(), String> {
    let _transition = TRANSITION.lock().unwrap_or_else(|e| e.into_inner());
    if locked { lock_desktop_inner() } else { unlock_desktop_inner() }
}

pub fn lock_desktop() -> Result<(), String> {
    set_desktop_locked(true)
}

pub fn unlock_desktop() -> Result<(), String> {
    set_desktop_locked(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_unlock_state_machine() {
        // Don't actually lock in tests (no desktop) — verify state bits.
        assert!(!is_locked());
        LOCKED.store(true, Ordering::Relaxed);
        assert!(is_locked());
        LOCKED.store(false, Ordering::Relaxed);
        assert!(!is_locked());
    }

    #[test]
    fn crash_recovery_restores_desktop_without_local_lock_state() {
        // A watchdog is a separate process, so its LOCKED static always starts
        // false. Recovery must still restore the shell windows it inherits.
        LOCKED.store(false, Ordering::Relaxed);
        restore_desktop_after_process_exit().unwrap();
        assert!(!is_locked());
    }

    #[test]
    fn setting_unlocked_is_idempotent_without_local_lock_state() {
        LOCKED.store(false, Ordering::Relaxed);
        set_desktop_locked(false).unwrap();
        assert!(!is_locked());
    }
}
