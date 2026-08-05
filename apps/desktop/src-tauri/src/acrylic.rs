//! Custom Win10/11 acrylic (frosted glass) via the undocumented
//! `SetWindowCompositionAttribute` (SWCA) API.
//!
//! Why not window-vibrancy? On Windows 11 (build >= 22523) its `apply_acrylic`
//! switches to `DWMWA_SYSTEMBACKDROP_TYPE = DWMSBT_TRANSIENTWINDOW` and
//! IGNORES the tint color, so the window ends up with the system's default
//! light-gray frosted backdrop ("纯浅灰色"). Here we always use the SWCA
//! `ACCENT_ENABLE_ACRYLICBLURBEHIND` path with our own low-alpha deep-green
//! tint, so the glass stays a clear frosted blur of whatever is behind the
//! window (no gray pigment), while the content stays opaque (CSS "ink").

use std::ffi::c_void;
use windows::core::{s, w};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

const ACCENT_ENABLE_ACRYLICBLURBEHIND: i32 = 4;
const WCA_ACCENT_POLICY: u32 = 19; // 0x13

#[repr(C)]
struct AccentPolicy {
    accent_state: i32,
    accent_flags: i32,
    gradient_color: u32,
    animation_id: i32,
}

#[repr(C)]
struct WindowCompositionAttribData {
    attrib: u32,
    pv_data: *mut c_void,
    cb_data: usize,
}

type SetWindowCompositionAttributeFn =
    unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> i32;

fn resolve() -> Option<SetWindowCompositionAttributeFn> {
    unsafe {
        let user32 = LoadLibraryW(w!("user32.dll")).ok()?;
        let Some(proc) = GetProcAddress(user32, s!("SetWindowCompositionAttribute")) else {
            return None;
        };
        Some(std::mem::transmute(proc))
    }
}

/// Apply frosted-glass acrylic to `hwnd` (raw HWND pointer; kept version-
/// agnostic because tauri re-exports its own windows crate version) with
/// `tint = (r, g, b, a)`. Low alpha keeps the glass mostly see-through (just
/// blurred); set `FOCUS_NO_ACRYLIC=1` to skip (CSS fallback).
pub fn apply(hwnd: *mut c_void, tint: (u8, u8, u8, u8)) {
    let Some(f) = resolve() else { return };
    let hwnd = HWND(hwnd);
    let gradient = (tint.0 as u32)
        | ((tint.1 as u32) << 8)
        | ((tint.2 as u32) << 16)
        | ((tint.3 as u32) << 24);
    let mut policy = AccentPolicy {
        accent_state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
        accent_flags: 0,
        gradient_color: gradient,
        animation_id: 0,
    };
    let mut data = WindowCompositionAttribData {
        attrib: WCA_ACCENT_POLICY,
        pv_data: &mut policy as *mut _ as *mut c_void,
        cb_data: std::mem::size_of::<AccentPolicy>(),
    };
    unsafe {
        let _ = f(hwnd, &mut data);
    }
}
