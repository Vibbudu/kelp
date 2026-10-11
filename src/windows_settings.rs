use crate::models::{FileMetadata, FileType, SearchResult};
use tracing::info;

pub struct SettingEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub uri_or_cmd: &'static str,
    pub icon: &'static str,
    pub keywords: &'static [&'static str],
}

pub const SETTINGS_CATALOG: &[SettingEntry] = &[
    // --- Direct Quick Actions ---
    SettingEntry {
        id: "act-dark-mode",
        name: "Toggle Dark / Light Mode",
        description: "Instantly switch Windows and applications between Dark and Light theme",
        category: "Quick Action",
        uri_or_cmd: "action:toggle_dark_mode",
        icon: "contrast",
        keywords: &["dark mode", "light mode", "theme", "toggle dark", "toggle light", "color mode", "switch theme"],
    },
    SettingEntry {
        id: "act-empty-recycle-bin",
        name: "Empty Recycle Bin",
        description: "Permanently delete all files stored in the Windows Recycle Bin",
        category: "Quick Action",
        uri_or_cmd: "action:empty_recycle_bin",
        icon: "delete_sweep",
        keywords: &["empty recycle bin", "trash", "recycle bin", "clear trash", "delete temp", "free disk"],
    },
    SettingEntry {
        id: "act-lock-pc",
        name: "Lock Workstation",
        description: "Lock your computer immediately",
        category: "Quick Action",
        uri_or_cmd: "action:lock_workstation",
        icon: "lock",
        keywords: &["lock", "lock pc", "lock computer", "lock screen", "workstation"],
    },
    SettingEntry {
        id: "act-mute",
        name: "Mute / Unmute Volume",
        description: "Toggle master audio mute state",
        category: "Quick Action",
        uri_or_cmd: "action:mute_volume",
        icon: "volume_off",
        keywords: &["mute", "unmute", "silence", "audio mute", "sound mute", "quiet"],
    },

    // --- System Settings ---
    SettingEntry {
        id: "set-display",
        name: "Display Settings",
        description: "Configure screen resolution, refresh rate, scaling, and multiple monitors",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:display",
        icon: "monitor",
        keywords: &["display", "resolution", "screen", "monitor", "scaling", "hz", "refresh rate", "4k", "multi monitor", "dual screen"],
    },
    SettingEntry {
        id: "set-sound",
        name: "Sound Settings",
        description: "Select audio output, microphone input, and master volume levels",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:sound",
        icon: "volume_up",
        keywords: &["sound", "audio", "volume", "speakers", "headphones", "mic", "microphone", "input", "output"],
    },
    SettingEntry {
        id: "set-volume-mixer",
        name: "Volume Mixer",
        description: "Adjust volume and audio devices individually per application",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:sound-devices",
        icon: "tune",
        keywords: &["volume mixer", "app volume", "audio mixer", "individual volume", "mixer", "per app sound"],
    },
    SettingEntry {
        id: "set-notifications",
        name: "Notifications & Actions",
        description: "Control notifications, badges, and focus alerts from Windows and apps",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:notifications",
        icon: "notifications",
        keywords: &["notifications", "alerts", "banners", "popups", "dnd", "do not disturb", "notification center"],
    },
    SettingEntry {
        id: "set-focus",
        name: "Focus & Do Not Disturb",
        description: "Silence alerts and reduce distractions with Focus sessions",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:quiethours",
        icon: "do_not_disturb_on",
        keywords: &["focus", "focus assist", "quiet hours", "do not disturb", "dnd", "silence notifications"],
    },
    SettingEntry {
        id: "set-power-sleep",
        name: "Power & Sleep",
        description: "Set screen timeout, sleep schedule, and energy recommendation options",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:powersleep",
        icon: "energy_savings_leaf",
        keywords: &["power", "sleep", "battery", "energy", "screen timeout", "standby", "hibernate", "turn off screen"],
    },
    SettingEntry {
        id: "set-battery",
        name: "Battery & Energy Saver",
        description: "View battery usage per app and adjust Battery Saver thresholds",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:batterysaver",
        icon: "battery_charging_full",
        keywords: &["battery", "battery saver", "battery percentage", "power mode", "energy saver", "drain"],
    },
    SettingEntry {
        id: "set-storage",
        name: "Storage & Storage Sense",
        description: "Manage local disk usage, clean temporary files, and enable Storage Sense",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:storagesense",
        icon: "hard_drive",
        keywords: &["storage", "disk", "free space", "clean disk", "storage sense", "temp files", "drive c", "disk cleanup"],
    },
    SettingEntry {
        id: "set-multitasking",
        name: "Multitasking & Snap",
        description: "Configure Snap Windows, virtual desktops, and Alt+Tab behavior",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:multitasking",
        icon: "view_quilt",
        keywords: &["multitasking", "snap", "snap windows", "split screen", "virtual desktop", "alt tab", "shake"],
    },
    SettingEntry {
        id: "set-about",
        name: "About This PC",
        description: "View device specifications, processor, RAM, and Windows version",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:about",
        icon: "info",
        keywords: &["about", "specs", "system info", "device name", "ram", "cpu", "processor", "windows version", "build"],
    },
    SettingEntry {
        id: "set-activation",
        name: "Activation & Product Key",
        description: "Check Windows activation status and update license key",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:activation",
        icon: "verified_user",
        keywords: &["activation", "license", "product key", "activate windows", "genuine"],
    },

    // --- Bluetooth & Devices ---
    SettingEntry {
        id: "set-bluetooth",
        name: "Bluetooth & Devices",
        description: "Pair and manage Bluetooth accessories, mice, keyboards, and controllers",
        category: "Settings > Devices",
        uri_or_cmd: "ms-settings:bluetooth",
        icon: "bluetooth",
        keywords: &["bluetooth", "pair", "wireless", "headphones", "airpods", "controller", "mouse", "keyboard"],
    },
    SettingEntry {
        id: "set-printers",
        name: "Printers & Scanners",
        description: "Add, view, and troubleshoot printers and scanners",
        category: "Settings > Devices",
        uri_or_cmd: "ms-settings:printers",
        icon: "print",
        keywords: &["printers", "scanners", "print", "fax", "add printer", "hp", "epson", "canon"],
    },
    SettingEntry {
        id: "set-mouse",
        name: "Mouse Settings",
        description: "Configure mouse pointer speed, primary button, and scroll wheel lines",
        category: "Settings > Devices",
        uri_or_cmd: "ms-settings:mousetouchpad",
        icon: "mouse",
        keywords: &["mouse", "cursor", "pointer speed", "scroll speed", "double click", "primary button"],
    },
    SettingEntry {
        id: "set-touchpad",
        name: "Touchpad Settings",
        description: "Customize touchpad gestures, scroll direction, and sensitivity",
        category: "Settings > Devices",
        uri_or_cmd: "ms-settings:devices-touchpad",
        icon: "touch_app",
        keywords: &["touchpad", "trackpad", "gestures", "two finger scroll", "pinch to zoom", "tap to click"],
    },
    SettingEntry {
        id: "set-autoplay",
        name: "AutoPlay Settings",
        description: "Choose what happens when you connect drives and memory cards",
        category: "Settings > Devices",
        uri_or_cmd: "ms-settings:autoplay",
        icon: "play_circle",
        keywords: &["autoplay", "usb drive", "sd card", "removable media", "flash drive"],
    },

    // --- Network & Internet ---
    SettingEntry {
        id: "set-wifi",
        name: "Wi-Fi Settings",
        description: "Connect to wireless networks, view IP address, and manage known Wi-Fi",
        category: "Settings > Network",
        uri_or_cmd: "ms-settings:network-wifi",
        icon: "wifi",
        keywords: &["wifi", "wi-fi", "wireless", "internet", "network", "ssid", "hotspot", "wlan", "connect wifi"],
    },
    SettingEntry {
        id: "set-ethernet",
        name: "Ethernet Settings",
        description: "View wired network properties, DHCP, DNS, and IP address configuration",
        category: "Settings > Network",
        uri_or_cmd: "ms-settings:network-ethernet",
        icon: "lan",
        keywords: &["ethernet", "lan", "cable", "wired", "ip address", "dns", "gateway", "dhcp"],
    },
    SettingEntry {
        id: "set-vpn",
        name: "VPN Settings",
        description: "Set up and manage Virtual Private Network connections",
        category: "Settings > Network",
        uri_or_cmd: "ms-settings:network-vpn",
        icon: "vpn_key",
        keywords: &["vpn", "virtual private network", "tunnel", "proxy vpn", "secure connection"],
    },
    SettingEntry {
        id: "set-mobile-hotspot",
        name: "Mobile Hotspot",
        description: "Share your internet connection with other devices over Wi-Fi",
        category: "Settings > Network",
        uri_or_cmd: "ms-settings:network-mobilehotspot",
        icon: "wifi_tethering",
        keywords: &["hotspot", "mobile hotspot", "tether", "share internet", "wifi sharing"],
    },
    SettingEntry {
        id: "set-airplane-mode",
        name: "Airplane Mode",
        description: "Turn off all wireless communications including Wi-Fi and Bluetooth",
        category: "Settings > Network",
        uri_or_cmd: "ms-settings:network-airplanemode",
        icon: "airplanemode_active",
        keywords: &["airplane mode", "flight mode", "turn off wifi", "disable wireless"],
    },
    SettingEntry {
        id: "set-network-adapters",
        name: "Network Connections (Adapters)",
        description: "Open classic Network Connections control panel to enable or reset adapters",
        category: "Control Panel > Network",
        uri_or_cmd: "control.exe ncpa.cpl",
        icon: "router",
        keywords: &["network connections", "ncpa.cpl", "adapters", "reset adapter", "ipv4", "dns settings"],
    },

    // --- Personalization ---
    SettingEntry {
        id: "set-background",
        name: "Background & Wallpaper",
        description: "Change desktop wallpaper, choose pictures, or configure slideshows",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:personalization-background",
        icon: "wallpaper",
        keywords: &["background", "wallpaper", "desktop picture", "photo", "slideshow", "image"],
    },
    SettingEntry {
        id: "set-colors",
        name: "Colors & Theme Options",
        description: "Pick accent color, enable transparency effects, and choose Dark or Light mode",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:colors",
        icon: "palette",
        keywords: &["colors", "color mode", "accent color", "transparency", "dark mode", "light mode", "theme colors"],
    },
    SettingEntry {
        id: "set-themes",
        name: "Themes",
        description: "Browse and install complete desktop themes, sounds, and cursor packages",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:themes",
        icon: "style",
        keywords: &["themes", "desktop theme", "custom theme", "sounds", "mouse cursors"],
    },
    SettingEntry {
        id: "set-lockscreen",
        name: "Lock Screen Settings",
        description: "Customize lock screen picture, status widgets, and screen timeout",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:lockscreen",
        icon: "lock_clock",
        keywords: &["lock screen", "windows spotlight", "lock picture", "screen timeout", "widgets"],
    },
    SettingEntry {
        id: "set-taskbar",
        name: "Taskbar Settings",
        description: "Configure taskbar alignment, auto-hide, tray overflow icons, and badges",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:taskbar",
        icon: "dock",
        keywords: &["taskbar", "taskbar alignment", "center taskbar", "hide taskbar", "system tray", "corner icons"],
    },
    SettingEntry {
        id: "set-fonts",
        name: "Font Settings",
        description: "View installed system fonts, download new typefaces, and preview glyphs",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:fonts",
        icon: "font_download",
        keywords: &["fonts", "typography", "install fonts", "font preview", "ttf", "otf"],
    },
    SettingEntry {
        id: "set-night-light",
        name: "Night Light",
        description: "Filter blue light in the evening to reduce eye strain and sleep better",
        category: "Settings > Display",
        uri_or_cmd: "ms-settings:nightlight",
        icon: "bedtime",
        keywords: &["night light", "blue light", "warm screen", "flux", "eye strain", "night mode"],
    },

    // --- Apps & Programs ---
    SettingEntry {
        id: "set-installed-apps",
        name: "Installed Apps",
        description: "Search, repair, and uninstall installed applications and software",
        category: "Settings > Apps",
        uri_or_cmd: "ms-settings:appsfeatures",
        icon: "apps",
        keywords: &["installed apps", "uninstall", "remove app", "programs", "add remove programs", "app list"],
    },
    SettingEntry {
        id: "set-default-apps",
        name: "Default Apps",
        description: "Set default web browser, media player, email client, and file associations",
        category: "Settings > Apps",
        uri_or_cmd: "ms-settings:defaultapps",
        icon: "check_circle",
        keywords: &["default apps", "default browser", "default player", "file associations", "open with"],
    },
    SettingEntry {
        id: "set-startup-apps",
        name: "Startup Apps",
        description: "Manage applications that automatically launch when signing in to Windows",
        category: "Settings > Apps",
        uri_or_cmd: "ms-settings:startupapps",
        icon: "power_settings_new",
        keywords: &["startup", "startup apps", "launch on boot", "autostart", "startup impact", "login items"],
    },

    // --- Accounts & Sign-in ---
    SettingEntry {
        id: "set-signin-options",
        name: "Sign-in Options",
        description: "Configure Windows Hello Facial Recognition, Fingerprint, PIN, and Security Key",
        category: "Settings > Accounts",
        uri_or_cmd: "ms-settings:signinoptions",
        icon: "fingerprint",
        keywords: &["sign in", "pin", "windows hello", "fingerprint", "face id", "password", "security key"],
    },

    // --- Time & Language ---
    SettingEntry {
        id: "set-date-time",
        name: "Date & Time",
        description: "Adjust clock, configure automatic time synchronization, and set time zone",
        category: "Settings > Time & Language",
        uri_or_cmd: "ms-settings:dateandtime",
        icon: "schedule",
        keywords: &["date", "time", "clock", "time zone", "sync clock", "daylight savings"],
    },
    SettingEntry {
        id: "set-language",
        name: "Language & Region",
        description: "Add display languages, regional formatting, and keyboard input methods",
        category: "Settings > Time & Language",
        uri_or_cmd: "ms-settings:regionlanguage",
        icon: "language",
        keywords: &["language", "region", "keyboard layout", "input method", "locale", "dictionary"],
    },

    // --- Privacy & Security ---
    SettingEntry {
        id: "set-windows-security",
        name: "Windows Security & Defender",
        description: "View antivirus health, virus scans, and device protection dashboard",
        category: "Settings > Security",
        uri_or_cmd: "ms-settings:windowsdefender",
        icon: "shield",
        keywords: &["windows security", "defender", "antivirus", "virus protection", "scan pc", "malware"],
    },
    SettingEntry {
        id: "set-firewall",
        name: "Windows Defender Firewall",
        description: "Manage firewall rules, app permissions, and network security profiles",
        category: "Control Panel > Security",
        uri_or_cmd: "control.exe firewall.cpl",
        icon: "security",
        keywords: &["firewall", "firewall rules", "network security", "block port", "allow app"],
    },
    SettingEntry {
        id: "set-privacy-camera",
        name: "Camera Privacy Permissions",
        description: "Choose which apps can access your webcam and camera",
        category: "Settings > Privacy",
        uri_or_cmd: "ms-settings:privacy-webcam",
        icon: "videocam",
        keywords: &["camera privacy", "webcam access", "camera permissions", "mic privacy"],
    },
    SettingEntry {
        id: "set-privacy-microphone",
        name: "Microphone Privacy Permissions",
        description: "Control which apps can access your microphone",
        category: "Settings > Privacy",
        uri_or_cmd: "ms-settings:privacy-microphone",
        icon: "mic",
        keywords: &["microphone privacy", "mic access", "mic permissions"],
    },

    // --- Windows Update ---
    SettingEntry {
        id: "set-windows-update",
        name: "Windows Update",
        description: "Check for Windows updates, download patches, and view update history",
        category: "Settings > Update",
        uri_or_cmd: "ms-settings:windowsupdate",
        icon: "system_update",
        keywords: &["windows update", "check for updates", "patches", "update pc", "windows upgrade"],
    },

    // --- System Management & Power Tools ---
    SettingEntry {
        id: "tool-task-manager",
        name: "Task Manager",
        description: "View running processes, performance graphs, CPU/RAM usage, and kill apps",
        category: "Windows Tool",
        uri_or_cmd: "taskmgr.exe",
        icon: "monitoring",
        keywords: &["task manager", "taskmgr", "processes", "kill process", "performance", "cpu", "ram"],
    },
    SettingEntry {
        id: "tool-device-manager",
        name: "Device Manager",
        description: "View hardware devices, update drivers, and troubleshoot components",
        category: "Windows Tool",
        uri_or_cmd: "devmgmt.msc",
        icon: "memory",
        keywords: &["device manager", "devmgmt", "drivers", "hardware", "gpu", "sound card", "usb devices"],
    },
    SettingEntry {
        id: "tool-disk-management",
        name: "Disk Management",
        description: "Manage hard drives, partitions, format drives, and assign drive letters",
        category: "Windows Tool",
        uri_or_cmd: "diskmgmt.msc",
        icon: "storage",
        keywords: &["disk management", "partition", "format", "drive letters", "ssd", "hard drive"],
    },
    SettingEntry {
        id: "tool-services",
        name: "Windows Services",
        description: "Start, stop, and configure background Windows services",
        category: "Windows Tool",
        uri_or_cmd: "services.msc",
        icon: "settings_suggest",
        keywords: &["services", "background services", "start service", "stop service", "daemon"],
    },
    SettingEntry {
        id: "tool-event-viewer",
        name: "Event Viewer",
        description: "View system logs, application crashes, and security audit events",
        category: "Windows Tool",
        uri_or_cmd: "eventvwr.msc",
        icon: "receipt_long",
        keywords: &["event viewer", "logs", "crash logs", "system errors", "diagnostics"],
    },
    SettingEntry {
        id: "tool-env-vars",
        name: "Environment Variables",
        description: "Configure user and system PATH and environment variables",
        category: "System Properties",
        uri_or_cmd: "control.exe sysdm.cpl,,3",
        icon: "code",
        keywords: &["environment variables", "path", "sysdm.cpl", "system variables", "set path", "env"],
    },
    SettingEntry {
        id: "tool-control-panel",
        name: "Classic Control Panel",
        description: "Open the traditional Windows Control Panel all-items interface",
        category: "Windows Tool",
        uri_or_cmd: "control.exe",
        icon: "grid_view",
        keywords: &["control panel", "classic control panel", "cpl"],
    },
];

pub fn search_settings(raw_query: &str) -> Vec<SearchResult> {
    let q = raw_query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }

    let q_words: Vec<&str> = q.split_whitespace().collect();
    let mut matches = Vec::new();

    for entry in SETTINGS_CATALOG {
        let name_lower = entry.name.to_lowercase();
        let cat_lower = entry.category.to_lowercase();
        let desc_lower = entry.description.to_lowercase();

        let mut score: f64 = 0.0;

        // Exact or prefix match on name
        if name_lower == q {
            score = 1.0;
        } else if name_lower.starts_with(&q) {
            score = 0.95;
        } else if name_lower.contains(&q) {
            score = 0.85;
        } else {
            // Check keywords
            let mut keyword_match = false;
            for &kw in entry.keywords {
                if kw == q {
                    score = 0.92;
                    keyword_match = true;
                    break;
                } else if kw.starts_with(&q) {
                    score = 0.82;
                    keyword_match = true;
                    break;
                } else if kw.contains(&q) {
                    score = 0.75;
                    keyword_match = true;
                    break;
                }
            }

            if !keyword_match {
                // Word-based match
                let all_words_match = q_words.iter().all(|qw| {
                    name_lower.contains(qw)
                        || desc_lower.contains(qw)
                        || cat_lower.contains(qw)
                        || entry.keywords.iter().any(|kw| kw.contains(qw))
                });

                if all_words_match {
                    score = 0.65;
                }
            }
        }

        if score > 0.4 {
            matches.push(SearchResult {
                metadata: FileMetadata {
                    id: None,
                    name: entry.name.to_string(),
                    extension: String::new(),
                    parent_folder: entry.category.to_string(),
                    full_path: entry.uri_or_cmd.to_string(),
                    modified_date: 0,
                    size: 0,
                    file_type: FileType::Shortcut,
                },
                score,
                match_type: "WindowsSetting".to_string(),
                icon_base64: None,
            });
        }
    }

    // Sort by score descending
    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    matches.truncate(6);
    matches
}

pub fn execute_setting(action_or_uri: &str) -> Result<(), String> {
    info!("Executing Windows Setting / Action: {}", action_or_uri);

    if action_or_uri.starts_with("action:") {
        let action = action_or_uri.trim_start_matches("action:");
        match action {
            "toggle_dark_mode" => toggle_dark_mode(),
            "empty_recycle_bin" => empty_recycle_bin(),
            "lock_workstation" => lock_workstation(),
            "mute_volume" => toggle_mute_volume(),
            _ => Err(format!("Unknown action: {}", action)),
        }
    } else if action_or_uri.starts_with("ms-settings:") {
        open_uri(action_or_uri)
    } else {
        // Execute as system command or control panel applet
        open_command(action_or_uri)
    }
}

fn open_uri(uri: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::core::PCWSTR;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let uri_wide: Vec<u16> = format!("{}\0", uri).encode_utf16().collect();

        unsafe {
            let res = ShellExecuteW(
                None,
                PCWSTR(op.as_ptr()),
                PCWSTR(uri_wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            if res.0 as usize <= 32 {
                return Err(format!("ShellExecute failed with code {:?}", res.0));
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("ms-settings is only supported on Windows".to_string())
    }
}

fn open_command(cmd: &str) -> Result<(), String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let program = parts[0];
    let args = &parts[1..];

    std::process::Command::new(program)
        .args(args)
        .spawn()
        .map_err(|e| format!("Failed to spawn command '{}': {}", cmd, e))?;

    Ok(())
}

fn toggle_dark_mode() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY_CURRENT_USER,
            KEY_READ, KEY_WRITE, REG_DWORD,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
        };

        let subkey: Vec<u16> =
            "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
                .encode_utf16()
                .collect();
        let app_val: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();
        let sys_val: Vec<u16> = "SystemUsesLightTheme\0".encode_utf16().collect();

        unsafe {
            let mut hkey = windows::Win32::System::Registry::HKEY::default();
            let open_res = RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                0,
                KEY_READ | KEY_WRITE,
                &mut hkey,
            );
            if open_res.is_err() {
                return Err("Failed to open Personalize registry key".to_string());
            }

            // Read current value
            let mut current: u32 = 0;
            let mut size: u32 = 4;
            let mut val_type = REG_DWORD;
            let _ = RegQueryValueExW(
                hkey,
                PCWSTR(app_val.as_ptr()),
                None,
                Some(&mut val_type),
                Some(&mut current as *mut u32 as *mut u8),
                Some(&mut size),
            );

            // Toggle: if currently light (1) -> switch to dark (0); else -> switch to light (1)
            let new_val: u32 = if current == 1 { 0 } else { 1 };

            let _ = RegSetValueExW(
                hkey,
                PCWSTR(app_val.as_ptr()),
                0,
                REG_DWORD,
                Some(std::slice::from_raw_parts(&new_val as *const u32 as *const u8, 4)),
            );

            let _ = RegSetValueExW(
                hkey,
                PCWSTR(sys_val.as_ptr()),
                0,
                REG_DWORD,
                Some(std::slice::from_raw_parts(&new_val as *const u32 as *const u8, 4)),
            );

            let _ = RegCloseKey(hkey);

            // Broadcast WM_SETTINGCHANGE so system and running apps update their theme
            let immerse_param: Vec<u16> = "ImmersiveColorSet\0".encode_utf16().collect();
            let mut dummy_result: usize = 0;
            let _ = SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                WPARAM(0),
                LPARAM(immerse_param.as_ptr() as isize),
                SMTO_ABORTIFHUNG,
                100,
                Some(&mut dummy_result),
            );
        }

        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Dark mode toggle is only supported on Windows".to_string())
    }
}

fn empty_recycle_bin() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::{
            SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
        };

        unsafe {
            let res = SHEmptyRecycleBinW(
                HWND(std::ptr::null_mut()),
                None,
                SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
            );
            if res.is_err() {
                return Err("Failed to empty recycle bin".to_string());
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Recycle bin is only supported on Windows".to_string())
    }
}

fn lock_workstation() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("rundll32.exe")
            .args(["user32.dll,LockWorkStation"])
            .spawn()
            .map_err(|e| format!("Failed to lock workstation: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Lock workstation is only supported on Windows".to_string())
    }
}

fn toggle_mute_volume() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            keybd_event, KEYEVENTF_KEYUP, VK_VOLUME_MUTE,
        };
        unsafe {
            keybd_event(VK_VOLUME_MUTE.0 as u8, 0, Default::default(), 0);
            keybd_event(VK_VOLUME_MUTE.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Mute volume is only supported on Windows".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_settings_sound() {
        let results = search_settings("sound");
        assert!(!results.is_empty(), "Should find Sound Settings");
        assert!(results.iter().any(|r| r.metadata.name.contains("Sound")));
    }

    #[test]
    fn test_search_settings_bluetooth() {
        let results = search_settings("bluetooth");
        assert!(!results.is_empty(), "Should find Bluetooth Settings");
        assert!(results.iter().any(|r| r.metadata.name.contains("Bluetooth")));
    }

    #[test]
    fn test_search_settings_dark_mode() {
        let results = search_settings("dark mode");
        assert!(!results.is_empty(), "Should find Dark Mode toggle");
        assert!(results.iter().any(|r| r.metadata.name.contains("Dark")));
    }

    #[test]
    fn test_search_settings_recycle_bin() {
        let results = search_settings("recycle bin");
        assert!(!results.is_empty(), "Should find Empty Recycle Bin action");
        assert!(results.iter().any(|r| r.metadata.name.contains("Recycle Bin")));
    }
}
