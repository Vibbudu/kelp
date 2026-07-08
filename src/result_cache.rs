use crate::models::{FileMetadata, SearchResult};
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::warn;

struct CacheInner {
    // Map of raw lowercase query string to final ranked results (for exact backspace hits)
    exact_results: HashMap<String, Vec<SearchResult>>,
    // Map of raw lowercase query string to matching candidate metadata (for subset search filters)
    subset_files: HashMap<String, Vec<FileMetadata>>,
    // Insertion order tracking for simple FIFO eviction
    order: Vec<String>,
}

use std::sync::atomic::{AtomicUsize, Ordering};

pub struct ResultCache {
    inner: Mutex<CacheInner>,
    max_capacity: usize,
    pub hits: AtomicUsize,
    pub misses: AtomicUsize,
}

impl ResultCache {
    /// Creates a new ResultCache with the default capacity of 20 queries.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CacheInner {
                exact_results: HashMap::new(),
                subset_files: HashMap::new(),
                order: Vec::new(),
            }),
            max_capacity: 20,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }

    /// Clears the cache completely (called on DB indexing, folder deletions, or watch events).
    pub fn clear(&self) {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("ResultCache lock poisoned on clear(), recovering");
                poisoned.into_inner()
            }
        };
        inner.exact_results.clear();
        inner.subset_files.clear();
        inner.order.clear();
    }

    /// Inserts a query along with its matched files and final ranked results.
    pub fn insert(&self, query: &str, files: Vec<FileMetadata>, results: Vec<SearchResult>) {
        let query_lower = query.trim().to_lowercase();
        if query_lower.is_empty() {
            return;
        }

        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("ResultCache lock poisoned on insert(), recovering");
                poisoned.into_inner()
            }
        };

        // If the entry already exists, update it and move to back of eviction queue
        if inner.exact_results.contains_key(&query_lower) {
            inner.exact_results.insert(query_lower.clone(), results);
            inner.subset_files.insert(query_lower, files);
            return;
        }

        // Handle capacity eviction (FIFO)
        if inner.order.len() >= self.max_capacity {
            let evicted = inner.order.remove(0);
            inner.exact_results.remove(&evicted);
            inner.subset_files.remove(&evicted);
        }

        inner.order.push(query_lower.clone());
        inner.exact_results.insert(query_lower.clone(), results);
        inner.subset_files.insert(query_lower, files);
    }

    /// Performs an exact cache lookup for backspace actions or repeated queries.
    pub fn get_exact(&self, query: &str) -> Option<Vec<SearchResult>> {
        let query_lower = query.trim().to_lowercase();
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("ResultCache lock poisoned on get_exact(), recovering");
                poisoned.into_inner()
            }
        };
        if let Some(res) = inner.exact_results.get(&query_lower) {
            self.hits.fetch_add(1, Ordering::SeqCst);
            Some(res.clone())
        } else {
            self.misses.fetch_add(1, Ordering::SeqCst);
            None
        }
    }

    /// Finds the longest cached query prefix that matches the current query,
    /// returning the subset of matching files to dramatically reduce search space.
    ///
    /// Example:
    /// - Current query: "hel"
    /// - Cached queries: "h", "he"
    /// - Matches "he" and returns the subset files matched by "he".
    pub fn get_longest_prefix_subset(&self, query: &str) -> Option<(String, Vec<FileMetadata>)> {
        let query_lower = query.trim().to_lowercase();
        if query_lower.is_empty() {
            return None;
        }

        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("ResultCache lock poisoned on get_longest_prefix_subset(), recovering");
                poisoned.into_inner()
            }
        };
        let mut best_prefix = String::new();
        let mut best_files = None;

        for (cached_query, files) in inner.subset_files.iter() {
            // Check if cached query is a proper prefix and longer than our best match so far
            if query_lower.starts_with(cached_query) && cached_query.len() > best_prefix.len() {
                // Avoid using the query itself (as that would be an exact hit handled elsewhere)
                if cached_query.len() < query_lower.len() {
                    best_prefix = cached_query.clone();
                    best_files = Some(files.clone());
                }
            }
        }

        best_files.map(|files| (best_prefix, files))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileType;

    fn mock_file(name: &str) -> FileMetadata {
        FileMetadata {
            id: None,
            name: name.to_string(),
            extension: String::new(),
            parent_folder: String::new(),
            full_path: format!("C:\\{}", name),
            modified_date: 0,
            size: 0,
            file_type: FileType::File,
        }
    }

    #[test]
    fn test_result_cache_flow() {
        let cache = ResultCache::new();

        // Exact hit test
        assert!(cache.get_exact("test").is_none());
        
        let files = vec![mock_file("Chrome"), mock_file("Chromium")];
        cache.insert("chr", files.clone(), Vec::new());

        // Subset hit test
        let subset = cache.get_longest_prefix_subset("chro");
        assert!(subset.is_some());
        let (prefix, cached_files) = subset.unwrap();
        assert_eq!(prefix, "chr");
        assert_eq!(cached_files.len(), 2);

        // Clears properly
        cache.clear();
        assert!(cache.get_longest_prefix_subset("chro").is_none());
    }
}
