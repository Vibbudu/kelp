use crate::indexer::Indexer;
use crate::learning::LearningEngine;
use crate::memory_index::MemoryIndex;
use crate::models::{FileMetadata, SearchResult};
use crate::ranking_engine::RankingEngine;
use crate::result_cache::ResultCache;
use crate::search_engine::SearchEngine;
use crate::storage::Storage;
use crate::watcher::{FileWatcher, WatcherEvent};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct UIBridge {
    _storage: Storage,
    learning: Arc<LearningEngine>,
    pub index: Arc<MemoryIndex>,
    pub cache: Arc<ResultCache>,
    pub search_engine: SearchEngine,
    pub ranking_engine: RankingEngine,
    _watcher: Option<FileWatcher>,
    pub config: crate::config::AppConfig,
}

impl UIBridge {
    /// Initializes the search system: opens database, crawls if empty, initializes memory indices,
    /// query caches, and starts the file watcher.
    pub async fn initialize(db_path: &Path, paths_to_watch: &[String]) -> Result<Self, String> {
        info!("Initializing Search System Bridge...");

        // Load configuration file
        let config_path = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("config.json");
        let app_config = crate::config::AppConfig::load_or_create(&config_path);

        // 1. Setup SQLite Storage
        let storage = Storage::new(db_path).map_err(|e| format!("Database init failed: {}", e))?;

        // 2. Setup Learning Engine
        let learning = Arc::new(LearningEngine::new(storage.clone()));

        // 3. Load files from SQLite into RAM Index
        let loaded_files = storage
            .load_all_files()
            .map_err(|e| format!("Failed to load files from database: {}", e))?;

        let index = Arc::new(MemoryIndex::new(loaded_files));
        let cache = Arc::new(ResultCache::new());

        let index_c = Arc::clone(&index);
        let storage_c = storage.clone();
        let paths_c = paths_to_watch.to_vec();
        let config_c = app_config.clone();

        // 4. Crawl if empty
        if index.len() == 0 {
            info!("Database index empty. Triggering initial background scan...");
            let config_thread = config_c.clone();
            tokio::task::spawn_blocking(move || {
                let indexer = Indexer::new(storage_c.clone(), config_thread);
                match indexer.index_paths(&paths_c) {
                    Ok(count) => {
                        info!("Initial indexing scanned {} items.", count);
                        if let Ok(new_files) = storage_c.load_all_files() {
                            index_c.rebuild(new_files);
                        }
                    }
                    Err(e) => {
                        error!("Initial background indexing failed: {}", e);
                    }
                }
            })
            .await
            .map_err(|e| format!("Indexer join failed: {}", e))?;
        } else {
            info!("Loaded {} items from database into memory index.", index.len());
            // Trigger incremental scan in the background to sync any updates since last closure
            let paths_inc = paths_to_watch.to_vec();
            let storage_inc = storage.clone();
            let index_inc = Arc::clone(&index);
            let config_thread = config_c.clone();
            tokio::spawn(async move {
                tokio::task::spawn_blocking(move || {
                    let indexer = Indexer::new(storage_inc.clone(), config_thread);
                    if let Ok(count) = indexer.index_paths(&paths_inc) {
                        info!("Startup sync finished indexing. Total: {} items.", count);
                        if let Ok(new_files) = storage_inc.load_all_files() {
                            index_inc.rebuild(new_files);
                        }
                    }
                });
            });
        }

        // 5. Setup Watcher & Event Channel
        let (tx, mut rx) = mpsc::unbounded_channel();
        let watcher = FileWatcher::new(paths_to_watch, tx, app_config.clone())?;

        let watcher_index = Arc::clone(&index);
        let watcher_storage = storage.clone();
        let watcher_cache = Arc::clone(&cache);

        // 6. Spawn Background Event Listener Task
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    WatcherEvent::CreatedOrModified(path) => {
                        if !path.exists() {
                            continue;
                        }
                        let storage = watcher_storage.clone();
                        let idx = Arc::clone(&watcher_index);
                        let query_cache = Arc::clone(&watcher_cache);

                        tokio::task::spawn_blocking(move || {
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                let is_dir = metadata.is_dir();
                                let name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                if name.is_empty() {
                                    return;
                                }

                                let extension = if is_dir {
                                    String::new()
                                } else {
                                    path.extension()
                                        .map(|e| e.to_string_lossy().to_string().to_lowercase())
                                        .unwrap_or_default()
                                };

                                let parent = path
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let size = if is_dir { 0 } else { metadata.len() as i64 };
                                let modified = metadata
                                    .modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs() as i64)
                                    .unwrap_or(0);

                                let file_type = if is_dir {
                                    crate::models::FileType::Folder
                                } else if extension == "exe" {
                                    crate::models::FileType::Application
                                } else if extension == "lnk" {
                                    if let Some(target) = crate::utilities::resolve_lnk(&path) {
                                        if target.extension().map_or(false, |ext| ext == "exe") {
                                            crate::models::FileType::Application
                                        } else {
                                            crate::models::FileType::Shortcut
                                        }
                                    } else {
                                        crate::models::FileType::Shortcut
                                    }
                                } else {
                                    crate::models::FileType::File
                                };

                                let file_meta = FileMetadata {
                                    id: None,
                                    name,
                                    extension,
                                    parent_folder: parent,
                                    full_path: path.to_string_lossy().to_string(),
                                    modified_date: modified,
                                    size,
                                    file_type,
                                };

                                // Update SQLite
                                if let Err(e) = storage.save_file(&file_meta) {
                                    error!("Failed to save watched file: {:?}", e);
                                }

                                // Update Memory Index
                                idx.add_or_update(file_meta);

                                // Invalidate query cache
                                query_cache.clear();
                            }
                        });
                    }
                    WatcherEvent::Deleted(path) => {
                        let path_str = path.to_string_lossy().to_string();
                        let storage = watcher_storage.clone();
                        let idx = Arc::clone(&watcher_index);
                        let query_cache = Arc::clone(&watcher_cache);

                        tokio::task::spawn_blocking(move || {
                            // Update SQLite
                            if let Err(e) = storage.delete_folder_recursive(&path_str) {
                                error!("Failed to delete watched folder: {:?}", e);
                            }

                            // Update Memory Index
                            idx.remove_prefix(&path_str);

                            // Invalidate query cache
                            query_cache.clear();
                        });
                    }
                }
            }
        });

        // 7. Setup Engines
        let search_engine = SearchEngine::new(Arc::clone(&index), Arc::clone(&cache));
        let ranking_engine = RankingEngine::default_config(Arc::clone(&learning));

        Ok(Self {
            _storage: storage,
            learning,
            index,
            cache,
            search_engine,
            ranking_engine,
            _watcher: Some(watcher),
            config: app_config,
        })
    }

    /// Executes query search, ranks results, updates result caches,
    /// and returns results along with execution latency in microseconds.
    pub fn search(&self, raw_query: &str) -> (Vec<SearchResult>, u32) {
        let start_time = std::time::Instant::now();
        let query = crate::query_parser::parse_query(raw_query);

        // 1. Execute Search (leveraging prefix subset cache if available)
        let (mut results, _matched_files) = self.search_engine.search(&query);

        // 2. Score and Sort matching results
        self.ranking_engine.rank(&mut results, &query);

        // 3. Dynamic Quality Filtering based on query length
        let q_len = query.raw.len();
        let threshold = if q_len <= 2 {
            0.3
        } else if q_len <= 4 {
            0.4
        } else {
            0.5
        };
        results.retain(|r| r.score >= threshold);

        // 4. Hard Limit at 15 results
        results.truncate(15);

        // 5. Populate Base64 icon strings for top 15 results (cached)
        for r in &mut results {
            r.icon_base64 = Some(crate::utilities::get_icon_cached(&r.metadata));
        }

        // 6. Update Result Cache with only the truncated/high-quality set
        let top_matched_files: Vec<FileMetadata> = results.iter().map(|r| r.metadata.clone()).collect();
        self.cache.insert(raw_query, top_matched_files, results.clone());

        let latency_us = start_time.elapsed().as_micros() as u32;
        (results, latency_us)
    }

    /// Logs result selection to learning engine cache and storage.
    pub fn select_result(&self, query: &str, path: &str) -> Result<(), String> {
        self.learning.record_selection(query, path)
    }

    /// Returns the number of files currently indexed in memory.
    pub fn total_files(&self) -> usize {
        self.index.len()
    }

    /// Helper to check if the background filesystem watcher thread is active
    pub fn is_watcher_running(&self) -> bool {
        self._watcher.is_some()
    }

    /// Helper to check if the learning selection database cache is ready
    pub fn is_learning_ready(&self) -> bool {
        self.learning.is_ready()
    }
}
