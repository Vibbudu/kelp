use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub enum WatcherEvent {
    CreatedOrModified(PathBuf),
    Deleted(PathBuf),
}

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    /// Starts watching the specified paths recursively, sending events to the provided channel.
    pub fn new(
        paths: &[String],
        tx: UnboundedSender<WatcherEvent>,
        config: crate::config::AppConfig,
    ) -> Result<Self, String> {
        let config_c = config.clone();
        let event_handler = move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    for path in event.paths {
                        // Skip paths that shouldn't be indexed (like hidden or dev folders)
                        if should_exclude_path(&path, &config_c) {
                            continue;
                        }

                        // Determine if file exists to distinguish between create/modify and delete
                        let exists = path.exists();
                        
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                if exists {
                                    let _ = tx.send(WatcherEvent::CreatedOrModified(path));
                                }
                            }
                            EventKind::Remove(_) => {
                                let _ = tx.send(WatcherEvent::Deleted(path));
                            }
                            _ => {
                                // Fallback based on existence
                                if exists {
                                    let _ = tx.send(WatcherEvent::CreatedOrModified(path));
                                } else {
                                    let _ = tx.send(WatcherEvent::Deleted(path));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("File watcher error: {:?}", e);
                }
            }
        };

        let mut watcher = RecommendedWatcher::new(event_handler, Config::default())
            .map_err(|e| format!("Failed to create watcher: {:?}", e))?;

        for raw_path in paths {
            let expanded = crate::utilities::expand_env_vars(raw_path);
            let path = Path::new(&expanded);
            if path.exists() {
                info!("Starting file watch on: {:?}", path);
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    warn!("Failed to watch path {:?}: {:?}", path, e);
                }
            }
        }

        Ok(Self { _watcher: watcher })
    }
}

/// Exclude typical noisy development or hidden system paths (duplicate logic for watcher events)
fn should_exclude_path(path: &Path, config: &crate::config::AppConfig) -> bool {
    let path_str = path.to_string_lossy();
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
    ];

    for excl in &exclusions {
        if path_str.contains(excl) {
            return true;
        }
    }

    let is_dir = path.is_dir();
    if !is_dir {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if !config.supported_extensions.contains(&ext_str) {
                return true;
            }
        } else {
            return true; // Exclude files with no extension
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(metadata) = path.metadata() {
            let attributes = metadata.file_attributes();
            if (attributes & 0x2) != 0 || (attributes & 0x4) != 0 {
                return true;
            }
        }
    }

    false
}
