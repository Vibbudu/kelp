use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub supported_extensions: Vec<String>,
    pub hotkey: String,
    pub theme: String,
    pub max_results: usize,
    pub search_paths: Vec<String>,
    pub auto_hide_on_blur: bool,
    pub always_on_top: bool,
    pub animation_speed_ms: usize,
    pub debug_logging: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let exts = [
            "exe", "lnk", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "md", "rtf",
            "json", "xml", "yaml", "yml", "csv", "png", "jpg", "jpeg", "webp", "gif", "bmp", "ico",
            "mp4", "mkv", "mov", "avi", "mp3", "wav", "flac", "zip", "rar", "7z", "tar", "gz",
            "rs", "cpp", "c", "h", "py", "js", "ts", "java", "cs", "html", "css", "toml",
            "bat", "cmd", "ps1", "msi", "msix"
        ];
        Self {
            supported_extensions: exts.iter().map(|s| s.to_string()).collect(),
            hotkey: "Alt+Space".to_string(),
            theme: "dark".to_string(),
            max_results: 15,
            search_paths: vec![
                "%USERPROFILE%\\Desktop".to_string(),
                "%USERPROFILE%\\Documents".to_string(),
                "%USERPROFILE%\\Downloads".to_string(),
                "%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
                "%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
            ],
            auto_hide_on_blur: true,
            always_on_top: true,
            animation_speed_ms: 150,
            debug_logging: false,
        }
    }
}

impl AppConfig {
    /// Loads the configuration file, or creates a default one if it doesn't exist.
    pub fn load_or_create(config_path: &Path) -> Self {
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(config_path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                    info!("Loaded configuration from {:?}", config_path);
                    return config;
                }
            }
        }
        
        let default_config = Self::default();
        if let Ok(content) = serde_json::to_string_pretty(&default_config) {
            let _ = fs::write(config_path, content);
            info!("Created default configuration at {:?}", config_path);
        }
        default_config
    }
}
