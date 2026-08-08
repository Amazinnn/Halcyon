//! v1.10.3 (#42): native rounded-corner clipping for the float windows.
//!
//! The WebView2 compositing surface is a rectangle; CSS `border-radius` +
//! `overflow: hidden` cannot clip the compositor's rectangular edge, so the
//! page shows a second "web frame" around the CSS rounded glass container.
//! Clipping the native HWND with a rounded region removes that edge.
//!
//! `SetWindowRgn` takes ownership of the region handle: never delete it.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};

/// CSS corner radius of the glass container (must match --r-lg in styles.css).
const CORNER_RADIUS_CSS: f64 = 16.0;

/// Re-apply a rounded clipping region to `win` using its current physical
/// size. Silent no-op on any failure (HWND/size unavailable). Call on the
/// main thread only (outer_size/scale_factor are window getters).
pub(crate) fn sync_window_region(win: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        let Ok(hwnd) = win.hwnd() else { return };
        let Ok(size) = win.outer_size() else { return };
        if size.width == 0 || size.height == 0 {
            return;
        }
        let scale = win.scale_factor().unwrap_or(1.0);
        let r = ((CORNER_RADIUS_CSS * scale).round() as i32).max(1);
        // tauri links windows 0.61 while we depend on 0.62; convert via the
        // raw pointer (both HWNDs wrap *mut c_void).
        let hwnd_win = HWND(hwnd.0 as *mut core::ffi::c_void);
        unsafe {
            // +1 px so the region fully covers the right/bottom edges.
            let region = CreateRoundRectRgn(
                0,
                0,
                size.width as i32 + 1,
                size.height as i32 + 1,
                r * 2,
                r * 2,
            );
            if !region.is_invalid() {
                // SetWindowRgn takes ownership of `region`.
                let _ = SetWindowRgn(hwnd_win, Some(region), true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_scales_with_dpi() {
        let radius = |scale: f64| ((CORNER_RADIUS_CSS * scale).round() as i32).max(1);
        assert_eq!(radius(1.0), 16);
        assert_eq!(radius(1.25), 20);
        assert_eq!(radius(1.5), 24);
        assert_eq!(radius(2.0), 32);
        assert_eq!(radius(0.0), 1); // degenerate scale clamps to 1
    }
}