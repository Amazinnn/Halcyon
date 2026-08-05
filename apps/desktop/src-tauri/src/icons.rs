//! v1.4 shell-icon extraction: pull the large icon of a file/.exe as a
//! 32x32 RGBA buffer for application shortcut cards (no third-party assets).

use std::ffi::c_void;

use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, DIB_USAGE, HGDIOBJ,
};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, DrawIconEx, DI_NORMAL, HICON,
};

pub const ICON_SIZE: i32 = 32;

/// Extract the shell icon as 32x32 RGBA (top-down, 4 bytes/pixel). Returns
/// None when the file has no icon or extraction fails.
pub fn extract_icon_rgba(path: &str) -> Option<Vec<u8>> {
    unsafe {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut large: HICON = HICON(std::ptr::null_mut());
        let mut small: HICON = HICON(std::ptr::null_mut());
        let count = ExtractIconExW(PCWSTR(wide.as_ptr()), 0, Some(&mut large), Some(&mut small), 1);
        if count == 0 || large.is_invalid() {
            if !small.is_invalid() {
                let _ = DestroyIcon(small);
            }
            return None;
        }

        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            let _ = DestroyIcon(large);
            if !small.is_invalid() {
                let _ = DestroyIcon(small);
            }
            return None;
        }

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: ICON_SIZE,
                biHeight: -ICON_SIZE, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0, // BI_RGB
                ..Default::default()
            },
            bmiColors: [Default::default()],
        };

        let mut bits: *mut c_void = std::ptr::null_mut();
        let Ok(bmp) = CreateDIBSection(Some(dc), &info, DIB_USAGE(0), &mut bits, None, 0) else {
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(large);
            if !small.is_invalid() {
                let _ = DestroyIcon(small);
            }
            return None;
        };

        let old = SelectObject(dc, HGDIOBJ(bmp.0));
        let _ = DrawIconEx(dc, 0, 0, large, ICON_SIZE, ICON_SIZE, 0, None, DI_NORMAL);

        let len = (ICON_SIZE * ICON_SIZE * 4) as usize;
        let bgra = std::slice::from_raw_parts(bits as *const u8, len);
        let mut rgba = vec![0u8; len];
        for i in 0..(ICON_SIZE * ICON_SIZE) as usize {
            rgba[i * 4 + 0] = bgra[i * 4 + 2];
            rgba[i * 4 + 1] = bgra[i * 4 + 1];
            rgba[i * 4 + 2] = bgra[i * 4 + 0];
            rgba[i * 4 + 3] = bgra[i * 4 + 3];
        }

        let _ = SelectObject(dc, old);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(dc);
        let _ = DestroyIcon(large);
        if !small.is_invalid() {
            let _ = DestroyIcon(small);
        }
        Some(rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_notepad_icon_smoke() {
        // notepad.exe exists on every Windows install and has a shell icon.
        let rgba = extract_icon_rgba(r"C:\Windows\System32\notepad.exe");
        if let Some(d) = rgba {
            assert_eq!(d.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
        }
    }

    #[test]
    fn missing_file_returns_none() {
        assert!(extract_icon_rgba(r"C:\does-not-exist-xyz\missing.exe").is_none());
    }
}
