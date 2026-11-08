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
pub fn get_local_clock() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .map_err(|error| format!("System clock is before the Unix epoch: {error}"))
}

/// Opens a native folder picker and previews duplicate files; it never deletes or changes files.
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

fn scan_duplicates(folder: &Path, token: &CancellationToken) -> Result<DuplicateScanResult, String> {
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
    ).map_err(|error| format!("Cannot scan selected folder: {error}"))?;
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
        if entry.size_bytes == 0 || entry.size_bytes > MAX_FILE_BYTES {
            if entry.size_bytes > MAX_FILE_BYTES { truncated = true; }
            continue;
        }
        let file = match crate::safe_fs::open_regular_file(&entry.path) {
            Ok(file) => file,
            Err(_) => { truncated = true; continue; }
        };
        let metadata = match file.metadata() {
            Ok(metadata) if metadata.is_file() => metadata,
            _ => { truncated = true; continue; }
        };
        if metadata.len() == 0 || metadata.len() > MAX_FILE_BYTES {
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
    'sizes: for (size, candidates) in by_size {
        if candidates.len() < 2 { continue; }
        for candidate in candidates {
            if token.is_cancelled() { return Err("Duplicate scan cancelled by user.".into()); }
            if total_hash_bytes.saturating_add(size) > MAX_TOTAL_HASH_BYTES {
                truncated = true;
                break 'sizes;
            }
            let digest = match hash_file(&candidate.path, MAX_FILE_BYTES) {
                Ok(digest) => digest,
                Err(_) => { truncated = true; continue; }
            };
            total_hash_bytes = total_hash_bytes.saturating_add(size);
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

fn hash_file(path: &Path, limit: u64) -> Result<String, String> {
    let mut file = crate::safe_fs::open_regular_file(path).map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > limit {
        return Err("File is not eligible for bounded hashing.".into());
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    file.by_ref().take(limit.saturating_add(1)).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit { return Err("File grew beyond the hashing limit.".into()); }
    Ok(super::sha256_hex(&bytes))
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
