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

/// Exclude typical noisy development or hidden system paths (delegates to shared utility)
fn should_exclude_path(path: &Path, config: &crate::config::AppConfig) -> bool {
    crate::utilities::should_exclude_path(path, &config.supported_extensions)
}
