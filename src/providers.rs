use crate::models::{FileMetadata, FileType, SearchQuery, SearchResult};
use crate::search::match_file;

/// Base trait for search providers. Allows extending search functionality
/// with new result sources (e.g. plugins, clipboard history).
pub trait SearchProvider: Send + Sync {
    /// Return the name of the provider.
    fn name(&self) -> &'static str;

    /// Evaluates the query against a set of candidate files and returns matches.
    fn search(&self, candidates: &[FileMetadata], query: &SearchQuery) -> Vec<SearchResult>;
}

/// Provider for executable applications (.exe)
pub struct ApplicationProvider;
impl SearchProvider for ApplicationProvider {
    fn name(&self) -> &'static str {
        "Application"
    }

    fn search(&self, candidates: &[FileMetadata], query: &SearchQuery) -> Vec<SearchResult> {
        candidates
            .iter()
            .filter(|f| f.file_type == FileType::Application)
            .filter_map(|f| match_file(f, query))
            .collect()
    }
}

/// Provider for desktop and start menu shortcuts (.lnk)
pub struct ShortcutProvider;
impl SearchProvider for ShortcutProvider {
    fn name(&self) -> &'static str {
        "Shortcut"
    }

    fn search(&self, candidates: &[FileMetadata], query: &SearchQuery) -> Vec<SearchResult> {
        candidates
            .iter()
            .filter(|f| f.file_type == FileType::Shortcut)
            .filter_map(|f| match_file(f, query))
            .collect()
    }
}

/// Provider for folders/directories
pub struct FolderProvider;
impl SearchProvider for FolderProvider {
    fn name(&self) -> &'static str {
        "Folder"
    }

    fn search(&self, candidates: &[FileMetadata], query: &SearchQuery) -> Vec<SearchResult> {
        candidates
            .iter()
            .filter(|f| f.file_type == FileType::Folder)
            .filter_map(|f| match_file(f, query))
            .collect()
    }
}

/// Provider for general files (documents, downloads, user files)
pub struct FileProvider;
impl SearchProvider for FileProvider {
    fn name(&self) -> &'static str {
        "File"
    }

    fn search(&self, candidates: &[FileMetadata], query: &SearchQuery) -> Vec<SearchResult> {
        candidates
            .iter()
            .filter(|f| f.file_type == FileType::File)
            .filter_map(|f| match_file(f, query))
            .collect()
    }
}
