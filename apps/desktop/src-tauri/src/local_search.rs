//! Bounded, user-initiated local file finder.
//! Folder selection is performed by the native OS dialog; no path is accepted from the webview.
//! Search is read-only. PDF text is extracted only in the existing resource-limited worker.

use serde::Serialize;
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use tauri_plugin_dialog::DialogExt;

const MAX_DEPTH: usize = 6;
const MAX_ENTRIES: usize = 5_000;
const MAX_MATCHES: usize = 250;
const MAX_TEXT_FILE_BYTES: u64 = 256 * 1024;
const MAX_TEXT_TOTAL_BYTES: u64 = 10 * 1024 * 1024;
const MAX_PDF_BYTES: u64 = 10 * 1024 * 1024;
const MAX_PDF_FILES: usize = 20;

#[derive(Serialize)]
pub struct LocalSearchMatch {
    path: String,
    file_name: String,
    extension: String,
    size_bytes: u64,
    modified_unix_ms: Option<u64>,
    matched_in: Vec<String>,
}

#[derive(Serialize)]
pub struct LocalSearchResult {
    cancelled: bool,
    root_name: Option<String>,
    scanned_files: usize,
    matches: Vec<LocalSearchMatch>,
    truncated: bool,
    note: String,
}

struct Candidate {
    path: PathBuf,
    name: String,
    extension: String,
    size: u64,
    modified_unix_ms: Option<u64>,
}

#[tauri::command]
pub async fn find_local_files(
    app: tauri::AppHandle,
    query: String,
    search_pdf_text: bool,
) -> Result<LocalSearchResult, String> {
    let query = query.trim().to_lowercase();
    if query.is_empty() || query.len() > 160 || query.chars().any(char::is_control) {
        return Err("Enter a search phrase between 1 and 160 characters.".into());
    }

    let selected = tauri::async_runtime::spawn_blocking(move || -> Result<Option<PathBuf>, String> {
        let Some(folder) = app.dialog().file().set_title("Choose a folder to search").blocking_pick_folder() else {
            return Ok(None);
        };
        let path = folder.into_path()
            .map_err(|_| "The selected folder is not a local filesystem path.".to_string())?;
        Ok(Some(path))
    }).await.map_err(|e| format!("Folder picker failed: {e}"))??;

    let Some(folder) = selected else {
        return Ok(LocalSearchResult {
            cancelled: true, root_name: None, scanned_files: 0, matches: Vec::new(),
            truncated: false, note: "Folder selection cancelled. Nothing was searched.".into(),
        });
    };

    let (root, candidates, scanned_files, truncated) = tauri::async_runtime::spawn_blocking(move || {
        collect_candidates(&folder)
    }).await.map_err(|e| format!("File search worker failed: {e}"))??;

    let root_name = root.file_name().map(|v| v.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());
    let mut matches = Vec::new();
    let mut text_bytes = 0u64;
    let mut pdf_count = 0usize;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(180);

    for candidate in candidates {
        if matches.len() >= MAX_MATCHES { break; }
        let mut matched_in = Vec::new();
        if candidate.name.to_lowercase().contains(&query) {
            matched_in.push("filename".to_string());
        }

        let is_text = matches!(candidate.extension.as_str(),
            "txt" | "md" | "csv" | "log" | "json" | "xml" | "html" | "htm" | "rtf");
        if matched_in.is_empty() && is_text && candidate.size <= MAX_TEXT_FILE_BYTES
            && text_bytes.saturating_add(candidate.size) <= MAX_TEXT_TOTAL_BYTES
        {
            if let Ok(bytes) = read_bounded(&candidate.path, MAX_TEXT_FILE_BYTES) {
                text_bytes = text_bytes.saturating_add(bytes.len() as u64);
                if let Ok(content) = String::from_utf8(bytes) {
                    if content.to_lowercase().contains(&query) {
                        matched_in.push("text content".to_string());
                    }
                }
            }
        }

        if matched_in.is_empty() && candidate.extension == "pdf" && search_pdf_text
            && candidate.size <= MAX_PDF_BYTES && pdf_count < MAX_PDF_FILES
            && tokio::time::Instant::now() < deadline
        {
            pdf_count += 1;
            if let Ok(bytes) = read_bounded(&candidate.path, MAX_PDF_BYTES) {
                if bytes.starts_with(b"%PDF-") {
                    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                    if !remaining.is_zero() {
                        if let Ok(Ok(text)) = tokio::time::timeout(
                            remaining.min(std::time::Duration::from_secs(45)),
                            crate::task_engine::extract_pdf_isolated(bytes)
                        ).await {
                            if text.to_lowercase().contains(&query) {
                                matched_in.push("PDF text (isolated worker)".to_string());
                            }
                        }
                    }
                }
            }
        }

        if !matched_in.is_empty() {
            matches.push(LocalSearchMatch {
                path: candidate.path.display().to_string(),
                file_name: candidate.name,
                extension: candidate.extension,
                size_bytes: candidate.size,
                modified_unix_ms: candidate.modified_unix_ms,
                matched_in,
            });
        }
    }

    Ok(LocalSearchResult {
        cancelled: false,
        root_name: Some(root_name),
        scanned_files,
        matches,
        truncated,
        note: if search_pdf_text {
            "Read-only search. Text PDFs are inspected in the existing resource-limited worker; scanned-image OCR is not included. Search is bounded to 20 PDFs and 180 seconds.".into()
        } else {
            "Read-only search. PDF contents were not inspected; enable PDF text search to look inside text-based PDFs. Scanned-image OCR is not included.".into()
        },
    })
}

fn collect_candidates(folder: &Path) -> Result<(PathBuf, Vec<Candidate>, usize, bool), String> {
    let meta = fs::symlink_metadata(folder).map_err(|e| format!("Cannot inspect folder: {e}"))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err("Choose a real folder, not a symbolic link.".into());
    }
    let root = fs::canonicalize(folder).map_err(|e| format!("Cannot resolve folder: {e}"))?;
    let mut stack = vec![(root.clone(), 0usize)];
    let mut candidates = Vec::new();
    let mut entries_seen = 0usize;
    let mut scanned_files = 0usize;
    let mut truncated = false;

    while let Some((directory, depth)) = stack.pop() {
        if depth > MAX_DEPTH { truncated = true; continue; }
        let directory_meta = fs::symlink_metadata(&directory)
            .map_err(|e| format!("Cannot inspect selected directory: {e}"))?;
        if directory_meta.file_type().is_symlink() || !directory_meta.is_dir() { continue; }
        let canonical_directory = fs::canonicalize(&directory)
            .map_err(|e| format!("Cannot resolve selected directory: {e}"))?;
        if !canonical_directory.starts_with(&root) { continue; }
        let iterator = fs::read_dir(&canonical_directory).map_err(|e| format!("Cannot read selected folder: {e}"))?;
        for item in iterator {
            let item = item.map_err(|e| format!("Cannot inspect a folder entry: {e}"))?;
            entries_seen += 1;
            if entries_seen > MAX_ENTRIES { truncated = true; break; }
            let kind = item.file_type().map_err(|e| format!("Cannot inspect entry type: {e}"))?;
            if kind.is_symlink() { continue; }
            if kind.is_dir() {
                if depth < MAX_DEPTH { stack.push((item.path(), depth + 1)); } else { truncated = true; }
                continue;
            }
            if !kind.is_file() { continue; }
            scanned_files += 1;
            let path = fs::canonicalize(item.path()).map_err(|e| format!("Cannot resolve file: {e}"))?;
            if !path.starts_with(&root) { continue; }
            let metadata = fs::metadata(&path).map_err(|e| format!("Cannot inspect file metadata: {e}"))?;
            let name = item.file_name().to_string_lossy().into_owned();
            let extension = path.extension().and_then(|v| v.to_str()).unwrap_or("").to_ascii_lowercase();
            let modified_unix_ms = metadata.modified().ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64);
            candidates.push(Candidate { path, name, extension, size: metadata.len(), modified_unix_ms });
        }
        if entries_seen > MAX_ENTRIES { break; }
    }
    candidates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok((root, candidates, scanned_files, truncated))
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let before = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if before.file_type().is_symlink() || !before.is_file() || before.len() > limit {
        return Err("File is not eligible for bounded reading.".into());
    }
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.by_ref().take(limit + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit { return Err("File grew beyond the search limit.".into()); }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn skips_symlinks_and_limits_directory_depth() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-search-{unique}"));
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("contract.txt"), "supplier agreement").unwrap();
        let (canonical, files, count, truncated) = collect_candidates(&root).unwrap();
        assert_eq!(canonical, fs::canonicalize(&root).unwrap());
        assert_eq!(count, 1);
        assert_eq!(files.len(), 1);
        assert!(!truncated);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_a_symlink_as_the_selected_root() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-search-root-{unique}"));
        fs::create_dir_all(&root).unwrap();
        let link = root.with_extension("link");
        #[cfg(unix)] std::os::unix::fs::symlink(&root, &link).unwrap();
        #[cfg(windows)] std::os::windows::fs::symlink_dir(&root, &link).unwrap();
        #[cfg(any(unix, windows))] assert!(collect_candidates(&link).is_err());
        fs::remove_dir_all(root).unwrap();
        #[cfg(any(unix, windows))] fs::remove_file(link).unwrap();
    }
}
