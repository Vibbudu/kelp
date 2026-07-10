use crate::models::SearchQuery;

/// Parses a raw query string into a structured `SearchQuery`.
///
/// Filters out dot-prefixed terms (e.g. `.pdf`) as extension filters
/// and leaves the remaining words as search terms.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
