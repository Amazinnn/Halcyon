//! v1.12 DEVELOPMENT-ONLY crash defenses for the desktop lock. Removed after
//! development is complete (product keeps Drop + focus-cli unlock). This
//! module ONLY calls desktop_lock's public API — deleting this file leaves
//! the core untouched.
//! Layers: 1) panic hook  2) watchdog child process (hard-crash recovery)
//!         3) escape file (%TEMP%/focus-lock-escape.tmp → unlock).

use std::process::Command;

/// Install all development-only defenses.
pub fn install_all() {
    install_panic_hook();
    spawn_watchdog(std::process::id());
    watch_escape_file();
}

/// Layer 1: panic → unlock before the process dies.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        if crate::desktop_lock::is_locked() {
            let _ = crate::desktop_lock::unlock_desktop();
        }
        eprintln!("[focus] panic: {info}");
    }));
}

/// Layer 2: watchdog child — restores the desktop if the main process dies
/// in ANY way (incl. taskkill /F). The child re-launches itself with
/// `--focus-watchdog <pid>`; lib.rs run() handles that mode.
pub fn spawn_watchdog(parent_pid: u32) {
    let Some(exe) = std::env::current_exe().ok() else { return };
    if let Err(e) = Command::new(&exe)
        .arg("--focus-watchdog")
        .arg(parent_pid.to_string())
        .spawn()
    {
        eprintln!("[lock] watchdog spawn failed: {e}");
    }
}

/// Layer 3: escape file — %TEMP%/focus-lock-escape.tmp appears → unlock.
fn watch_escape_file() {
    std::thread::spawn(|| {
        let path = std::env::temp_dir().join("focus-lock-escape.tmp");
        loop {
            if crate::desktop_lock::is_locked() && path.exists() {
                let _ = crate::desktop_lock::unlock_desktop();
                let _ = std::fs::remove_file(&path);
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });
}
