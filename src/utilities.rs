use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use image::ImageEncoder;
use windows::{
    core::{Interface, PCWSTR},
    Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, IPersistFile,
    },
    Win32::UI::Shell::{IShellLinkW, ShellLink},
};
pub const FOLDERID_DOCUMENTS: windows::core::GUID = windows::core::GUID::from_u128(0xFDD39AD0_238F_46AF_ADB4_6C85480369C7);
pub const FOLDERID_DESKTOP: windows::core::GUID = windows::core::GUID::from_u128(0xB4BFCC3A_1B2C_4054_9020_85B7EE2BEB86);
pub const FOLDERID_DOWNLOADS: windows::core::GUID = windows::core::GUID::from_u128(0x374DE290_123F_4565_9164_39C4925E467B);
pub const FOLDERID_MUSIC: windows::core::GUID = windows::core::GUID::from_u128(0x4BD8D571_B4BA_4F07_856B_2D8D0E1C68E6);
pub const FOLDERID_PICTURES: windows::core::GUID = windows::core::GUID::from_u128(0x33E28130_4E1E_4676_835A_98395C3BC3BB);
pub const FOLDERID_VIDEOS: windows::core::GUID = windows::core::GUID::from_u128(0x18989B1D_99B5_455B_841C_AB7C74E4DDFC);
pub const FOLDERID_PROGRAMS: windows::core::GUID = windows::core::GUID::from_u128(0xA77F5D77_2E2B_44C3_A6A2_ABA601054A51);
pub const FOLDERID_COMMON_PROGRAMS: windows::core::GUID = windows::core::GUID::from_u128(0x0139D44E_6AFE_49F2_8690_3DAFCAE6FFB8);

pub fn get_known_folder(folder_id: &windows::core::GUID) -> Option<PathBuf> {
    use windows::Win32::UI::Shell::SHGetKnownFolderPath;
    unsafe {
        if let Ok(path_ptr) = SHGetKnownFolderPath(folder_id, windows::Win32::UI::Shell::KNOWN_FOLDER_FLAG(0), None) {
            let path_str = path_ptr.to_string().ok()?;
            windows::Win32::System::Com::CoTaskMemFree(Some(path_ptr.0 as *const _));
            Some(PathBuf::from(path_str))
        } else {
            None
        }
    }
}

static ICON_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

/// Resolves a Windows shortcut (.lnk) file to its target path.
/// If resolution fails or the file is not a shortcut, returns None.
pub fn resolve_lnk(lnk_path: &Path) -> Option<PathBuf> {
    unsafe {
        // Initialize COM library on the current thread (idempotent — safe to call multiple times).
        // We intentionally do NOT call CoUninitialize here because this function runs on
        // various worker threads and uninitializing COM would break subsequent COM calls
        // on the same thread (COM apartment threading violation).
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let res = (|| -> windows::core::Result<PathBuf> {
            let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            let persist_file: IPersistFile = shell_link.cast()?;
            
            // Convert path to wide string
            let wide_path: Vec<u16> = lnk_path.to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            
            persist_file.Load(PCWSTR(wide_path.as_ptr()), windows::Win32::System::Com::STGM(0))?;
            
            let mut buffer = [0u16; 1024];
            shell_link.GetPath(&mut buffer, std::ptr::null_mut(), 0)?;
            
            let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
            let target = String::from_utf16_lossy(&buffer[..len]);
            Ok(PathBuf::from(target))
        })();

        res.ok().filter(|p| !p.as_os_str().is_empty())
    }
}

/// Expands environment variables in a string (e.g. "%USERPROFILE%\\Documents" -> "C:\\Users\\name\\Documents").
pub fn expand_env_vars(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let mut var_name = String::new();
            let mut found_end = false;
            while let Some(&next_c) = chars.peek() {
                if next_c == '%' {
                    chars.next(); // consume closing '%'
                    found_end = true;
                    break;
                } else {
                    if let Some(nc) = chars.next() {
                        var_name.push(nc);
                    }
                }
            }
            if found_end {
                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                } else {
                    result.push('%');
                    result.push_str(&var_name);
                    result.push('%');
                }
            } else {
                result.push('%');
                result.push_str(&var_name);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Custom thread-safe Base64 encoder (keeps dependencies minimal)
fn to_base64(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        
        let c0 = CHARSET[((n >> 18) & 63) as usize] as char;
        let c1 = CHARSET[((n >> 12) & 63) as usize] as char;
        let c2 = CHARSET[((n >> 6) & 63) as usize] as char;
        let c3 = CHARSET[(n & 63) as usize] as char;

        result.push(c0);
        result.push(c1);
        if i + 1 < bytes.len() {
            result.push(c2);
        } else {
            result.push('=');
        }
        if i + 2 < bytes.len() {
            result.push(c3);
        } else {
            result.push('=');
        }
        i += 3;
    }
    result
}

/// Reads an image from disk and downscales it to max dimensions, returning base64 PNG data.
/// Memory-efficient: limits max source dimensions and downsizes before encoding.
pub fn generate_image_thumbnail_base64(path: &str, max_width: u32, max_height: u32) -> Result<String, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("File not found".to_string());
    }

    let meta = std::fs::metadata(p).map_err(|e| format!("Failed to read file metadata: {}", e))?;
    if meta.len() > 100 * 1024 * 1024 {
        return Err("File too large for preview (>100MB)".to_string());
    }

    let img = image::io::Reader::open(p)
        .map_err(|e| format!("Cannot open image: {}", e))?
        .with_guessed_format()
        .map_err(|e| format!("Unsupported image format: {}", e))?
        .decode()
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    let thumb = img.thumbnail(max_width, max_height);
    let (width, height) = (thumb.width(), thumb.height());
    let rgba = thumb.to_rgba8();

    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(&rgba, width, height, image::ColorType::Rgba8)
        .map_err(|e| format!("Failed to encode thumbnail: {}", e))?;

    Ok(to_base64(&png_bytes))
}

/// High-performance Windows icon extractor.
/// Convers standard desktop HICON resources to compact PNG byte buffers using Windows GDI and the image crate.
pub fn extract_icon_to_png(path: &str, is_dir: bool, ext: &str) -> Option<Vec<u8>> {
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES, SHFILEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP,
        BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_DIRECTORY};

    let mut path_wide: Vec<u16> = path.encode_utf16().collect();
    path_wide.push(0);

    let mut shfi = SHFILEINFOW::default();
    let mut flags = SHGFI_ICON | SHGFI_LARGEICON;
    let mut file_attributes = FILE_ATTRIBUTE_NORMAL.0;

    // Use Shell file attributes mode to get generic folder/extension icons without hitting disk
    if is_dir {
        flags |= SHGFI_USEFILEATTRIBUTES;
        file_attributes = FILE_ATTRIBUTE_DIRECTORY.0;
        path_wide = "C:\\dummy_folder".encode_utf16().chain(std::iter::once(0)).collect();
    } else if !ext.is_empty() && !path.ends_with(".exe") && !path.ends_with(".lnk") {
        flags |= SHGFI_USEFILEATTRIBUTES;
        let dummy_name = format!("dummy.{}", ext);
        path_wide = dummy_name.encode_utf16().chain(std::iter::once(0)).collect();
    }

    let res = unsafe {
        SHGetFileInfoW(
            PCWSTR(path_wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(file_attributes),
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };

    if res == 0 || shfi.hIcon.0 as usize == 0 {
        return None;
    }

    let hicon = shfi.hIcon;
    let mut icon_info = windows::Win32::UI::WindowsAndMessaging::ICONINFO::default();
    let mut png_bytes = None;

    if unsafe { GetIconInfo(hicon, &mut icon_info) }.is_ok() {
        unsafe {
            let mut bmp = BITMAP::default();
            let get_obj_res = GetObjectW(
                HGDIOBJ(icon_info.hbmColor.0),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bmp as *mut _ as *mut _),
            );

            if get_obj_res > 0 {
                let width = bmp.bmWidth;
                let height = bmp.bmHeight;

                let hdc = CreateCompatibleDC(None);
                if hdc.0 as usize != 0 {
                    let mut bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: width,
                            biHeight: -height, // top-down bitmap
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: 0, // BI_RGB
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    let mut buffer = vec![0u8; (width * height * 4) as usize];
                    let get_bits_res = GetDIBits(
                        hdc,
                        icon_info.hbmColor,
                        0,
                        height as u32,
                        Some(buffer.as_mut_ptr() as *mut _),
                        &mut bmi,
                        DIB_RGB_COLORS,
                    );

                    if get_bits_res > 0 {
                        // Swap BGRA (Windows DIB format) to RGBA (PNG format)
                        for chunk in buffer.chunks_mut(4) {
                            if chunk.len() == 4 {
                                let b = chunk[0];
                                chunk[0] = chunk[2];
                                chunk[2] = b;
                            }
                        }

                        // Encode RGBA buffer into PNG bytes using in-memory encoder
                        let mut bytes = Vec::new();
                        let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
                        if encoder.write_image(&buffer, width as u32, height as u32, image::ColorType::Rgba8).is_ok() {
                            png_bytes = Some(bytes);
                        }
                    }
                    let _ = DeleteDC(hdc);
                }
            }
            if icon_info.hbmColor.0 as usize != 0 {
                let _ = DeleteObject(HGDIOBJ(icon_info.hbmColor.0));
            }
            if icon_info.hbmMask.0 as usize != 0 {
                let _ = DeleteObject(HGDIOBJ(icon_info.hbmMask.0));
            }
        }
    }

    unsafe {
        let _ = DestroyIcon(hicon);
    }

    png_bytes
}

/// Extracts the high-resolution application icon for a shell item (e.g. `shell:AppsFolder\...` UWP apps).
pub fn extract_shell_item_icon(path: &str) -> Option<Vec<u8>> {
    use windows::Win32::UI::Shell::{SHCreateItemFromParsingName, IShellItemImageFactory, SIIGBF_BIGGERSIZEOK};
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP,
        BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HGDIOBJ,
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let item_factory: IShellItemImageFactory = SHCreateItemFromParsingName(
            PCWSTR(wide_path.as_ptr()),
            None,
        ).ok()?;

        let size = SIZE { cx: 48, cy: 48 };
        let hbitmap = item_factory.GetImage(size, SIIGBF_BIGGERSIZEOK).ok()?;

        let mut bmp = BITMAP::default();
        let get_obj_res = GetObjectW(
            HGDIOBJ(hbitmap.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut _),
        );

        let mut png_bytes = None;
        if get_obj_res > 0 {
            let width = bmp.bmWidth;
            let height = bmp.bmHeight;

            let hdc = CreateCompatibleDC(None);
            if hdc.0 as usize != 0 {
                let mut bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: width,
                        biHeight: -height,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: 0,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                let mut buffer = vec![0u8; (width * height * 4) as usize];
                let get_bits_res = GetDIBits(
                    hdc,
                    hbitmap,
                    0,
                    height as u32,
                    Some(buffer.as_mut_ptr() as *mut _),
                    &mut bmi,
                    DIB_RGB_COLORS,
                );

                if get_bits_res > 0 {
                    for chunk in buffer.chunks_mut(4) {
                        if chunk.len() == 4 {
                            let b = chunk[0];
                            chunk[0] = chunk[2];
                            chunk[2] = b;
                        }
                    }

                    let mut bytes = Vec::new();
                    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
                    if encoder.write_image(&buffer, width as u32, height as u32, image::ColorType::Rgba8).is_ok() {
                        png_bytes = Some(bytes);
                    }
                }
                let _ = DeleteDC(hdc);
            }
        }
        let _ = DeleteObject(HGDIOBJ(hbitmap.0));
        png_bytes
    }
}

/// Retrieves and caches file/folder icons in-memory, resolving shortcuts to target executables.
/// Handles shell:AppsFolder paths (UWP/Store apps) and provides fallback icons.
pub fn get_icon_cached(metadata: &crate::models::FileMetadata) -> String {
    let cache = ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    
    let is_dir = metadata.file_type == crate::models::FileType::Folder;
    let is_shell_app = metadata.full_path.starts_with("shell:");
    let key = if is_dir {
        "folder".to_string()
    } else if is_shell_app {
        // UWP/Store apps use shell:AppsFolder paths — cache by the full shell path
        metadata.full_path.clone()
    } else if metadata.file_type == crate::models::FileType::Application || metadata.full_path.ends_with(".lnk") {
        if metadata.full_path.ends_with(".lnk") {
            resolve_lnk(Path::new(&metadata.full_path))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| metadata.full_path.clone())
        } else {
            metadata.full_path.clone()
        }
    } else {
        metadata.extension.to_lowercase()
    };

    // Cache hit path
    {
        let guard = match cache.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(base64) = guard.get(&key) {
            return base64.clone();
        }
    }

    // Cache miss: extract and encode
    let extracted = if is_dir {
        extract_icon_to_png("folder", true, "")
    } else if is_shell_app {
        // Try extracting high-res shell item icon first, fallback to generic app icon
        extract_shell_item_icon(&metadata.full_path)
            .or_else(|| extract_icon_to_png("app.exe", false, "exe"))
    } else if key.ends_with(".exe") {
        extract_icon_to_png(&key, false, "")
    } else {
        extract_icon_to_png(&metadata.full_path, false, &metadata.extension)
    };

    // If extraction failed and this is an Application, try the generic .exe fallback icon
    let extracted = if extracted.is_none() && metadata.file_type == crate::models::FileType::Application {
        extract_icon_to_png("app.exe", false, "exe")
    } else {
        extracted
    };

    let base64_str = if let Some(png_bytes) = extracted {
        to_base64(&png_bytes)
    } else {
        String::new()
    };

    // Store in cache (even if empty, to prevent repeat failures)
    {
        let mut guard = match cache.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.insert(key, base64_str.clone());
    }

    base64_str
}

/// Retrieves and caches an application icon from an executable file path.
pub fn get_icon_from_exe_path(exe_path: &str) -> Option<String> {
    let cache = ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = exe_path.to_string();
    {
        let guard = match cache.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(base64) = guard.get(&key) {
            if !base64.is_empty() {
                return Some(base64.clone());
            }
        }
    }

    let extracted = extract_icon_to_png(exe_path, false, "");
    if let Some(png_bytes) = extracted {
        let base64_str = to_base64(&png_bytes);
        let mut guard = match cache.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.insert(key, base64_str.clone());
        Some(base64_str)
    } else {
        None
    }
}

/// Retrieves the current process memory working set size in bytes
pub fn get_memory_usage() -> usize {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
        use windows::Win32::System::Threading::GetCurrentProcess;
        unsafe {
            let mut counters = PROCESS_MEMORY_COUNTERS::default();
            let handle = GetCurrentProcess();
            if GetProcessMemoryInfo(handle, &mut counters, std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32).is_ok() {
                counters.WorkingSetSize
            } else {
                0
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

/// Retrieves the writeable local AppData directory for storing configurations, databases, and logs.
/// Uses a robust fallback chain: LOCALAPPDATA → APPDATA → TEMP.
/// Never falls back to current_dir, which would fail in protected locations like Program Files.
pub fn get_app_data_dir() -> PathBuf {
    let base = if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_appdata)
    } else if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
    } else if let Ok(temp) = std::env::var("TEMP") {
        PathBuf::from(temp)
    } else {
        // Last resort: use the Windows temp directory
        std::env::temp_dir()
    };
    let mut path = base;
    path.push("Kelp");
    if let Err(e) = std::fs::create_dir_all(&path) {
        eprintln!("CRITICAL: Failed to create app data directory {:?}: {}", path, e);
    }
    path
}

/// Checks if a directory name is a known system, developer, or legacy junction folder.
pub fn is_excluded_dir_name(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    matches!(
        name_lower.as_str(),
        "node_modules"
            | ".git"
            | "target"
            | ".cargo"
            | ".rustup"
            | ".next"
            | ".nuxt"
            | "dist"
            | "build"
            | "out"
            | ".venv"
            | "venv"
            | "env"
            | ".env"
            | "__pycache__"
            | ".idea"
            | ".vscode"
            | ".gradle"
            | "vendor"
            | "bower_components"
            | "bin"
            | "obj"
            | ".turbo"
            | ".cache"
            | "$recycle.bin"
            | "system volume information"
            | "winsxs"
            | "npm-cache"
            | "my music"
            | "my pictures"
            | "my videos"
            | "my documents"
            | "application data"
            | "local settings"
    )
}

/// Checks if a path falls under any of the user-configured excluded paths.
pub fn is_path_in_user_exclusions(path_str: &str, excluded_paths: &[String]) -> bool {
    let path_norm = path_str.replace('/', "\\").to_lowercase();
    for excl in excluded_paths {
        if excl.is_empty() {
            continue;
        }
        let expanded = expand_env_vars(excl).replace('/', "\\").to_lowercase();
        let trimmed = expanded.trim_end_matches('\\');
        if path_norm == trimmed || path_norm.starts_with(&format!("{}\\", trimmed)) {
            return true;
        }
    }
    false
}

/// Fast single-pass filter for WalkDir entry pruning.
/// When returning true for a directory, WalkDir will not descend into its children.
pub fn should_exclude_dir_entry(entry: &walkdir::DirEntry, config: &crate::config::AppConfig) -> bool {
    // Never prune the search root itself
    if entry.depth() == 0 {
        return false;
    }

    let file_name = entry.file_name().to_string_lossy();
    let is_dir = entry.file_type().is_dir();

    // 1. Prune known excluded directory names immediately (O(1) name check, no syscalls)
    if is_dir && is_excluded_dir_name(&file_name) {
        return true;
    }

    let path = entry.path();
    let path_str = path.to_string_lossy();

    // Prune Temp folder under AppData\Local
    if is_dir && file_name.eq_ignore_ascii_case("temp") && path_str.to_lowercase().contains("appdata\\local") {
        return true;
    }

    // 2. Prune user-configured excluded paths
    if is_path_in_user_exclusions(&path_str, &config.excluded_paths) {
        return true;
    }

    // 3. Substring checks for deep nested paths
    let exclusions = [
        "\\node_modules\\",
        "\\.git\\",
        "\\target\\",
        "\\AppData\\Roaming\\npm-cache",
        "\\.cargo\\",
        "\\.rustup\\",
        "\\$RECYCLE.BIN",
        "\\System Volume Information",
        "\\Local Settings\\Temporary Internet Files",
        "\\Windows\\WinSxS",
        "\\Windows\\System32",
    ];
    for excl in &exclusions {
        if path_str.contains(excl) {
            return true;
        }
    }

    // 4. Windows file attributes check (prunes reparse points/junctions and hidden/system directories)
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(meta) = entry.metadata() {
            let attrs = meta.file_attributes();
            // 0x2 = HIDDEN, 0x4 = SYSTEM, 0x400 = REPARSE_POINT
            if (attrs & 0x2) != 0 || (attrs & 0x4) != 0 || (attrs & 0x400) != 0 {
                return true;
            }
        } else {
            return true;
        }
    }

    // 5. File-level filtering
    if !is_dir {
        if file_name.starts_with("~$") || file_name.starts_with('.') {
            return true;
        }
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if !config.supported_extensions.contains(&ext_str) {
                return true;
            }
        } else {
            return true;
        }
    }

    false
}

/// Shared path exclusion logic for indexing and file watching against AppConfig.
/// Returns true if the path should be excluded.
pub fn should_exclude_path(path: &Path, config: &crate::config::AppConfig) -> bool {
    let path_str = path.to_string_lossy();

    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        if is_excluded_dir_name(&name_str) {
            return true;
        }
    }

    if is_path_in_user_exclusions(&path_str, &config.excluded_paths) {
        return true;
    }

    let exclusions = [
        "\\node_modules\\",
        "\\.git\\",
        "\\target\\",
        "\\AppData\\Local\\Temp",
        "\\AppData\\Roaming\\npm-cache",
        "\\.cargo\\",
        "\\.rustup\\",
        "\\$RECYCLE.BIN",
        "\\System Volume Information",
        "\\Local Settings\\Temporary Internet Files",
        "\\Windows\\WinSxS",
        "\\Windows\\System32",
        "\\Documents\\My Music",
        "\\Documents\\My Pictures",
        "\\Documents\\My Videos",
        "\\My Documents",
        "\\Application Data",
        "\\Local Settings",
    ];

    for excl in &exclusions {
        if path_str.contains(excl) {
            return true;
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        let sym_meta = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(_) => return true,
        };

        let attributes = sym_meta.file_attributes();
        if (attributes & 0x2) != 0 || (attributes & 0x4) != 0 || (attributes & 0x400) != 0 {
            return true;
        }

        let is_dir = sym_meta.is_dir();
        if !is_dir {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !config.supported_extensions.contains(&ext_str) {
                    return true;
                }
            } else {
                return true;
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if !path.is_dir() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !config.supported_extensions.contains(&ext_str) {
                    return true;
                }
            } else {
                return true;
            }
        }
    }

    false
}
