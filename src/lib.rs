pub mod indexer;
pub mod config;
pub mod logger;
pub mod learning;
pub mod memory_index;
pub mod models;
pub mod providers;
pub mod query_parser;
pub mod ranking_engine;
pub mod result_cache;
pub mod search;
pub mod search_engine;
pub mod storage;
pub mod ui_bridge;
pub mod utilities;
pub mod watcher;

// Re-export major structs for convenience
pub use crate::models::{FileMetadata, FileType, SearchQuery, SearchResult};
pub use crate::ui_bridge::UIBridge;
