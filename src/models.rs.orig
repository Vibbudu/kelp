use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Application,
    Shortcut,
    Folder,
    File,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Application => "Application",
            FileType::Shortcut => "Shortcut",
            FileType::Folder => "Folder",
            FileType::File => "File",
        }
    }
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for FileType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Application" => Ok(FileType::Application),
            "Shortcut" => Ok(FileType::Shortcut),
            "Folder" => Ok(FileType::Folder),
            "File" => Ok(FileType::File),
            _ => Err(format!("Unknown file type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: Option<i64>,
    pub name: String,
    pub extension: String,
    pub parent_folder: String,
    pub full_path: String,
    pub modified_date: i64, // Unix timestamp in seconds
    pub size: i64,          // In bytes
    pub file_type: FileType,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub raw: String,
    pub extension_filter: Option<String>, // e.g. Some("pdf") if they typed ".pdf"
    pub terms: Vec<String>,               // Remaining terms, e.g. ["tax", "report"]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub metadata: FileMetadata,
    pub score: f64,
    pub match_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
}
