use crate::storage::Storage;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, warn};

#[derive(Clone)]
pub struct LearningEngine {
    storage: Storage,
    // In-memory cache: query -> (path -> (selection_count, last_selected_at))
    cache: Arc<RwLock<HashMap<String, HashMap<String, (i64, i64)>>>>,
}

impl LearningEngine {
    /// Creates a new learning engine and pre-populates the in-memory frequency cache.
    pub fn new(storage: Storage) -> Self {
        let engine = Self {
            storage,
            cache: Arc::new(RwLock::new(HashMap::new())),
        };
        if let Err(e) = engine.load_cache() {
            warn!("Failed to load learning engine cache: {}", e);
        }
        engine
    }

    /// Loads the frequency and recency data from SQLite into the RAM cache on startup.
    fn load_cache(&self) -> Result<(), String> {
        let data = self.storage.get_learning_data().map_err(|e| e.to_string())?;
        let mut cache = match self.cache.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("LearningEngine write lock poisoned on load_cache(), recovering");
                poisoned.into_inner()
            }
        };
        for (query, path, count, timestamp) in data {
            cache
                .entry(query.to_lowercase())
                .or_default()
                .insert(path, (count, timestamp));
        }
        Ok(())
    }

    /// Logs that a user selected a specific path for a given query, updating cache and database.
    pub fn record_selection(&self, query: &str, path: &str) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let query_lower = query.trim().to_lowercase();
        let path_str = path.to_string();

        if query_lower.is_empty() || path_str.is_empty() {
            return Ok(());
        }

        // 1. Update in-memory cache immediately (O(1) write for subsequent searches)
        {
            let mut cache = match self.cache.write() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("LearningEngine write lock poisoned on record_selection(), recovering");
                    poisoned.into_inner()
                }
            };
            let entry = cache
                .entry(query_lower.clone())
                .or_default()
                .entry(path_str.clone())
                .or_insert((0, 0));
            entry.0 += 1;
            entry.1 = now;
        }

        // 2. Persist to SQLite on a background thread pool to prevent blocking the caller thread
        let storage = self.storage.clone();
        let query_str = query.to_string();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = storage.record_selection(&query_str, &path_str, now) {
                error!("Failed to persist query selection: {:?}", e);
            }
        });

        Ok(())
    }

    /// Looks up the selection statistics for a given query and file path.
    ///
    /// Implements "partial queries" support by checking if the query string matches
    /// the prefix of any query stored in the history cache.
    pub fn lookup(&self, query: &str, path: &str) -> Option<(i64, i64)> {
        let cache = match self.cache.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("LearningEngine read lock poisoned on lookup(), recovering");
                poisoned.into_inner()
            }
        };
        let query_lower = query.trim().to_lowercase();

        if query_lower.is_empty() {
            return None;
        }

        // 1. Try exact query lookup (fast path)
        if let Some(paths) = cache.get(&query_lower) {
            if let Some(&stats) = paths.get(path) {
                return Some(stats);
            }
        }

        // 2. Fallback prefix query lookup (for partial queries like 'hel' matching 'helium browser' history)
        // Aggregates statistics if the file was selected under queries matching this prefix
        let mut max_count = 0;
        let mut max_time = 0;
        let mut found = false;

        for (hist_query, paths) in cache.iter() {
            if hist_query.starts_with(&query_lower) {
                if let Some(&(count, timestamp)) = paths.get(path) {
                    found = true;
                    if count > max_count {
                        max_count = count;
                    }
                    if timestamp > max_time {
                        max_time = timestamp;
                    }
                }
            }
        }

        if found {
            Some((max_count, max_time))
        } else {
            None
        }
    }

    /// Checks if the cache read lock is intact and available
    pub fn is_ready(&self) -> bool {
        self.cache.read().is_ok()
    }
}
