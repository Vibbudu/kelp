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
                                        if target.extension().is_some_and(|ext| ext == "exe") {
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
                                            if target.extension().is_some_and(|ext| ext == "exe") {
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
        let (results, _, _) = self.search_with_metrics(raw_query);
        let latency_us = start_time.elapsed().as_micros() as u32;
        (results, latency_us)
    }

    /// Unified search pipeline returning `(results, search_time_ms, rank_time_ms)`.
    pub fn search_with_metrics(&self, raw_query: &str) -> (Vec<SearchResult>, f64, f64) {
        let trimmed = raw_query.trim();
        if trimmed.is_empty() {
            return (Vec::new(), 0.0, 0.0);
        }

        let lower = trimmed.to_lowercase();

        // 0a. Process Killer Command Workflow ("kill" or "kill <app>")
        if lower == "kill" {
            let results = vec![SearchResult {
                metadata: crate::models::FileMetadata {
                    id: None,
                    name: "Kill Process".to_string(),
                    extension: String::new(),
                    parent_folder: "Type 'kill <app>' to search and terminate running apps".to_string(),
                    full_path: "kill_cmd_hint".to_string(),
                    modified_date: 0,
                    size: 0,
                    file_type: crate::models::FileType::Application,
                },
                score: 1.0,
                match_type: "KillCommandHint".to_string(),
                icon_base64: None,
            }];
            return (results, 0.0, 0.0);
        } else if let Some(target_raw) = lower.strip_prefix("kill ") {
            let target = target_raw.trim();
            let procs = crate::process::get_running_user_applications();
            let mut results = Vec::new();
            for p in procs {
                if target.is_empty()
                    || p.name.to_lowercase().contains(target)
                    || p.title.to_lowercase().contains(target)
                {
                    let icon_base64 = p
                        .exe_path
                        .as_deref()
                        .and_then(crate::utilities::get_icon_from_exe_path);
                    results.push(SearchResult {
                        metadata: crate::models::FileMetadata {
                            id: None,
                            name: format!("{} (PID: {})", p.name, p.pid),
                            extension: String::new(),
                            parent_folder: format!("{} — Terminate Application", p.title),
                            full_path: format!("kill://{}", p.pid),
                            modified_date: 0,
                            size: 0,
                            file_type: crate::models::FileType::Application,
                        },
                        score: 1.0,
                        match_type: "KillProcess".to_string(),
                        icon_base64,
                    });
                }
            }
            results.truncate(20);
            return (results, 0.0, 0.0);
        }

        // 0b. Window Switcher Command Workflow ("win" or "win <window/app>")
        if lower == "win" || lower.starts_with("win ") {
            let target = lower.strip_prefix("win").unwrap_or("").trim();
            let windows = crate::process::get_open_windows();
            let mut results = Vec::new();
            for w in windows {
                if target.is_empty()
                    || w.name.to_lowercase().contains(target)
                    || w.title.to_lowercase().contains(target)
                {
                    let icon_base64 = w
                        .exe_path
                        .as_deref()
                        .and_then(crate::utilities::get_icon_from_exe_path);
                    results.push(SearchResult {
                        metadata: crate::models::FileMetadata {
                            id: None,
                            name: w.title.clone(),
                            extension: "SwitchWindow".to_string(),
                            parent_folder: format!("{} (PID: {}) — Switch to Window", w.name, w.pid),
                            full_path: format!("focus://{}", w.hwnd),
                            modified_date: 0,
                            size: 0,
                            file_type: crate::models::FileType::Application,
                        },
                        score: 1.0,
                        match_type: "SwitchWindow".to_string(),
                        icon_base64,
                    });
                }
            }
            if results.is_empty() {
                results.push(SearchResult {
                    metadata: crate::models::FileMetadata {
                        id: None,
                        name: if target.is_empty() {
                            "No open windows found".to_string()
                        } else {
                            format!("No open window matching \"{}\"", target)
                        },
                        extension: "SwitchWindow".to_string(),
                        parent_folder: "Window Switcher • Type 'win' to list open windows".to_string(),
                        full_path: "win_cmd_hint".to_string(),
                        modified_date: 0,
                        size: 0,
                        file_type: crate::models::FileType::Shortcut,
                    },
                    score: 1.0,
                    match_type: "WindowCommandHint".to_string(),
                    icon_base64: None,
                });
            }
            results.truncate(15);
            return (results, 0.0, 0.0);
        }

        // 0c. Clipboard History Command Workflow ("clip", "clipboard", "cb", "clip <query>")
        if lower != "clipboard settings" {
            if let Some(mut clip_results) = crate::clipboard::search_clipboard_query(trimmed) {
                let setting_matches = crate::windows_settings::search_settings(trimmed);
                clip_results.extend(setting_matches);
                clip_results.truncate(15);
                return (clip_results, 0.0, 0.0);
            }
        }

        // Check if query triggers a dynamic non-deterministic generator (UUID, timestamp, password, coin, dice)
        let is_dynamic_tool = crate::quick_tools::is_dynamic_query(trimmed);

        // Exact cache hit check (fast path for backspacing or repeated queries)
        if !is_dynamic_tool {
            if let Some(cached) = self.cache.get_exact(trimmed) {
                return (cached, 0.01, 0.0);
            }
        }

        let search_start = std::time::Instant::now();
        let query = crate::query_parser::parse_query(raw_query);

        // 1. Execute File Search (leveraging prefix subset cache if available)
        let (mut results, matched_files) = self.search_engine.search(&query);
        let search_time_ms = search_start.elapsed().as_micros() as f64 / 1000.0;

        // 2. Score and Sort matching file results
        let rank_start = std::time::Instant::now();
        self.ranking_engine.rank(&mut results, &query);
        let rank_time_ms = rank_start.elapsed().as_micros() as f64 / 1000.0;

        // 2b. Deduplicate applications and shortcuts by normalized name so the UI never renders duplicate apps
        let mut seen_apps = std::collections::HashSet::new();
        results.retain(|r| {
            if r.metadata.file_type == crate::models::FileType::Application
                || r.metadata.file_type == crate::models::FileType::Shortcut
            {
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

        // 3a. Evaluate Instant Calculator & Unit Conversions
        let calc_result = crate::calculator::evaluate_query(trimmed);

        // 3b. Evaluate Quick Developer & Productivity Tools
        let tool_results = crate::quick_tools::evaluate_quick_tool(trimmed);

        // 3c. Website Shortcuts Matching
        let mut exact_shortcuts = Vec::new();
        let mut prefix_shortcuts = Vec::new();
        {
            let cfg = self.get_config();
            for sc in &cfg.web_shortcuts {
                if sc.alias.eq_ignore_ascii_case(trimmed) {
                    exact_shortcuts.push(SearchResult {
                        metadata: crate::models::FileMetadata {
                            id: None,
                            name: sc.name.clone(),
                            extension: String::new(),
                            parent_folder: format!("{} (Website Shortcut)", sc.url),
                            full_path: sc.url.clone(),
                            modified_date: 0,
                            size: 0,
                            file_type: crate::models::FileType::Shortcut,
                        },
                        score: 1.0,
                        match_type: "WebShortcut".to_string(),
                        icon_base64: None,
                    });
                } else if sc.alias.to_lowercase().starts_with(&lower)
                    || (lower.len() >= 2 && sc.name.to_lowercase().starts_with(&lower))
                {
                    prefix_shortcuts.push(SearchResult {
                        metadata: crate::models::FileMetadata {
                            id: None,
                            name: sc.name.clone(),
                            extension: String::new(),
                            parent_folder: format!("{} (Website Shortcut)", sc.url),
                            full_path: sc.url.clone(),
                            modified_date: 0,
                            size: 0,
                            file_type: crate::models::FileType::Shortcut,
                        },
                        score: 0.85,
                        match_type: "WebShortcut".to_string(),
                        icon_base64: None,
                    });
                }
            }
        }

        // 3d. Search Windows Settings, Quick Actions & System Tools
        let setting_results = crate::windows_settings::search_settings(trimmed);
        let (high_settings, other_settings): (Vec<_>, Vec<_>) = setting_results
            .into_iter()
            .partition(|s| s.score >= 0.92);

        // Merge all sources in priority order
        let mut merged = Vec::with_capacity(
            1 + tool_results.len()
                + exact_shortcuts.len()
                + high_settings.len()
                + prefix_shortcuts.len()
                + results.len()
                + other_settings.len(),
        );

        if let Some(calc) = calc_result {
            merged.push(calc);
        }
        merged.extend(tool_results);
        merged.extend(exact_shortcuts);
        merged.extend(high_settings);
        merged.extend(prefix_shortcuts);
        merged.extend(results);
        merged.extend(other_settings);

        let mut results = merged;

        // 4. Hard Limit at 15 results
        let max_res = self.get_config().max_results.clamp(5, 30);
        results.truncate(max_res);

        // 5. Populate Base64 icon strings for file/app results (cached)
        for r in &mut results {
            if matches!(
                r.match_type.as_str(),
                "WindowsSetting"
                    | "WebShortcut"
                    | "Calculator"
                    | "UnitConversion"
                    | "QuickTool"
                    | "ClipboardItem"
                    | "ClipboardHint"
                    | "SwitchWindow"
                    | "WindowCommandHint"
                    | "KillProcess"
                    | "KillCommandHint"
            ) {
                continue;
            }
            r.icon_base64 = Some(crate::utilities::get_icon_cached(&r.metadata));
        }

        // 6. Update Result Cache with matching file candidates and final merged results
        if !is_dynamic_tool {
            self.cache.insert(raw_query, matched_files, results.clone());
        }

        (results, search_time_ms, rank_time_ms)
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

    #[tokio::test]
    async fn test_ui_bridge_unified_search_features() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kelp_test_unified_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let db_path = temp_dir.join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let files = vec![
            FileMetadata {
                id: None,
                name: "report.pdf".to_string(),
                extension: "pdf".to_string(),
                parent_folder: "C:\\Docs".to_string(),
                full_path: "C:\\Docs\\report.pdf".to_string(),
                modified_date: 1700000000,
                size: 1024,
                file_type: crate::models::FileType::File,
            },
            FileMetadata {
                id: None,
                name: "notes.txt".to_string(),
                extension: "txt".to_string(),
                parent_folder: "C:\\Docs".to_string(),
                full_path: "C:\\Docs\\notes.txt".to_string(),
                modified_date: 1700000000,
                size: 512,
                file_type: crate::models::FileType::File,
            },
            FileMetadata {
                id: None,
                name: "Visual Studio Code.lnk".to_string(),
                extension: "lnk".to_string(),
                parent_folder: "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
                full_path: "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Visual Studio Code.lnk".to_string(),
                modified_date: 1700000000,
                size: 1024,
                file_type: crate::models::FileType::Shortcut,
            },
            FileMetadata {
                id: None,
                name: "Explorer.exe".to_string(),
                extension: "exe".to_string(),
                parent_folder: "C:\\Windows".to_string(),
                full_path: "C:\\Windows\\Explorer.exe".to_string(),
                modified_date: 1700000000,
                size: 2048,
                file_type: crate::models::FileType::Application,
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
            index,
            cache,
            search_engine,
            ranking_engine,
            _watcher: None,
            config: RwLock::new(config),
            is_indexing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        // 1. Natural language Windows action: "change theme to light mode"
        let (res_light, _) = bridge.search("change theme to light mode");
        assert!(!res_light.is_empty(), "Expected WindowsSetting match for 'change theme to light mode'");
        assert_eq!(res_light[0].match_type, "WindowsSetting");
        assert_eq!(res_light[0].metadata.full_path, "action:set_light_mode");

        // 1b. Negated natural language Windows action: "turn off dark mode" -> Light Mode
        let (res_neg_dark, _) = bridge.search("turn off dark mode");
        assert!(!res_neg_dark.is_empty());
        assert_eq!(res_neg_dark[0].metadata.full_path, "action:set_light_mode");

        // 2. Natural language Windows action: "switch to dark mode"
        let (res_dark, _) = bridge.search("switch to dark mode");
        assert!(!res_dark.is_empty());
        assert_eq!(res_dark[0].metadata.full_path, "action:set_dark_mode");

        // 3. Calculator: "25 * 4 + 10"
        let (res_calc, _) = bridge.search("25 * 4 + 10");
        assert!(!res_calc.is_empty());
        assert_eq!(res_calc[0].match_type, "Calculator");
        assert_eq!(res_calc[0].metadata.name, "= 110");
        assert_eq!(res_calc[0].metadata.full_path, "copy:110");

        // 4. Unit conversion: "100 km in miles"
        let (res_unit, _) = bridge.search("100 km in miles");
        assert!(!res_unit.is_empty());
        assert_eq!(res_unit[0].match_type, "UnitConversion");
        assert!(res_unit[0].metadata.name.contains("62.1371"));

        // 5. Quick tool: "base64 encode kelp"
        let (res_b64, _) = bridge.search("base64 encode kelp");
        assert!(!res_b64.is_empty());
        assert_eq!(res_b64[0].match_type, "QuickTool");
        assert_eq!(res_b64[0].metadata.name, "a2VscA==");

        // 6. Prefix cache poisoning regression check: ".pd" (generic) -> ".pdf" (semantic extension)
        let _ = bridge.search(".pd");
        let (res_pdf, _) = bridge.search(".pdf");
        assert!(
            res_pdf.iter().any(|r| r.metadata.name == "report.pdf"),
            "Expected '.pdf' to match report.pdf even after '.pd' was cached"
        );

        // 7. Short prefix query "co" ranks application (Visual Studio Code) above prefix-matched settings
        let (res_co, _) = bridge.search("co");
        assert!(!res_co.is_empty());
        assert_eq!(
            res_co[0].metadata.name, "Visual Studio Code.lnk",
            "Expected app match to rank #1 for short prefix 'co', got {:?}",
            res_co[0].metadata.name
        );

        // 8. Single-char query "e" ranks Explorer.exe #1 instead of Euler's constant
        let (res_e, _) = bridge.search("e");
        assert!(!res_e.is_empty());
        assert_ne!(res_e[0].match_type, "Calculator");
        assert_eq!(res_e[0].metadata.name, "Explorer.exe");

        // 9. Dynamic tool "pw" bypasses cache and generates fresh passwords on repeated calls
        let (pw1, _) = bridge.search("pw");
        let (pw2, _) = bridge.search("pw");
        assert_eq!(pw1[0].match_type, "QuickTool");
        assert_ne!(pw1[0].metadata.name, pw2[0].metadata.name);

        // 10. Window switcher with unmatched target returns WindowCommandHint instead of empty vec
        let (win_empty, _) = bridge.search("win __nonexistent_window_99999__");
        assert_eq!(win_empty.len(), 1);
        assert_eq!(win_empty[0].match_type, "WindowCommandHint");
        assert_eq!(win_empty[0].metadata.full_path, "win_cmd_hint");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
