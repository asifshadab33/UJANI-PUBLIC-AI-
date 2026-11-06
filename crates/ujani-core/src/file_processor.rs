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
    let extension = std::path::Path::new(file_name).extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
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
    // Keep English stop words scoped to ASCII tokens. Do not split CJK text on
    // whitespace and pretend the resulting sentence is a useful keyword.
    const ENGLISH_STOP_WORDS: &[&str] = &[
        "the", "and", "for", "are", "but", "not", "you", "all", "can", "her",
        "was", "one", "our", "out", "this", "that", "with", "have", "from",
        "they", "been", "said",
    ];
    let mut frequencies = std::collections::BTreeMap::<String, usize>::new();
    for token in content.split_whitespace() {
        let word: String = token
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();
        if word.is_empty() || word.chars().any(is_cjk) { continue; }
        let char_count = word.chars().count();
        let useful = if word.is_ascii() {
            char_count > 3 && !ENGLISH_STOP_WORDS.contains(&word.as_str())
        } else {
            // Arabic and other alphabetic scripts often have meaningful short words.
            char_count >= 2
        };
        if useful { *frequencies.entry(word).or_default() += 1; }
    }

    // CJK text is commonly written without whitespace. Extract overlapping
    // bigrams from contiguous CJK runs instead of treating whole sentences as words.
    let mut run = Vec::<char>::new();
    let mut add_run = |run: &mut Vec<char>, frequencies: &mut std::collections::BTreeMap<String, usize>| {
        for pair in run.windows(2) {
            let token: String = pair.iter().copied().collect();
            *frequencies.entry(token).or_default() += 1;
        }
        run.clear();
    };
    for ch in content.chars() {
        if is_cjk(ch) { run.push(ch); } else { add_run(&mut run, &mut frequencies); }
    }
    add_run(&mut run, &mut frequencies);

    let mut sorted: Vec<_> = frequencies.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    sorted.into_iter().take(top_n).map(|(word, _)| word).collect()
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x2E80..=0x2EFF | 0x2F00..=0x2FDF | 0x3040..=0x30FF |
        0x3100..=0x312F | 0x3130..=0x318F | 0x31A0..=0x31BF |
        0x31F0..=0x31FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF |
        0xF900..=0xFAFF | 0x20000..=0x2FA1F
    )
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
    fn rejects_names_without_a_real_extension() {
        assert!(analyze_text("txt", "content").is_err());
        assert!(analyze_text("notes.", "content").is_err());
        assert!(analyze_text(".txt", "content").is_err());
    }

    #[test]
    fn rejects_empty_and_overlong_file_names() {
        assert!(analyze_text("  ", "content").is_err());
        assert!(analyze_text(&"n".repeat(256), "content").is_err());
    }

    #[test]
    fn multilingual_keywords_do_not_apply_english_length_or_stop_words() {
        let arabic = extract_keywords("ملخص وثيقة محلي ملخص", 10);
        assert!(arabic.contains(&"ملخص".to_string()));
        assert!(arabic.contains(&"وثيقة".to_string()));
        let cjk = extract_keywords("文書処理 文書処理", 10);
        assert!(cjk.contains(&"文書".to_string()));
        assert!(cjk.contains(&"処理".to_string()));
    }

    #[test]
    fn preview_is_bounded_by_characters_and_preserves_unicode() {
        let content = "é".repeat(501);
        let analysis = analyze_text("unicode.txt", &content).unwrap();
        assert_eq!(analysis.content_preview.chars().count(), 501);
        assert!(analysis.content_preview.ends_with('…'));
    }
}
