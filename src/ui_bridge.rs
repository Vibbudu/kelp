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
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct UIBridge {
    _storage: Storage,
    learning: Arc<LearningEngine>,
    pub index: Arc<MemoryIndex>,
    pub cache: Arc<ResultCache>,
    pub search_engine: SearchEngine,
    pub ranking_engine: RankingEngine,
    _watcher: Option<FileWatcher>,
    config: RwLock<crate::config::AppConfig>,
    pub is_indexing: Arc<std::sync::atomic::AtomicBool>,
}

impl UIBridge {
    /// Initializes the search system: opens database, crawls if empty, initializes memory indices,
    /// query caches, and starts the file watcher.
    pub async fn initialize(db_path: &Path, paths_to_watch: &[String]) -> Result<Self, String> {
        info!("Initializing Search System Bridge...");

        // Load configuration file
        let config_path = crate::utilities::get_app_data_dir().join("config.json");
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

        let config_c = app_config.clone();

        // 4. Spawn non-blocking background crawler (initial or incremental)
        let paths_bg = paths_to_watch.to_vec();
        let storage_bg = storage.clone();
        let index_bg = Arc::clone(&index);
        let config_thread = config_c.clone();
        let is_initial = index.len() == 0;
        let is_indexing = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let is_indexing_bg = Arc::clone(&is_indexing);

        if is_initial {
            info!("Database index empty. Triggering initial non-blocking background scan...");
        } else {
            info!("Loaded {} items from database into memory index. Triggering background sync...", index.len());
        }

        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                let indexer = Indexer::new(storage_bg.clone(), config_thread);
                match indexer.index_paths(&paths_bg) {
                    Ok(count) => {
                        info!("Background indexing finished. Total: {} items.", count);
                        if let Ok(new_files) = storage_bg.load_all_files() {
                            index_bg.rebuild(new_files);
                        }
                    }
                    Err(e) => {
                        error!("Background indexing failed: {}", e);
                    }
                }
                is_indexing_bg.store(false, std::sync::atomic::Ordering::SeqCst);
            });
        });

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
                    WatcherEvent::BatchUpdated { created_or_modified, deleted } => {
                        let storage = watcher_storage.clone();
                        let idx = Arc::clone(&watcher_index);
                        let query_cache = Arc::clone(&watcher_cache);

                        tokio::task::spawn_blocking(move || {
                            let mut batch_to_save = Vec::new();

                            for path in created_or_modified {
                                if !path.exists() {
                                    continue;
                                }
                                if let Ok(metadata) = std::fs::metadata(&path) {
                                    let is_dir = metadata.is_dir();
                                    let mut name = path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    if name.is_empty() {
                                        continue;
                                    }

                                    let extension = if is_dir {
                                        String::new()
                                    } else {
                                        path.extension()
                                            .map(|e| e.to_string_lossy().to_string().to_lowercase())
                                            .unwrap_or_default()
                                    };

                                    if extension == "lnk" && name.to_lowercase().ends_with(".lnk") {
                                        name = name[..name.len() - 4].to_string();
                                    }

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

                                    batch_to_save.push(FileMetadata {
                                        id: None,
                                        name,
                                        extension,
                                        parent_folder: parent,
                                        full_path: path.to_string_lossy().to_string(),
                                        modified_date: modified,
                                        size,
                                        file_type,
                                    });
                                }
                            }

                            // 1. Batch write to SQLite and update MemoryIndex
                            if !batch_to_save.is_empty() {
                                if let Err(e) = storage.save_files(&batch_to_save) {
                                    error!("Failed to save batch watched files: {:?}", e);
                                }
                                idx.add_or_update_batch(batch_to_save);
                            }

                            // 2. Batch delete from SQLite and update MemoryIndex
                            if !deleted.is_empty() {
                                let deleted_strings: Vec<String> = deleted.iter().map(|p| p.to_string_lossy().to_string()).collect();
                                for path_str in &deleted_strings {
                                    let _ = storage.delete_folder_recursive(path_str);
                                }
                                idx.remove_prefix_batch(&deleted_strings);
                            }

                            // 3. Clear cache once for the entire batch
                            query_cache.clear();
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
            config: RwLock::new(app_config),
            is_indexing,
        })
    }

    /// Executes query search, ranks results, updates result caches,
    /// and returns results along with execution latency in microseconds.
    pub fn search(&self, raw_query: &str) -> (Vec<SearchResult>, u32) {
        let start_time = std::time::Instant::now();
        let query = crate::query_parser::parse_query(raw_query);

        // 1. Execute Search (leveraging prefix subset cache if available)
        let (mut results, matched_files) = self.search_engine.search(&query);

        // 2. Score and Sort matching results
        self.ranking_engine.rank(&mut results, &query);

        // 2b. Deduplicate applications and shortcuts by normalized name so the UI never renders duplicate apps
        let mut seen_apps = std::collections::HashSet::new();
        results.retain(|r| {
            if r.metadata.file_type == crate::models::FileType::Application || r.metadata.file_type == crate::models::FileType::Shortcut {
                let name_key = r.metadata.name.to_lowercase().trim().to_string();
                if seen_apps.contains(&name_key) {
                    false
                } else {
                    seen_apps.insert(name_key);
                    true
                }
            } else {
                true
            }
        });

        // 3. Dynamic Quality Filtering based on query length
        let q_len = query.raw.len();
        let threshold = if q_len <= 2 {
            0.2
        } else if q_len <= 4 {
            0.3
        } else {
            0.4
        };
        results.retain(|r| r.score >= threshold);

        // 4. Hard Limit at 15 results
        results.truncate(15);

        // 5. Populate Base64 icon strings for top 15 results (cached)
        for r in &mut results {
            r.icon_base64 = Some(crate::utilities::get_icon_cached(&r.metadata));
        }

        // 6. Update Result Cache with all matching candidates and top 15 final results
        self.cache.insert(raw_query, matched_files, results.clone());

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

    /// Performs a manual full reindexing of given paths, updating memory index and caches.
    pub fn reindex_blocking(&self, paths: &[String]) -> Result<usize, String> {
        self.is_indexing.store(true, std::sync::atomic::Ordering::SeqCst);
        let indexer = Indexer::new(self._storage.clone(), self.get_config());
        let res = indexer.index_paths(paths).map_err(|e| e.to_string());
        self.is_indexing.store(false, std::sync::atomic::Ordering::SeqCst);
        let count = res?;
        if let Ok(new_files) = self._storage.load_all_files() {
            self.index.rebuild(new_files);
        }
        self.cache.clear();
        Ok(count)
    }

    /// Returns true if background crawling or manual reindexing is active.
    pub fn is_indexing(&self) -> bool {
        self.is_indexing.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Clears both in-memory and SQLite learning query history and clears result caches.
    pub fn clear_learning_cache(&self) -> Result<(), String> {
        self.learning.clear_cache()?;
        self.cache.clear();
        Ok(())
    }

    /// Returns a clone of the current AppConfig.
    pub fn get_config(&self) -> crate::config::AppConfig {
        match self.config.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => {
                warn!("UIBridge config read lock poisoned, recovering");
                poisoned.into_inner().clone()
            }
        }
    }

    /// Updates the in-memory AppConfig reference.
    pub fn update_config(&self, config: crate::config::AppConfig) {
        match self.config.write() {
            Ok(mut guard) => *guard = config,
            Err(poisoned) => {
                warn!("UIBridge config write lock poisoned, recovering");
                *poisoned.into_inner() = config;
            }
        }
    }

    /// Immediately purges an excluded path from RAM index and deletes from SQLite without full rebuild.
    pub fn add_exclusion(&self, path: &str) -> Result<(), String> {
        let expanded = crate::utilities::expand_env_vars(path);

        // 1. Update config
        {
            let mut cfg = match self.config.write() {
                Ok(guard) => guard,
                Err(p) => p.into_inner(),
            };
            if !cfg.excluded_paths.iter().any(|p| p.eq_ignore_ascii_case(path)) {
                cfg.excluded_paths.push(path.to_string());
                let cfg_path = crate::utilities::get_app_data_dir().join("config.json");
                let _ = cfg.save(&cfg_path);
            }
        }

        // 2. Instant memory removal (< 10ms)
        self.index.remove_prefix(&expanded);

        // 3. Clear result cache
        self.cache.clear();

        // 4. Background SQLite deletion
        let storage = self._storage.clone();
        let path_clone = expanded.clone();
        tokio::task::spawn_blocking(move || {
            let _ = storage.delete_folder_recursive(&path_clone);
        });

        Ok(())
    }

    /// Removes an excluded path from config and triggers background reindexing for that path.
    pub fn remove_exclusion(&self, path: &str) -> Result<(), String> {
        // 1. Update config
        {
            let mut cfg = match self.config.write() {
                Ok(guard) => guard,
                Err(p) => p.into_inner(),
            };
            cfg.excluded_paths.retain(|p| !p.eq_ignore_ascii_case(path));
            let cfg_path = crate::utilities::get_app_data_dir().join("config.json");
            let _ = cfg.save(&cfg_path);
        }

        // 2. Trigger background reindexing for this path to restore files
        let storage = self._storage.clone();
        let index = Arc::clone(&self.index);
        let path_to_scan = path.to_string();
        let config = self.get_config();

        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                let indexer = Indexer::new(storage.clone(), config);
                if let Ok(count) = indexer.index_paths(&[path_to_scan]) {
                    info!("Restored {} items after removing exclusion.", count);
                    if let Ok(new_files) = storage.load_all_files() {
                        index.rebuild(new_files);
                    }
                }
            });
        });

        self.cache.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ui_bridge_exclusion_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("kelp_test_bridge_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let db_path = temp_dir.join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let files = vec![
            FileMetadata {
                id: None,
                name: "keep.txt".to_string(),
                extension: "txt".to_string(),
                parent_folder: "C:\\keep".to_string(),
                full_path: "C:\\keep\\keep.txt".to_string(),
                modified_date: 0,
                size: 10,
                file_type: crate::models::FileType::File,
            },
            FileMetadata {
                id: None,
                name: "drop.txt".to_string(),
                extension: "txt".to_string(),
                parent_folder: "C:\\drop_folder".to_string(),
                full_path: "C:\\drop_folder\\drop.txt".to_string(),
                modified_date: 0,
                size: 10,
                file_type: crate::models::FileType::File,
            },
        ];
        let index = Arc::new(MemoryIndex::new(files));
        let cache = Arc::new(ResultCache::new());
        let learning = Arc::new(LearningEngine::new(storage.clone()));
        let search_engine = SearchEngine::new(Arc::clone(&index), Arc::clone(&cache));
        let ranking_engine = RankingEngine::default_config(Arc::clone(&learning));
        let config = crate::config::AppConfig::default();

        let bridge = UIBridge {
            _storage: storage,
            learning,
            index: Arc::clone(&index),
            cache,
            search_engine,
            ranking_engine,
            _watcher: None,
            config: RwLock::new(config),
            is_indexing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        assert_eq!(bridge.index.len(), 2);
        bridge.add_exclusion("C:\\drop_folder").unwrap();
        assert_eq!(bridge.index.len(), 1);
        assert_eq!(bridge.index.get_all()[0].name, "keep.txt");
        assert!(bridge.get_config().excluded_paths.iter().any(|p| p == "C:\\drop_folder"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
