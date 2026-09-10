use crate::models::{FileMetadata, SearchResult};
use std::sync::RwLock;
use tracing::warn;

struct MemoryIndexInner {
    files: Vec<FileMetadata>,
    lowercase_names: Vec<String>,
    // Indices into `files` sorted by lowercase file name
    sorted_by_name: Vec<usize>,
}

impl MemoryIndexInner {
    fn new(files: Vec<FileMetadata>) -> Self {
        let mut inner = Self {
            files,
            lowercase_names: Vec::new(),
            sorted_by_name: Vec::new(),
        };
        inner.rebuild_index();
        inner
    }

    /// Rebuilds the pre-lowercased names and sorted index list.
    fn rebuild_index(&mut self) {
        self.lowercase_names = self.files.iter().map(|f| f.name.to_lowercase()).collect();
        let mut indices: Vec<usize> = (0..self.files.len()).collect();
        let l_names = &self.lowercase_names;
        // Sort indices based on pre-lowercased names with zero allocations during comparison
        indices.sort_unstable_by(|&a, &b| {
            l_names[a].cmp(&l_names[b])
        });
        self.sorted_by_name = indices;
    }
}

pub struct MemoryIndex {
    inner: RwLock<MemoryIndexInner>,
}

impl MemoryIndex {
    /// Creates a new MemoryIndex containing the given list of files.
    pub fn new(files: Vec<FileMetadata>) -> Self {
        Self {
            inner: RwLock::new(MemoryIndexInner::new(files)),
        }
    }

    /// Replaces the index entirely with a new set of files.
    pub fn rebuild(&self, files: Vec<FileMetadata>) {
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex write lock poisoned on rebuild(), recovering");
                poisoned.into_inner()
            }
        };
        *inner = MemoryIndexInner::new(files);
    }

    /// Incremental add or update for a single file.
    pub fn add_or_update(&self, file: FileMetadata) {
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex write lock poisoned on add_or_update(), recovering");
                poisoned.into_inner()
            }
        };
        if let Some(pos) = inner.files.iter().position(|f| f.full_path == file.full_path) {
            inner.files[pos] = file;
        } else {
            inner.files.push(file);
        }
        inner.rebuild_index();
    }

    /// Batch add or update for multiple files in a single pass.
    pub fn add_or_update_batch(&self, files: Vec<FileMetadata>) {
        if files.is_empty() {
            return;
        }
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex write lock poisoned on add_or_update_batch(), recovering");
                poisoned.into_inner()
            }
        };
        for file in files {
            if let Some(pos) = inner.files.iter().position(|f| f.full_path == file.full_path) {
                inner.files[pos] = file;
            } else {
                inner.files.push(file);
            }
        }
        inner.rebuild_index();
    }

    /// Incremental remove for a single file.
    pub fn remove(&self, path: &str) {
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex write lock poisoned on remove(), recovering");
                poisoned.into_inner()
            }
        };
        let original_len = inner.files.len();
        inner.files.retain(|f| f.full_path != path);
        if inner.files.len() != original_len {
            inner.rebuild_index();
        }
    }

    /// Batch remove for multiple paths.
    pub fn remove_batch(&self, paths: &[String]) {
        if paths.is_empty() {
            return;
        }
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex write lock poisoned on remove_batch(), recovering");
                poisoned.into_inner()
            }
        };
        let original_len = inner.files.len();
        inner.files.retain(|f| !paths.iter().any(|p| f.full_path == *p));
        if inner.files.len() != original_len {
            inner.rebuild_index();
        }
    }

    /// Incremental folder prefix remove (removes a folder and all its contents recursively).
    pub fn remove_prefix(&self, prefix: &str) {
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex write lock poisoned on remove_prefix(), recovering");
                poisoned.into_inner()
            }
        };
        let original_len = inner.files.len();
        inner.files.retain(|f| !f.full_path.starts_with(prefix));
        if inner.files.len() != original_len {
            inner.rebuild_index();
        }
    }

    /// Batch prefix remove for multiple folders.
    pub fn remove_prefix_batch(&self, prefixes: &[String]) {
        if prefixes.is_empty() {
            return;
        }
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex write lock poisoned on remove_prefix_batch(), recovering");
                poisoned.into_inner()
            }
        };
        let original_len = inner.files.len();
        inner.files.retain(|f| !prefixes.iter().any(|prefix| f.full_path.starts_with(prefix)));
        if inner.files.len() != original_len {
            inner.rebuild_index();
        }
    }

    /// Returns the total number of files indexed.
    pub fn len(&self) -> usize {
        let inner = match self.inner.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex read lock poisoned on len(), recovering");
                poisoned.into_inner()
            }
        };
        inner.files.len()
    }

    /// Returns a copy of all files in the index.
    pub fn get_all(&self) -> Vec<FileMetadata> {
        let inner = match self.inner.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex read lock poisoned on get_all(), recovering");
                poisoned.into_inner()
            }
        };
        inner.files.clone()
    }

    /// Zero-copy candidate search that matches and filters directly under a single read lock,
    /// avoiding massive vector clones for unmatched file entries.
    pub fn search_and_match(&self, query: &crate::models::SearchQuery) -> (Vec<SearchResult>, Vec<FileMetadata>) {
        let inner = match self.inner.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex read lock poisoned on search_and_match(), recovering");
                poisoned.into_inner()
            }
        };
        let mut results = Vec::new();
        let mut matched_files = Vec::new();
        for file in &inner.files {
            if let Some(res) = crate::search::match_file(file, query) {
                results.push(res);
                matched_files.push(file.clone());
            }
        }
        (results, matched_files)
    }

    /// Performs an O(log N) prefix binary search.
    /// Returns copies of metadata for all files whose name starts with the given prefix.
    pub fn search_prefix(&self, prefix: &str) -> Vec<FileMetadata> {
        let prefix_lower = prefix.to_lowercase();
        let inner = match self.inner.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("MemoryIndex read lock poisoned on search_prefix(), recovering");
                poisoned.into_inner()
            }
        };

        if prefix_lower.is_empty() {
            return Vec::new();
        }

        // Binary search to find the lower bound of matching prefixes using zero-alloc precomputed lowercase names
        let start_idx = inner.sorted_by_name.partition_point(|&i| {
            inner.lowercase_names[i] < prefix_lower
        });

        let mut results = Vec::new();
        let mut idx = start_idx;

        // Iterate forward from the lower bound until prefixes diverge
        while idx < inner.sorted_by_name.len() {
            let file_idx = inner.sorted_by_name[idx];
            if inner.lowercase_names[file_idx].starts_with(&prefix_lower) {
                results.push(inner.files[file_idx].clone());
                idx += 1;
            } else {
                break;
            }
        }

        results
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
    fn test_memory_index_search_prefix() {
        let files = vec![
            mock_file("Google Chrome"),
            mock_file("Helium Browser"),
            mock_file("Discord"),
            mock_file("GitHub Desktop"),
            mock_file("Visual Studio Code"),
        ];

        let index = MemoryIndex::new(files);
        assert_eq!(index.len(), 5);

        // Test prefix matches
        let res = index.search_prefix("gi");
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "GitHub Desktop");

        let res2 = index.search_prefix("he");
        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].name, "Helium Browser");

        // Case insensitivity
        let res3 = index.search_prefix("VISUAL");
        assert_eq!(res3.len(), 1);
        assert_eq!(res3[0].name, "Visual Studio Code");

        // Non-matching
        let res4 = index.search_prefix("xyz");
        assert!(res4.is_empty());
    }

    #[test]
    fn test_memory_index_batch_operations() {
        let index = MemoryIndex::new(vec![mock_file("File1"), mock_file("File2")]);
        assert_eq!(index.len(), 2);

        // Batch add
        index.add_or_update_batch(vec![mock_file("File3"), mock_file("File4")]);
        assert_eq!(index.len(), 4);

        // Batch remove by prefix
        index.remove_prefix_batch(&["C:\\File1".to_string(), "C:\\File3".to_string()]);
        assert_eq!(index.len(), 2);
        let remaining = index.get_all();
        assert!(remaining.iter().any(|f| f.name == "File2"));
        assert!(remaining.iter().any(|f| f.name == "File4"));
    }
}
