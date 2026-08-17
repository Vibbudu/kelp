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
        if let Some(p) = crate::utilities::get_known_folder(&crate::utilities::FOLDERID_MUSIC) {
            paths.push(p.to_string_lossy().to_string());
        }
        if let Some(p) = crate::utilities::get_known_folder(&crate::utilities::FOLDERID_PICTURES) {
            paths.push(p.to_string_lossy().to_string());
        }
        if let Some(p) = crate::utilities::get_known_folder(&crate::utilities::FOLDERID_VIDEOS) {
            paths.push(p.to_string_lossy().to_string());
        }

        // Standard user profile paths as reliable supplements
        let user_dirs = [
            "%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs",
            "%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs",
            "%USERPROFILE%\\Desktop",
            "%USERPROFILE%\\Documents",
            "%USERPROFILE%\\Downloads",
            "%USERPROFILE%\\Music",
            "%USERPROFILE%\\Pictures",
            "%USERPROFILE%\\Videos",
        ];

        for dir in user_dirs {
            let expanded = crate::utilities::expand_env_vars(dir);
            let p = std::path::Path::new(&expanded);
            if p.exists() {
                let s = p.to_string_lossy().to_string();
                if !paths.iter().any(|existing| existing.eq_ignore_ascii_case(&s)) {
                    paths.push(s);
                }
            }
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
                
                let extension = if is_dir {
                    String::new()
                } else {
                    file_path
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_string().to_lowercase())
                        .unwrap_or_default()
                };

                let mut name = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                if name.is_empty() {
                    continue;
                }

                if extension == "lnk" && name.to_lowercase().ends_with(".lnk") {
                    name = name[..name.len() - 4].to_string();
                }

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

        // Clean up stale or newly-excluded files in DB
        info!("Running database clean up...");
        if let Ok(db_files) = self.storage.load_all_files() {
            let mut deleted_count = 0;
            for db_file in db_files {
                if db_file.full_path.starts_with("shell:") {
                    continue; // UWP cleanup handled below
                }

                let p = Path::new(&db_file.full_path);
                let belongs_to_scanned_dir = paths.iter().any(|raw| {
                    let expanded = expand_env_vars(raw);
                    db_file.full_path.starts_with(&expanded)
                });

                let should_delete = if !p.exists() || self.should_exclude(p) {
                    true
                } else if belongs_to_scanned_dir && !seen_paths.contains(&db_file.full_path) {
                    true
                } else {
                    false
                };

                if should_delete {
                    if let Err(e) = self.storage.delete_file(&db_file.full_path) {
                        warn!("Failed to delete stale file {:?} from DB: {:?}", db_file.full_path, e);
                    } else {
                        deleted_count += 1;
                    }
                }
            }
            if deleted_count > 0 {
                info!("Cleaned up {} stale/excluded entries from database.", deleted_count);
            }
        }

        // Discover, deduplicate, and clean-resync UWP / Microsoft Store apps
        match Self::discover_uwp_apps() {
            Ok(uwp_apps) => {
                // Build set of known Application and Shortcut names from filesystem scan
                let existing_app_names: HashSet<String> = if let Ok(db_files) = self.storage.load_all_files() {
                    db_files
                        .iter()
                        .filter(|f| (f.file_type == FileType::Application || f.file_type == FileType::Shortcut) && !f.full_path.starts_with("shell:"))
                        .map(|f| f.name.to_lowercase().trim().to_string())
                        .collect()
                } else {
                    HashSet::new()
                };

                let deduplicated: Vec<FileMetadata> = uwp_apps
                    .into_iter()
                    .filter(|app| {
                        let name_lower = app.name.to_lowercase().trim().to_string();
                        if existing_app_names.contains(&name_lower) {
                            info!("Skipping duplicate UWP app '{}' (already found via filesystem)", app.name);
                            false
                        } else {
                            true
                        }
                    })
                    .collect();

                // Cleanly wipe all old UWP records and insert deduplicated set
                let _ = self.storage.delete_all_uwp_apps();

                let uwp_count = deduplicated.len();
                if !deduplicated.is_empty() {
                    if let Err(e) = self.storage.save_files(&deduplicated) {
                        warn!("Failed to save UWP apps to DB: {:?}", e);
                    } else {
                        total_indexed += uwp_count;
                        info!("Discovered and indexed {} UWP/Store apps ({} duplicates skipped).", uwp_count, existing_app_names.len());
                    }
                }
            }
            Err(e) => {
                warn!("UWP app discovery failed: {}", e);
            }
        }

        info!("Indexing complete. Total items indexed: {}", total_indexed);
        Ok(total_indexed)
    }

    /// Discover UWP / Microsoft Store apps via PowerShell Get-StartApps
    fn discover_uwp_apps() -> Result<Vec<FileMetadata>, String> {
        #[cfg(target_os = "windows")]
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-StartApps | ConvertTo-Json"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("PowerShell exited with error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json_val: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse Get-StartApps JSON: {}", e))?;

        let apps_array = match json_val {
            serde_json::Value::Array(arr) => arr,
            single @ serde_json::Value::Object(_) => vec![single],
            _ => return Ok(Vec::new()),
        };

        let mut results = Vec::new();
        for app in apps_array {
            let name = app.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let app_id = app.get("AppID").and_then(|v| v.as_str()).unwrap_or("").to_string();

            if name.is_empty() || app_id.is_empty() {
                continue;
            }

            let full_path = format!("shell:AppsFolder\\{}", app_id);

            results.push(FileMetadata {
                id: None,
                name,
                extension: String::new(),
                parent_folder: "UWP Apps".to_string(),
                full_path,
                modified_date: 0,
                size: 0,
                file_type: FileType::Application,
            });
        }

        info!("PowerShell Get-StartApps returned {} apps.", results.len());
        Ok(results)
    }

    /// Exclude typical noisy development or hidden system paths
    fn should_exclude(&self, path: &Path) -> bool {
        crate::utilities::should_exclude_path(path, &self.config.supported_extensions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_junction_exclusion() {
        let exts = vec!["exe".to_string(), "pdf".to_string(), "txt".to_string()];
        
        // Legacy Windows junction points in Documents MUST be excluded
        assert!(crate::utilities::should_exclude_path(Path::new("C:\\Users\\user\\Documents\\My Music"), &exts));
        assert!(crate::utilities::should_exclude_path(Path::new("C:\\Users\\user\\Documents\\My Pictures"), &exts));
        assert!(crate::utilities::should_exclude_path(Path::new("C:\\Users\\user\\Documents\\My Videos"), &exts));
        assert!(crate::utilities::should_exclude_path(Path::new("C:\\Users\\user\\My Documents"), &exts));
        assert!(crate::utilities::should_exclude_path(Path::new("C:\\Users\\user\\Application Data"), &exts));
        assert!(crate::utilities::should_exclude_path(Path::new("C:\\Users\\user\\Local Settings"), &exts));
        
        // Noisy folders MUST be excluded
        assert!(crate::utilities::should_exclude_path(Path::new("C:\\project\\node_modules\\pkg"), &exts));
        assert!(crate::utilities::should_exclude_path(Path::new("C:\\project\\.git\\objects"), &exts));
        assert!(crate::utilities::should_exclude_path(Path::new("C:\\project\\target\\debug"), &exts));
    }

    #[test]
    fn test_check_music_folder() {
        let paths = Indexer::default_windows_paths();
        assert!(paths.iter().any(|p| p.to_lowercase().ends_with("\\music")));
    }
}
