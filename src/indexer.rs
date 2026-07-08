use crate::models::{FileMetadata, FileType};
use crate::storage::Storage;
use crate::utilities::{expand_env_vars, resolve_lnk};
use std::collections::HashSet;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tracing::{info, warn, error};
use walkdir::WalkDir;

pub struct Indexer {
    storage: Storage,
    config: crate::config::AppConfig,
}

impl Indexer {
    pub fn new(storage: Storage, config: crate::config::AppConfig) -> Self {
        Self { storage, config }
    }

    /// Default paths to index on Windows
    pub fn default_windows_paths() -> Vec<String> {
        let mut paths = Vec::new();
        
        // Resolve actual paths using Windows Known Folders API
        if let Some(p) = crate::utilities::get_known_folder(&crate::utilities::FOLDERID_PROGRAMS) {
            paths.push(p.to_string_lossy().to_string());
        }
        if let Some(p) = crate::utilities::get_known_folder(&crate::utilities::FOLDERID_COMMON_PROGRAMS) {
            paths.push(p.to_string_lossy().to_string());
        }
        if let Some(p) = crate::utilities::get_known_folder(&crate::utilities::FOLDERID_DESKTOP) {
            paths.push(p.to_string_lossy().to_string());
        }
        if let Some(p) = crate::utilities::get_known_folder(&crate::utilities::FOLDERID_DOCUMENTS) {
            paths.push(p.to_string_lossy().to_string());
        }
        if let Some(p) = crate::utilities::get_known_folder(&crate::utilities::FOLDERID_DOWNLOADS) {
            paths.push(p.to_string_lossy().to_string());
        }

        // Fallbacks if Shell API fails (extremely rare)
        if paths.is_empty() {
            paths.push(crate::utilities::expand_env_vars("%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs"));
            paths.push(crate::utilities::expand_env_vars("%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs"));
            paths.push(crate::utilities::expand_env_vars("%USERPROFILE%\\Desktop"));
            paths.push(crate::utilities::expand_env_vars("%USERPROFILE%\\Documents"));
            paths.push(crate::utilities::expand_env_vars("%USERPROFILE%\\Downloads"));
        }
        
        paths
    }

    /// Scans directories, updates new/modified files in SQLite, and cleans up deleted files.
    pub fn index_paths(&self, paths: &[String]) -> Result<usize, String> {
        info!("Starting indexing process...");
        let mut total_indexed = 0;
        let mut seen_paths = HashSet::new();

        for raw_path in paths {
            let expanded = expand_env_vars(raw_path);
            let path = Path::new(&expanded);
            if !path.exists() {
                warn!("Path does not exist, skipping: {:?}", path);
                continue;
            }

            info!("Scanning path: {:?}", path);
            let mut batch = Vec::new();
            
            // Walk dir, don't follow symlinks/junctions to avoid cycles
            let walker = WalkDir::new(path)
                .follow_links(false)
                .into_iter();

            for entry in walker {
                let entry = match entry {
                    Ok(e) => e,
                    Err(err) => {
                        // Skip entries we can't access
                        warn!("Error walking entry: {:?}", err);
                        continue;
                    }
                };

                let file_path = entry.path();
                
                // Exclude noisy developer / system directories
                if self.should_exclude(file_path) {
                    continue;
                }

                let full_path_str = file_path.to_string_lossy().to_string();
                seen_paths.insert(full_path_str.clone());

                // Read metadata
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("Could not read metadata for {:?}: {:?}", file_path, e);
                        continue;
                    }
                };

                let is_dir = metadata.is_dir();
                
                let name = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                if name.is_empty() {
                    continue;
                }

                let extension = if is_dir {
                    String::new()
                } else {
                    file_path
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_string().to_lowercase())
                        .unwrap_or_default()
                };

                let parent_folder = file_path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let modified_date = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                let size = if is_dir { 0 } else { metadata.len() as i64 };

                // Determine file type
                let file_type = if is_dir {
                    FileType::Folder
                } else if extension == "exe" {
                    FileType::Application
                } else if extension == "lnk" {
                    // Check if shortcut target is exe
                    if let Some(target) = resolve_lnk(file_path) {
                        if target.extension().map_or(false, |ext| ext == "exe") {
                            FileType::Application
                        } else {
                            FileType::Shortcut
                        }
                    } else {
                        FileType::Shortcut
                    }
                } else {
                    FileType::File
                };

                // Add to batch
                batch.push(FileMetadata {
                    id: None,
                    name,
                    extension,
                    parent_folder,
                    full_path: full_path_str,
                    modified_date,
                    size,
                    file_type,
                });

                // Write in batches of 1000
                if batch.len() >= 1000 {
                    total_indexed += batch.len();
                    if let Err(e) = self.storage.save_files(&batch) {
                        error!("Failed to save batch to DB: {:?}", e);
                        return Err(e.to_string());
                    }
                    batch.clear();
                }
            }

            // Save remaining batch
            if !batch.is_empty() {
                total_indexed += batch.len();
                if let Err(e) = self.storage.save_files(&batch) {
                    error!("Failed to save final batch to DB: {:?}", e);
                    return Err(e.to_string());
                }
            }
        }

        // Clean up stale files in DB that are no longer present on disk
        info!("Running database clean up...");
        if let Ok(db_files) = self.storage.load_all_files() {
            let mut deleted_count = 0;
            for db_file in db_files {
                // If the file is in a directory we scanned, but we did not see it during this scan:
                let belongs_to_scanned_dir = paths.iter().any(|p| {
                    let expanded = expand_env_vars(p);
                    db_file.full_path.starts_with(&expanded)
                });

                if belongs_to_scanned_dir && !seen_paths.contains(&db_file.full_path) {
                    if let Err(e) = self.storage.delete_file(&db_file.full_path) {
                        warn!("Failed to delete stale file {:?} from DB: {:?}", db_file.full_path, e);
                    } else {
                        deleted_count += 1;
                    }
                }
            }
            if deleted_count > 0 {
                info!("Cleaned up {} stale entries from database.", deleted_count);
            }
        }

        info!("Indexing complete. Total items indexed: {}", total_indexed);
        Ok(total_indexed)
    }

    /// Exclude typical noisy development or hidden system paths
    fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        // 1. Exclude directories matching these patterns
        let exclusions = [
            "\\node_modules\\",
            "\\.git\\",
            "\\target\\",
            "\\AppData\\Local\\Temp",
            "\\AppData\\Roaming\\npm-cache",
            "\\.cargo\\",
            "\\.rustup\\",
            "\\$RECYCLE.BIN",
            "\\System Volume Information",
            "\\Local Settings\\Temporary Internet Files",
            "\\Windows\\WinSxS",
            "\\Windows\\System32",
        ];

        for excl in &exclusions {
            if path_str.contains(excl) {
                return true;
            }
        }

        // 2. Filter out files whose extensions are NOT in the supported whitelist
        let is_dir = path.is_dir();
        if !is_dir {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !self.config.supported_extensions.contains(&ext_str) {
                    return true;
                }
            } else {
                return true; // Exclude files with no extensions
            }
        }

        // 3. Filter out hidden system files (check attributes on Windows)
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::MetadataExt;
            if let Ok(metadata) = path.metadata() {
                let attributes = metadata.file_attributes();
                // 0x2 is FILE_ATTRIBUTE_HIDDEN, 0x4 is FILE_ATTRIBUTE_SYSTEM
                if (attributes & 0x2) != 0 || (attributes & 0x4) != 0 {
                    return true;
                }
            }
        }

        false
    }
}
