//! Running-apps enumeration (v1.4.1): lists the basenames of processes that
//! own visible top-level windows, so the settings popover can offer a picker
//! for the distraction blacklist / allowlist. Reuses the same process-name
//! helper pattern as `activity::probe_foreground`; no new dependencies.

use std::collections::BTreeSet;
use std::os::windows::process::CommandExt;

use windows::core::PWSTR;
use windows::core::BOOL;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
};

pub const MAX_APPS: usize = 100;

/// Sort (case-insensitive), dedup (case-insensitive), drop blanks, cap.
/// Pure + unit-testable; `list_running_apps` feeds this.
pub fn normalize_app_list(names: Vec<String>) -> Vec<String> {
    let mut names = names;
    names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    let mut out: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for n in names {
        let key = n.to_lowercase();
        if n.trim().is_empty() || seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        out.push(n);
        if out.len() >= MAX_APPS {
            break;
        }
    }
    out
}

fn process_basename(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(handle);
        if !ok {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        Some(
            std::path::Path::new(&full)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or(full),
        )
    }
}

unsafe extern "system" fn collect_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let names = unsafe { &mut *(lparam.0 as *mut BTreeSet<String>) };
    if unsafe { IsWindowVisible(hwnd) }.as_bool() {
        let mut pid: u32 = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        if pid != 0 && pid != std::process::id() {
            if let Some(name) = process_basename(pid) {
                names.insert(name);
            }
        }
    }
    BOOL(1)
}

/// All visible windowed apps (exe basenames), sorted + deduped, capped.
pub fn list_running_apps() -> Vec<String> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    unsafe {
        let _ = EnumWindows(Some(collect_window_proc), LPARAM(&mut names as *mut _ as isize));
    }
    normalize_app_list(names.into_iter().collect())
}

/// Installed desktop programs plus currently visible applications. The
/// registry normally records DisplayIcon, whose executable basename is the
/// stable thing the supervision engine compares against.
pub fn list_apps_catalog() -> Vec<String> {
    let script = r#"$roots=@('HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*','HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'); Get-ItemProperty $roots -ErrorAction SilentlyContinue | ForEach-Object { if ($_.DisplayIcon) { [IO.Path]::GetFileName(($_.DisplayIcon -replace '^"|"$','').Split(',')[0]) } } | Where-Object { $_ -and $_.ToLower().EndsWith('.exe') } | Sort-Object -Unique"#;
    let installed = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000)
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).lines().map(str::trim)
            .filter(|line| !line.is_empty()).map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    let running = list_running_apps();
    let mut all = running.clone();
    all.extend(installed);
    let normalized = normalize_app_list(all);
    let mut ordered = running;
    ordered.retain(|name| normalized.iter().any(|item| item.eq_ignore_ascii_case(name)));
    for name in normalized {
        if !ordered.iter().any(|item| item.eq_ignore_ascii_case(&name)) { ordered.push(name); }
    }
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_sorts_dedups_caps() {
        let out = normalize_app_list(vec![
            "chrome.exe".into(),
            "code.exe".into(),
            "CHROME.EXE".into(),
            "".into(),
            "  ".into(),
        ]);
        assert_eq!(out, vec!["chrome.exe".to_string(), "code.exe".to_string()]);
    }

    #[test]
    fn normalize_caps_at_limit() {
        let many: Vec<String> = (0..250).map(|i| format!("app{}.exe", i)).collect();
        let out = normalize_app_list(many);
        assert_eq!(out.len(), MAX_APPS);
    }

    #[test]
    fn list_running_apps_smoke() {
        let apps = list_running_apps();
        assert!(!apps.is_empty(), "system should have visible windows");
        assert!(apps.len() <= MAX_APPS);
        assert!(apps.iter().all(|a| !a.trim().is_empty()));
        let mut sorted = apps.clone();
        sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        assert_eq!(apps, sorted);
    }
}
