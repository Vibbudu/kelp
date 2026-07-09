use crate::learning::LearningEngine;
use crate::models::{FileType, SearchQuery, SearchResult};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Trait defining a ranking factor.
/// Allows adding new ranking signals without modifying core ranking code.
pub trait RankingSignal: Send + Sync {
    /// Evaluates the signal score for a search result.
    fn score(&self, result: &SearchResult, query: &SearchQuery) -> f64;
}

/// Evaluates the base structural matching score (Exact, Prefix, Contains, Fuzzy, etc.)
pub struct BaseMatchSignal;
impl RankingSignal for BaseMatchSignal {
    fn score(&self, result: &SearchResult, _query: &SearchQuery) -> f64 {
        // Boost match quality dominance
        result.score * 1.5
    }
}

/// Evaluates category/file type priority tiers
pub struct CategorySignal;
impl RankingSignal for CategorySignal {
    fn score(&self, result: &SearchResult, _query: &SearchQuery) -> f64 {
        match result.metadata.file_type {
            FileType::Application => 0.30,
            FileType::Shortcut => 0.25,
            FileType::Folder => 0.05,
            FileType::File => 0.00,
        }
    }
}

/// Evaluates user learning (frequency and recency) boosts
pub struct HistorySignal {
    learning: Arc<LearningEngine>,
}

impl HistorySignal {
    pub fn new(learning: Arc<LearningEngine>) -> Self {
        Self { learning }
    }
}

impl RankingSignal for HistorySignal {
    fn score(&self, result: &SearchResult, query: &SearchQuery) -> f64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        if let Some((count, last_time)) = self.learning.lookup(&query.raw, &result.metadata.full_path) {
            // Frequency boost: logarithmic scaling, maxing out at 10 selections
            let f = ((count as f64).min(10.0) + 1.0).ln() / (11.0f64).ln();

            // Recency boost: decays based on elapsed time (half-life of 1 day)
            let diff = (now - last_time).max(0) as f64;
            let r = 1.0 / (1.0 + diff / 86400.0);

            // HistoryScore = (0.3 * frequency) + (0.2 * recency)
            (0.3 * f) + (0.2 * r)
        } else {
            0.0
        }
    }
}

/// Evaluates short path bonus (files/folders closer to root folders)
pub struct ShortPathSignal;
impl RankingSignal for ShortPathSignal {
    fn score(&self, result: &SearchResult, _query: &SearchQuery) -> f64 {
        let path = &result.metadata.full_path;
        let depth = path.chars().filter(|&c| c == '\\' || c == '/').count();
        0.10 * (1.0 - (depth as f64 / 10.0).min(1.0))
    }
}

/// Evaluates filename length compared to query length
pub struct NameLengthSignal;
impl RankingSignal for NameLengthSignal {
    fn score(&self, result: &SearchResult, _query: &SearchQuery) -> f64 {
        let name = &result.metadata.name;
        0.05 * (1.0 - (name.len() as f64 / 60.0).min(1.0))
    }
}

/// Evaluates system application status to boost built-in apps and terminal utilities
pub struct SystemAppSignal;
impl RankingSignal for SystemAppSignal {
    fn score(&self, result: &SearchResult, _query: &SearchQuery) -> f64 {
        if result.metadata.file_type == FileType::Application {
            let path_lower = result.metadata.full_path.to_lowercase();
            if path_lower.contains("\\windows\\")
                || path_lower.ends_with("explorer.exe")
                || path_lower.ends_with("calc.exe")
                || path_lower.ends_with("notepad.exe")
                || path_lower.ends_with("cmd.exe")
                || path_lower.ends_with("powershell.exe")
            {
                return 0.05;
            }
        }
        0.0
    }
}

pub struct RankingEngine {
    signals: Vec<Box<dyn RankingSignal>>,
}

impl RankingEngine {
    /// Creates a RankingEngine with a customized set of signals.
    pub fn new(signals: Vec<Box<dyn RankingSignal>>) -> Self {
        Self { signals }
    }

    /// Default configuration combining match type, category, history, short path, name length, and system app.
    pub fn default_config(learning: Arc<LearningEngine>) -> Self {
        Self {
            signals: vec![
                Box::new(BaseMatchSignal),
                Box::new(CategorySignal),
                Box::new(HistorySignal::new(learning)),
                Box::new(ShortPathSignal),
                Box::new(NameLengthSignal),
                Box::new(SystemAppSignal),
            ],
        }
    }

    /// Ranks results in-place based on all registered signals.
    pub fn rank(&self, results: &mut [SearchResult], query: &SearchQuery) {
        for result in results.iter_mut() {
            let mut final_score = 0.0;
            for signal in &self.signals {
                final_score += signal.score(result, query);
            }
            result.score = final_score;
        }

        // Sort descending by final score. On tie, sort alphabetically, then path length
        results.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.metadata.name.to_lowercase().cmp(&b.metadata.name.to_lowercase()))
                .then_with(|| a.metadata.full_path.len().cmp(&b.metadata.full_path.len()))
                .then_with(|| a.metadata.full_path.cmp(&b.metadata.full_path))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FileMetadata, FileType};
    use crate::search::{parse_query, match_file};
    use crate::learning::LearningEngine;
    use crate::storage::Storage;

    fn mock_folder(name: &str) -> FileMetadata {
        FileMetadata {
            id: None,
            name: name.to_string(),
            extension: String::new(),
            parent_folder: "C:\\".to_string(),
            full_path: format!("C:\\{}", name),
            modified_date: 0,
            size: 0,
            file_type: FileType::Folder,
        }
    }

    #[test]
    fn test_aadhar_search_ranking() {
        let folder = mock_folder("Aadhar");
        let query_exact = parse_query("aadhar");
        let query_fuzzy = parse_query("adhr");

        let res_exact = match_file(&folder, &query_exact).expect("Should match exact");
        let res_fuzzy = match_file(&folder, &query_fuzzy).expect("Should match fuzzy");

        assert_eq!(res_exact.match_type, "Exact");
        assert_eq!(res_fuzzy.match_type, "Fuzzy");

        let storage = Storage::new(std::path::Path::new(":memory:")).unwrap();
        let learning = Arc::new(LearningEngine::new(storage));
        let ranker = RankingEngine::default_config(learning);

        // Test ranking under exact query
        let mut results = vec![res_fuzzy.clone(), res_exact.clone()];
        ranker.rank(&mut results, &query_exact);

        println!("--- RANKED RESULTS FOR QUERY 'aadhar' ---");
        for r in &results {
            println!("  - name='{}', score={}, match_type={}", r.metadata.name, r.score, r.match_type);
        }

        assert_eq!(results[0].metadata.name, "Aadhar");
        assert_eq!(results[0].match_type, "Exact");
        assert!(results[0].score > results[1].score);
    }
}
