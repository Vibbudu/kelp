use crate::config::AppConfig;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub trait SettingsManager: Send + Sync {
    /// Gets the current active configuration
    fn get_config(&self) -> AppConfig;

    /// Updates the current configuration and saves it to disk
    fn update_config(&mut self, new_config: AppConfig) -> Result<(), String>;

    /// Reloads the configuration from the filesystem
    fn reload(&mut self) -> Result<(), String>;
}

pub struct KelpSettings {
    config_path: PathBuf,
    current_config: Arc<RwLock<AppConfig>>,
}

impl KelpSettings {
    pub fn new(config_path: &Path) -> Self {
        let current_config = AppConfig::load_or_create(config_path);
        Self {
            config_path: config_path.to_path_buf(),
            current_config: Arc::new(RwLock::new(current_config)),
        }
    }
}

impl SettingsManager for KelpSettings {
    fn get_config(&self) -> AppConfig {
        let guard = self.current_config.read().unwrap();
        guard.clone()
    }

    fn update_config(&mut self, new_config: AppConfig) -> Result<(), String> {
        let mut guard = self.current_config.write().unwrap();
        
        // Write to filesystem
        if let Ok(content) = serde_json::to_string_pretty(&new_config) {
            std::fs::write(&self.config_path, content)
                .map_err(|e| format!("Failed to write configuration: {}", e))?;
        }
        
        *guard = new_config;
        Ok(())
    }

    fn reload(&mut self) -> Result<(), String> {
        let mut guard = self.current_config.write().unwrap();
        let loaded = AppConfig::load_or_create(&self.config_path);
        *guard = loaded;
        Ok(())
    }
}
