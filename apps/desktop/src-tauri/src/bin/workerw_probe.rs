//! WorkerW feasibility probe (manual, feature-flag style: run explicitly).
//!   cargo run --bin workerw_probe
//!
//! Verifies on this machine whether a window can be attached to the WorkerW
//! "wallpaper layer" (below desktop icons) so DesktopWindow could render as a
//! true desktop overlay instead of a plain fullscreen window. Investigation
//! only — not part of the app, and no final implementation is made here
//! (ADR-0003 records the recommendation).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use windows::core::{w, BOOL, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

unsafe extern "system" fn enum_top_level(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let vec = lparam.0 as *mut Vec<HWND>;
        (*vec).push(hwnd);
    }
    BOOL(1)
}

fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

fn has_child_class(parent: HWND, class_name: PCWSTR) -> bool {
    unsafe { matches!(FindWindowExW(Some(parent), None, class_name, None), Ok(h) if !h.is_invalid()) }
}

fn main() {
    let mut top: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_top_level), LPARAM(&mut top as *mut Vec<HWND> as isize));
    }
    println!("[probe] total top-level windows: {}", top.len());

    let progman = top.iter().copied().find(|&h| class_of(h) == "Progman");
    println!("[probe] Progman: {:?}", progman);

    let workerws: Vec<HWND> = top
        .iter()
        .copied()
        .filter(|&h| class_of(h) == "WorkerW")
        .collect();
    println!("[probe] WorkerW count: {}", workerws.len());
    for h in &workerws {
        println!(
            "[probe]   WorkerW {:?} has SHELLDLL_DefView: {}",
            h,
            has_child_class(*h, w!("SHELLDLL_DefView"))
        );
    }

    let wallpaper_workerw = workerws
        .iter()
        .copied()
        .find(|&h| !has_child_class(h, w!("SHELLDLL_DefView")));

    match wallpaper_workerw {
        None => {
            println!(
                "[probe] no wallpaper WorkerW found; overlay-via-WorkerW NOT available on this machine"
            );
        }
        Some(worker) => {
            println!("[probe] wallpaper WorkerW = {:?}", worker);

            let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None) }
                .map(|m| HINSTANCE(m.0))
                .unwrap_or(HINSTANCE(std::ptr::null_mut()));
            let class_name = w!("FocusWorkerWProbe");
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance,
                lpszClassName: class_name,
                ..Default::default()
            };
            unsafe {
                let _ = RegisterClassW(&wc as *const WNDCLASSW);
            }

            let test = unsafe {
                CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    class_name,
                    w!("WorkerW Probe"),
                    WS_OVERLAPPEDWINDOW,
                    0,
                    0,
                    320,
                    200,
                    None,
                    None,
                    Some(hinstance),
                    None,
                )
            };

            match test {
                Ok(hwnd) => {
                    unsafe {
                        let parent = SetParent(hwnd, Some(worker));
                        println!("[probe] SetParent -> {:?}", parent);
                        println!("[probe] GetParent after attach = {:?}", GetParent(hwnd));
                        let _ = DestroyWindow(hwnd);
                    }
                    println!("[probe] attach test OK: window attached to WorkerW, then destroyed");
                }
                Err(e) => {
                    println!("[probe] CreateWindowExW failed: {}", e);
                }
            }
        }
    }

    println!("[probe] done. (visual z-order check requires a human to look at the screen)");
}