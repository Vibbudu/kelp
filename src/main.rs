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
    ResizeRequest { height: f64 },
    HideWindow,
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
    info!("Starting Nova Search Engine Launcher...");

    // 3. Initialize Windows COM library for Shell link resolving
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
    }

    // 4. Resolve database location (saved in execution folder)
    let db_path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("search_engine.db");

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
            let _ = hotkey_proxy.send_event(UserEvent::GlobalHotkey(hotkey_event));
        }
    });

    // 8. Build borderless, transparent Window centered elevated on monitor (start HIDDEN)
    let mut builder = WindowBuilder::new()
        .with_title("Nova Launcher")
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

    // 9. Attach transparent Wry WebView hosting the embedded HTML client
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
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                info!("Nova Search Launcher event loop started.");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(false),
                ..
            } => {
                // Premium Auto-Hide on blur
                let _ = webview.evaluate_script("hideLauncher()");
            }
            Event::UserEvent(user_event) => {
                match user_event {
                    UserEvent::Ready => {
                        info!("Frontend WebView is ready and listening.");
                    }
                    UserEvent::GlobalHotkey(hotkey_event) => {
                        if hotkey_event.id == alt_space_id && hotkey_event.state == global_hotkey::HotKeyState::Pressed {
                            let is_visible = window.is_visible();
                            if is_visible {
                                let _ = webview.evaluate_script("hideLauncher()");
                            } else {
                                window.set_visible(true);
                                window.set_focus();
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
                            let (mut results, _matched_files) = engine_c.search_engine.search(&parsed_query);
                            let search_time_us = search_start.elapsed().as_micros() as f64 / 1000.0;

                            let rank_start = std::time::Instant::now();
                            engine_c.ranking_engine.rank(&mut results, &parsed_query);
                            let rank_time_us = rank_start.elapsed().as_micros() as f64 / 1000.0;

                            // Re-apply truncation and population (same as bridge search)
                            let q_len = parsed_query.raw.len();
                            let threshold = if q_len <= 2 { 0.3 } else if q_len <= 4 { 0.4 } else { 0.5 };
                            results.retain(|r| r.score >= threshold);
                            results.truncate(15);

                            for r in &mut results {
                                r.icon_base64 = Some(engine::utilities::get_icon_cached(&r.metadata));
                            }

                            let top_matched_files: Vec<engine::FileMetadata> = results.iter().map(|r| r.metadata.clone()).collect();
                            engine_c.cache.insert(&query, top_matched_files, results.clone());

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
                    UserEvent::ResizeRequest { height } => {
                        let clamped = height.max(80.0).min(800.0);
                        window.set_inner_size(LogicalSize::new(800.0, clamped));
                    }
                    UserEvent::HideWindow => {
                        window.set_visible(false);
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
    let path_w = windows::core::HSTRING::from(path);
    unsafe {
        let _ = windows::Win32::UI::Shell::ShellExecuteW(
            windows::Win32::Foundation::HWND(std::ptr::null_mut()),
            windows::core::PCWSTR(std::ptr::null()),
            windows::core::PCWSTR(path_w.as_ptr()),
            windows::core::PCWSTR(std::ptr::null()),
            windows::core::PCWSTR(std::ptr::null()),
            windows::Win32::UI::WindowsAndMessaging::SW_SHOW,
        );
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
    let db_path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("search_engine.db");
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

    info!("=================================================================");
}
