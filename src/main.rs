#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use engine::UIBridge;
use std::sync::Arc;
use tracing::{info, error, warn};

use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
    dpi::LogicalSize,
};
use wry::WebViewBuilder;
use global_hotkey::{
    GlobalHotKeyManager,
    hotkey::{HotKey, Modifiers, Code},
    GlobalHotKeyEvent,
};
use serde::Deserialize;
use tray_icon::{
    TrayIconBuilder,
    menu::{Menu, MenuItem, MenuEvent},
    TrayIconEvent,
};

fn load_icon_from_memory() -> tray_icon::Icon {
    let icon_bytes = include_bytes!("../assets/logo.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon from memory")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    tray_icon::Icon::from_rgba(rgba, width, height).expect("Failed to build icon")
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum IpcMessage {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "search")]
    Search { query: String },
    #[serde(rename = "select")]
    Select { query: String, path: String },
    #[serde(rename = "search_web")]
    SearchWeb { query: String },
    #[serde(rename = "browse_folder")]
    BrowseFolder { path: String },
    #[serde(rename = "run_as_admin")]
    RunAsAdmin { path: String },
    #[serde(rename = "open_file_location")]
    OpenFileLocation { path: String },
    #[serde(rename = "copy_to_clipboard")]
    CopyToClipboard { text: String },
    #[serde(rename = "system_command")]
    SystemCommand { command: String },
    #[serde(rename = "resize")]
    Resize { height: f64 },
    #[serde(rename = "hide_window")]
    HideWindow,
}

#[derive(Debug)]
enum UserEvent {
    Ready,
    GlobalHotkey(global_hotkey::GlobalHotKeyEvent),
    SearchRequest { query: String },
    SearchCompleted { results_json: String, debug_json: Option<String> },
    SelectRequest { query: String, path: String },
    SearchWeb { query: String },
    BrowseFolderRequest { path: String },
    BrowseFolderCompleted { results_json: String },
    RunAsAdminRequest { path: String },
    OpenFileLocationRequest { path: String },
    CopyToClipboardRequest { text: String },
    SystemCommandRequest { command: String },
    ResizeRequest { height: f64 },
    HideWindow,
    TrayIcon(tray_icon::TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
}

#[cfg(target_os = "windows")]
unsafe fn get_foreground_window_title() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
    let hwnd = GetForegroundWindow();
    if hwnd.0 as usize == 0 {
        return "None".to_string();
    }
    let mut buffer = [0u16; 512];
    let len = GetWindowTextW(hwnd, &mut buffer);
    if len > 0 {
        String::from_utf16_lossy(&buffer[..len as usize])
    } else {
        format!("HWND({:?})", hwnd.0)
    }
}

#[cfg(target_os = "windows")]
unsafe fn center_window_on_active_monitor(window: &tao::window::Window) {
    use tao::platform::windows::WindowExtWindows;
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use windows::Win32::Graphics::Gdi::{
        MonitorFromWindow, GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
    };

    let hwnd = windows::Win32::Foundation::HWND(window.hwnd() as *mut _);
    let foreground_hwnd = GetForegroundWindow();
    
    // Fallback to our own window if no foreground window exists
    let target_hwnd = if foreground_hwnd.0 as usize != 0 {
        foreground_hwnd
    } else {
        hwnd
    };

    let hmonitor = MonitorFromWindow(target_hwnd, MONITOR_DEFAULTTOPRIMARY);
    let mut mi = MONITORINFO::default();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

    if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
        let work_left = mi.rcWork.left;
        let work_top = mi.rcWork.top;
        let work_width = mi.rcWork.right - mi.rcWork.left;
        let work_height = mi.rcWork.bottom - mi.rcWork.top;

        let scale_factor = window.scale_factor();
        let kelp_width_phys = (800.0 * scale_factor) as i32;

        let x = work_left + (work_width - kelp_width_phys) / 2;
        let y = work_top + work_height / 5;

        info!(
            "[Monitor] Active Monitor Work Area: left={}, top={}, width={}, height={}. Positioning window at x={}, y={}",
            work_left, work_top, work_width, work_height, x, y
        );

        window.set_outer_position(tao::dpi::PhysicalPosition::new(x, y));
    } else {
        warn!("[Monitor] Failed to retrieve monitor info for HWND {:?}", target_hwnd.0);
    }
}

#[cfg(target_os = "windows")]
unsafe fn force_set_foreground_window(window: &tao::window::Window) {
    use tao::platform::windows::WindowExtWindows;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, SetForegroundWindow, SetWindowPos, HWND_TOPMOST,
        SET_WINDOW_POS_FLAGS,
    };

    let hwnd = windows::Win32::Foundation::HWND(window.hwnd() as *mut _);

    // 1. Log transition details
    let prev_fg = get_foreground_window_title();
    info!(
        "[Window] Showing Kelp window. Previous foreground window: '{}' (HWND: {:?})",
        prev_fg, GetForegroundWindow().0
    );

    // 2. Set visible first
    window.set_visible(true);

    // 3. Make sure it is topmost
    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0, 0, 0, 0,
        SET_WINDOW_POS_FLAGS(0x0002 | 0x0001 | 0x0040), // SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW
    );

    // 4. Force foreground activation.
    // Thanks to AllowSetForegroundWindow called on the hotkey thread, SetForegroundWindow will succeed natively!
    let _ = SetForegroundWindow(hwnd);
    
    // 5. Let Tao set focus cleanly to keep event loop state in sync
    window.set_focus();
    info!("[Window] Visibility and focus set via Tao APIs.");
}

#[tokio::main]
async fn main() {
    // 1. Install Global Panic Hook
    engine::logger::setup_panic_handler();

    // 2. Initialize Structured Logging
    let logger = engine::logger::FileLogger::new();
    if let Err(e) = tracing::subscriber::set_global_default(logger) {
        eprintln!("Failed to set global structured logger: {:?}", e);
    }
    info!("Starting Kelp Search Engine Launcher...");

    // 3. Initialize Windows COM library for Shell link resolving
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
    }

    // 4. Resolve database location (saved in Local AppData folder)
    let db_path = engine::utilities::get_app_data_dir().join("kelp.db");

    // Automatic database migration from legacy Nova Launcher database
    if !db_path.exists() {
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let old_db = std::path::PathBuf::from(local_appdata)
                .join("Nova Launcher")
                .join("search_engine.db");
            if old_db.exists() {
                let _ = std::fs::copy(&old_db, &db_path);
                info!("Successfully migrated legacy database from Nova Launcher to Kelp.");
            }
        }
    }

    // 5. Default Windows paths to crawl
    let watch_paths = engine::indexer::Indexer::default_windows_paths();

    // 6. Initialize search engine bridge
    let engine = Arc::new(
        UIBridge::initialize(&db_path, &watch_paths)
            .await
            .expect("Failed to initialize Search Engine Bridge")
    );

    // 7. Create Tao Event Loop
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let event_loop_proxy = event_loop.create_proxy();

    // 8. Register Alt + Space Global Hotkey (with error recovery)
    let hotkey_manager = match GlobalHotKeyManager::new() {
        Ok(mgr) => mgr,
        Err(e) => {
            error!("Failed to create global hotkey manager: {:?}", e);
            panic!("Cannot create hotkey manager");
        }
    };
    let alt_space_hotkey = HotKey::new(Some(Modifiers::ALT), Code::Space);
    let mut hotkey_registered = false;
    if let Err(e) = hotkey_manager.register(alt_space_hotkey) {
        warn!("Failed to register Alt+Space hotkey (may already be registered): {:?}", e);
    } else {
        hotkey_registered = true;
    }
    let alt_space_id = alt_space_hotkey.id();

    // Run Startup Self-Validation and Search-Validation
    run_self_validation(&engine, hotkey_registered);

    // Spawn background hotkey polling thread to ensure sleep-proof global wakeups
    let hotkey_proxy = event_loop_proxy.clone();
    std::thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();
        while let Ok(hotkey_event) = receiver.recv() {
            // Delegate foreground activation permission to this process before sending event
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(
                    windows::Win32::System::Threading::GetCurrentProcessId()
                );
            }
            let _ = hotkey_proxy.send_event(UserEvent::GlobalHotkey(hotkey_event));
        }
    });

    // 8. Build borderless, transparent Window centered elevated on monitor (start HIDDEN)
    let mut builder = WindowBuilder::new()
        .with_title("Kelp")
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_visible(false) // Startup invisible requirement
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(800.0, 96.0));

    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_undecorated_shadow(false).with_skip_taskbar(true);
    }

    let window = builder.build(&event_loop).unwrap();

    // Center window horizontally and place at top third of screen
    if let Some(monitor) = window.current_monitor() {
        let monitor_size = monitor.size();
        let monitor_pos = monitor.position();
        let scale_factor = window.scale_factor();

        let win_width_phys = (800.0 * scale_factor) as i32;

        let x = monitor_pos.x + (monitor_size.width as i32 - win_width_phys) / 2;
        let y = monitor_pos.y + (monitor_size.height as i32) / 5;
        window.set_outer_position(tao::dpi::PhysicalPosition::new(x, y));
    }

    // 9. Initialize System Tray
    let tray_menu = Menu::new();
    let show_item = MenuItem::new("Show Kelp", true, None);
    let exit_item = MenuItem::new("Exit Kelp", true, None);
    let _ = tray_menu.append(&show_item);
    let _ = tray_menu.append(&exit_item);

    let show_item_id = show_item.id().clone();
    let exit_item_id = exit_item.id().clone();

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Kelp Search")
        .with_icon(load_icon_from_memory())
        .build()
        .unwrap();

    let tray_proxy = event_loop_proxy.clone();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = tray_proxy.send_event(UserEvent::TrayIcon(event));
    }));

    let menu_proxy = event_loop_proxy.clone();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_proxy.send_event(UserEvent::MenuEvent(event));
    }));

    // 10. Attach transparent Wry WebView hosting the embedded HTML client
    let html_content = include_str!("ui.html");

    let webview = WebViewBuilder::new(&window)
        .with_transparent(true)
        .with_html(html_content)
        .with_ipc_handler({
            let proxy = event_loop_proxy.clone();
            move |request| {
                let msg_str = request.body();
                match serde_json::from_str::<IpcMessage>(msg_str) {
                    Ok(msg) => {
                        let event = match msg {
                            IpcMessage::Ready => UserEvent::Ready,
                            IpcMessage::Search { query } => UserEvent::SearchRequest { query },
                            IpcMessage::Select { query, path } => UserEvent::SelectRequest { query, path },
                            IpcMessage::SearchWeb { query } => UserEvent::SearchWeb { query },
                            IpcMessage::BrowseFolder { path } => UserEvent::BrowseFolderRequest { path },
                            IpcMessage::RunAsAdmin { path } => UserEvent::RunAsAdminRequest { path },
                            IpcMessage::OpenFileLocation { path } => UserEvent::OpenFileLocationRequest { path },
                            IpcMessage::CopyToClipboard { text } => UserEvent::CopyToClipboardRequest { text },
                            IpcMessage::SystemCommand { command } => UserEvent::SystemCommandRequest { command },
                            IpcMessage::Resize { height } => UserEvent::ResizeRequest { height },
                            IpcMessage::HideWindow => UserEvent::HideWindow,
                        };
                        let _ = proxy.send_event(event);
                    }
                    Err(e) => {
                        warn!("Failed to parse IPC message: {} — raw: {}", e, msg_str);
                    }
                }
            }
        })
        .build()
        .unwrap();

    // 10. Run Event Loop
    let _keep_alive = (hotkey_manager, tray_icon);
    event_loop.run(move |event, _, control_flow| {
        let _ = &_keep_alive; // Force moving into closure to keep hotkey and tray registered forever
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                info!("Kelp Search Launcher event loop started.");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                window.set_visible(false);
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(focused),
                ..
            } => {
                info!("[Window] Focus changed: focused={}", focused);
                if focused {
                    let _ = webview.evaluate_script("cancelHide()");
                } else {
                    // Premium Auto-Hide on blur
                    let _ = webview.evaluate_script("hideLauncher()");
                }
            }
            Event::UserEvent(user_event) => {
                match user_event {
                    UserEvent::Ready => {
                        info!("Frontend WebView is ready and listening.");
                        let debug_startup = std::env::var("DEBUG_STARTUP")
                            .map(|v| v.to_lowercase() == "true")
                            .unwrap_or(false);
                        if debug_startup {
                            info!("[Debug Startup] DEBUG_STARTUP=true detected. Showing launcher immediately.");
                            unsafe {
                                center_window_on_active_monitor(&window);
                                force_set_foreground_window(&window);
                            }
                            let _ = webview.evaluate_script("showLauncher()");
                        }
                    }
                    UserEvent::GlobalHotkey(hotkey_event) => {
                        if hotkey_event.id == alt_space_id && hotkey_event.state == global_hotkey::HotKeyState::Pressed {
                            let is_visible = window.is_visible();
                            info!("[Hotkey] Pressed. Current Kelp window visibility: {}", is_visible);
                            if is_visible {
                                let _ = webview.evaluate_script("hideLauncher()");
                            } else {
                                unsafe {
                                    center_window_on_active_monitor(&window);
                                    force_set_foreground_window(&window);
                                }
                                let _ = webview.evaluate_script("showLauncher()");
                            }
                        }
                    }
                    UserEvent::SearchRequest { query } => {
                        let proxy = event_loop_proxy.clone();
                        let engine_c = engine.clone();
                        tokio::task::spawn_blocking(move || {
                            // Measure exact search and ranking timings
                            let search_start = std::time::Instant::now();
                            let parsed_query = engine::query_parser::parse_query(&query);
                            let (mut results, matched_files) = engine_c.search_engine.search(&parsed_query);
                            let search_time_us = search_start.elapsed().as_micros() as f64 / 1000.0;

                            let all_files = engine_c.index.get_all();
                            let mut exact_matches = Vec::new();
                            let mut prefix_matches = Vec::new();
                            let mut contains_matches = Vec::new();
                            let mut fuzzy_matches = Vec::new();

                            for file in &all_files {
                                if let Some(res) = engine::search::match_file(file, &parsed_query) {
                                    match res.match_type.as_str() {
                                        "Exact" => exact_matches.push(res.metadata.name.clone()),
                                        "Prefix" => prefix_matches.push(res.metadata.name.clone()),
                                        "Contains" => contains_matches.push(res.metadata.name.clone()),
                                        "Fuzzy" => fuzzy_matches.push(res.metadata.name.clone()),
                                        _ => {}
                                    }
                                }
                            }

                            info!("Normalized query: '{}'", parsed_query.raw);
                            info!("Exact matches: {:?}", exact_matches);
                            info!("Prefix matches: {:?}", prefix_matches);
                            info!("Contains matches: {:?}", contains_matches);
                            info!("Fuzzy matches: {:?}", fuzzy_matches);

                            let rank_start = std::time::Instant::now();
                            engine_c.ranking_engine.rank(&mut results, &parsed_query);
                            let rank_time_us = rank_start.elapsed().as_micros() as f64 / 1000.0;

                            // Re-apply truncation and population (same as bridge search)
                            let q_len = parsed_query.raw.len();
                            let threshold = if q_len <= 2 { 0.2 } else if q_len <= 4 { 0.3 } else { 0.4 };
                            results.retain(|r| r.score >= threshold);
                            results.truncate(15);

                            info!("Final ranked list: {:?}", results.iter().map(|r| format!("{} (score={:.3}, type={})", r.metadata.name, r.score, r.match_type)).collect::<Vec<_>>());

                            for r in &mut results {
                                r.icon_base64 = Some(engine::utilities::get_icon_cached(&r.metadata));
                            }

                            engine_c.cache.insert(&query, matched_files, results.clone());

                            let results_json = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());

                            let debug_json = if cfg!(debug_assertions) {
                                use std::sync::atomic::Ordering;
                                let mem_bytes = engine::utilities::get_memory_usage();
                                let mem_mb = mem_bytes as f64 / (1024.0 * 1024.0);
                                let info = serde_json::json!({
                                    "files_count": engine_c.total_files(),
                                    "search_time_ms": search_time_us,
                                    "rank_time_ms": rank_time_us,
                                    "cache_hits": engine_c.cache.hits.load(Ordering::SeqCst),
                                    "cache_misses": engine_c.cache.misses.load(Ordering::SeqCst),
                                    "memory_mb": mem_mb,
                                });
                                Some(info.to_string())
                            } else {
                                None
                            };

                            let _ = proxy.send_event(UserEvent::SearchCompleted { results_json, debug_json });
                        });
                    }
                    UserEvent::SearchCompleted { results_json, debug_json } => {
                        let script = if let Some(ref dbg) = debug_json {
                            format!("setResults({}, {})", results_json, dbg)
                        } else {
                            format!("setResults({}, null)", results_json)
                        };
                        if let Err(e) = webview.evaluate_script(&script) {
                            error!("Failed to push results to webview: {:?}", e);
                        }
                    }
                    UserEvent::SelectRequest { query, path } => {
                        let engine_c = engine.clone();
                        let path_clone = path.clone();
                        // Asynchronously record selection to database
                        tokio::task::spawn_blocking(move || {
                            if let Err(e) = engine_c.select_result(&query, &path_clone) {
                                error!("Failed to save learning selection: {}", e);
                            }
                        });
                        // Asynchronously execute target file shell launch to prevent event loop blocking
                        let path_launch = path.clone();
                        tokio::task::spawn_blocking(move || {
                            launch_file(&path_launch);
                        });
                    }
                    UserEvent::SearchWeb { query } => {
                        tokio::task::spawn_blocking(move || {
                            launch_url(&query);
                        });
                    }
                    UserEvent::BrowseFolderRequest { path } => {
                        let proxy = event_loop_proxy.clone();
                        tokio::task::spawn_blocking(move || {
                            let mut entries: Vec<serde_json::Value> = Vec::new();
                            let dir = match std::fs::read_dir(&path) {
                                Ok(d) => d,
                                Err(e) => {
                                    warn!("Failed to read directory '{}': {}", path, e);
                                    let _ = proxy.send_event(UserEvent::BrowseFolderCompleted {
                                        results_json: "[]".to_string(),
                                    });
                                    return;
                                }
                            };

                            let mut dirs_list = Vec::new();
                            let mut files_list = Vec::new();

                            for entry in dir {
                                let entry = match entry {
                                    Ok(e) => e,
                                    Err(_) => continue,
                                };
                                let entry_path = entry.path();
                                let meta = match entry.metadata() {
                                    Ok(m) => m,
                                    Err(_) => continue,
                                };

                                // Skip hidden/system files on Windows
                                #[cfg(target_os = "windows")]
                                {
                                    use std::os::windows::fs::MetadataExt;
                                    let attrs = meta.file_attributes();
                                    if (attrs & 0x2) != 0 || (attrs & 0x4) != 0 {
                                        continue;
                                    }
                                }

                                let name = entry_path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let is_dir = meta.is_dir();
                                let extension = if is_dir {
                                    String::new()
                                } else {
                                    entry_path
                                        .extension()
                                        .map(|e| e.to_string_lossy().to_string().to_lowercase())
                                        .unwrap_or_default()
                                };
                                let file_type = if is_dir {
                                    engine::models::FileType::Folder
                                } else if extension == "exe" || extension == "lnk" {
                                    engine::models::FileType::Application
                                } else {
                                    engine::models::FileType::File
                                };
                                let modified_date = meta
                                    .modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs() as i64)
                                    .unwrap_or(0);
                                let size = if is_dir { 0 } else { meta.len() as i64 };
                                let full_path_str = entry_path.to_string_lossy().to_string();
                                let parent_folder = entry_path
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default();

                                let file_meta = engine::models::FileMetadata {
                                    id: None,
                                    name: name.clone(),
                                    extension: extension.clone(),
                                    parent_folder,
                                    full_path: full_path_str,
                                    modified_date,
                                    size,
                                    file_type,
                                };

                                let icon = engine::utilities::get_icon_cached(&file_meta);

                                let value = serde_json::json!({
                                    "metadata": file_meta,
                                    "icon_base64": icon,
                                });

                                if is_dir {
                                    dirs_list.push((name.to_lowercase(), value));
                                } else {
                                    files_list.push((name.to_lowercase(), value));
                                }
                            }

                            // Sort: directories first (alphabetically), then files (alphabetically)
                            dirs_list.sort_by(|a, b| a.0.cmp(&b.0));
                            files_list.sort_by(|a, b| a.0.cmp(&b.0));

                            for (_, v) in dirs_list {
                                entries.push(v);
                                if entries.len() >= 50 {
                                    break;
                                }
                            }
                            if entries.len() < 50 {
                                for (_, v) in files_list {
                                    entries.push(v);
                                    if entries.len() >= 50 {
                                        break;
                                    }
                                }
                            }

                            let results_json = serde_json::to_string(&entries)
                                .unwrap_or_else(|_| "[]".to_string());
                            let _ = proxy.send_event(UserEvent::BrowseFolderCompleted { results_json });
                        });
                    }
                    UserEvent::BrowseFolderCompleted { results_json } => {
                        let script = format!("setFolderContents({})", results_json);
                        if let Err(e) = webview.evaluate_script(&script) {
                            error!("Failed to push folder contents to webview: {:?}", e);
                        }
                    }
                    UserEvent::RunAsAdminRequest { path } => {
                        info!("[Launch] Run as administrator: '{}'", path);
                        tokio::task::spawn_blocking(move || {
                            let path_w = windows::core::HSTRING::from(path.as_str());
                            let verb_w = windows::core::HSTRING::from("runas");
                            unsafe {
                                let res = windows::Win32::UI::Shell::ShellExecuteW(
                                    windows::Win32::Foundation::HWND(std::ptr::null_mut()),
                                    windows::core::PCWSTR(verb_w.as_ptr()),
                                    windows::core::PCWSTR(path_w.as_ptr()),
                                    windows::core::PCWSTR(std::ptr::null()),
                                    windows::core::PCWSTR(std::ptr::null()),
                                    windows::Win32::UI::WindowsAndMessaging::SW_SHOW,
                                );
                                info!("[Launch] RunAsAdmin ShellExecuteW result: {:?}", res);
                            }
                        });
                    }
                    UserEvent::OpenFileLocationRequest { path } => {
                        info!("[Launch] Open file location: '{}'", path);
                        tokio::task::spawn_blocking(move || {
                            let explorer_w = windows::core::HSTRING::from("explorer.exe");
                            let args = format!("/select,\"{}\"", path);
                            let args_w = windows::core::HSTRING::from(args.as_str());
                            unsafe {
                                let _ = windows::Win32::UI::Shell::ShellExecuteW(
                                    windows::Win32::Foundation::HWND(std::ptr::null_mut()),
                                    windows::core::PCWSTR(std::ptr::null()),
                                    windows::core::PCWSTR(explorer_w.as_ptr()),
                                    windows::core::PCWSTR(args_w.as_ptr()),
                                    windows::core::PCWSTR(std::ptr::null()),
                                    windows::Win32::UI::WindowsAndMessaging::SW_SHOW,
                                );
                            }
                        });
                    }
                    UserEvent::CopyToClipboardRequest { text } => {
                        info!("[Clipboard] Copying text: '{}'", text);
                        tokio::task::spawn_blocking(move || {
                            #[cfg(target_os = "windows")]
                            use std::os::windows::process::CommandExt;
                            const CREATE_NO_WINDOW: u32 = 0x08000000;
                            let _ = std::process::Command::new("powershell")
                                .args(["-NoProfile", "-Command", &format!("Set-Clipboard -Value '{}'", text.replace('\'', "''"))])
                                .creation_flags(CREATE_NO_WINDOW)
                                .output();
                        });
                    }
                    UserEvent::SystemCommandRequest { command } => {
                        info!("[System] Executing system command: '{}'", command);
                        tokio::task::spawn_blocking(move || {
                            #[cfg(target_os = "windows")]
                            use std::os::windows::process::CommandExt;
                            const CREATE_NO_WINDOW: u32 = 0x08000000;
                            let _ = std::process::Command::new("cmd")
                                .args(["/C", &command])
                                .creation_flags(CREATE_NO_WINDOW)
                                .spawn();
                        });
                    }
                    UserEvent::ResizeRequest { height } => {
                        let clamped = height.max(80.0).min(800.0);
                        window.set_inner_size(LogicalSize::new(800.0, clamped));
                    }
                    UserEvent::HideWindow => {
                        info!("[Window] Hiding Kelp window.");
                        window.set_visible(false);
                    }
                    UserEvent::TrayIcon(tray_event) => {
                        match tray_event {
                            tray_icon::TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. } => {
                                let is_visible = window.is_visible();
                                if is_visible {
                                    let _ = webview.evaluate_script("hideLauncher()");
                                } else {
                                    unsafe {
                                        center_window_on_active_monitor(&window);
                                        force_set_foreground_window(&window);
                                    }
                                    let _ = webview.evaluate_script("showLauncher()");
                                }
                            }
                            _ => {}
                        }
                    }
                    UserEvent::MenuEvent(menu_event) => {
                        if menu_event.id == show_item_id {
                            unsafe {
                                center_window_on_active_monitor(&window);
                                force_set_foreground_window(&window);
                            }
                            let _ = webview.evaluate_script("showLauncher()");
                        } else if menu_event.id == exit_item_id {
                            info!("Exiting Kelp via Tray Icon Exit command.");
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
            }
            _ => (),
        }
    });
}

/// Launch selected file/folder using standard default association on Windows
fn launch_file(path: &str) {
    if path.is_empty() {
        return;
    }
    info!("[Launch] Request to execute: '{}'", path);
    if path.starts_with("shell:AppsFolder\\") {
        let exe_w = windows::core::HSTRING::from("explorer.exe");
        let args_w = windows::core::HSTRING::from(path);
        unsafe {
            let res = windows::Win32::UI::Shell::ShellExecuteW(
                windows::Win32::Foundation::HWND(std::ptr::null_mut()),
                windows::core::PCWSTR(std::ptr::null()),
                windows::core::PCWSTR(exe_w.as_ptr()),
                windows::core::PCWSTR(args_w.as_ptr()),
                windows::core::PCWSTR(std::ptr::null()),
                windows::Win32::UI::WindowsAndMessaging::SW_SHOW,
            );
            info!("[Launch] UWP App ShellExecuteW result for '{}': {:?}", path, res);
        }
        return;
    }
    let path_w = windows::core::HSTRING::from(path);
    unsafe {
        let res = windows::Win32::UI::Shell::ShellExecuteW(
            windows::Win32::Foundation::HWND(std::ptr::null_mut()),
            windows::core::PCWSTR(std::ptr::null()),
            windows::core::PCWSTR(path_w.as_ptr()),
            windows::core::PCWSTR(std::ptr::null()),
            windows::core::PCWSTR(std::ptr::null()),
            windows::Win32::UI::WindowsAndMessaging::SW_SHOW,
        );
        info!("[Launch] ShellExecuteW result for '{}': {:?}", path, res);
    }
}

/// Encodes query spaces and triggers Google Search in the default browser
fn launch_url(query: &str) {
    if query.trim().is_empty() {
        return;
    }
    let encoded = query.replace(' ', "+");
    let url = format!("https://www.google.com/search?q={}", encoded);
    launch_file(&url);
}

/// Run startup self-validation diagnostics and search engine checks
fn run_self_validation(engine: &UIBridge, hotkey_registered: bool) {
    info!("==================== STARTUP SELF-VALIDATION ====================");
    
    // 1. Check if database exists
    let db_path = engine::utilities::get_app_data_dir().join("kelp.db");
    if db_path.exists() {
        info!("✓ [Self-Validation] SQLite Database exists: {:?}", db_path);
    } else {
        warn!("✗ [Self-Validation] SQLite Database file not found at {:?}", db_path);
    }

    // 2. Check if index is loaded
    let file_count = engine.total_files();
    if file_count > 0 {
        info!("✓ [Self-Validation] Memory Index loaded successfully: {} files in RAM.", file_count);
    } else {
        warn!("✗ [Self-Validation] Memory Index is empty.");
    }

    // 3. Check if hotkey is registered
    if hotkey_registered {
        info!("✓ [Self-Validation] Global Alt+Space hotkey registered successfully.");
    } else {
        error!("✗ [Self-Validation] Global Alt+Space hotkey registration failed! Hotkey may be in use by another app.");
    }

    // 4. Check if Watcher is active
    if engine.is_watcher_running() {
        info!("✓ [Self-Validation] FileWatcher is running in the background.");
    } else {
        warn!("✗ [Self-Validation] FileWatcher is not running.");
    }

    // 5. Check if Learning Engine is ready
    if engine.is_learning_ready() {
        info!("✓ [Self-Validation] Learning Database cache loaded successfully.");
    } else {
        warn!("✗ [Self-Validation] Learning Database cache is not initialized.");
    }
    
    // 6. Search Validation Checks
    info!("Running search validation checks...");
    
    let (res_resume, _) = engine.search("resume");
    if res_resume.iter().any(|r| r.metadata.name.to_lowercase().contains("resume")) {
        info!("✓ [Search-Validation] Searching 'resume' successfully returns Resume matches.");
    } else {
        warn!("✗ [Search-Validation] Searching 'resume' did not return any Resume matches.");
    }

    let extensions_to_test = ["pdf", "png", "rs", "exe", "lnk"];
    for ext in &extensions_to_test {
        let (res_ext, _) = engine.search(&format!(".{}", ext));
        if res_ext.iter().any(|r| r.metadata.extension.to_lowercase() == *ext) {
            info!("✓ [Search-Validation] Searching '.{}' successfully returns {} files.", ext, ext.to_uppercase());
        } else {
            warn!("✗ [Search-Validation] Searching '.{}' returned no results.", ext);
        }
    }

    let (res_aadhar, _) = engine.search("aadhar");
    info!("[Search-Validation] Query 'aadhar' returned {} results:", res_aadhar.len());
    for r in &res_aadhar {
        info!("  - name='{}', score={}, match_type={}", r.metadata.name, r.score, r.match_type);
    }

    info!("=================================================================");
}
