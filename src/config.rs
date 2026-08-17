use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_extensions")]
    pub supported_extensions: Vec<String>,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_search_paths")]
    pub search_paths: Vec<String>,
    #[serde(default = "default_excluded_paths")]
    pub excluded_paths: Vec<String>,
    #[serde(default = "default_true")]
    pub auto_hide_on_blur: bool,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed_ms: usize,
    #[serde(default)]
    pub debug_logging: bool,
    #[serde(default)]
    pub launch_at_startup: bool,
    #[serde(default = "default_true")]
    pub power_failsafe: bool,
    #[serde(default = "default_web_engine")]
    pub default_web_engine: String,
    #[serde(default = "default_web_bang")]
    pub web_bang_prefix: String,
}

fn default_true() -> bool { true }
fn default_hotkey() -> String { "Alt+Space".to_string() }
fn default_theme() -> String { "dark".to_string() }
fn default_max_results() -> usize { 15 }
fn default_animation_speed() -> usize { 150 }
fn default_web_engine() -> String { "google".to_string() }
fn default_web_bang() -> String { "!b".to_string() }
fn default_excluded_paths() -> Vec<String> { Vec::new() }

fn default_extensions() -> Vec<String> {
    let exts = [
        "exe", "lnk", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "md", "rtf",
        "json", "xml", "yaml", "yml", "csv", "png", "jpg", "jpeg", "webp", "gif", "bmp", "ico",
        "mp4", "mkv", "mov", "avi", "mp3", "wav", "flac", "zip", "rar", "7z", "tar", "gz",
        "rs", "cpp", "c", "h", "py", "js", "ts", "java", "cs", "html", "css", "toml",
        "bat", "cmd", "ps1", "msi", "msix"
    ];
    exts.iter().map(|s| s.to_string()).collect()
}

fn default_search_paths() -> Vec<String> {
    vec![
        "%USERPROFILE%\\Desktop".to_string(),
        "%USERPROFILE%\\Documents".to_string(),
        "%USERPROFILE%\\Downloads".to_string(),
        "%USERPROFILE%\\Music".to_string(),
        "%USERPROFILE%\\Pictures".to_string(),
        "%USERPROFILE%\\Videos".to_string(),
        "%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
        "%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
    ]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            supported_extensions: default_extensions(),
            hotkey: default_hotkey(),
            theme: default_theme(),
            max_results: default_max_results(),
            search_paths: default_search_paths(),
            excluded_paths: default_excluded_paths(),
            auto_hide_on_blur: true,
            always_on_top: true,
            animation_speed_ms: default_animation_speed(),
            debug_logging: false,
            launch_at_startup: false,
            power_failsafe: true,
            default_web_engine: default_web_engine(),
            web_bang_prefix: default_web_bang(),
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
        let _ = default_config.save(config_path);
        default_config
    }

    /// Saves configuration to disk safely
    pub fn save(&self, config_path: &Path) -> std::io::Result<()> {
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        fs::write(config_path, json)?;
        info!("Saved configuration to {:?}", config_path);
        Ok(())
    }
}
