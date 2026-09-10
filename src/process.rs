use std::path::Path;
use tracing::{info, warn};
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindow, GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, GWL_EXSTYLE, GW_OWNER, WS_EX_TOOLWINDOW,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunningProcess {
    pub pid: u32,
    pub name: String,
    pub title: String,
    pub exe_path: Option<String>,
}

const SYSTEM_PROCESS_BLACKLIST: &[&str] = &[
    "system",
    "registry",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "services.exe",
    "lsass.exe",
    "winlogon.exe",
    "fontdrvhost.exe",
    "dwm.exe",
    "svchost.exe",
    "spoolsv.exe",
    "sihost.exe",
    "taskhostw.exe",
    "explorer.exe",
    "searchhost.exe",
    "shellexperiencehost.exe",
    "startmenuexperiencehost.exe",
    "textinputhost.exe",
    "ctfmon.exe",
    "applicationframehost.exe",
    "conhost.exe",
    "runtimebroker.exe",
    "systemsettings.exe",
    "lockapp.exe",
    "smartscreen.exe",
    "winstore.app.exe",
    "securityhealthsystray.exe",
    "securityhealthservice.exe",
    "searchindexer.exe",
    "compattelrunner.exe",
];

pub fn is_blacklisted_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    let file_name = Path::new(&lower)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or(lower);
    
    SYSTEM_PROCESS_BLACKLIST.iter().any(|&b| b == file_name)
}

struct EnumState {
    current_pid: u32,
    processes: Vec<RunningProcess>,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = &mut *(lparam.0 as *mut EnumState);

    // 1. Must be visible
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    // 2. Must not be a tool window (skip taskbar helpers, notification icons, floating toolbars)
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    if (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
        return BOOL(1);
    }

    // 3. Must not have an owner window (top-level application windows only)
    if let Ok(owner) = GetWindow(hwnd, GW_OWNER) {
        if owner.0 as usize != 0 {
            return BOOL(1);
        }
    }

    // 4. Must have a non-zero rectangular size
    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect).is_ok() {
        if (rect.right - rect.left) <= 0 || (rect.bottom - rect.top) <= 0 {
            return BOOL(1);
        }
    }

    // 5. Must have a non-empty window title
    let text_len = GetWindowTextLengthW(hwnd);
    if text_len == 0 {
        return BOOL(1);
    }

    let mut title_buf = vec![0u16; (text_len + 1) as usize];
    let copied = GetWindowTextW(hwnd, &mut title_buf);
    if copied == 0 {
        return BOOL(1);
    }
    let title = String::from_utf16_lossy(&title_buf[..copied as usize]).trim().to_string();
    if title.is_empty()
        || title == "Kelp"
        || title == "Program Manager"
        || title == "Default IME"
        || title == "MSCTFIME UI"
        || title == "Settings"
        || title == "Windows Input Experience"
    {
        return BOOL(1);
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 || pid == state.current_pid {
        return BOOL(1);
    }

    // If we already have this PID, don't duplicate
    if state.processes.iter().any(|p| p.pid == pid) {
        return BOOL(1);
    }

    let (proc_name, exe_path) = get_process_info(pid);
    if is_blacklisted_process(&proc_name) {
        return BOOL(1);
    }

    state.processes.push(RunningProcess {
        pid,
        name: proc_name,
        title,
        exe_path,
    });

    BOOL(1)
}

pub fn get_process_info(pid: u32) -> (String, Option<String>) {
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 1024];
            let mut size = buf.len() as u32;
            if windows::Win32::System::Threading::QueryFullProcessImageNameW(
                handle,
                windows::Win32::System::Threading::PROCESS_NAME_FORMAT(0),
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut size,
            )
            .is_ok()
            {
                let _ = CloseHandle(handle);
                let full_path = String::from_utf16_lossy(&buf[..size as usize]);
                let file_name = Path::new(&full_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("PID: {}", pid));
                return (file_name, Some(full_path));
            }
            let _ = CloseHandle(handle);
        }
    }
    (format!("Process {}", pid), None)
}

pub fn get_process_name(pid: u32) -> String {
    get_process_info(pid).0
}

/// Lists running user applications with visible top-level windows.
pub fn get_running_user_applications() -> Vec<RunningProcess> {
    let current_pid = unsafe { GetCurrentProcessId() };
    let mut state = EnumState {
        current_pid,
        processes: Vec::new(),
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut state as *mut _ as isize),
        );
    }

    // Sort by name case-insensitive
    state.processes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    state.processes
}

/// Safely terminates a process by PID with boundary and privilege checks.
pub fn terminate_process(pid: u32) -> Result<(), String> {
    let current_pid = unsafe { GetCurrentProcessId() };
    if pid == current_pid {
        return Err("Cannot terminate Kelp process.".to_string());
    }

    let proc_name = get_process_name(pid);
    if is_blacklisted_process(&proc_name) {
        return Err(format!("Cannot terminate protected system process '{}'.", proc_name));
    }

    unsafe {
        let handle = match OpenProcess(PROCESS_TERMINATE, false, pid) {
            Ok(h) => h,
            Err(e) => {
                warn!("OpenProcess failed for PID {}: {:?}", pid, e);
                return Err(format!("Access denied or process not found (PID: {})", pid));
            }
        };

        let result = TerminateProcess(handle, 1);
        let _ = CloseHandle(handle);

        if result.is_ok() {
            info!("Successfully terminated process '{}' (PID {})", proc_name, pid);
            Ok(())
        } else {
            Err(format!("Failed to terminate process (PID: {})", pid))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_process_blacklist() {
        assert!(is_blacklisted_process("csrss.exe"));
        assert!(is_blacklisted_process("explorer.exe"));
        assert!(is_blacklisted_process("dwm.exe"));
        assert!(is_blacklisted_process("C:\\Windows\\System32\\svchost.exe"));
        assert!(!is_blacklisted_process("spotify.exe"));
        assert!(!is_blacklisted_process("code.exe"));
        assert!(!is_blacklisted_process("chrome.exe"));
    }

    #[test]
    fn test_running_applications_enumeration() {
        let apps = get_running_user_applications();
        // Enumeration should not crash and should never contain blacklisted processes
        for app in &apps {
            assert!(!is_blacklisted_process(&app.name));
        }
    }
}
