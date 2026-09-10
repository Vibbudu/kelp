use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Move a single file or directory to a destination directory.
/// Uses atomic std::fs::rename first, falling back to recursive copy + remove for cross-volume moves.
pub fn move_entry(src: &Path, dst_dir: &Path, overwrite: bool) -> Result<PathBuf, String> {
    if !src.exists() {
        return Err(format!("Source path does not exist: {:?}", src));
    }
    if !dst_dir.is_dir() {
        return Err(format!("Destination is not a valid directory: {:?}", dst_dir));
    }

    let file_name = src
        .file_name()
        .ok_or_else(|| "Invalid source path filename".to_string())?;

    let target_path = if overwrite {
        dst_dir.join(file_name)
    } else {
        generate_collision_free_path(dst_dir, file_name)
    };

    // 1. Try atomic rename (super fast O(1) on same drive/volume)
    match fs::rename(src, &target_path) {
        Ok(_) => {
            info!("Moved '{:?}' to '{:?}' via atomic rename", src, target_path);
            Ok(target_path)
        }
        Err(e) => {
            // Check for cross-volume move (Windows ERROR_NOT_SAME_DEVICE = 17)
            let is_cross_device = e.raw_os_error() == Some(17)
                || e.kind() == io::ErrorKind::CrossesDevices;

            if is_cross_device {
                info!("Cross-volume move detected for '{:?}'. Falling back to copy + delete.", src);
                copy_and_delete_entry(src, &target_path)?;
                Ok(target_path)
            } else {
                warn!("Rename failed: {:?} (code: {:?})", e, e.raw_os_error());
                Err(format!("Move failed: {}", e))
            }
        }
    }
}

/// Helper to copy recursively and delete original on success.
fn copy_and_delete_entry(src: &Path, dst: &Path) -> Result<(), String> {
    if src.is_dir() {
        copy_dir_all(src, dst).map_err(|e| format!("Failed to copy directory: {}", e))?;
        fs::remove_dir_all(src).map_err(|e| format!("Copied to destination, but failed to remove original: {}", e))?;
    } else {
        fs::copy(src, dst).map_err(|e| format!("Failed to copy file: {}", e))?;
        fs::remove_file(src).map_err(|e| format!("Copied to destination, but failed to remove original: {}", e))?;
    }
    Ok(())
}

/// Recursively copies a directory tree.
fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target_item = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target_item)?;
        } else {
            fs::copy(entry.path(), &target_item)?;
        }
    }
    Ok(())
}

/// Generates a unique collision-free path like "name (1).ext".
fn generate_collision_free_path(dst_dir: &Path, file_name: &std::ffi::OsStr) -> PathBuf {
    let base_path = dst_dir.join(file_name);
    if !base_path.exists() {
        return base_path;
    }

    let file_path = Path::new(file_name);
    let stem = file_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = file_path.extension().map(|e| e.to_string_lossy().to_string());

    let mut counter = 1;
    loop {
        let candidate_name = match &ext {
            Some(extension) => format!("{} ({}).{}", stem, counter, extension),
            None => format!("{} ({})", stem, counter),
        };
        let candidate_path = dst_dir.join(candidate_name);
        if !candidate_path.exists() {
            return candidate_path;
        }
        counter += 1;
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MoveSummary {
    pub moved: Vec<String>,
    pub failed: Vec<(String, String)>,
}

/// Moves multiple files or folders to a target directory.
pub fn move_entries(sources: &[String], destination: &str) -> MoveSummary {
    let dst_path = Path::new(destination);
    let mut moved = Vec::new();
    let mut failed = Vec::new();

    for src_str in sources {
        let src_path = Path::new(src_str);
        match move_entry(src_path, dst_path, false) {
            Ok(new_path) => {
                moved.push(new_path.to_string_lossy().to_string());
            }
            Err(e) => {
                failed.push((src_str.clone(), e));
            }
        }
    }

    MoveSummary { moved, failed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_single_file() {
        let temp_dir = std::env::temp_dir().join("kelp_test_move_single");
        let _ = fs::remove_dir_all(&temp_dir);
        let src_dir = temp_dir.join("src");
        let dst_dir = temp_dir.join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        let file_path = src_dir.join("sample.txt");
        fs::write(&file_path, "test content").unwrap();

        let res = move_entry(&file_path, &dst_dir, false);
        assert!(res.is_ok());
        let new_path = res.unwrap();
        assert!(new_path.exists());
        assert!(!file_path.exists());
        assert_eq!(fs::read_to_string(&new_path).unwrap(), "test content");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_move_collision_rename() {
        let temp_dir = std::env::temp_dir().join("kelp_test_move_col");
        let _ = fs::remove_dir_all(&temp_dir);
        let src_dir = temp_dir.join("src");
        let dst_dir = temp_dir.join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        // Existing file in dst
        fs::write(dst_dir.join("doc.txt"), "original").unwrap();

        // New file in src with same name
        let src_file = src_dir.join("doc.txt");
        fs::write(&src_file, "new version").unwrap();

        let res = move_entry(&src_file, &dst_dir, false).unwrap();
        assert_eq!(res.file_name().unwrap(), "doc (1).txt");
        assert!(res.exists());
        assert!(dst_dir.join("doc.txt").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_move_folder_recursive() {
        let temp_dir = std::env::temp_dir().join("kelp_test_move_dir");
        let _ = fs::remove_dir_all(&temp_dir);
        let src_dir = temp_dir.join("src_folder");
        let dst_dir = temp_dir.join("dst_parent");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        fs::write(src_dir.join("nested.txt"), "nested data").unwrap();

        let res = move_entry(&src_dir, &dst_dir, false).unwrap();
        assert!(res.exists());
        assert!(res.join("nested.txt").exists());
        assert!(!src_dir.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
