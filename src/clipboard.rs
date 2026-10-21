use crate::models::{FileMetadata, FileType, SearchResult};
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_CLIPBOARD_HISTORY: usize = 25;
const MAX_ENTRY_CHARS: usize = 8192;

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub text: String,
    pub timestamp: i64,
}

static CLIPBOARD_HISTORY: OnceLock<Mutex<VecDeque<ClipboardEntry>>> = OnceLock::new();

fn history_store() -> &'static Mutex<VecDeque<ClipboardEntry>> {
    CLIPBOARD_HISTORY.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_CLIPBOARD_HISTORY)))
}

/// Records a text snippet into the in-memory clipboard history ring buffer (deduplicated, newest first).
pub fn record_clipboard_entry(text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    let stored_text: String = if text.chars().count() > MAX_ENTRY_CHARS {
        text.chars().take(MAX_ENTRY_CHARS).collect()
    } else {
        text.to_string()
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut guard = match history_store().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    // Remove existing duplicate if present
    guard.retain(|item| item.text != stored_text);

    // Push to front (most recent first)
    guard.push_front(ClipboardEntry {
        text: stored_text,
        timestamp: now,
    });

    while guard.len() > MAX_CLIPBOARD_HISTORY {
        guard.pop_back();
    }
}

/// Clears the in-memory clipboard history and system clipboard.
pub fn clear_clipboard_history() {
    let mut guard = match history_store().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.clear();
}

/// Synchronizes the current Windows system clipboard text into the in-memory history.
pub fn sync_system_clipboard() {
    if let Some(current) = get_clipboard_text() {
        if !current.trim().is_empty() {
            record_clipboard_entry(&current);
        }
    }
}

/// Returns a snapshot of the clipboard history (newest first).
pub fn get_history_snapshot() -> Vec<ClipboardEntry> {
    let guard = match history_store().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.iter().cloned().collect()
}

/// Searches clipboard history when query starts with `clip`, `clipboard`, or `cb`.
pub fn search_clipboard_query(raw_query: &str) -> Option<Vec<SearchResult>> {
    let trimmed = raw_query.trim();
    let lower = trimmed.to_lowercase();

    let filter_term = if matches!(
        lower.as_str(),
        "clip" | "clipboard" | "clipboard history" | "paste" | "cb"
    ) {
        ""
    } else if let Some(rest) = lower.strip_prefix("clip ") {
        rest.trim()
    } else if let Some(rest) = lower.strip_prefix("clipboard ") {
        rest.trim()
    } else {
        lower.strip_prefix("cb ")?.trim()
    };

    // Sync current system clipboard first so the latest copied item is always visible in production
    #[cfg(not(test))]
    sync_system_clipboard();

    let snapshot = get_history_snapshot();
    if snapshot.is_empty() {
        return Some(vec![SearchResult {
            metadata: FileMetadata {
                id: None,
                name: "Clipboard History is Empty".to_string(),
                extension: "Clipboard".to_string(),
                parent_folder: "Copy any text in Windows and type 'clip' to search recent items".to_string(),
                full_path: "clipboard_empty_hint".to_string(),
                modified_date: 0,
                size: 0,
                file_type: FileType::Shortcut,
            },
            score: 1.0,
            match_type: "ClipboardHint".to_string(),
            icon_base64: None,
        }]);
    }

    let mut results = Vec::new();
    for (idx, item) in snapshot.iter().enumerate() {
        if !filter_term.is_empty() && !item.text.to_lowercase().contains(filter_term) {
            continue;
        }

        // Collapse newlines and extra spaces for single-line preview title
        let single_line: String = item
            .text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" ↵ ");
        let display_name = if single_line.chars().count() > 80 {
            format!("{}…", single_line.chars().take(80).collect::<String>())
        } else if single_line.is_empty() {
            item.text.clone()
        } else {
            single_line
        };

        let char_count = item.text.chars().count();
        let line_count = item.text.lines().count().max(1);
        let subtitle = if idx == 0 {
            format!("Latest Clipboard • {} chars, {} line(s) — Press Enter to copy", char_count, line_count)
        } else {
            format!("Clipboard #{} • {} chars, {} line(s) — Press Enter to copy", idx + 1, char_count, line_count)
        };

        results.push(SearchResult {
            metadata: FileMetadata {
                id: None,
                name: display_name,
                extension: "Clipboard".to_string(),
                parent_folder: subtitle,
                full_path: format!("copy:{}", item.text),
                modified_date: item.timestamp,
                size: item.text.len() as i64,
                file_type: FileType::Shortcut,
            },
            score: 1.8 - (idx as f64 * 0.02),
            match_type: "ClipboardItem".to_string(),
            icon_base64: None,
        });
    }

    if results.is_empty() && !filter_term.is_empty() {
        results.push(SearchResult {
            metadata: FileMetadata {
                id: None,
                name: format!("No clipboard items matching '{}'", filter_term),
                extension: "Clipboard".to_string(),
                parent_folder: "Try a different keyword or type 'clip' to view all recent items".to_string(),
                full_path: "clipboard_empty_hint".to_string(),
                modified_date: 0,
                size: 0,
                file_type: FileType::Shortcut,
            },
            score: 1.0,
            match_type: "ClipboardHint".to_string(),
            icon_base64: None,
        });
    }

    Some(results)
}

#[cfg(target_os = "windows")]
mod win_clip {
    use std::ffi::c_void;
    use std::ptr::null_mut;

    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;

    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(hWndNewOwner: *mut c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(uFormat: u32, hMem: *mut c_void) -> *mut c_void;
        fn GetClipboardData(uFormat: u32) -> *mut c_void;
        fn IsClipboardFormatAvailable(format: u32) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(uFlags: u32, dwBytes: usize) -> *mut c_void;
        fn GlobalLock(hMem: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(hMem: *mut c_void) -> i32;
        fn GlobalFree(hMem: *mut c_void) -> *mut c_void;
    }

    pub fn set_clipboard_text_native(text: &str) -> Result<(), String> {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let byte_len = wide.len() * std::mem::size_of::<u16>();

        unsafe {
            let mut opened = false;
            for _ in 0..4 {
                if OpenClipboard(null_mut()) != 0 {
                    opened = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            if !opened {
                return Err("Failed to open Windows clipboard".to_string());
            }

            let _ = EmptyClipboard();
            let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len);
            if hmem.is_null() {
                let _ = CloseClipboard();
                return Err("GlobalAlloc failed for clipboard data".to_string());
            }

            let locked = GlobalLock(hmem) as *mut u16;
            if locked.is_null() {
                let _ = GlobalFree(hmem);
                let _ = CloseClipboard();
                return Err("GlobalLock failed for clipboard data".to_string());
            }

            std::ptr::copy_nonoverlapping(wide.as_ptr(), locked, wide.len());
            let _ = GlobalUnlock(hmem);

            let res = SetClipboardData(CF_UNICODETEXT, hmem);
            if res.is_null() {
                let _ = GlobalFree(hmem);
                let _ = CloseClipboard();
                return Err("SetClipboardData failed".to_string());
            }

            let _ = CloseClipboard();
            Ok(())
        }
    }

    pub fn get_clipboard_text_native() -> Option<String> {
        unsafe {
            if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
                return None;
            }
            if OpenClipboard(null_mut()) == 0 {
                return None;
            }

            let hdata = GetClipboardData(CF_UNICODETEXT);
            if hdata.is_null() {
                let _ = CloseClipboard();
                return None;
            }

            let ptr = GlobalLock(hdata) as *const u16;
            if ptr.is_null() {
                let _ = CloseClipboard();
                return None;
            }

            let mut len = 0usize;
            const MAX_SCAN: usize = 65536;
            while len < MAX_SCAN && *ptr.add(len) != 0 {
                len += 1;
            }

            let slice = std::slice::from_raw_parts(ptr, len);
            let text = String::from_utf16_lossy(slice);

            let _ = GlobalUnlock(hdata);
            let _ = CloseClipboard();
            Some(text)
        }
    }
}

/// Copies UTF-8 text to the system clipboard and records it in the in-memory history.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    record_clipboard_entry(text);
    #[cfg(target_os = "windows")]
    {
        win_clip::set_clipboard_text_native(text)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

/// Reads the current system clipboard UTF-8 text if available.
pub fn get_clipboard_text() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        win_clip::get_clipboard_text_native()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_history_dedup_and_order() {
        clear_clipboard_history();
        record_clipboard_entry("first item");
        record_clipboard_entry("second item");
        record_clipboard_entry("first item"); // duplicate should move to front

        let snap = get_history_snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].text, "first item");
        assert_eq!(snap[1].text, "second item");

        let matches = search_clipboard_query("clip second").unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].metadata.name.contains("second item"));

        let cb_matches = search_clipboard_query("cb first").unwrap();
        assert_eq!(cb_matches.len(), 1);
        assert!(cb_matches[0].metadata.name.contains("first item"));

        let empty_filter = search_clipboard_query("cb nonexistent_xyz").unwrap();
        assert_eq!(empty_filter.len(), 1);
        assert_eq!(empty_filter[0].match_type, "ClipboardHint");
        assert_eq!(empty_filter[0].metadata.full_path, "clipboard_empty_hint");
    }

    #[test]
    fn test_non_clipboard_query_returns_none() {
        assert!(search_clipboard_query("chrome").is_none());
        assert!(search_clipboard_query("clipping.pdf").is_none());
    }
}
