use crate::models::{FileMetadata, SearchQuery, SearchResult};

/// Parses a raw search string into a structured `SearchQuery`.
///
/// Example:
/// - ".pdf report" -> SearchQuery { raw: ".pdf report", extension_filter: Some("pdf"), terms: ["report"] }
/// - "vsc" -> SearchQuery { raw: "vsc", extension_filter: None, terms: ["vsc"] }
pub fn parse_query(raw: &str) -> SearchQuery {
    let mut extension_filter = None;
    let mut terms = Vec::new();

    for word in raw.split_whitespace() {
        if word.starts_with('.') && word.len() > 1 {
            // Treat as extension filter (strip the leading dot)
            extension_filter = Some(word[1..].to_lowercase());
        } else {
            terms.push(word.to_string());
        }
    }

    SearchQuery {
        raw: raw.trim().to_string(),
        extension_filter,
        terms,
    }
}

/// Matches a single file against the search query, returning a `SearchResult` if it matches.
pub fn match_file(file: &FileMetadata, query: &SearchQuery) -> Option<SearchResult> {
    // 1. Check extension filter if present
    if let Some(ref ext_filter) = query.extension_filter {
        if file.extension.to_lowercase() != *ext_filter {
            return None;
        }
        // If query was ONLY the extension (e.g. ".pdf"), match with perfect score
        if query.terms.is_empty() {
            return Some(SearchResult {
                metadata: file.clone(),
                score: 1.0,
                match_type: "Extension".to_string(),
                icon_base64: None,
            });
        }
    }

    // If query has no terms (but had extension filter that passed), we already returned.
    // If query is empty altogether, no match.
    if query.terms.is_empty() && query.extension_filter.is_none() {
        return None;
    }

    // Join remaining terms to search within the file name
    let search_term = query.terms.join(" ");
    if search_term.is_empty() {
        return None;
    }

    // Perform matching algorithms in order of precedence:
    // 1. Exact Match (Score: 1.0)
    // 2. Prefix Match (Score: 0.8 - 0.9)
    // 3. Acronym Match (Score: 0.75)
    // 4. Camel Case Match (Score: 0.70)
    // 5. Contains Match (Score: 0.5 - 0.7)
    // 6. Fuzzy Match (Score: 0.1 - 0.9)

    let name = &file.name;
    let name_without_ext = if let Some(dot_idx) = file.name.rfind('.') {
        &file.name[..dot_idx]
    } else {
        &file.name
    };

    let mut best_score = 0.0;
    let mut best_match_type = String::new();

    if is_exact_match(name, &search_term) || is_exact_match(name_without_ext, &search_term) {
        best_score = 1.0;
        best_match_type = "Exact".to_string();
    }

    if best_score < 0.9 {
        if is_prefix_match(name, &search_term) {
            let ratio = search_term.len() as f64 / name.len() as f64;
            let score = 0.8 + 0.1 * ratio;
            if score > best_score {
                best_score = score;
                best_match_type = "Prefix".to_string();
            }
        }
        if is_prefix_match(name_without_ext, &search_term) {
            let ratio = search_term.len() as f64 / name_without_ext.len() as f64;
            let score = 0.8 + 0.1 * ratio;
            if score > best_score {
                best_score = score;
                best_match_type = "Prefix".to_string();
            }
        }
    }

    if best_score < 0.75 && (is_acronym_match(name, &search_term) || is_acronym_match(name_without_ext, &search_term)) {
        best_score = 0.75;
        best_match_type = "Acronym".to_string();
    }

    if best_score < 0.70 && (is_camel_case_match(name, &search_term) || is_camel_case_match(name_without_ext, &search_term)) {
        best_score = 0.70;
        best_match_type = "CamelCase".to_string();
    }

    if best_score < 0.70 {
        if is_contains_match(name, &search_term) {
            let ratio = search_term.len() as f64 / name.len() as f64;
            let score = 0.5 + 0.2 * ratio;
            if score > best_score {
                best_score = score;
                best_match_type = "Contains".to_string();
            }
        }
        if is_contains_match(name_without_ext, &search_term) {
            let ratio = search_term.len() as f64 / name_without_ext.len() as f64;
            let score = 0.5 + 0.2 * ratio;
            if score > best_score {
                best_score = score;
                best_match_type = "Contains".to_string();
            }
        }
    }

    if best_score < 0.9 {
        if let Some(fuzzy_score) = compute_fuzzy_match(name, &search_term) {
            if fuzzy_score > best_score {
                best_score = fuzzy_score;
                best_match_type = "Fuzzy".to_string();
            }
        }
        if let Some(fuzzy_score) = compute_fuzzy_match(name_without_ext, &search_term) {
            if fuzzy_score > best_score {
                best_score = fuzzy_score;
                best_match_type = "Fuzzy".to_string();
            }
        }
    }

    if best_score < 0.40 && (is_typo_match(name, &search_term) || is_typo_match(name_without_ext, &search_term)) {
        best_score = 0.40;
        best_match_type = "Typo".to_string();
    }

    if best_score > 0.0 {
        return Some(SearchResult {
            metadata: file.clone(),
            score: best_score,
            match_type: best_match_type,
            icon_base64: None,
        });
    }

    None
}

// ==========================================
// MATCHING ALGORITHM IMPLEMENTATIONS
// ==========================================

/// Exact Match: The query matches the file name exactly (case-insensitive).
fn is_exact_match(target: &str, query: &str) -> bool {
    target.eq_ignore_ascii_case(query)
}

/// Prefix Match: The file name starts with the query string (case-insensitive).
fn is_prefix_match(target: &str, query: &str) -> bool {
    if query.len() > target.len() {
        return false;
    }
    target[..query.len()].eq_ignore_ascii_case(query)
}

/// Contains Match: The file name contains the query as a substring (case-insensitive).
fn is_contains_match(target: &str, query: &str) -> bool {
    let target_lower = target.to_lowercase();
    let query_lower = query.to_lowercase();
    target_lower.contains(&query_lower)
}

/// Acronym Match: The query characters match the initials of words in the target.
/// Words are defined by spaces, underscores, dashes, or dots.
///
/// Example:
/// - "vsc" matches "Visual Studio Code" (V-S-C)
/// - "ghd" matches "GitHub Desktop" (G-H-D)
fn is_acronym_match(target: &str, query: &str) -> bool {
    let target_chars: Vec<char> = target.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    if query_chars.is_empty() {
        return false;
    }

    // Collect initials (treating delimiters and CamelCase transitions as boundaries)
    let mut initials = Vec::new();
    for i in 0..target_chars.len() {
        let c = target_chars[i];
        if i == 0 {
            initials.push(c.to_ascii_lowercase());
        } else {
            let prev = target_chars[i - 1];
            let is_boundary = prev.is_whitespace()
                || prev == '_'
                || prev == '-'
                || prev == '.'
                || (prev.is_lowercase() && c.is_uppercase());
            if is_boundary && !c.is_whitespace() && c != '_' && c != '-' && c != '.' {
                initials.push(c.to_ascii_lowercase());
            }
        }
    }

    // Match query against initials in sequence
    if query_chars.len() > initials.len() {
        return false;
    }

    let mut q_idx = 0;
    for &init in &initials {
        if q_idx < query_chars.len() && query_chars[q_idx].to_ascii_lowercase() == init {
            q_idx += 1;
        }
    }

    q_idx == query_chars.len()
}

/// Camel Case Match: Matches query characters against uppercase word boundaries.
///
/// Example:
/// - "hlbr" matches "HeliumBrowser" (H-B + other letters, but we look at camel boundaries)
fn is_camel_case_match(target: &str, query: &str) -> bool {
    let target_chars: Vec<char> = target.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    if query_chars.is_empty() {
        return false;
    }

    let mut boundaries = Vec::new();
    for i in 0..target_chars.len() {
        let c = target_chars[i];
        if i == 0 {
            boundaries.push(c.to_ascii_lowercase());
        } else {
            let prev = target_chars[i - 1];
            // Transition from lowercase to uppercase, or after delimiter
            let is_boundary = (prev.is_lowercase() && c.is_uppercase())
                || prev.is_whitespace()
                || prev == '_'
                || prev == '-'
                || prev == '.';
            if is_boundary {
                boundaries.push(c.to_ascii_lowercase());
            }
        }
    }

    if query_chars.len() > boundaries.len() {
        return false;
    }

    let mut q_idx = 0;
    for &bound in &boundaries {
        if q_idx < query_chars.len() && query_chars[q_idx].to_ascii_lowercase() == bound {
            q_idx += 1;
        }
    }

    q_idx == query_chars.len()
}

/// Fuzzy Match: Alignment algorithm matching characters in sequence with gaps.
/// Computes score based on matching positions, word boundaries, and gap penalties.
///
/// Returns Some(score) if all query characters are found in order, otherwise None.
fn compute_fuzzy_match(target: &str, query: &str) -> Option<f64> {
    let target_chars: Vec<char> = target.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    if query_chars.is_empty() || target_chars.is_empty() {
        return None;
    }

    // Quick verification that all query characters exist in target sequentially
    let mut t_idx = 0;
    let mut q_idx = 0;
    while q_idx < query_chars.len() && t_idx < target_chars.len() {
        if query_chars[q_idx].to_ascii_lowercase() == target_chars[t_idx].to_ascii_lowercase() {
            q_idx += 1;
        }
        t_idx += 1;
    }

    if q_idx != query_chars.len() {
        return None; // Characters not found in sequence
    }

    // DP matrix to find the optimal matching path.
    // dp[q][t] represents the maximum alignment score of query[0..q] with target[0..t]
    // where query[q] is matched exactly at target[t].
    //
    // Scoring rules:
    // - Base match: +5.0 points
    // - Case match: +2.0 points (e.g. 'A' matching 'A')
    // - Start of word / boundary match: +10.0 points
    // - Camel case boundary: +8.0 points
    // - Consecutive bonus: +15.0 points if the previous query char matched the previous target char
    // - Gap penalty: -1.0 per skipped target character in between matches
    // - Target length penalty: -0.05 per character in the target (favors shorter targets)
    
    let q_len = query_chars.len();
    let t_len = target_chars.len();

    let mut dp = vec![vec![f64::MIN; t_len]; q_len];

    // Initialize first row (matching query[0])
    for j in 0..t_len {
        if query_chars[0].to_ascii_lowercase() == target_chars[j].to_ascii_lowercase() {
            let mut score = 5.0;
            // Case match
            if query_chars[0] == target_chars[j] {
                score += 2.0;
            }
            // Word boundaries
            if j == 0 {
                score += 10.0;
            } else {
                let prev = target_chars[j - 1];
                if prev.is_whitespace() || prev == '_' || prev == '-' || prev == '.' {
                    score += 10.0;
                } else if prev.is_lowercase() && target_chars[j].is_uppercase() {
                    score += 8.0;
                }
            }
            // Initial gap penalty from start of target
            score -= (j as f64) * 0.5;

            dp[0][j] = score;
        }
    }

    // Populate DP matrix
    for i in 1..q_len {
        for j in 0..t_len {
            if query_chars[i].to_ascii_lowercase() == target_chars[j].to_ascii_lowercase() {
                // Find the best match for query[i-1] at target[k] where k < j
                let mut best_prev_score = f64::MIN;

                for k in 0..j {
                    if dp[i - 1][k] > f64::MIN {
                        let mut score = dp[i - 1][k] + 5.0; // Match points
                        
                        // Case match
                        if query_chars[i] == target_chars[j] {
                            score += 2.0;
                        }

                        // Boundary checks
                        let prev = target_chars[j - 1];
                        if prev.is_whitespace() || prev == '_' || prev == '-' || prev == '.' {
                            score += 10.0;
                        } else if prev.is_lowercase() && target_chars[j].is_uppercase() {
                            score += 8.0;
                        }

                        // Consecutive matching character bonus
                        if k == j - 1 {
                            score += 15.0;
                        } else {
                            // Gap penalty
                            score -= ((j - k - 1) as f64) * 1.0;
                        }

                        if score > best_prev_score {
                            best_prev_score = score;
                        }
                    }
                }
                dp[i][j] = best_prev_score;
            }
        }
    }

    // Find the max score in the last row
    let mut max_raw_score = f64::MIN;
    for j in 0..t_len {
        if dp[q_len - 1][j] > max_raw_score {
            max_raw_score = dp[q_len - 1][j];
        }
    }

    if max_raw_score <= f64::MIN {
        return None;
    }

    // Apply target length penalty
    max_raw_score -= (t_len as f64) * 0.05;

    // Normalize score. Maximum potential score is around (q_len * 32.0)
    let max_possible = (q_len as f64) * 32.0;
    let normalized = (max_raw_score / max_possible).clamp(0.01, 0.90);

    Some(normalized)
}

/// Typo Match: Checks if any word in the target filename matches the query
/// with a Levenshtein distance of <= 1 (query length 3-5) or <= 2 (query length > 5).
fn is_typo_match(target: &str, query: &str) -> bool {
    let q_len = query.len();
    if q_len < 3 {
        return false;
    }

    let max_distance = if q_len <= 5 { 1 } else { 2 };
    let query_lower = query.to_lowercase();

    // Split target into words (e.g. "Helium Browser" -> ["Helium", "Browser"])
    let words = target.split(|c: char| c.is_whitespace() || c == '_' || c == '-' || c == '.');
    for word in words {
        if word.len() >= q_len - max_distance && word.len() <= q_len + max_distance {
            let dist = levenshtein_distance(&word.to_lowercase(), &query_lower);
            if dist <= max_distance {
                return true;
            }
        }
    }

    false
}

/// Standard Levenshtein distance implementation
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let v1: Vec<char> = s1.chars().collect();
    let v2: Vec<char> = s2.chars().collect();

    let mut prev = (0..=v2.len()).collect::<Vec<usize>>();
    let mut curr = vec![0; v2.len() + 1];

    for i in 0..v1.len() {
        curr[0] = i + 1;
        for j in 0..v2.len() {
            let cost = if v1[i] == v2[j] { 0 } else { 1 };
            curr[j + 1] = std::cmp::min(
                curr[j] + 1,
                std::cmp::min(prev[j + 1] + 1, prev[j] + cost),
            );
        }
        prev.copy_from_slice(&curr);
    }

    prev[v2.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typo_match() {
        assert!(is_typo_match("Helium Browser", "heliu"));
        assert!(is_typo_match("Chrome", "chorme")); // Transposed (dist 2, len 6 -> matches)
        assert!(is_typo_match("Discord", "disord")); // Missing letter (dist 1, len 6 -> matches)
        assert!(!is_typo_match("Chrome", "abc"));
    }

    #[test]
    fn test_parse_query() {
        let q = parse_query(".pdf report");
        assert_eq!(q.extension_filter, Some("pdf".to_string()));
        assert_eq!(q.terms, vec!["report".to_string()]);

        let q2 = parse_query("vsc");
        assert_eq!(q2.extension_filter, None);
        assert_eq!(q2.terms, vec!["vsc".to_string()]);

        let q3 = parse_query(".pdf");
        assert_eq!(q3.extension_filter, Some("pdf".to_string()));
        assert!(q3.terms.is_empty());
    }

    #[test]
    fn test_acronym_match() {
        assert!(is_acronym_match("Visual Studio Code", "vsc"));
        assert!(is_acronym_match("GitHub Desktop", "ghd"));
        assert!(!is_acronym_match("Visual Studio Code", "vsd"));
    }

    #[test]
    fn test_camel_case_match() {
        assert!(is_camel_case_match("HeliumBrowser", "hb"));
        assert!(is_camel_case_match("Helium Browser", "hb"));
        assert!(!is_camel_case_match("HeliumBrowser", "hr"));
    }

    #[test]
    fn test_fuzzy_match() {
        let score1 = compute_fuzzy_match("Helium Browser", "hlbr");
        assert!(score1.is_some());
        
        let score2 = compute_fuzzy_match("Discord", "dsc");
        assert!(score2.is_some());

        let score3 = compute_fuzzy_match("Google Chrome", "chr");
        assert!(score3.is_some());

        // Better alignment should score higher
        let score_consec = compute_fuzzy_match("Chrome", "chr").unwrap();
        let score_scatter = compute_fuzzy_match("Camera Hotrod", "chr").unwrap();
        assert!(score_consec > score_scatter);
    }
}
