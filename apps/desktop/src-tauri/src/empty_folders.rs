//! Bounded, read-only empty-directory discovery scoped to a native folder selection.
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

const MAX_DEPTH: usize = 6;
const MAX_DIRS: usize = 5_000;
const MAX_ENTRIES: usize = 10_000;
const MAX_RESULTS: usize = 100;
const MAX_RUNTIME: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize)]
pub struct EmptyFolderEntry {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct EmptyFolderScanResult {
    pub root_name: String,
    pub scanned_directories: usize,
    pub empty_folders: Vec<EmptyFolderEntry>,
    pub truncated: bool,
    pub note: String,
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn find_empty_folders(
    app: AppHandle,
    control: State<'_, super::TaskControlState>,
    task_id: String,
) -> Result<EmptyFolderScanResult, String> {
    let registration = super::register_active_task(&control, &task_id)?;
    let token = registration.token.clone();
    super::ensure_task_not_cancelled(&token)?;
    let snapshot = super::get_selected_folder_capability_snapshot_for_task(&token).await;
    super::emit_task_progress(
        &app,
        &task_id,
        "starting",
        "Choose a folder for an empty-folder preview.",
        1,
    );
    let picker_app = app.clone();
    let selected = super::run_selected_folder_preflight(
        &token,
        snapshot.as_ref().map_err(Clone::clone),
        &["file.empty_dirs.preview"],
        move || {
            tauri::async_runtime::spawn_blocking(move || {
                let Some(folder) = picker_app
                    .dialog()
                    .file()
                    .set_title("Choose a folder to find empty folders")
                    .blocking_pick_folder()
                else {
                    return Ok(None);
                };
                folder
                    .into_path()
                    .map(Some)
                    .map_err(|_| "Selected folder is not a local filesystem path.".to_string())
            })
        },
    )?;
    let selected = selected
        .await
        .map_err(|error| format!("Folder picker failed: {error}"))?;
    let folder = super::require_selected_folder(
        selected,
        "Folder selection cancelled. Nothing was scanned.",
    )?;
    let worker_token = token.clone();
    let worker =
        tauri::async_runtime::spawn_blocking(move || scan_empty_folders(&folder, &worker_token));
    let result = tokio::select! {
        _ = token.cancelled() => Err("Empty-folder scan cancelled by user.".into()),
        outcome = tokio::time::timeout(MAX_RUNTIME, worker) => match outcome {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(format!("Empty-folder scan worker failed: {error}")),
            Err(_) => {
                token.cancel();
                Err("Empty-folder scan exceeded its 60-second time budget. Choose a smaller folder.".into())
            }
        }
    };
    match &result {
        Ok(_) => super::emit_task_progress(
            &app,
            &task_id,
            "completed",
            "Empty-folder preview finished. No directories were changed.",
            100,
        ),
        Err(error) if error.contains("cancelled") => super::emit_task_progress(
            &app,
            &task_id,
            "cancelled",
            "Scan cancelled.",
            0,
        ),
        Err(error) => super::emit_task_progress(&app, &task_id, "failed", error, 0),
    }
    result
}

fn scan_empty_folders(
    root: &Path,
    token: &CancellationToken,
) -> Result<EmptyFolderScanResult, String> {
    let root_meta = fs::symlink_metadata(root)
        .map_err(|error| format!("Cannot inspect selected root: {error}"))?;
    if crate::safe_fs::is_reparse_point(&root_meta) || !root_meta.is_dir() {
        return Err("Choose a real directory, not a symbolic link or reparse point.".into());
    }
    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("Cannot resolve selected folder: {error}"))?;
    let root_name = canonical_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Selected folder".into());
    let start = Instant::now();
    let mut stack: Vec<(PathBuf, usize)> = vec![(canonical_root.clone(), 0)];
    let mut empty_folders = Vec::new();
    let mut visited = 0usize;
    let mut entries_seen = 0usize;
    let mut truncated = false;

    while let Some((directory, depth)) = stack.pop() {
        if token.is_cancelled() {
            return Err("Empty-folder scan cancelled by user.".into());
        }
        if start.elapsed() >= MAX_RUNTIME || visited >= MAX_DIRS || entries_seen >= MAX_ENTRIES {
            truncated = true;
            break;
        }
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.is_dir() && !crate::safe_fs::is_reparse_point(&metadata) => {
                metadata
            }
            Ok(_) => continue,
            Err(_) => {
                truncated = true;
                continue;
            }
        };
        let canonical = match fs::canonicalize(&directory) {
            Ok(path) if path.starts_with(&canonical_root) => path,
            _ => {
                truncated = true;
                continue;
            }
        };
        let _ = metadata;
        visited += 1;
        let mut entries = match fs::read_dir(&canonical) {
            Ok(entries) => entries,
            Err(_) => {
                truncated = true;
                continue;
            }
        };
        match entries.next() {
            None => {
                let name = canonical
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                empty_folders.push(EmptyFolderEntry {
                    path: canonical.display().to_string(),
                    name,
                });
            }
            Some(Err(_)) => {
                entries_seen += 1;
                truncated = true;
            }
            Some(Ok(first)) => {
                entries_seen += 1;
                if depth < MAX_DEPTH {
                    // Continue enumeration to discover child directories, without following links.
                    let mut pending = vec![first];
                    for entry in entries {
                        if token.is_cancelled() {
                            return Err("Empty-folder scan cancelled by user.".into());
                        }
                        if start.elapsed() >= MAX_RUNTIME || entries_seen >= MAX_ENTRIES {
                            truncated = true;
                            break;
                        }
                        entries_seen += 1;
                        match entry {
                            Ok(item) => pending.push(item),
                            Err(_) => truncated = true,
                        }
                        if pending.len() > MAX_DIRS {
                            truncated = true;
                            break;
                        }
                    }
                    for item in pending.into_iter().rev() {
                        let child = item.path();
                        match fs::symlink_metadata(&child) {
                            Ok(metadata)
                                if metadata.is_dir()
                                    && !crate::safe_fs::is_reparse_point(&metadata) =>
                            {
                                stack.push((child, depth + 1));
                            }
                            Ok(_) => {}
                            Err(_) => truncated = true,
                        }
                    }
                } else {
                    truncated = true;
                }
            }
        }
    }

    empty_folders.sort_by(|left, right| left.path.cmp(&right.path));
    if empty_folders.len() > MAX_RESULTS {
        empty_folders.truncate(MAX_RESULTS);
        truncated = true;
    }
    Ok(EmptyFolderScanResult {
        root_name,
        scanned_directories: visited,
        empty_folders,
        truncated,
        note: "Read-only preview. No directory contents were changed, deleted or moved. Symbolic links and reparse points are skipped.".into(),
    })
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn find_empty_folders(
    _app: AppHandle,
    _control: State<'_, super::TaskControlState>,
    _task_id: String,
) -> Result<EmptyFolderScanResult, String> {
    Err("Folder-based empty-folder scanning is not available in the Android preview; Storage Access Framework support is not implemented yet.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "ujani-empty-folders-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn finds_only_truly_empty_directories_and_does_not_modify_files() {
        let root = temp_dir();
        fs::create_dir_all(root.join("empty")).unwrap();
        fs::create_dir_all(root.join("nonempty/sub")).unwrap();
        fs::write(root.join("nonempty/file.txt"), b"x").unwrap();
        fs::write(root.join("nonempty/sub/keep.txt"), b"nested").unwrap();
        let result = scan_empty_folders(&root, &CancellationToken::new()).unwrap();
        assert_eq!(result.empty_folders.len(), 1);
        assert_eq!(result.empty_folders[0].name, "empty");
        assert!(root.join("nonempty/file.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancellation_is_observed_before_traversal() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let token = CancellationToken::new();
        token.cancel();
        assert!(
            scan_empty_folders(&root, &token)
                .unwrap_err()
                .contains("cancelled")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn selected_root_symlink_is_rejected() {
        use std::os::unix::fs::symlink;

        let real_root = temp_dir();
        let link_root = temp_dir();
        fs::create_dir_all(&real_root).unwrap();
        symlink(&real_root, &link_root).unwrap();
        assert!(scan_empty_folders(&link_root, &CancellationToken::new()).is_err());
        fs::remove_file(link_root).unwrap();
        fs::remove_dir_all(real_root).unwrap();
    }

    #[test]
    fn marks_results_incomplete_when_depth_limit_is_reached() {
        let root = temp_dir();
        let mut current = root.clone();
        fs::create_dir_all(&current).unwrap();
        for _ in 0..=MAX_DEPTH {
            current = current.join("nested");
            fs::create_dir_all(&current).unwrap();
        }
        let result = scan_empty_folders(&root, &CancellationToken::new()).unwrap();
        assert!(result.truncated);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn skips_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let root = temp_dir();
        fs::create_dir_all(root.join("empty")).unwrap();
        let outside = temp_dir();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("link")).unwrap();
        let result = scan_empty_folders(&root, &CancellationToken::new()).unwrap();
        assert_eq!(result.empty_folders.len(), 1);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
