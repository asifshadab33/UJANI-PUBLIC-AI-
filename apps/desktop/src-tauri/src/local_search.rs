//! Bounded, user-initiated local file finder.
//! Folder selection is performed by the native OS dialog; no path is accepted from the webview.
//! Search is read-only. PDF text is extracted only in the existing resource-limited worker.

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

const MAX_DEPTH: usize = 6;
const MAX_ENTRIES: usize = 5_000;
const MAX_MATCHES: usize = 250;
const MAX_TEXT_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TEXT_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_PDF_BYTES: u64 = 10 * 1024 * 1024;
const MAX_PDF_FILES: usize = 1_000;
const MAX_PDF_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

/// Records a PDF text-search outcome without treating an extraction error as a negative match.
fn record_pdf_search_result(
    extraction: Result<String, String>,
    query: &str,
    matched_in: &mut Vec<String>,
    incomplete: &mut bool,
) {
    match extraction {
        Ok(text) if text.to_lowercase().contains(query) => {
            matched_in.push("PDF text (isolated worker)".to_string());
        }
        Ok(_) => {}
        Err(_) => *incomplete = true,
    }
}

/// Partial or truncated scans must never be persisted as complete search results.
fn search_result_is_cacheable(truncated: bool, incomplete: bool) -> bool {
    !truncated && !incomplete
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LocalSearchMatch {
    path: String,
    file_name: String,
    extension: String,
    size_bytes: u64,
    modified_unix_ms: Option<u64>,
    matched_in: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LocalSearchResult {
    cancelled: bool,
    root_name: Option<String>,
    scanned_files: usize,
    matches: Vec<LocalSearchMatch>,
    truncated: bool,
    note: String,
    cache_hit: bool,
}

#[derive(Serialize, Deserialize)]
struct Candidate {
    path: PathBuf,
    name: String,
    extension: String,
    size: u64,
    modified_unix_ms: Option<u64>,
    modified_unix_ns: Option<i64>,
    identity: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CachedSearchPayload {
    result: LocalSearchResult,
    file_identities: Vec<Option<String>>,
}

/// Searches a folder chosen by the user; never accepts a filesystem path from the webview.
#[tauri::command]
pub async fn find_local_files(
    app: AppHandle,
    control: State<'_, super::TaskControlState>,
    task_id: String,
    query: String,
    search_pdf_text: bool,
    force_refresh: bool,
) -> Result<LocalSearchResult, String> {
    if !super::valid_command_identifier(&task_id, 96) {
        return Err("Invalid task identifier.".into());
    }
    let query = query.trim().to_lowercase();
    if query.is_empty() || query.encode_utf16().count() > 160 || query.chars().any(char::is_control) {
        return Err("Enter a search phrase between 1 and 160 characters.".into());
    }

    let token = CancellationToken::new();
    {
        let mut active = control.active.lock().map_err(|_| "Task registry unavailable.".to_string())?;
        if active.contains_key(&task_id) {
            return Err("Task identifier is already active.".into());
        }
        active.insert(task_id.clone(), token.clone());
    }
    let _active_guard = super::ActiveTaskGuard { control: &control, task_id: task_id.clone() };
    super::emit_task_progress(&app, &task_id, "starting", "Choose a folder to search.", 1);

    let picker_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || -> Result<Option<PathBuf>, String> {
        let Some(folder) = picker_app.dialog().file().set_title("Choose a folder to search").blocking_pick_folder() else {
            return Ok(None);
        };
        let path = folder.into_path()
            .map_err(|_| "The selected folder is not a local filesystem path.".to_string())?;
        Ok(Some(path))
    }).await.map_err(|e| format!("Folder picker failed: {e}"))??;

    if token.is_cancelled() {
        return Err("File search cancelled by user.".into());
    }
    let Some(folder) = selected else {
        return Ok(LocalSearchResult {
            cancelled: true, root_name: None, scanned_files: 0, matches: Vec::new(),
            truncated: false, note: "Folder selection cancelled. Nothing was searched.".into(), cache_hit: false,
        });
    };

    super::emit_task_progress(&app, &task_id, "scanning", "Scanning the selected folder.", 10);
    let collect_token = token.clone();
    let (root, candidates, scanned_files, mut truncated) = tauri::async_runtime::spawn_blocking(move || {
        collect_candidates(&folder, &collect_token)
    }).await.map_err(|e| format!("File search worker failed: {e}"))??;

    let root_name = root.file_name().map(|v| v.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());
    // Non-Unicode paths may not serialize losslessly; in that case, run fresh without caching.
    let inventory_json = serde_json::to_string(&candidates).ok();
    if !force_refresh && !truncated {
        if let Some(inventory_json) = inventory_json.as_deref() {
            let load_app = app.clone();
            let load_root = root.clone();
            let load_query = query.clone();
            let load_inventory = inventory_json.to_owned();
            // Credential-vault and SQLite work must not block an async runtime worker.
            let cached = tauri::async_runtime::spawn_blocking(move || {
                crate::search_cache::load(&load_app, &load_root, &load_query, search_pdf_text, &load_inventory)
            }).await.ok().and_then(Result::ok).flatten();
            if let Some(cached_json) = cached {
                if let Ok(cached) = serde_json::from_str::<CachedSearchPayload>(&cached_json) {
                    if validate_cached_matches(&root, &candidates, &cached.result, &cached.file_identities) {
                        let mut cached_result = cached.result;
                        cached_result.scanned_files = scanned_files;
                        cached_result.cache_hit = true;
                        cached_result.note = format!("{} Cached metadata was revalidated against the selected folder just now; no document contents were retained.", cached_result.note);
                        super::emit_task_progress(&app, &task_id, "completed", "Validated recent local search results.", 100);
                        return Ok(cached_result);
                    }
                }
            }
        }
    }
    let mut matches = Vec::new();
    let mut cached_file_identities = Vec::new();
    let mut text_bytes = 0u64;
    let mut pdf_count = 0usize;
    let mut pdf_bytes = 0u64;
    let mut incomplete = false;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(600);

    for candidate in candidates {
        if token.is_cancelled() { return Err("File search cancelled by user.".into()); }
        if tokio::time::Instant::now() >= deadline { truncated = true; break; }
        if matches.len() >= MAX_MATCHES { truncated = true; break; }
        let mut matched_in = Vec::new();
        if candidate.name.to_lowercase().contains(&query) {
            matched_in.push("filename".to_string());
        }

        let is_text = matches!(candidate.extension.as_str(),
            "txt" | "md" | "csv" | "log" | "json" | "xml" | "html" | "htm" | "rtf");
        if matched_in.is_empty() && is_text {
            if candidate.size > MAX_TEXT_FILE_BYTES
                || text_bytes.saturating_add(candidate.size) > MAX_TEXT_TOTAL_BYTES {
                incomplete = true;
            } else {
                let read_path = candidate.path.clone();
                match tauri::async_runtime::spawn_blocking(move || read_bounded(&read_path, MAX_TEXT_FILE_BYTES)).await {
                    Ok(Ok(bytes)) => {
                        text_bytes = text_bytes.saturating_add(bytes.len() as u64);
                        match String::from_utf8(bytes) {
                            Ok(content) if content.to_lowercase().contains(&query) => matched_in.push("text content".to_string()),
                            Ok(_) => {},
                            Err(_) => incomplete = true,
                        }
                    }
                    _ => incomplete = true,
                }
            }
        }

        if matched_in.is_empty() && candidate.extension == "pdf" && search_pdf_text {
            if candidate.size > MAX_PDF_BYTES || pdf_count >= MAX_PDF_FILES
                || pdf_bytes.saturating_add(candidate.size) > MAX_PDF_TOTAL_BYTES {
                incomplete = true;
                continue;
            }
            if tokio::time::Instant::now() >= deadline { truncated = true; break; }
            pdf_count += 1;
            pdf_bytes = pdf_bytes.saturating_add(candidate.size);
            let read_path = candidate.path.clone();
            match tauri::async_runtime::spawn_blocking(move || read_bounded(&read_path, MAX_PDF_BYTES)).await {
                Ok(Ok(bytes)) if bytes.starts_with(b"%PDF-") => {
                    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                    if remaining.is_zero() {
                        incomplete = true;
                    } else {
                        let extraction = tokio::time::timeout(
                            remaining.min(std::time::Duration::from_secs(45)),
                            crate::task_engine::extract_pdf_isolated(bytes)
                        );
                        let extraction_result = tokio::select! {
                            _ = token.cancelled() => return Err("File search cancelled by user.".into()),
                            result = extraction => result,
                        };
                        match extraction_result {
                            Ok(result) => record_pdf_search_result(result, &query, &mut matched_in, &mut incomplete),
                            Err(_) => incomplete = true,
                        }
                    }
                }
                _ => incomplete = true,
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
            cached_file_identities.push(candidate.identity.clone());
        }
    }

    super::emit_task_progress(&app, &task_id, "completed", "File search completed.", 100);
    let result = LocalSearchResult {
        cancelled: false,
        root_name: Some(root_name),
        scanned_files,
        matches,
        truncated,
        cache_hit: false,
        note: {
            let base = if search_pdf_text {
                "Read-only search. Text PDFs are inspected in the existing resource-limited worker; scanned-image OCR is not included. Search is bounded to 1,000 PDFs, 512 MiB of PDF input, and 10 minutes."
            } else {
                "Read-only search. PDF contents were not inspected; enable PDF text search to look inside text-based PDFs. Scanned-image OCR is not included."
            };
            if incomplete {
                format!("{base} Some eligible files could not be inspected within the supported limits; this partial result was not cached.")
            } else {
                base.into()
            }
        },
    };
    if search_result_is_cacheable(result.truncated, incomplete) {
        let cached_payload = CachedSearchPayload { result: result.clone(), file_identities: cached_file_identities };
        if let (Some(inventory_json), Ok(payload)) = (inventory_json.as_deref(), serde_json::to_string(&cached_payload)) {
            let store_app = app.clone();
            let store_root = root.clone();
            let store_query = query.clone();
            let store_inventory = inventory_json.to_owned();
            let store_payload = payload;
            let negative = result.matches.is_empty();
            let _ = tauri::async_runtime::spawn_blocking(move || {
                crate::search_cache::store(
                    &store_app, &store_root, &store_query, search_pdf_text,
                    &store_inventory, &store_payload, negative
                )
            }).await;
        }
    }
    Ok(result)
}


/// Stable filesystem identity where the platform exposes it. Unsupported platforms
/// deliberately return None; size, high-resolution mtime and canonical path remain required.
#[cfg(unix)]
/// Returns the device and inode identity for an opened Unix file handle.
fn file_identity(_file: &File, metadata: &fs::Metadata) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    Some(format!("unix:{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(windows)]
/// Returns the volume and file-index identity from an opened Windows file handle.
fn file_identity(file: &File, _metadata: &fs::Metadata) -> Option<String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    let succeeded = unsafe {
        GetFileInformationByHandle(file.as_raw_handle() as _, info.as_mut_ptr())
    };
    if succeeded == 0 { return None; }
    let info = unsafe { info.assume_init() };
    Some(format!(
        "windows:{}:{}:{}",
        info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow
    ))
}

#[cfg(not(any(unix, windows)))]
/// Returns no stable file identity on platforms without a supported handle API.
fn file_identity(_file: &File, _metadata: &fs::Metadata) -> Option<String> {
    None
}

/// Returns a signed nanosecond modification timestamp, including pre-epoch files.
fn modified_unix_ns(metadata: &fs::Metadata) -> Option<i64> {
    let time = metadata.modified().ok()?;
    let nanos = match time.duration_since(UNIX_EPOCH) {
        Ok(after) => after.as_nanos().min(i64::MAX as u128) as i64,
        Err(before) => -(before.duration().as_nanos().min(i64::MAX as u128) as i64),
    };
    Some(nanos)
}

/// A cache hit is accepted only when the entire current inventory matched and each
/// cached result can be re-opened without following a symlink/reparse point.
/// Rejects a cached result unless every match still resolves inside the selected scope.
fn validate_cached_matches(root: &Path, candidates: &[Candidate], cached: &LocalSearchResult, cached_identities: &[Option<String>]) -> bool {
    if cached.matches.len() != cached_identities.len() { return false; }
    for (item, cached_identity) in cached.matches.iter().zip(cached_identities) {
        let path = PathBuf::from(&item.path);
        if !path.starts_with(root) { return false; }
        let Ok(rechecked_path) = fs::canonicalize(&path) else { return false; };
        if rechecked_path != path || !rechecked_path.starts_with(root) { return false; }
        let Some(candidate) = candidates.iter().find(|candidate| candidate.path == path) else { return false; };
        if candidate.size != item.size_bytes || candidate.modified_unix_ms != item.modified_unix_ms
            || candidate.identity.as_ref() != cached_identity.as_ref() {
            return false;
        }
        let Ok(file) = crate::safe_fs::open_regular_file(&path) else { return false; };
        let Ok(metadata) = file.metadata() else { return false; };
        if !metadata.is_file() || metadata.len() != candidate.size
            || modified_unix_ns(&metadata) != candidate.modified_unix_ns
            || file_identity(&file, &metadata) != candidate.identity {
            return false;
        }
        #[cfg(windows)] {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 { return false; }
        }
    }
    true
}

/// Collects bounded file metadata beneath the folder selected through the native picker.
/// Walks a user-selected directory with depth and entry-count limits.
fn collect_candidates(folder: &Path, token: &CancellationToken) -> Result<(PathBuf, Vec<Candidate>, usize, bool), String> {
    let walked = crate::safe_fs::walk_directory(
        folder,
        crate::safe_fs::WalkOptions {
            max_depth: MAX_DEPTH,
            max_entries: MAX_ENTRIES,
            recursive: true,
            links: crate::safe_fs::LinkPolicy::Skip,
            errors: crate::safe_fs::WalkErrorPolicy::SkipAndMarkTruncated,
            file_filter: None,
        },
        token,
    ).map_err(|error| {
        if error.contains("cancelled") {
            "File search cancelled by user.".to_string()
        } else {
            format!("Cannot scan selected folder: {error}")
        }
    })?;

    let mut candidates = Vec::with_capacity(walked.files.len());
    let mut scanned_files = 0usize;
    let mut truncated = walked.truncated;
    for entry in walked.files {
        if token.is_cancelled() {
            return Err("File search cancelled by user.".into());
        }
        scanned_files += 1;
        let file = match crate::safe_fs::open_regular_file(&entry.path) {
            Ok(file) => file,
            Err(_) => { truncated = true; continue; }
        };
        let metadata = match file.metadata() {
            Ok(metadata) if metadata.is_file() => metadata,
            _ => { truncated = true; continue; }
        };
        let extension = entry.path.extension().and_then(|value| value.to_str())
            .unwrap_or("").to_ascii_lowercase();
        let modified_unix_ms = metadata.modified().ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64);
        let modified_unix_ns = modified_unix_ns(&metadata);
        let identity = file_identity(&file, &metadata);
        candidates.push(Candidate {
            path: entry.path,
            name: entry.name.to_string_lossy().into_owned(),
            extension,
            size: metadata.len(),
            modified_unix_ms,
            modified_unix_ns,
            identity,
        });
    }
    candidates.sort_by_key(|candidate| candidate.name.to_lowercase());
    Ok((walked.root, candidates, scanned_files, truncated))
}

/// Opens and reads one regular file with a hard byte cap; validation uses the opened handle.
/// Reads at most `limit` bytes from a regular file without following its final link.
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut file = crate::safe_fs::open_regular_file(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("File is not eligible for bounded reading.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err("Reparse-point files are not eligible for bounded reading.".into());
        }
    }
    let capacity = usize::try_from(metadata.len()).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    file.by_ref().take(limit.saturating_add(1)).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit { return Err("File grew beyond the search limit.".into()); }
    Ok(bytes)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn truncated_pdf_extraction_is_incomplete_and_never_cacheable() {
        let mut matches = Vec::new();
        let mut incomplete = false;
        record_pdf_search_result(
            Err("PDF text extraction exceeded the 1 MiB worker output limit.".into()),
            "needle",
            &mut matches,
            &mut incomplete,
        );
        assert!(matches.is_empty());
        assert!(incomplete);
        assert!(!search_result_is_cacheable(false, incomplete));
    }

    #[test]
    fn worker_truncation_marker_reaches_search_and_prevents_negative_cache() {
        let marker = b"UJANI_PDF_OUTPUT_TRUNCATED_V1\n";
        let extraction = match crate::task_engine::pdf_worker_output_was_truncated(marker) {
            Ok(true) => Err("PDF text extraction exceeded the 1 MiB worker output limit.".into()),
            Ok(false) => Ok(String::new()),
            Err(error) => Err(error),
        };
        let mut matches = Vec::new();
        let mut incomplete = false;
        record_pdf_search_result(extraction, "needle", &mut matches, &mut incomplete);
        assert!(matches.is_empty());
        assert!(incomplete);
        assert!(!search_result_is_cacheable(false, incomplete));
    }

    #[test]
    fn complete_pdf_non_match_remains_a_cacheable_negative_result() {
        let mut matches = Vec::new();
        let mut incomplete = false;
        record_pdf_search_result(Ok("complete text without target".into()), "needle", &mut matches, &mut incomplete);
        assert!(matches.is_empty());
        assert!(!incomplete);
        assert!(search_result_is_cacheable(false, incomplete));
    }

    #[test]
    fn complete_pdf_match_is_recorded_without_marking_search_incomplete() {
        let mut matches = Vec::new();
        let mut incomplete = false;
        record_pdf_search_result(Ok("prefix needle suffix".into()), "needle", &mut matches, &mut incomplete);
        assert_eq!(matches, vec!["PDF text (isolated worker)"]);
        assert!(!incomplete);
    }

    #[test]
    fn skips_symlinks_and_limits_directory_depth() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-search-{unique}"));
        let outside = std::env::temp_dir().join(format!("ujani-search-outside-{unique}"));
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(root.join("sub").join("contract.txt"), "supplier agreement").unwrap();
        fs::write(outside.join("secret.txt"), "outside secret").unwrap();
        #[cfg(unix)] {
            std::os::unix::fs::symlink(outside.join("secret.txt"), root.join("linked-file.txt")).unwrap();
            std::os::unix::fs::symlink(&outside, root.join("linked-dir")).unwrap();
        }
        let (canonical, files, count, truncated) = collect_candidates(&root, &CancellationToken::new()).unwrap();
        assert_eq!(canonical, fs::canonicalize(&root).unwrap());
        assert_eq!(count, 1);
        assert_eq!(files.len(), 1);
        assert!(!truncated);
        assert!(files.iter().all(|file| !file.path.starts_with(&outside)));
        let mut deep = root.clone();
        for level in 0..=MAX_DEPTH { deep.push(format!("d{level}")); }
        fs::create_dir_all(&deep).unwrap();
        let (_, _, _, deep_truncated) = collect_candidates(&root, &CancellationToken::new()).unwrap();
        assert!(deep_truncated);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn rejects_a_regular_file_as_the_selected_root() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let file = std::env::temp_dir().join(format!("ujani-search-root-file-{unique}"));
        fs::write(&file, "not a folder").unwrap();
        assert!(collect_candidates(&file, &CancellationToken::new()).is_err());
        fs::remove_file(file).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlink_as_the_selected_root() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-search-root-{unique}"));
        fs::create_dir_all(&root).unwrap();
        let link = root.with_extension("link");
        std::os::unix::fs::symlink(&root, &link).unwrap();
        assert!(collect_candidates(&link, &CancellationToken::new()).is_err());
        fs::remove_file(link).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn rejects_a_symlink_as_the_selected_root_when_runner_allows_creation() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-search-root-{unique}"));
        fs::create_dir_all(&root).unwrap();
        let link = root.with_extension("link");
        match std::os::windows::fs::symlink_dir(&root, &link) {
            Ok(()) => {
                assert!(collect_candidates(&link, &CancellationToken::new()).is_err());
                fs::remove_dir(&link).unwrap();
            }
            // Standard hosted Windows runners may not grant symlink-creation privilege.
            // The cross-platform regular-file rejection test still runs in that environment.
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {}
            Err(error) => panic!("Could not create test symlink: {error}"),
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn inventory_snapshot_changes_when_files_are_added_moved_modified_or_deleted() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-cache-inventory-{unique}"));
        fs::create_dir_all(&root).unwrap();
        let first = root.join("one.txt");
        let second = root.join("two.txt");
        let moved = root.join("renamed.txt");
        fs::write(&first, "needle").unwrap();
        let snapshot = |folder: &Path| {
            let (_, candidates, _, truncated) = collect_candidates(folder, &CancellationToken::new()).unwrap();
            assert!(!truncated);
            serde_json::to_string(&candidates).unwrap()
        };
        let initial = snapshot(&root);
        fs::write(&second, "another file").unwrap();
        let with_new_file = snapshot(&root);
        assert_ne!(initial, with_new_file);
        fs::rename(&second, &moved).unwrap();
        let after_move = snapshot(&root);
        assert_ne!(with_new_file, after_move);
        fs::write(&first, "needle with changed bytes").unwrap();
        let after_mutation = snapshot(&root);
        assert_ne!(after_move, after_mutation);
        fs::remove_file(&moved).unwrap();
        let after_delete = snapshot(&root);
        assert_ne!(after_mutation, after_delete);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cached_result_requires_current_file_identity_and_handle_validation() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-cache-validate-{unique}"));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("invoice.txt");
        fs::write(&path, "invoice 2026").unwrap();
        let (canonical, candidates, _, _) = collect_candidates(&root, &CancellationToken::new()).unwrap();
        let candidate = &candidates[0];
        let cached = LocalSearchResult {
            cancelled: false,
            root_name: Some("cache-validate".into()),
            scanned_files: 1,
            matches: vec![LocalSearchMatch {
                path: candidate.path.display().to_string(),
                file_name: candidate.name.clone(),
                extension: candidate.extension.clone(),
                size_bytes: candidate.size,
                modified_unix_ms: candidate.modified_unix_ms,
                matched_in: vec!["text content".into()],
            }],
            truncated: false,
            note: "cached".into(),
            cache_hit: false,
        };
        assert!(validate_cached_matches(&canonical, &candidates, &cached, std::slice::from_ref(&candidate.identity)));
        fs::write(&path, "invoice changed to a longer value").unwrap();
        let (_, changed_candidates, _, _) = collect_candidates(&root, &CancellationToken::new()).unwrap();
        assert!(!validate_cached_matches(&canonical, &changed_candidates, &cached, std::slice::from_ref(&candidate.identity)));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn cached_result_is_rejected_after_symlink_replacement() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-cache-link-{unique}"));
        let outside = std::env::temp_dir().join(format!("ujani-cache-link-outside-{unique}"));
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let path = root.join("result.txt");
        let outside_file = outside.join("secret.txt");
        fs::write(&path, "public result").unwrap();
        fs::write(&outside_file, "outside secret").unwrap();
        let (canonical, candidates, _, _) = collect_candidates(&root, &CancellationToken::new()).unwrap();
        let candidate = &candidates[0];
        let cached = LocalSearchResult {
            cancelled: false,
            root_name: Some("cache-link".into()),
            scanned_files: 1,
            matches: vec![LocalSearchMatch {
                path: candidate.path.display().to_string(),
                file_name: candidate.name.clone(),
                extension: candidate.extension.clone(),
                size_bytes: candidate.size,
                modified_unix_ms: candidate.modified_unix_ms,
                matched_in: vec!["filename".into()],
            }],
            truncated: false,
            note: "cached".into(),
            cache_hit: false,
        };
        assert!(validate_cached_matches(&canonical, &candidates, &cached, std::slice::from_ref(&candidate.identity)));
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&outside_file, &path).unwrap();
        let (_, replaced_candidates, _, _) = collect_candidates(&root, &CancellationToken::new()).unwrap();
        assert!(!validate_cached_matches(&canonical, &replaced_candidates, &cached, std::slice::from_ref(&candidate.identity)));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

}
