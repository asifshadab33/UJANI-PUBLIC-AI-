//! Read-only everyday desktop utilities: local clock and duplicate-file preview.
//! All filesystem access is scoped to a folder chosen through the native OS picker.
use serde::Serialize;
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

const MAX_DEPTH: usize = 6;
const MAX_ENTRIES: usize = 5_000;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_HASH_BYTES: u64 = 512 * 1024 * 1024;
const MAX_DUPLICATE_GROUPS: usize = 100;
const SCAN_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Serialize)]
pub struct DuplicateFile {
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct DuplicateGroup {
    pub sha256: String,
    pub size_bytes: u64,
    pub files: Vec<DuplicateFile>,
}

#[derive(Debug, Serialize)]
pub struct DuplicateScanResult {
    pub root_name: String,
    pub scanned_files: usize,
    pub hashed_files: usize,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub truncated: bool,
    pub note: String,
}

struct Candidate {
    path: PathBuf,
    file_name: String,
    size_bytes: u64,
}

#[tauri::command]
/// Returns the current local device clock as milliseconds since the Unix epoch.
pub fn get_local_clock() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .map_err(|error| format!("System clock is before the Unix epoch: {error}"))
}

/// Opens a native folder picker and previews duplicate files; it never deletes or changes files.
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn find_duplicate_files(
    app: AppHandle,
    control: State<'_, super::TaskControlState>,
    task_id: String,
) -> Result<DuplicateScanResult, String> {
    if !super::valid_command_identifier(&task_id, 96) {
        return Err("Invalid task identifier.".into());
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
    super::emit_task_progress(&app, &task_id, "starting", "Choose a folder to scan for duplicate files.", 1);

    let picker_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        let Some(folder) = picker_app.dialog().file().set_title("Choose a folder to find duplicates").blocking_pick_folder() else {
            return Ok(None);
        };
        folder.into_path()
            .map(Some)
            .map_err(|_| "The selected folder is not a local filesystem path.".to_string())
    })
    .await
    .map_err(|error| format!("Folder picker failed: {error}"))??;

    if token.is_cancelled() {
        return Err("Duplicate scan cancelled by user.".into());
    }
    let Some(folder) = selected else {
        return Err("Folder selection cancelled. Nothing was scanned.".into());
    };
    super::emit_task_progress(&app, &task_id, "scanning", "Scanning the selected folder for duplicates.", 10);

    let worker_token = token.clone();
    let worker = tauri::async_runtime::spawn_blocking(move || scan_duplicates(&folder, &worker_token));
    let result = tokio::select! {
        _ = token.cancelled() => Err("Duplicate scan cancelled by user.".to_string()),
        outcome = tokio::time::timeout(std::time::Duration::from_secs(SCAN_TIMEOUT_SECS), worker) => {
            match outcome {
                Ok(Ok(value)) => value,
                Ok(Err(error)) => Err(format!("Duplicate scan worker failed: {error}")),
                Err(_) => {
                    token.cancel();
                    Err("Duplicate scan exceeded its 60-second time budget. Choose a smaller folder.".into())
                }
            }
        }
    };

    match &result {
        Ok(_) => super::emit_task_progress(&app, &task_id, "completed", "Duplicate scan finished. Review the preview; no files were changed.", 100),
        Err(error) if error.contains("cancelled") => super::emit_task_progress(&app, &task_id, "cancelled", "Duplicate scan cancelled.", 0),
        Err(error) => super::emit_task_progress(&app, &task_id, "failed", error, 0),
    }
    result
}

/// Scans a selected folder for duplicate files using bounded, no-follow traversal.
///
/// The operation is read-only and observes cancellation between directory entries.
fn scan_duplicates(folder: &Path, token: &CancellationToken) -> Result<DuplicateScanResult, String> {
    scan_duplicates_with_budget(folder, token, MAX_TOTAL_HASH_BYTES)
}

/// Scans a selected folder under a caller-supplied total hashing budget.
///
/// Smaller size buckets and path-sorted candidates make the same folder produce
/// deterministic results when the same budget is applied.
fn scan_duplicates_with_budget(
    folder: &Path,
    token: &CancellationToken,
    total_hash_budget: u64,
) -> Result<DuplicateScanResult, String> {
    let walked = crate::safe_fs::walk_directory(
        folder,
        crate::safe_fs::WalkOptions {
            max_depth: MAX_DEPTH,
            max_entries: MAX_ENTRIES,
            recursive: true,
            links: crate::safe_fs::LinkPolicy::Skip,
            errors: crate::safe_fs::WalkErrorPolicy::SkipAndMarkTruncated,
            file_filter: None,
        max_duration: None,
},
        token,
    ).map_err(|error| {
        if error.contains("cancelled") {
            "Duplicate scan cancelled by user.".to_string()
        } else {
            format!("Cannot scan selected folder: {error}")
        }
    })?;
    let selected_root = walked.root.clone();
    let root_name = walked.root.file_name().map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Selected folder".into());
    let mut by_size: HashMap<u64, Vec<Candidate>> = HashMap::new();
    let mut scanned_files = 0usize;
    let mut truncated = walked.truncated;

    for entry in walked.files {
        if token.is_cancelled() {
            return Err("Duplicate scan cancelled by user.".into());
        }
        scanned_files += 1;
        if entry.size_bytes > MAX_FILE_BYTES {
            truncated = true;
            continue;
        }
        let file = match crate::safe_fs::open_regular_file_under_root(&selected_root, &entry.path) {
            Ok(file) => file,
            Err(_) => { truncated = true; continue; }
        };
        let metadata = match file.metadata() {
            Ok(metadata) if metadata.is_file() => metadata,
            _ => { truncated = true; continue; }
        };
        if metadata.len() > MAX_FILE_BYTES {
            truncated = true;
            continue;
        }
        by_size.entry(metadata.len()).or_default().push(Candidate {
            path: entry.path,
            file_name: entry.name.to_string_lossy().into_owned(),
            size_bytes: metadata.len(),
        });
    }

    let mut total_hash_bytes = 0u64;
    let mut hashed_files = 0usize;
    let mut by_digest: HashMap<(u64, String), Vec<DuplicateFile>> = HashMap::new();
    let mut sized_buckets: Vec<(u64, Vec<Candidate>)> = by_size.into_iter().collect();
    // The shared byte budget must produce the same result for the same folder.
    // Hash smaller files first and sort each bucket by path before consuming it.
    sized_buckets.sort_by_key(|(size, _)| *size);
    for (size, mut candidates) in sized_buckets {
        if candidates.len() < 2 { continue; }
        candidates.sort_by(|left, right| left.path.cmp(&right.path));
        for candidate in candidates {
            if token.is_cancelled() { return Err("Duplicate scan cancelled by user.".into()); }
            let remaining_budget = total_hash_budget.saturating_sub(total_hash_bytes);
            if size > remaining_budget {
                truncated = true;
                continue;
            }
            let (digest, bytes_read) = match hash_file(&selected_root, &candidate.path, candidate.size_bytes, remaining_budget) {
                Ok(result) => result,
                Err((_, bytes_read)) => {
                    total_hash_bytes = total_hash_bytes.saturating_add(bytes_read);
                    truncated = true;
                    continue;
                }
            };
            total_hash_bytes = total_hash_bytes.saturating_add(bytes_read);
            hashed_files += 1;
            by_digest.entry((size, digest.clone())).or_default().push(DuplicateFile {
                file_name: candidate.file_name,
                path: candidate.path.display().to_string(),
                size_bytes: candidate.size_bytes,
            });
        }
    }

    let mut duplicate_groups: Vec<DuplicateGroup> = by_digest.into_iter()
        .filter_map(|((size_bytes, sha256), mut files)| {
            if files.len() < 2 { return None; }
            files.sort_by(|a, b| a.path.cmp(&b.path));
            Some(DuplicateGroup { sha256, size_bytes, files })
        })
        .collect();
    duplicate_groups.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then_with(|| a.sha256.cmp(&b.sha256)));
    if duplicate_groups.len() > MAX_DUPLICATE_GROUPS {
        duplicate_groups.truncate(MAX_DUPLICATE_GROUPS);
        truncated = true;
    }

    Ok(DuplicateScanResult {
        root_name,
        scanned_files,
        hashed_files,
        duplicate_groups,
        truncated,
        note: "Read-only duplicate preview. Files are grouped by matching size and SHA-256 content hash. Nothing is deleted, moved, renamed or overwritten.".into(),
    })
}

/// Computes a bounded content hash for a regular file without following a final-component link.
fn hash_file(root: &Path, path: &Path, expected_size: u64, limit: u64) -> Result<(String, u64), (String, u64)> {
    let mut file = crate::safe_fs::open_regular_file_under_root(root, path)
        .map_err(|error| (error.to_string(), 0))?;
    let metadata = file.metadata().map_err(|error| (error.to_string(), 0))?;
    if !metadata.is_file() || metadata.len() != expected_size || expected_size > limit {
        return Err(("File changed or is not eligible for bounded hashing.".into(), 0));
    }

    let capacity = usize::try_from(expected_size).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    if let Err(error) = file.by_ref().take(expected_size).read_to_end(&mut bytes) {
        return Err((error.to_string(), bytes.len() as u64));
    }
    let bytes_read = bytes.len() as u64;
    if bytes_read != expected_size {
        return Err(("File changed while it was being hashed.".into(), bytes_read));
    }
    let final_metadata = file.metadata().map_err(|error| (error.to_string(), bytes_read))?;
    if final_metadata.len() != expected_size {
        return Err(("File changed while it was being hashed.".into(), bytes_read));
    }
    Ok((super::sha256_hex(&bytes), bytes_read))
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("ujani-{label}-{unique}"))
    }

    #[test]
    fn detects_empty_files_as_duplicates() {
        let root = temp_dir("empty-duplicates");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("empty-a.txt"), b"").unwrap();
        fs::write(root.join("empty-b.txt"), b"").unwrap();
        let result = scan_duplicates(&root, &CancellationToken::new()).unwrap();
        assert_eq!(result.duplicate_groups.len(), 1);
        assert_eq!(result.duplicate_groups[0].size_bytes, 0);
        assert_eq!(result.duplicate_groups[0].files.len(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shared_hash_budget_processes_size_buckets_deterministically() {
        let root = temp_dir("duplicate-budget-order");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("small-a.txt"), b"aa").unwrap();
        fs::write(root.join("small-b.txt"), b"aa").unwrap();
        fs::write(root.join("large-a.txt"), b"bbbb").unwrap();
        fs::write(root.join("large-b.txt"), b"bbbb").unwrap();

        let first = scan_duplicates_with_budget(&root, &CancellationToken::new(), 4).unwrap();
        let second = scan_duplicates_with_budget(&root, &CancellationToken::new(), 4).unwrap();

        assert_eq!(first.hashed_files, 2);
        assert!(first.truncated);
        assert_eq!(first.duplicate_groups.len(), 1);
        assert_eq!(first.duplicate_groups[0].size_bytes, 2);
        assert_eq!(first.duplicate_groups[0].files[0].path, second.duplicate_groups[0].files[0].path);
        assert_eq!(first.duplicate_groups[0].sha256, second.duplicate_groups[0].sha256);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exhausted_hash_budget_does_not_hide_empty_file_duplicates() {
        let root = temp_dir("empty-duplicates-budget");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("empty-a.txt"), b"").unwrap();
        fs::write(root.join("empty-b.txt"), b"").unwrap();
        fs::write(root.join("nonempty-a.txt"), b"data").unwrap();
        fs::write(root.join("nonempty-b.txt"), b"data").unwrap();

        let result = scan_duplicates_with_budget(&root, &CancellationToken::new(), 0).unwrap();

        assert!(result.truncated);
        assert_eq!(result.hashed_files, 2);
        assert_eq!(result.duplicate_groups.len(), 1);
        assert_eq!(result.duplicate_groups[0].size_bytes, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detects_duplicates_by_content_not_name() {
        let root = temp_dir("duplicates");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("one.txt"), "same content").unwrap();
        fs::write(root.join("nested").join("renamed.bin"), "same content").unwrap();
        fs::write(root.join("different.txt"), "other content").unwrap();
        let result = scan_duplicates(&root, &CancellationToken::new()).unwrap();
        assert_eq!(result.scanned_files, 3);
        assert_eq!(result.duplicate_groups.len(), 1);
        assert_eq!(result.duplicate_groups[0].files.len(), 2);
        assert_eq!(result.duplicate_groups[0].files[0].size_bytes, 12);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn does_not_follow_symlinks() {
        let root = temp_dir("symlinks");
        let outside = temp_dir("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), "same content").unwrap();
        #[cfg(unix)] std::os::unix::fs::symlink(outside.join("secret.txt"), root.join("linked.txt")).unwrap();
        let result = scan_duplicates(&root, &CancellationToken::new()).unwrap();
        assert_eq!(result.scanned_files, 0);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn enforces_directory_depth_limit() {
        let root = temp_dir("depth");
        let mut deep = root.clone();
        for level in 0..=MAX_DEPTH { deep.push(format!("d{level}")); }
        fs::create_dir_all(&deep).unwrap();
        let result = scan_duplicates(&root, &CancellationToken::new()).unwrap();
        assert!(result.truncated);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancellation_stops_traversal() {
        let root = temp_dir("cancel");
        fs::create_dir_all(&root).unwrap();
        let token = CancellationToken::new();
        token.cancel();
        assert!(scan_duplicates(&root, &token).unwrap_err().contains("cancelled"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_a_symlink_as_the_selected_root() {
        let root = temp_dir("root");
        fs::create_dir_all(&root).unwrap();
        let _link = root.with_extension("link");
        #[cfg(unix)] {
            std::os::unix::fs::symlink(&root, &_link).unwrap();
            assert!(scan_duplicates(&_link, &CancellationToken::new()).is_err());
            fs::remove_file(_link).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}


#[cfg(target_os = "android")]
#[tauri::command]
pub async fn find_duplicate_files(
    _app: AppHandle,
    _control: State<'_, super::TaskControlState>,
    _task_id: String,
) -> Result<DuplicateScanResult, String> {
    Err("Folder-based duplicate scanning is not available in the Android preview; Android Storage Access Framework support is not implemented yet.".into())
}
