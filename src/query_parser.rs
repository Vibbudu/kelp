use crate::models::SearchQuery;

pub fn semantic_extensions_for_word(word: &str) -> Option<&'static [&'static str]> {
    match word.to_lowercase().as_str() {
        "image" | "images" | "picture" | "pictures" | "photo" | "photos" => {
            Some(&["png", "jpg", "jpeg", "webp", "gif", "bmp", "svg", "ico", "tiff"])
        }
        "pdf" | "pdfs" => {
            Some(&["pdf"])
        }
        "doc" | "docs" | "document" | "documents" | "word" | "docx" => {
            Some(&["doc", "docx", "txt", "rtf", "odt", "pdf", "md"])
        }
        "music" | "audio" | "song" | "songs" | "sound" => {
            Some(&["mp3", "wav", "flac", "aac", "ogg", "m4a", "wma"])
        }
        "video" | "videos" | "movie" | "movies" => {
            Some(&["mp4", "mkv", "mov", "avi", "wmv", "webm", "flv"])
        }
        "sheet" | "sheets" | "excel" | "spreadsheet" | "spreadsheets" | "csv" | "xlsx" => {
            Some(&["xls", "xlsx", "csv", "tsv", "ods"])
        }
        "slides" | "presentation" | "powerpoint" | "ppt" => {
            Some(&["ppt", "pptx", "odp"])
        }
        "archive" | "zip" | "archives" | "compressed" => {
            Some(&["zip", "rar", "7z", "tar", "gz"])
        }
        "code" | "source" | "script" => {
            Some(&["rs", "py", "js", "ts", "cpp", "c", "h", "cs", "java", "html", "css", "json", "toml", "yaml", "sh", "bat", "ps1"])
        }
        _ => None,
    }
}

/// Parses a raw query string into a structured `SearchQuery`.
///
/// Filters out dot-prefixed terms (e.g. `.pdf`) as extension filters,
/// maps semantic category words (e.g. `images`, `audio`) to extension sets,
/// and leaves the remaining words as search terms.
pub fn parse_query(raw: &str) -> SearchQuery {
    let mut extension_filter = None;
    let mut semantic_extensions = None;
    let mut terms = Vec::new();

    for word in raw.split_whitespace() {
        if word.starts_with('.') && word.len() > 1 {
            // Treat as extension filter (strip the leading dot)
            extension_filter = Some(word[1..].to_lowercase());
        } else if let Some(exts) = semantic_extensions_for_word(word) {
            semantic_extensions = Some(exts.iter().map(|s| s.to_string()).collect());
        } else {
            terms.push(word.to_string());
        }
    }

    let term_string = terms.join(" ");

    SearchQuery {
        raw: raw.trim().to_string(),
        extension_filter,
        terms,
        term_string,
        semantic_extensions,
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

        let q4 = parse_query("images");
        assert!(q4.semantic_extensions.is_some());
        assert!(q4.terms.is_empty());

        let q5 = parse_query("invoice excel");
        assert!(q5.semantic_extensions.is_some());
        assert_eq!(q5.terms, vec!["invoice".to_string()]);
    }
}
