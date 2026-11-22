//! Bounded, read-only recent-file discovery scoped to a native folder selection.
use serde::Serialize;
use std::{path::Path, time::{Duration, SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

const MAX_DEPTH: usize = 6;
const MAX_ENTRIES: usize = 5_000;
const MAX_RESULTS: usize = 100;
const SCAN_TIMEOUT_SECS: u64 = 60;

/// Validate caller-controlled scope before task registration or native OS interaction.
fn validate_recent_file_window(days: u32) -> Result<Duration, String> {
    if !(1..=365).contains(&days) {
        return Err("Choose a recent-file window between 1 and 365 days.".into());
    }
    Ok(Duration::from_secs(u64::from(days) * 24 * 60 * 60))
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentFileEntry {
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_unix_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct RecentFileScanResult {
    pub root_name: String,
    pub scanned_files: usize,
    pub recent_files: Vec<RecentFileEntry>,
    pub truncated: bool,
    pub note: String,
}

/// Find files modified within the requested number of days, metadata-only.
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn find_recent_files(
    app: AppHandle,
    control: State<'_, super::TaskControlState>,
    task_id: String,
    days: u32,
) -> Result<RecentFileScanResult, String> {
    let recent_window = validate_recent_file_window(days)?;
    let registration = super::register_active_task(&control, &task_id)?;
    let token = registration.token.clone();
    super::ensure_task_not_cancelled(&token)?;
    let snapshot = super::get_selected_folder_capability_snapshot_for_task(&token).await;
    super::emit_task_progress(&app, &task_id, "starting", "Checking local permissions and choosing a folder for recent-file preview.", 1);
    let picker_app = app.clone();
    let selected = super::run_selected_folder_preflight(
        &token,
        snapshot.as_ref().map_err(Clone::clone),
        &["file.recent.preview"],
        move || tauri::async_runtime::spawn_blocking(move || {
            let Some(folder) = picker_app.dialog().file().set_title("Choose a folder to find recent files").blocking_pick_folder() else {
                return Ok(None);
            };
            folder.into_path().map(Some)
                .map_err(|_| "The selected folder is not a local filesystem path.".to_string())
        }),
    )?;
    let selected = tokio::select! {
        _ = token.cancelled() => return Err("Recent-file scan cancelled by user.".into()),
        selected = selected => selected.map_err(|error| format!("Folder picker failed: {error}"))?,
    };
    let folder = super::require_selected_folder(selected, "Folder selection cancelled. Nothing was scanned.")?;
    if token.is_cancelled() {
        return Err("Recent-file scan cancelled by user.".into());
    }
    super::emit_task_progress(&app, &task_id, "scanning", "Reading file metadata in the selected folder.", 10);
    let cutoff = SystemTime::now()
        .checked_sub(recent_window)
        .ok_or_else(|| "Requested recent-file window is outside the supported time range.".to_string())?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "System clock is before the Unix epoch.".to_string())?
        .as_millis();
    let cutoff = u64::try_from(cutoff).map_err(|_| "System clock timestamp is out of range.".to_string())?;
    let worker_token = token.clone();
    let worker = tauri::async_runtime::spawn_blocking(move || scan_recent_files(&folder, cutoff, &worker_token));
    let result = tokio::select! {
        _ = token.cancelled() => Err("Recent-file scan cancelled by user.".to_string()),
        outcome = tokio::time::timeout(Duration::from_secs(SCAN_TIMEOUT_SECS), worker) => {
            match outcome {
                Ok(Ok(value)) => value,
                Ok(Err(error)) => Err(format!("Recent-file scan worker failed: {error}")),
                Err(_) => {
                    token.cancel();
                    Err("Recent-file scan exceeded its 60-second time budget. Choose a smaller folder.".into())
                }
            }
        }
    };
    match &result {
        Ok(_) => super::emit_task_progress(&app, &task_id, "completed", "Recent-file scan finished. No files were changed.", 100),
        Err(error) if error.contains("cancelled") => super::emit_task_progress(&app, &task_id, "cancelled", "Recent-file scan cancelled.", 0),
        Err(error) => super::emit_task_progress(&app, &task_id, "failed", error, 0),
    }
    result
}

fn scan_recent_files(folder: &Path, cutoff_unix_ms: u64, token: &CancellationToken) -> Result<RecentFileScanResult, String> {
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
            "Recent-file scan cancelled by user.".to_string()
        } else {
            format!("Cannot scan selected folder: {error}")
        }
    })?;
    let root_name = walked.root.file_name().map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Selected folder".into());
    let mut recent_files = walked.files.into_iter()
        .filter_map(|entry| {
            let modified = entry.modified_unix_ms?;
            (modified >= cutoff_unix_ms).then_some(RecentFileEntry {
                file_name: entry.name.to_string_lossy().into_owned(),
                path: entry.path.display().to_string(),
                size_bytes: entry.size_bytes,
                modified_unix_ms: modified,
            })
        })
        .collect::<Vec<_>>();
    recent_files.sort_by(|a, b| b.modified_unix_ms.cmp(&a.modified_unix_ms).then_with(|| a.path.cmp(&b.path)));
    let mut truncated = walked.truncated;
    if recent_files.len() > MAX_RESULTS {
        recent_files.truncate(MAX_RESULTS);
        truncated = true;
    }
    Ok(RecentFileScanResult {
        root_name,
        scanned_files: walked.files_examined,
        recent_files,
        truncated,
        note: "Metadata-only, read-only preview. File contents were not read and no files were changed.".into(),
    })
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn find_recent_files(
    _app: AppHandle,
    _control: State<'_, super::TaskControlState>,
    _task_id: String,
    _days: u32,
) -> Result<RecentFileScanResult, String> {
    Err("Folder-based recent-file scanning is not available in the Android preview; Android Storage Access Framework support is not implemented yet.".into())
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
    fn recent_file_window_accepts_only_one_through_365_days() {
        assert_eq!(
            validate_recent_file_window(1).unwrap(),
            Duration::from_secs(24 * 60 * 60),
        );
        assert_eq!(
            validate_recent_file_window(365).unwrap(),
            Duration::from_secs(365 * 24 * 60 * 60),
        );
        assert!(validate_recent_file_window(0).is_err());
        assert!(validate_recent_file_window(366).is_err());
    }

    #[test]
    fn ipc_validates_window_and_authorizes_before_opening_the_native_picker() {
        let source = include_str!("recent_files.rs");
        let command_start = source.find("pub async fn find_recent_files(")
            .expect("desktop recent-file IPC command exists");
        let command_end = source[command_start..]
            .find("\n#[cfg(target_os = \"android\")]\n#[tauri::command]")
            .expect("desktop recent-file command has a bounded end");
        let command = &source[command_start..command_start + command_end];
        let signature_end = command.find(") -> Result")
            .expect("recent-file command signature exists");
        let signature = &command[..signature_end];
        assert!(!signature.contains("Path"), "IPC must not accept a caller-supplied filesystem path");

        let validation = command.find("validate_recent_file_window(days)?")
            .expect("untrusted day window must be validated");
        let registration = command.find("register_active_task")
            .expect("task registration must be present");
        let initial_cancel_check = command.find("ensure_task_not_cancelled(&token)?")
            .expect("cancellation must be checked after task registration");
        let capability = command.find("get_selected_folder_capability_snapshot_for_task")
            .expect("fresh local capability snapshot must be present");
        let preflight = command.find("run_selected_folder_preflight")
            .expect("native local policy preflight must be present");
        let picker = command.find("blocking_pick_folder")
            .expect("native folder picker must be present");
        assert!(
            command[preflight..picker].contains("\"file.recent.preview\""),
            "recent-file IPC must request its own registered capability",
        );
        let picker_cancel_check = command.find("_ = token.cancelled()")
            .expect("cancellation must be observed while the native picker is pending");
        let selected_scope = command.find("require_selected_folder")
            .expect("selected folder must be required before scanning");
        let post_selection_cancel_check = command.find("if token.is_cancelled()")
            .expect("cancellation must be rechecked before traversal");
        let scan = command.find("scan_recent_files(&folder")
            .expect("bounded scanner must run only after selection");

        assert!(
            validation < registration
                && registration < initial_cancel_check
                && initial_cancel_check < capability
                && capability < preflight
                && preflight < picker
                && picker < picker_cancel_check
                && picker_cancel_check < selected_scope
                && selected_scope < post_selection_cancel_check
                && post_selection_cancel_check < scan,
            "recent-file IPC must validate input, register and observe cancellation, authorize its exact selected-folder operation, obtain native selection, and only then scan",
        );
    }

    #[test]
    fn returns_recent_files_sorted_newest_first_and_excludes_old_files() {
        let root = temp_dir("recent-files");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("old.txt"), b"old").unwrap();
        fs::write(root.join("new.txt"), b"new").unwrap();
        fs::write(root.join("nested").join("newer.txt"), b"newer").unwrap();
        let now = SystemTime::now();
        fs::File::options().write(true).open(root.join("old.txt")).unwrap().set_modified(now - Duration::from_secs(120)).unwrap();
        fs::File::options().write(true).open(root.join("new.txt")).unwrap().set_modified(now - Duration::from_secs(5)).unwrap();
        fs::File::options().write(true).open(root.join("nested").join("newer.txt")).unwrap().set_modified(now - Duration::from_secs(1)).unwrap();
        let cutoff = now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 - 60_000;
        let result = scan_recent_files(&root, cutoff, &CancellationToken::new()).unwrap();
        assert_eq!(result.recent_files.len(), 2);
        assert_eq!(result.recent_files[0].file_name, "newer.txt");
        assert_eq!(result.recent_files[1].file_name, "new.txt");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn skips_symbolic_links() {
        use std::os::unix::fs::symlink;
        let root = temp_dir("recent-file-links");
        fs::create_dir_all(&root).unwrap();
        let outside = temp_dir("recent-file-outside");
        fs::write(&outside, b"secret").unwrap();
        symlink(&outside, root.join("linked.txt")).unwrap();
        let cutoff = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 - 60_000;
        let result = scan_recent_files(&root, cutoff, &CancellationToken::new()).unwrap();
        assert!(result.recent_files.is_empty());
        fs::remove_dir_all(root).unwrap();
        fs::remove_file(outside).unwrap();
    }
}
