use crate::models::{FileMetadata, FileType};
use rusqlite::{params, Connection, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

#[derive(Clone)]
pub struct Storage {
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    /// Opens the SQLite database connection and runs migrations.
    pub fn new(db_path: &Path) -> Result<Self> {
        // Create parent directories if they don't exist
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                    warn!("Failed to create database directory: {:?}", e);
                });
            }
        }

        let conn = Connection::open(db_path)?;
        
        // Enable WAL mode for performance
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Acquires the database connection, recovering from poison if needed.
    fn get_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        match self.conn.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Storage Mutex poisoned, recovering");
                poisoned.into_inner()
            }
        }
    }

    /// Initializes the SQLite tables if they do not exist.
    pub fn init_schema(&self) -> Result<()> {
        let conn = self.get_conn();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                extension TEXT NOT NULL,
                parent_folder TEXT NOT NULL,
                full_path TEXT NOT NULL UNIQUE,
                modified_date INTEGER NOT NULL,
                size INTEGER NOT NULL,
                file_type TEXT NOT NULL
            );",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_files_path ON files(full_path);",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_files_name ON files(name);",
            [],
        )?;

        // Search history for selection logging
        conn.execute(
            "CREATE TABLE IF NOT EXISTS search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                selected_path TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            );",
            [],
        )?;

        // Frequencies for selection counts
        conn.execute(
            "CREATE TABLE IF NOT EXISTS query_frequencies (
                query TEXT NOT NULL,
                selected_path TEXT NOT NULL,
                selection_count INTEGER NOT NULL DEFAULT 1,
                last_selected_at INTEGER NOT NULL,
                PRIMARY KEY (query, selected_path)
            );",
            [],
        )?;

        info!("SQLite database schema initialized.");
        Ok(())
    }

    /// Saves a batch of files into the database.
    pub fn save_files(&self, files: &[FileMetadata]) -> Result<()> {
        let mut conn = self.get_conn();
        let tx = conn.transaction()?;
        
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO files (name, extension, parent_folder, full_path, modified_date, size, file_type)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
            )?;
            
            for file in files {
                stmt.execute(params![
                    file.name,
                    file.extension,
                    file.parent_folder,
                    file.full_path,
                    file.modified_date,
                    file.size,
                    file.file_type.as_str()
                ])?;
            }
        }
        
        tx.commit()?;
        Ok(())
    }

    /// Saves a single file to the database.
    pub fn save_file(&self, file: &FileMetadata) -> Result<()> {
        let conn = self.get_conn();
        conn.execute(
            "INSERT OR REPLACE INTO files (name, extension, parent_folder, full_path, modified_date, size, file_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                file.name,
                file.extension,
                file.parent_folder,
                file.full_path,
                file.modified_date,
                file.size,
                file.file_type.as_str()
            ],
        )?;
        Ok(())
    }

    /// Deletes a file from the database.
    pub fn delete_file(&self, path: &str) -> Result<()> {
        let conn = self.get_conn();
        conn.execute("DELETE FROM files WHERE full_path = ?1", params![path])?;
        Ok(())
    }

    /// Recursively deletes files starting with a folder path (for folder deletes).
    pub fn delete_folder_recursive(&self, folder_path: &str) -> Result<()> {
        let conn = self.get_conn();
        let folder_prefix = format!("{}\\", folder_path.trim_end_matches('\\'));
        conn.execute(
            "DELETE FROM files WHERE full_path = ?1 OR full_path LIKE ?2",
            params![folder_path, format!("{}%", folder_prefix)],
        )?;
        Ok(())
    }

    /// Loads all file metadata from the database into memory.
    pub fn load_all_files(&self) -> Result<Vec<FileMetadata>> {
        let conn = self.get_conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, extension, parent_folder, full_path, modified_date, size, file_type FROM files"
        )?;
        
        let rows = stmt.query_map([], |row| {
            let file_type_str: String = row.get(7)?;
            let file_type = file_type_str.parse::<FileType>().unwrap_or(FileType::File);
            Ok(FileMetadata {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                extension: row.get(2)?,
                parent_folder: row.get(3)?,
                full_path: row.get(4)?,
                modified_date: row.get(5)?,
                size: row.get(6)?,
                file_type,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Records that a user selected a specific path for a query.
    pub fn record_selection(&self, query: &str, path: &str, timestamp: i64) -> Result<()> {
        let conn = self.get_conn();
        
        // Log to history
        conn.execute(
            "INSERT INTO search_history (query, selected_path, timestamp) VALUES (?1, ?2, ?3)",
            params![query, path, timestamp],
        )?;

        // Update or insert into query_frequencies
        conn.execute(
            "INSERT INTO query_frequencies (query, selected_path, selection_count, last_selected_at)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(query, selected_path) DO UPDATE SET
                selection_count = selection_count + 1,
                last_selected_at = ?3",
            params![query, path, timestamp],
        )?;

        Ok(())
    }

    /// Retrieves all query frequencies data.
    pub fn get_learning_data(&self) -> Result<Vec<(String, String, i64, i64)>> {
        let conn = self.get_conn();
        let mut stmt = conn.prepare(
            "SELECT query, selected_path, selection_count, last_selected_at FROM query_frequencies"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}
