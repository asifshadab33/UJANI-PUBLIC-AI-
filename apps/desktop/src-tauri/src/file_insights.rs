//! Bounded, read-only large-file discovery scoped to a user-selected folder.
use serde::Serialize;
use std::{path::Path, time::Duration};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

const MAX_DEPTH: usize = 6;
const MAX_ENTRIES: usize = 5_000;
const MAX_RESULTS: usize = 100;
const SCAN_TIMEOUT_SECS: u64 = 60;
const MIN_THRESHOLD_BYTES: u64 = 1024 * 1024;
const MAX_THRESHOLD_BYTES: u64 = 1024 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct LargeFileEntry {
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_unix_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct LargeFileScanResult {
    pub root_name: String,
    pub scanned_files: usize,
    pub large_files: Vec<LargeFileEntry>,
    pub truncated: bool,
    pub note: String,
}

/// Finds large files in a folder selected through the native OS picker.
/// The command reads filesystem metadata only; it never opens file contents or mutates files.
#[tauri::command]
pub async fn find_large_files(
    app: AppHandle,
    control: State<'_, super::TaskControlState>,
    task_id: String,
    min_size_bytes: u64,
) -> Result<LargeFileScanResult, String> {
    if !super::valid_command_identifier(&task_id, 96) {
        return Err("Invalid task identifier.".into());
    }
    if !(MIN_THRESHOLD_BYTES..=MAX_THRESHOLD_BYTES).contains(&min_size_bytes) {
        return Err("Choose a minimum size between 1 MiB and 1 TiB.".into());
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
    super::emit_task_progress(&app, &task_id, "starting", "Choose a folder to find large files.", 1);

    let picker_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        let Some(folder) = picker_app.dialog().file().set_title("Choose a folder to find large files").blocking_pick_folder() else {
            return Ok(None);
        };
        folder.into_path()
            .map(Some)
            .map_err(|_| "The selected folder is not a local filesystem path.".to_string())
    })
    .await
    .map_err(|error| format!("Folder picker failed: {error}"))??;

    if token.is_cancelled() {
        return Err("Large-file scan cancelled by user.".into());
    }
    let Some(folder) = selected else {
        return Err("Folder selection cancelled. Nothing was scanned.".into());
    };
    super::emit_task_progress(&app, &task_id, "scanning", "Reading file metadata in the selected folder.", 10);

    let worker_token = token.clone();
    let worker = tauri::async_runtime::spawn_blocking(move || scan_large_files(&folder, min_size_bytes, &worker_token));
    let result = tokio::select! {
        _ = token.cancelled() => Err("Large-file scan cancelled by user.".to_string()),
        outcome = tokio::time::timeout(Duration::from_secs(SCAN_TIMEOUT_SECS), worker) => {
            match outcome {
                Ok(Ok(value)) => value,
                Ok(Err(error)) => Err(format!("Large-file scan worker failed: {error}")),
                Err(_) => {
                    token.cancel();
                    Err("Large-file scan exceeded its 60-second time budget. Choose a smaller folder.".into())
                }
            }
        }
    };

    match &result {
        Ok(_) => super::emit_task_progress(&app, &task_id, "completed", "Large-file scan finished. No files were changed.", 100),
        Err(error) if error.contains("cancelled") => super::emit_task_progress(&app, &task_id, "cancelled", "Large-file scan cancelled.", 0),
        Err(error) => super::emit_task_progress(&app, &task_id, "failed", error, 0),
    }
    result
}

fn scan_large_files(
    folder: &Path,
    min_size_bytes: u64,
    token: &CancellationToken,
) -> Result<LargeFileScanResult, String> {
    let walked = crate::safe_fs::walk_directory(
        folder,
        crate::safe_fs::WalkOptions {
            max_depth: MAX_DEPTH,
            max_entries: MAX_ENTRIES,
            recursive: true,
            links: crate::safe_fs::LinkPolicy::Skip,
            errors: crate::safe_fs::WalkErrorPolicy::SkipAndMarkTruncated,
            file_filter: None,
            max_duration: Some(Duration::from_secs(SCAN_TIMEOUT_SECS)),
        },
        token,
    ).map_err(|error| {
        if error.contains("cancelled") {
            "Large-file scan cancelled by user.".to_string()
        } else {
            format!("Cannot scan selected folder: {error}")
        }
    })?;

    let root_name = walked.root.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Selected folder".into());
    let mut large_files = walked.files.into_iter()
        .filter(|entry| entry.size_bytes >= min_size_bytes)
        .map(|entry| LargeFileEntry {
            file_name: entry.name.to_string_lossy().into_owned(),
            path: entry.path.display().to_string(),
            size_bytes: entry.size_bytes,
            modified_unix_ms: entry.modified_unix_ms,
        })
        .collect::<Vec<_>>();
    large_files.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then_with(|| a.path.cmp(&b.path)));
    let mut truncated = walked.truncated;
    if large_files.len() > MAX_RESULTS {
        large_files.truncate(MAX_RESULTS);
        truncated = true;
    }
    Ok(LargeFileScanResult {
        root_name,
        scanned_files: walked.files_examined,
        large_files,
        truncated,
        note: "Metadata-only, read-only preview. File contents were not read and no files were changed.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("ujani-{label}-{unique}"))
    }

    #[test]
    fn returns_large_files_sorted_by_size_without_reading_contents() {
        let root = temp_dir("large-files");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("small.bin"), vec![1u8; 8]).unwrap();
        fs::write(root.join("nested").join("larger.bin"), vec![2u8; 32]).unwrap();
        fs::write(root.join("largest.bin"), vec![3u8; 64]).unwrap();

        let token = CancellationToken::new();
        let result = scan_large_files(&root, 16, &token).unwrap();
        assert_eq!(result.scanned_files, 3);
        assert_eq!(result.large_files.len(), 2);
        assert_eq!(result.large_files[0].file_name, "largest.bin");
        assert_eq!(result.large_files[1].file_name, "larger.bin");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn skips_symbolic_links() {
        use std::os::unix::fs::symlink;
        let root = temp_dir("large-file-links");
        fs::create_dir_all(&root).unwrap();
        let outside = temp_dir("large-file-outside");
        fs::write(&outside, vec![9u8; 64]).unwrap();
        symlink(&outside, root.join("linked.bin")).unwrap();

        let result = scan_large_files(&root, 16, &CancellationToken::new()).unwrap();
        assert!(result.large_files.is_empty());
        fs::remove_dir_all(root).unwrap();
        fs::remove_file(outside).unwrap();
    }
}
