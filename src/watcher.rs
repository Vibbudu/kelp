use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub enum WatcherEvent {
    CreatedOrModified(PathBuf),
    Deleted(PathBuf),
    BatchUpdated {
        created_or_modified: Vec<PathBuf>,
        deleted: Vec<PathBuf>,
    },
}

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    /// Starts watching the specified paths recursively, debouncing events across a 500ms sliding window.
    pub fn new(
        paths: &[String],
        tx: UnboundedSender<WatcherEvent>,
        config: crate::config::AppConfig,
    ) -> Result<Self, String> {
        let (raw_tx, raw_rx) = std_mpsc::channel::<(PathBuf, bool)>(); // bool: true = delete, false = create/modify
        let config_c = config.clone();

        // 1. Raw notify event handler - filters exclusions and feeds the debouncer channel
        let event_handler = move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    for path in event.paths {
                        if should_exclude_path(&path, &config_c) {
                            continue;
                        }
                        let exists = path.exists();
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                if exists {
                                    let _ = raw_tx.send((path, false));
                                }
                            }
                            EventKind::Remove(_) => {
                                let _ = raw_tx.send((path, true));
                            }
                            _ => {
                                if exists {
                                    let _ = raw_tx.send((path, false));
                                } else {
                                    let _ = raw_tx.send((path, true));
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

        // 2. Sliding 500ms debounce worker thread
        std::thread::spawn(move || {
            let debounce_window = Duration::from_millis(500);
            let max_burst_window = Duration::from_millis(2000);

            while let Ok(first_event) = raw_rx.recv() {
                let mut created_modified: HashSet<PathBuf> = HashSet::new();
                let mut deleted: HashSet<PathBuf> = HashSet::new();

                let record_event = |created: &mut HashSet<PathBuf>, del: &mut HashSet<PathBuf>, path: PathBuf, is_del: bool| {
                    if is_del {
                        created.remove(&path);
                        del.insert(path);
                    } else {
                        del.remove(&path);
                        created.insert(path);
                    }
                };

                record_event(&mut created_modified, &mut deleted, first_event.0, first_event.1);

                let burst_start = Instant::now();
                let mut last_event_time = Instant::now();

                // Drain events within sliding debounce window
                loop {
                    let now = Instant::now();
                    if now.duration_since(burst_start) >= max_burst_window {
                        break;
                    }
                    let elapsed_since_last = now.duration_since(last_event_time);
                    if elapsed_since_last >= debounce_window {
                        break;
                    }
                    let time_to_wait = debounce_window.saturating_sub(elapsed_since_last);

                    match raw_rx.recv_timeout(time_to_wait) {
                        Ok(next_event) => {
                            last_event_time = Instant::now();
                            record_event(&mut created_modified, &mut deleted, next_event.0, next_event.1);
                        }
                        Err(std_mpsc::RecvTimeoutError::Timeout) => {
                            break;
                        }
                        Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                            return;
                        }
                    }
                }

                let created_vec: Vec<PathBuf> = created_modified.into_iter().collect();
                let deleted_vec: Vec<PathBuf> = deleted.into_iter().collect();

                if !created_vec.is_empty() || !deleted_vec.is_empty() {
                    let _ = tx.send(WatcherEvent::BatchUpdated {
                        created_or_modified: created_vec,
                        deleted: deleted_vec,
                    });
                }
            }
        });

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
    crate::utilities::should_exclude_path(path, config)
}
