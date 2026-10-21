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
        id: "act-set-light-mode",
        name: "Switch to Light Mode",
        description: "Change Windows system and application appearance to Light theme",
        category: "Quick Action",
        uri_or_cmd: "action:set_light_mode",
        icon: "light_mode",
        keywords: &[
            "light mode",
            "change theme to light mode",
            "switch to light mode",
            "set light mode",
            "enable light mode",
            "turn on light mode",
            "light theme",
            "change to light theme",
            "day mode",
        ],
    },
    SettingEntry {
        id: "act-set-dark-mode",
        name: "Switch to Dark Mode",
        description: "Change Windows system and application appearance to Dark theme",
        category: "Quick Action",
        uri_or_cmd: "action:set_dark_mode",
        icon: "dark_mode",
        keywords: &[
            "dark mode",
            "change theme to dark mode",
            "switch to dark mode",
            "set dark mode",
            "enable dark mode",
            "turn on dark mode",
            "dark theme",
            "change to dark theme",
            "night theme",
        ],
    },
    SettingEntry {
        id: "act-dark-mode",
        name: "Toggle Dark / Light Mode",
        description: "Invert Windows and applications between Dark and Light appearance",
        category: "Quick Action",
        uri_or_cmd: "action:toggle_dark_mode",
        icon: "contrast",
        keywords: &[
            "toggle dark mode",
            "toggle light mode",
            "toggle theme",
            "switch theme",
            "change theme",
            "color mode",
            "appearance",
        ],
    },
    SettingEntry {
        id: "act-empty-recycle-bin",
        name: "Empty Recycle Bin",
        description: "Permanently delete all files stored in the Windows Recycle Bin",
        category: "Quick Action",
        uri_or_cmd: "action:empty_recycle_bin",
        icon: "delete_sweep",
        keywords: &[
            "empty recycle bin",
            "empty trash",
            "clear recycle bin",
            "clear trash",
            "recycle bin",
            "trash",
            "clean trash",
        ],
    },
    SettingEntry {
        id: "act-open-recycle-bin",
        name: "Open Recycle Bin",
        description: "View deleted files inside the Windows Recycle Bin folder",
        category: "Quick Action",
        uri_or_cmd: "shell:RecycleBinFolder",
        icon: "recycling",
        keywords: &[
            "open recycle bin",
            "view recycle bin",
            "recycle bin folder",
            "restore deleted files",
        ],
    },
    SettingEntry {
        id: "act-lock-pc",
        name: "Lock Workstation",
        description: "Lock your computer screen immediately",
        category: "Quick Action",
        uri_or_cmd: "action:lock_workstation",
        icon: "lock",
        keywords: &["lock", "lock pc", "lock computer", "lock screen", "workstation", "lock windows"],
    },
    SettingEntry {
        id: "act-mute",
        name: "Mute / Unmute Volume",
        description: "Toggle system master audio mute state",
        category: "Quick Action",
        uri_or_cmd: "action:mute_volume",
        icon: "volume_off",
        keywords: &["mute", "unmute", "silence", "audio mute", "sound mute", "mute volume", "toggle mute"],
    },
    SettingEntry {
        id: "act-volume-up",
        name: "Volume Up",
        description: "Increase system master audio volume level",
        category: "Quick Action",
        uri_or_cmd: "action:volume_up",
        icon: "volume_up",
        keywords: &["volume up", "increase volume", "louder", "raise volume", "sound up"],
    },
    SettingEntry {
        id: "act-volume-down",
        name: "Volume Down",
        description: "Decrease system master audio volume level",
        category: "Quick Action",
        uri_or_cmd: "action:volume_down",
        icon: "volume_down",
        keywords: &["volume down", "decrease volume", "quieter", "lower volume", "sound down"],
    },
    SettingEntry {
        id: "act-snipping-tool",
        name: "Take Screenshot (Snipping Tool)",
        description: "Capture a region, window, or full-screen screenshot using Windows Snipping overlay",
        category: "Quick Action",
        uri_or_cmd: "ms-screenclip:",
        icon: "screenshot_monitor",
        keywords: &[
            "screenshot",
            "take screenshot",
            "snip",
            "snipping tool",
            "screen capture",
            "screen clip",
            "capture screen",
        ],
    },
    SettingEntry {
        id: "act-flush-dns",
        name: "Flush DNS Cache",
        description: "Clear and reset the Windows DNS resolver cache (ipconfig /flushdns)",
        category: "Quick Action",
        uri_or_cmd: "action:flush_dns",
        icon: "dns",
        keywords: &["flush dns", "clear dns", "reset dns", "ipconfig flushdns", "dns cache"],
    },
    SettingEntry {
        id: "act-clear-clipboard",
        name: "Clear Clipboard History",
        description: "Clear Kelp's in-memory clipboard history items",
        category: "Quick Action",
        uri_or_cmd: "action:clear_clipboard_history",
        icon: "content_paste_off",
        keywords: &["clear clipboard", "clear clipboard history", "wipe clipboard", "empty clipboard"],
    },

    // --- System Settings ---
    SettingEntry {
        id: "set-display",
        name: "Display Settings",
        description: "Configure screen resolution, refresh rate, scaling, and multiple monitors",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:display",
        icon: "monitor",
        keywords: &["display", "display settings", "resolution", "screen", "monitor", "scaling", "hz", "refresh rate", "4k", "multi monitor", "dual screen"],
    },
    SettingEntry {
        id: "set-sound",
        name: "Sound Settings",
        description: "Select audio output, microphone input, and master volume levels",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:sound",
        icon: "volume_up",
        keywords: &["sound", "sound settings", "audio", "audio settings", "speakers", "headphones", "mic", "microphone", "input", "output"],
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
        keywords: &["notifications", "notification settings", "alerts", "banners", "popups", "dnd", "do not disturb", "notification center"],
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
        keywords: &["power", "power settings", "sleep settings", "energy", "screen timeout", "standby", "hibernate", "turn off screen"],
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
        keywords: &["storage", "storage settings", "disk space", "free space", "clean disk", "storage sense", "temp files", "drive c", "disk cleanup"],
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
        id: "set-clipboard",
        name: "Clipboard Settings",
        description: "Enable Windows Clipboard History (Win+V), sync across devices, or clear clipboard",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:clipboard",
        icon: "content_paste",
        keywords: &["clipboard settings", "win v", "clipboard sync", "paste history"],
    },
    SettingEntry {
        id: "set-remote-desktop",
        name: "Remote Desktop",
        description: "Allow remote connections to this PC via Remote Desktop Protocol (RDP)",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:remotedesktop",
        icon: "desktop_windows",
        keywords: &["remote desktop", "rdp", "remote connection", "mstsc"],
    },
    SettingEntry {
        id: "set-about",
        name: "About This PC",
        description: "View device specifications, processor, RAM, and Windows version",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:about",
        icon: "info",
        keywords: &["about", "about pc", "specs", "system info", "device name", "ram", "cpu", "processor", "windows version", "build"],
    },
    SettingEntry {
        id: "set-developers",
        name: "For Developers (Developer Mode)",
        description: "Configure Developer Mode, PowerShell execution policy, and File Explorer dev options",
        category: "Settings > System",
        uri_or_cmd: "ms-settings:developers",
        icon: "terminal",
        keywords: &["developer", "developer mode", "for developers", "dev settings", "sideload"],
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
        keywords: &["bluetooth", "bluetooth settings", "pair", "wireless", "headphones", "airpods", "controller", "connect bluetooth"],
    },
    SettingEntry {
        id: "set-printers",
        name: "Printers & Scanners",
        description: "Add, view, and troubleshoot printers and scanners",
        category: "Settings > Devices",
        uri_or_cmd: "ms-settings:printers",
        icon: "print",
        keywords: &["printers", "scanners", "print", "fax", "add printer", "printer settings"],
    },
    SettingEntry {
        id: "set-mouse",
        name: "Mouse Settings",
        description: "Configure mouse pointer speed, primary button, and scroll wheel lines",
        category: "Settings > Devices",
        uri_or_cmd: "ms-settings:mousetouchpad",
        icon: "mouse",
        keywords: &["mouse", "mouse settings", "cursor", "pointer speed", "scroll speed", "double click", "primary button", "dpi"],
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
        id: "set-typing",
        name: "Typing & Keyboard Settings",
        description: "Configure autocorrect, text suggestions, hardware keyboard, and multilingual typing",
        category: "Settings > Devices",
        uri_or_cmd: "ms-settings:typing",
        icon: "keyboard",
        keywords: &["typing", "keyboard settings", "autocorrect", "spell check", "text suggestions"],
    },
    SettingEntry {
        id: "set-camera",
        name: "Camera Settings",
        description: "Manage connected webcams, brightness, contrast, and video rotation",
        category: "Settings > Devices",
        uri_or_cmd: "ms-settings:camera",
        icon: "photo_camera",
        keywords: &["camera", "camera settings", "webcam", "video device"],
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
        keywords: &["wifi", "wi-fi", "wifi settings", "wireless", "internet", "network", "ssid", "wlan", "connect wifi"],
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
        keywords: &["vpn", "vpn settings", "virtual private network", "tunnel", "secure connection"],
    },
    SettingEntry {
        id: "set-proxy",
        name: "Proxy Settings",
        description: "Configure automatic or manual HTTP/SOCKS proxy server settings",
        category: "Settings > Network",
        uri_or_cmd: "ms-settings:network-proxy",
        icon: "public",
        keywords: &["proxy", "proxy settings", "http proxy", "socks", "pac script"],
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
        keywords: &["network connections", "ncpa.cpl", "adapters", "network adapters", "reset adapter", "ipv4", "dns settings"],
    },

    // --- Personalization ---
    SettingEntry {
        id: "set-background",
        name: "Background & Wallpaper",
        description: "Change desktop wallpaper, choose pictures, or configure slideshows",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:personalization-background",
        icon: "wallpaper",
        keywords: &["background", "wallpaper", "change wallpaper", "desktop background", "desktop picture", "slideshow"],
    },
    SettingEntry {
        id: "set-colors",
        name: "Colors & Accent Settings",
        description: "Pick accent color, enable transparency effects, and customize color modes",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:colors",
        icon: "palette",
        keywords: &["colors", "accent color", "transparency", "mica", "acrylic", "theme colors"],
    },
    SettingEntry {
        id: "set-themes",
        name: "Themes",
        description: "Browse and install complete desktop themes, sounds, and cursor packages",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:themes",
        icon: "style",
        keywords: &["themes", "desktop theme", "custom theme", "mouse cursors"],
    },
    SettingEntry {
        id: "set-lockscreen",
        name: "Lock Screen Settings",
        description: "Customize lock screen picture, status widgets, and screen timeout",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:lockscreen",
        icon: "lock_clock",
        keywords: &["lock screen settings", "windows spotlight", "lock picture", "widgets"],
    },
    SettingEntry {
        id: "set-taskbar",
        name: "Taskbar Settings",
        description: "Configure taskbar alignment, auto-hide, tray overflow icons, and badges",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:taskbar",
        icon: "dock",
        keywords: &["taskbar", "taskbar settings", "taskbar alignment", "center taskbar", "hide taskbar", "system tray", "corner icons"],
    },
    SettingEntry {
        id: "set-start-menu",
        name: "Start Menu Settings",
        description: "Customize Start menu layout, pinned apps, and recommended recent files",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:personalization-start",
        icon: "apps",
        keywords: &["start menu", "start settings", "pinned apps", "recent files start"],
    },
    SettingEntry {
        id: "set-fonts",
        name: "Font Settings",
        description: "View installed system fonts, download new typefaces, and preview glyphs",
        category: "Settings > Personalization",
        uri_or_cmd: "ms-settings:fonts",
        icon: "font_download",
        keywords: &["fonts", "font settings", "typography", "install fonts", "font preview", "ttf", "otf"],
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
        name: "Installed Apps (Add or Remove Programs)",
        description: "Search, repair, and uninstall installed applications and software",
        category: "Settings > Apps",
        uri_or_cmd: "ms-settings:appsfeatures",
        icon: "apps",
        keywords: &["installed apps", "uninstall", "uninstall app", "remove app", "programs", "add remove programs", "app list"],
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
        name: "Sign-in Options (PIN & Windows Hello)",
        description: "Configure Windows Hello Facial Recognition, Fingerprint, PIN, and Password",
        category: "Settings > Accounts",
        uri_or_cmd: "ms-settings:signinoptions",
        icon: "fingerprint",
        keywords: &["sign in", "sign-in options", "pin", "windows hello", "fingerprint", "face id", "change password", "security key"],
    },
    SettingEntry {
        id: "set-user-accounts",
        name: "Your Account Info",
        description: "Manage your Microsoft or local user account profile picture and settings",
        category: "Settings > Accounts",
        uri_or_cmd: "ms-settings:yourinfo",
        icon: "person",
        keywords: &["account", "user account", "profile picture", "microsoft account", "your info"],
    },

    // --- Time & Language ---
    SettingEntry {
        id: "set-date-time",
        name: "Date & Time Settings",
        description: "Adjust clock, configure automatic time synchronization, and set time zone",
        category: "Settings > Time & Language",
        uri_or_cmd: "ms-settings:dateandtime",
        icon: "schedule",
        keywords: &["date", "time", "date and time", "clock", "time zone", "sync clock", "daylight savings"],
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

    // --- Gaming & Accessibility ---
    SettingEntry {
        id: "set-game-mode",
        name: "Game Mode Settings",
        description: "Optimize your PC for play by prioritizing game performance in Windows",
        category: "Settings > Gaming",
        uri_or_cmd: "ms-settings:gaming-gamemode",
        icon: "sports_esports",
        keywords: &["game mode", "gaming", "fps boost", "xbox game bar"],
    },
    SettingEntry {
        id: "set-accessibility",
        name: "Accessibility & Text Size",
        description: "Adjust text size, visual effects, magnifier, contrast themes, and captions",
        category: "Settings > Accessibility",
        uri_or_cmd: "ms-settings:easeofaccess-display",
        icon: "accessibility_new",
        keywords: &["accessibility", "ease of access", "text size", "magnifier", "high contrast", "screen reader"],
    },

    // --- Privacy & Security ---
    SettingEntry {
        id: "set-windows-security",
        name: "Windows Security & Defender",
        description: "View antivirus health, virus scans, and device protection dashboard",
        category: "Settings > Security",
        uri_or_cmd: "ms-settings:windowsdefender",
        icon: "shield",
        keywords: &["windows security", "defender", "windows defender", "antivirus", "virus protection", "scan pc", "malware"],
    },
    SettingEntry {
        id: "set-firewall",
        name: "Windows Defender Firewall",
        description: "Manage firewall rules, app permissions, and network security profiles",
        category: "Control Panel > Security",
        uri_or_cmd: "control.exe firewall.cpl",
        icon: "security",
        keywords: &["firewall", "firewall rules", "network security", "block port", "allow app", "firewall.cpl"],
    },
    SettingEntry {
        id: "set-privacy-camera",
        name: "Camera Privacy Permissions",
        description: "Choose which apps can access your webcam and camera",
        category: "Settings > Privacy",
        uri_or_cmd: "ms-settings:privacy-webcam",
        icon: "videocam",
        keywords: &["camera privacy", "webcam access", "camera permissions"],
    },
    SettingEntry {
        id: "set-privacy-microphone",
        name: "Microphone Privacy Permissions",
        description: "Control which apps can access your microphone",
        category: "Settings > Privacy",
        uri_or_cmd: "ms-settings:privacy-microphone",
        icon: "mic",
        keywords: &["microphone privacy", "mic access", "mic permissions", "microphone permissions"],
    },

    // --- Windows Update ---
    SettingEntry {
        id: "set-windows-update",
        name: "Windows Update",
        description: "Check for Windows updates, download patches, and view update history",
        category: "Settings > Update",
        uri_or_cmd: "ms-settings:windowsupdate",
        icon: "system_update",
        keywords: &["windows update", "check for updates", "updates", "patches", "update pc", "windows upgrade"],
    },

    // --- System Management & Power Tools ---
    SettingEntry {
        id: "tool-task-manager",
        name: "Task Manager",
        description: "View running processes, performance graphs, CPU/RAM usage, and kill apps",
        category: "Windows Tool",
        uri_or_cmd: "taskmgr.exe",
        icon: "monitoring",
        keywords: &["task manager", "taskmgr", "processes", "performance", "cpu usage", "ram usage"],
    },
    SettingEntry {
        id: "tool-resource-monitor",
        name: "Resource Monitor",
        description: "Inspect real-time CPU, memory, disk I/O, and network activity per process",
        category: "Windows Tool",
        uri_or_cmd: "resmon.exe",
        icon: "query_stats",
        keywords: &["resource monitor", "resmon", "network monitor", "disk io"],
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
        keywords: &["disk management", "diskmgmt", "partition", "format drive", "drive letters", "ssd", "hard drive"],
    },
    SettingEntry {
        id: "tool-services",
        name: "Windows Services",
        description: "Start, stop, and configure background Windows services",
        category: "Windows Tool",
        uri_or_cmd: "services.msc",
        icon: "settings_suggest",
        keywords: &["services", "services.msc", "background services", "start service", "stop service", "daemon"],
    },
    SettingEntry {
        id: "tool-event-viewer",
        name: "Event Viewer",
        description: "View system logs, application crashes, and security audit events",
        category: "Windows Tool",
        uri_or_cmd: "eventvwr.msc",
        icon: "receipt_long",
        keywords: &["event viewer", "eventvwr", "system logs", "crash logs", "system errors", "diagnostics"],
    },
    SettingEntry {
        id: "tool-regedit",
        name: "Registry Editor",
        description: "View and edit the Windows system registry hierarchy (regedit)",
        category: "Windows Tool",
        uri_or_cmd: "regedit.exe",
        icon: "account_tree",
        keywords: &["registry", "registry editor", "regedit", "hkey"],
    },
    SettingEntry {
        id: "tool-sysinfo",
        name: "System Information (msinfo32)",
        description: "View detailed hardware resources, BIOS version, motherboard, and software environment",
        category: "Windows Tool",
        uri_or_cmd: "msinfo32.exe",
        icon: "computer",
        keywords: &["system information", "msinfo32", "bios", "motherboard", "hardware info"],
    },
    SettingEntry {
        id: "tool-dxdiag",
        name: "DirectX Diagnostic Tool (dxdiag)",
        description: "Inspect GPU display drivers, DirectX features, and audio hardware diagnostics",
        category: "Windows Tool",
        uri_or_cmd: "dxdiag.exe",
        icon: "videogame_asset",
        keywords: &["dxdiag", "directx", "gpu info", "graphics card", "vram"],
    },
    SettingEntry {
        id: "tool-disk-cleanup",
        name: "Disk Cleanup",
        description: "Free up disk space by removing temporary files, caches, and system logs",
        category: "Windows Tool",
        uri_or_cmd: "cleanmgr.exe",
        icon: "cleaning_services",
        keywords: &["disk cleanup", "cleanmgr", "free up space", "clean temp files"],
    },
    SettingEntry {
        id: "tool-env-vars",
        name: "Environment Variables",
        description: "Edit user and system PATH and environment variables directly",
        category: "System Properties",
        uri_or_cmd: "rundll32.exe sysdm.cpl,EditEnvironmentVariables",
        icon: "code",
        keywords: &[
            "environment variables",
            "edit environment variables",
            "path",
            "edit path",
            "system variables",
            "env vars",
            "env",
            "sysdm.cpl",
        ],
    },
    SettingEntry {
        id: "tool-programs-cpl",
        name: "Programs and Features (Control Panel)",
        description: "Open classic Control Panel uninstaller and Windows Features dialog (appwiz.cpl)",
        category: "Control Panel",
        uri_or_cmd: "control.exe appwiz.cpl",
        icon: "inventory_2",
        keywords: &["appwiz.cpl", "programs and features", "classic uninstall", "turn windows features on or off"],
    },
    SettingEntry {
        id: "tool-sound-cpl",
        name: "Sound Control Panel (Classic)",
        description: "Configure playback and recording devices, levels, and enhancements (mmsys.cpl)",
        category: "Control Panel",
        uri_or_cmd: "control.exe mmsys.cpl",
        icon: "speaker",
        keywords: &["mmsys.cpl", "sound control panel", "recording devices", "playback devices"],
    },
    SettingEntry {
        id: "tool-startup-folder",
        name: "Startup Folder (shell:startup)",
        description: "Open the user Startup folder in File Explorer to add or remove login shortcuts",
        category: "Windows Tool",
        uri_or_cmd: "shell:startup",
        icon: "folder_special",
        keywords: &["shell:startup", "startup folder", "open startup folder"],
    },
    SettingEntry {
        id: "tool-control-panel",
        name: "Classic Control Panel",
        description: "Open the traditional Windows Control Panel interface",
        category: "Windows Tool",
        uri_or_cmd: "control.exe",
        icon: "grid_view",
        keywords: &["control panel", "classic control panel", "cpl"],
    },
];

/// Conversational filler words stripped when evaluating natural language queries
/// such as "change theme to light mode" or "open bluetooth settings".
fn is_conversational_stop_word(word: &str) -> bool {
    matches!(
        word,
        "change"
            | "switch"
            | "turn"
            | "set"
            | "to"
            | "on"
            | "off"
            | "the"
            | "my"
            | "open"
            | "show"
            | "go"
            | "in"
            | "for"
            | "into"
            | "windows"
            | "pc"
            | "computer"
            | "system"
            | "please"
            | "enable"
            | "disable"
            | "launch"
            | "configure"
            | "adjust"
            | "manage"
            | "view"
            | "check"
            | "make"
            | "use"
    )
}

pub fn search_settings(raw_query: &str) -> Vec<SearchResult> {
    let q = raw_query.trim().to_lowercase();
    if q.len() < 2 {
        return Vec::new();
    }

    let q_words: Vec<&str> = q.split_whitespace().collect();
    let core_words: Vec<&str> = q_words
        .iter()
        .copied()
        .filter(|w| !is_conversational_stop_word(w))
        .collect();
    let core_query = core_words.join(" ");

    let mut matches = Vec::new();

    for entry in SETTINGS_CATALOG {
        let name_lower = entry.name.to_lowercase();
        let cat_lower = entry.category.to_lowercase();
        let desc_lower = entry.description.to_lowercase();

        let mut score: f64 = 0.0;

        // 1. Exact or prefix match on entry name
        if name_lower == q || (!core_query.is_empty() && name_lower == core_query) {
            score = 1.0;
        } else if name_lower.starts_with(&q) || (core_query.len() >= 4 && name_lower.starts_with(&core_query)) {
            score = if q.len() >= 4 { 0.90 } else { 0.78 };
        } else if q.len() >= 3 && name_lower.contains(&q) {
            score = 0.84;
        }

        // 2. Check keywords (exact, prefix, substring, or query containing a multi-word keyword)
        for &kw in entry.keywords {
            if kw == q {
                score = score.max(0.98);
                break;
            } else if !core_query.is_empty() && kw == core_query {
                score = score.max(0.95);
            } else if kw.starts_with(&q) {
                score = score.max(if q.len() >= 4 { 0.86 } else { 0.75 });
            } else if q.len() >= 4 && kw.contains(&q) {
                score = score.max(0.78);
            } else if kw.len() >= 5 && q.contains(kw) {
                // Natural language query contains a specific keyword phrase (e.g. "change theme to light mode" contains "light mode")
                let all_core_compatible = core_words.iter().all(|&cw| {
                    name_lower.contains(cw)
                        || desc_lower.contains(cw)
                        || cat_lower.contains(cw)
                        || entry.keywords.iter().any(|k| k.contains(cw))
                });
                if all_core_compatible {
                    score = score.max(0.94);
                } else {
                    score = score.max(0.76);
                }
            }
        }

        // 3. Word-level matching across name, description, category, and keywords
        if score < 0.75 {
            let words_to_check = if !core_words.is_empty() {
                &core_words
            } else {
                &q_words
            };

            if !words_to_check.is_empty() {
                let all_words_match = words_to_check.iter().all(|&qw| {
                    qw.len() >= 2
                        && (name_lower.contains(qw)
                            || desc_lower.contains(qw)
                            || cat_lower.contains(qw)
                            || entry.keywords.iter().any(|kw| kw.contains(qw)))
                });

                if all_words_match {
                    let name_hits = words_to_check
                        .iter()
                        .filter(|&&qw| name_lower.contains(qw) || entry.keywords.iter().any(|kw| kw.contains(qw)))
                        .count();
                    let ratio = name_hits as f64 / words_to_check.len() as f64;
                    score = score.max(0.65 + 0.20 * ratio);
                }
            }
        }

        // 4. Disambiguate explicit Light Mode vs Dark Mode vs Toggle (including negated queries like "turn off dark mode")
        let has_light = q_words.contains(&"light");
        let has_dark = q_words.contains(&"dark");
        let has_toggle = q_words.contains(&"toggle");
        let has_negation = q_words.contains(&"off") || q_words.contains(&"disable") || q_words.contains(&"exit");
        let has_theme_context = q_words.contains(&"mode") || q_words.contains(&"theme") || score > 0.5;

        let wants_light = !has_toggle
            && ((has_light && !has_dark && !has_negation) || (has_dark && !has_light && has_negation));
        let wants_dark = !has_toggle
            && ((has_dark && !has_light && !has_negation) || (has_light && !has_dark && has_negation));

        if entry.id == "act-set-light-mode" {
            if wants_light && has_theme_context {
                score = score.max(0.97);
            } else if wants_dark {
                score = score.min(0.60);
            }
        } else if entry.id == "act-set-dark-mode" {
            if wants_dark && has_theme_context {
                score = score.max(0.97);
            } else if wants_light {
                score = score.min(0.60);
            }
        } else if entry.id == "act-dark-mode" && !has_toggle && (has_light ^ has_dark) && score > 0.85 {
            // Keep Toggle right below the explicit Switch to Light/Dark Mode action
            score = 0.89;
        }

        if score > 0.4 {
            matches.push(SearchResult {
                metadata: FileMetadata {
                    id: None,
                    name: entry.name.to_string(),
                    // Store the Material Symbol icon name in `extension` so the UI renders the exact icon
                    extension: entry.icon.to_string(),
                    parent_folder: format!("{} — {}", entry.category, entry.description),
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

    // Sort by score descending, then name
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.metadata.name.cmp(&b.metadata.name))
    });
    matches.truncate(6);
    matches
}

pub fn execute_setting(action_or_uri: &str) -> Result<Option<String>, String> {
    info!("Executing Windows Setting / Action: {}", action_or_uri);

    if let Some(action) = action_or_uri.strip_prefix("action:") {
        match action {
            "set_light_mode" => set_windows_theme(Some(true)).map(Some),
            "set_dark_mode" => set_windows_theme(Some(false)).map(Some),
            "toggle_dark_mode" => set_windows_theme(None).map(Some),
            "empty_recycle_bin" => empty_recycle_bin().map(|_| Some("Recycle Bin emptied".to_string())),
            "lock_workstation" => lock_workstation().map(|_| None),
            "mute_volume" => send_volume_key(0).map(|_| Some("Toggled audio mute".to_string())),
            "volume_up" => send_volume_key(1).map(|_| Some("Increased system volume".to_string())),
            "volume_down" => send_volume_key(-1).map(|_| Some("Decreased system volume".to_string())),
            "flush_dns" => flush_dns_cache().map(|_| Some("Windows DNS cache flushed".to_string())),
            "clear_clipboard_history" => {
                crate::clipboard::clear_clipboard_history();
                Ok(Some("Clipboard history cleared".to_string()))
            }
            _ => Err(format!("Unknown action: {}", action)),
        }
    } else if action_or_uri.starts_with("ms-settings:")
        || action_or_uri.starts_with("ms-screenclip:")
        || action_or_uri.starts_with("shell:")
    {
        open_uri(action_or_uri).map(|_| None)
    } else {
        open_command(action_or_uri).map(|_| None)
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
                return Err(format!("ShellExecute failed for '{}' with code {:?}", uri, res.0));
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = uri;
        Err("URI execution is only supported on Windows".to_string())
    }
}

fn open_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return Err("Empty command".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use windows::core::PCWSTR;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        // Split program and optional arguments so ShellExecuteW handles .exe, .msc, and .cpl reliably
        let (program, args) = match trimmed.split_once(char::is_whitespace) {
            Some((prog, rest)) => (prog.trim(), Some(rest.trim())),
            None => (trimmed, None),
        };

        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let prog_wide: Vec<u16> = format!("{}\0", program).encode_utf16().collect();
        let args_wide: Option<Vec<u16>> = args.map(|a| format!("{}\0", a).encode_utf16().collect());

        unsafe {
            let res = ShellExecuteW(
                None,
                PCWSTR(op.as_ptr()),
                PCWSTR(prog_wide.as_ptr()),
                args_wide.as_ref().map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr())),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            if res.0 as usize <= 32 {
                return Err(format!("ShellExecuteW failed for '{}' with code {:?}", cmd, res.0));
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        std::process::Command::new(parts[0])
            .args(&parts[1..])
            .spawn()
            .map_err(|e| format!("Failed to spawn command '{}': {}", cmd, e))?;
        Ok(())
    }
}

/// Sets or toggles Windows Light/Dark theme (`Some(true)` = Light, `Some(false)` = Dark, `None` = Toggle).
fn set_windows_theme(light_mode: Option<bool>) -> Result<String, String> {
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

            let new_val: u32 = match light_mode {
                Some(true) => 1,
                Some(false) => 0,
                None => {
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
                    if current == 1 { 0 } else { 1 }
                }
            };

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

            // Broadcast WM_SETTINGCHANGE so Windows shell and running apps update their theme
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

            let mode_label = if new_val == 1 { "Light Mode" } else { "Dark Mode" };
            Ok(format!("Switched Windows to {}", mode_label))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = light_mode;
        Err("Theme switching is only supported on Windows".to_string())
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
            let _ = SHEmptyRecycleBinW(
                HWND(std::ptr::null_mut()),
                None,
                SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
            );
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

/// Sends media volume virtual keys (`0` = mute toggle, `1` = volume up, `-1` = volume down).
fn send_volume_key(direction: i8) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            keybd_event, KEYEVENTF_KEYUP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
        };
        unsafe {
            let vk = if direction > 0 {
                VK_VOLUME_UP.0 as u8
            } else if direction < 0 {
                VK_VOLUME_DOWN.0 as u8
            } else {
                VK_VOLUME_MUTE.0 as u8
            };
            let steps = if direction == 0 { 1 } else { 2 };
            for _ in 0..steps {
                keybd_event(vk, 0, Default::default(), 0);
                keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = direction;
        Err("Volume control is only supported on Windows".to_string())
    }
}

fn flush_dns_cache() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("ipconfig")
            .arg("/flushdns")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to flush DNS: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Flush DNS is only supported on Windows".to_string())
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
        assert!(!results.is_empty(), "Should find Dark Mode action");
        assert_eq!(results[0].metadata.full_path, "action:set_dark_mode");
    }

    #[test]
    fn test_search_settings_change_theme_to_light_mode() {
        let results = search_settings("change theme to light mode");
        assert!(
            !results.is_empty(),
            "Should find Switch to Light Mode for natural language query"
        );
        assert_eq!(results[0].metadata.full_path, "action:set_light_mode");
        assert_eq!(results[0].metadata.name, "Switch to Light Mode");
    }

    #[test]
    fn test_search_settings_negated_theme_queries() {
        let r_off_dark = search_settings("turn off dark mode");
        assert!(!r_off_dark.is_empty());
        assert_eq!(r_off_dark[0].metadata.full_path, "action:set_light_mode");

        let r_disable_dark = search_settings("disable dark mode");
        assert!(!r_disable_dark.is_empty());
        assert_eq!(r_disable_dark[0].metadata.full_path, "action:set_light_mode");

        let r_off_light = search_settings("turn off light mode");
        assert!(!r_off_light.is_empty());
        assert_eq!(r_off_light[0].metadata.full_path, "action:set_dark_mode");
    }

    #[test]
    fn test_search_settings_natural_language_queries() {
        let r1 = search_settings("switch to dark mode");
        assert!(!r1.is_empty());
        assert_eq!(r1[0].metadata.full_path, "action:set_dark_mode");

        let r2 = search_settings("open bluetooth settings");
        assert!(!r2.is_empty());
        assert_eq!(r2[0].metadata.full_path, "ms-settings:bluetooth");

        let r3 = search_settings("edit environment variables");
        assert!(!r3.is_empty());
        assert!(r3[0].metadata.name.contains("Environment Variables"));

        let r4 = search_settings("check for windows updates");
        assert!(!r4.is_empty());
        assert_eq!(r4[0].metadata.full_path, "ms-settings:windowsupdate");
    }

    #[test]
    fn test_search_settings_recycle_bin() {
        let results = search_settings("recycle bin");
        assert!(!results.is_empty(), "Should find Empty Recycle Bin action");
        assert!(results.iter().any(|r| r.metadata.name.contains("Recycle Bin")));
    }
}
