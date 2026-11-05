//! Pure, bounded text-file analysis. This module never opens filesystem paths.
use serde::{Deserialize, Serialize};

const MAX_TEXT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileAnalysis {
    pub file_name: String,
    pub file_type: String,
    pub size_bytes: usize,
    pub content_preview: String,
    pub word_count: usize,
    pub line_count: usize,
    pub top_keywords: Vec<String>,
}

pub fn analyze_text(file_name: &str, content: &str) -> Result<FileAnalysis, String> {
    if file_name.trim().is_empty() || file_name.len() > 255 {
        return Err("A valid file name is required.".into());
    }
    if content.len() > MAX_TEXT_BYTES {
        return Err("Text file exceeds the 1 MiB processing limit.".into());
    }
    let extension = file_name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !matches!(extension.as_str(), "txt" | "md" | "csv" | "json" | "rs" | "py" | "js" | "ts" | "html" | "xml" | "log" | "pdf") {
        return Err("Unsupported text file type.".into());
    }
    let preview: String = content.chars().take(500).collect();
    let preview = if content.chars().count() > 500 { format!("{preview}…") } else { preview };
    Ok(FileAnalysis {
        file_name: file_name.to_string(),
        file_type: extension,
        size_bytes: content.len(),
        content_preview: preview,
        word_count: content.split_whitespace().count(),
        line_count: content.lines().count(),
        top_keywords: extract_keywords(content, 10),
    })
}

pub fn extract_keywords(content: &str, top_n: usize) -> Vec<String> {
    let stop_words = ["the", "and", "for", "are", "but", "not", "you", "all", "can", "her", "was", "one", "our", "out", "this", "that", "with", "have", "from", "they", "been", "said"];
    let mut frequencies = std::collections::BTreeMap::<String, usize>::new();
    for token in content.split_whitespace() {
        let word: String = token.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect();
        if word.chars().count() > 3 && !stop_words.contains(&word.as_str()) {
            *frequencies.entry(word).or_default() += 1;
        }
    }
    let mut sorted: Vec<_> = frequencies.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    sorted.into_iter().take(top_n).map(|(word, _)| word).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_supported_text_without_filesystem_access() {
        let a = analyze_text("notes.md", "Rust local AI\nRust stays local").unwrap();
        assert_eq!(a.word_count, 6);
        assert_eq!(a.line_count, 2);
        assert!(a.top_keywords.contains(&"rust".to_string()));
    }

    #[test]
    fn rejects_unsupported_and_oversized_input() {
        assert!(analyze_text("photo.exe", "x").is_err());
        assert!(analyze_text("notes.txt", &"x".repeat(MAX_TEXT_BYTES + 1)).is_err());
    }

    #[test]
    fn rejects_empty_and_overlong_file_names() {
        assert!(analyze_text("  ", "content").is_err());
        assert!(analyze_text(&"n".repeat(256), "content").is_err());
    }

    #[test]
    fn preview_is_bounded_by_characters_and_preserves_unicode() {
        let content = "é".repeat(501);
        let analysis = analyze_text("unicode.txt", &content).unwrap();
        assert_eq!(analysis.content_preview.chars().count(), 501);
        assert!(analysis.content_preview.ends_with('…'));
    }
}
