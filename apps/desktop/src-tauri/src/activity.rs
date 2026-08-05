//! Foreground window probe (minimal Activity Tracker stub, not the full tracker).
//! Samples the foreground window every 5s, writes one SQLite row and emits a
//! `probe.recorded` event so the spike has observable evidence.

use crate::event_bus::CoreEvent;
use crate::storage::Store;
use std::sync::{Arc, Mutex};
use windows::core::PWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

pub struct ForegroundInfo {
    pub process: String,
    pub title: String,
}

pub fn probe_foreground() -> Option<ForegroundInfo> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }

        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len.max(0) as usize]);

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return Some(ForegroundInfo {
                process: "(no pid)".to_string(),
                title,
            });
        }

        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return Some(ForegroundInfo {
                process: "(open process failed)".to_string(),
                title,
            });
        };

        let mut name_buf = [0u16; 1024];
        let mut name_len = name_buf.len() as u32;
        let process = if QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(name_buf.as_mut_ptr()),
            &mut name_len,
        )
        .is_ok()
        {
            let full = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            std::path::Path::new(&full)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or(full)
        } else {
            "(query image name failed)".to_string()
        };

        let _ = CloseHandle(handle);
        Some(ForegroundInfo { process, title })
    }
}

pub fn spawn_probe(
    tx: tokio::sync::broadcast::Sender<CoreEvent>,
    store: Arc<Mutex<Store>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        interval.tick().await; // first tick fires immediately; consume it
        loop {
            interval.tick().await;
            if let Some(info) = probe_foreground() {
                if let Ok(store) = store.lock() {
                    let _ = store.insert_probe(
                        "foreground",
                        &format!("{} | {}", info.process, info.title),
                    );
                }
                let _ = tx.send(CoreEvent::ProbeRecorded {
                    process: info.process,
                    title: info.title,
                });
            }
        }
    });
}