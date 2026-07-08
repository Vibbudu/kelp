use crate::memory_index::MemoryIndex;
use crate::models::{FileMetadata, SearchQuery, SearchResult};
use crate::result_cache::ResultCache;
use crate::search::match_file;
use std::sync::Arc;

pub struct SearchEngine {
    index: Arc<MemoryIndex>,
    cache: Arc<ResultCache>,
}

impl SearchEngine {
    /// Creates a new SearchEngine with shared index and query cache.
    pub fn new(index: Arc<MemoryIndex>, cache: Arc<ResultCache>) -> Self {
        Self { index, cache }
    }

    /// Searches the memory index or result cache for matches.
    ///
    /// Returns:
    /// 1. Unranked raw search results.
    /// 2. The subset of files that matched (used to update the result cache).
    pub fn search(&self, query: &SearchQuery) -> (Vec<SearchResult>, Vec<FileMetadata>) {
        // 1. If empty query, return immediately
        if query.terms.is_empty() && query.extension_filter.is_none() {
            return (Vec::new(), Vec::new());
        }

        let raw_query = &query.raw;

        // 2. Check exact cache hit (backspace or duplicate queries)
        if let Some(cached_results) = self.cache.get_exact(raw_query) {
            let files = cached_results.iter().map(|r| r.metadata.clone()).collect();
            return (cached_results, files);
        }

        // 3. Determine search candidate files (use subset cache if available to reduce space)
        let mut results = Vec::new();
        let mut matched_files = Vec::new();

        if let Some((_prefix, subset_files)) = self.cache.get_longest_prefix_subset(raw_query) {
            for file in subset_files {
                if let Some(result) = match_file(&file, query) {
                    results.push(result);
                    matched_files.push(file);
                }
            }
        } else {
            // Cache miss: search memory index with zero-clone matching under read lock
            let (results_res, matched_files_res) = self.index.search_and_match(query);
            results = results_res;
            matched_files = matched_files_res;
        }

        (results, matched_files)
    }
}
