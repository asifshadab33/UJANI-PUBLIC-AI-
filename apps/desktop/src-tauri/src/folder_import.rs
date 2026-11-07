//! Native, user-initiated folder import.
//!
//! The folder is selected by a native OS dialog inside Rust. No path is accepted
//! from the webview. Only direct child text files are read, in memory, with strict
//! batch limits. No directory grant is persisted and no recursive traversal occurs.

use serde::Serialize;
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};
use tauri_plugin_dialog::DialogExt;

const MAX_FILES: usize = 10;
const MAX_FILE_BYTES: usize = 256 * 1024;
const MAX_TOTAL_BYTES: usize = 1024 * 1024;
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "txt", "md", "csv", "json", "rs", "py", "js", "ts", "html", "xml", "log",
];

#[derive(Debug)]
struct SelectedTextFile {
    name: String,
    content: String,
    size_bytes: usize,
}

#[derive(Serialize)]
pub struct FolderFileResult {
    file_name: String,
    size_bytes: usize,
    word_count: usize,
    line_count: usize,
    top_keywords: Vec<String>,
    summary: Option<String>,
    note: String,
}

#[derive(Serialize)]
pub struct FolderImportResult {
    cancelled: bool,
    files: Vec<FolderFileResult>,
    total_bytes: usize,
}

/// Opens a native OS folder picker and processes only its immediate text-file children.
/// The command deliberately accepts no filesystem path from the webview.
#[tauri::command]
pub async fn process_selected_folder(
    app: tauri::AppHandle,
    control: tauri::State<'_, crate::TaskControlState>,
    task_id: String,
    mode: String,
    model: String,
) -> Result<FolderImportResult, String> {
    if mode != "fortress" && mode != "smart" {
        return Err("Folder processing is local-only. Select Fortress or Smart.".into());
    }
    if model.len() > 128 {
        return Err("Invalid local model identifier.".into());
    }

    // Keep the native folder picker and all path handling in Rust. The webview
    // never supplies a filesystem path or receives a persistent directory grant.
    let picker_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        let Some(folder) = picker_app.dialog().file().blocking_pick_folder() else {
            return Ok(None);
        };
        let path = folder
            .into_path()
            .map_err(|_| "The selected folder path is not a local filesystem path.".to_string())?;
        collect_selected_folder(&path).map(Some)
    })
    .await
    .map_err(|error| format!("Folder selection worker failed: {error}"))??;

    let Some(files) = selected else {
        return Ok(FolderImportResult {
            cancelled: true,
            files: Vec::new(),
            total_bytes: 0,
        });
    };

    // Bind the receipt to the exact selected filenames and contents, without
    // copying document data into the receipt ledger.
    let receipt_model = if model.trim().is_empty() {
        "analysis-only"
    } else {
        model.as_str()
    };
    // Preserve the caller-supplied model value separately so an actual model named
    // "analysis-only" cannot collide with the empty-model fallback above.
    let mut receipt_fields: Vec<&[u8]> = vec![mode.as_bytes(), model.as_bytes()];
    for file in &files {
        receipt_fields.push(file.name.as_bytes());
        receipt_fields.push(file.content.as_bytes());
    }
    let receipt = crate::local_receipt_context(
        "folder.import",
        receipt_model,
        &receipt_fields,
    )?;

    crate::run_managed_task(
        app,
        &control,
        task_id,
        "Folder import",
        receipt,
        async move {
            let total_bytes = files.iter().map(|file| file.size_bytes).sum();
            let ollama_available =
                !model.trim().is_empty() && crate::ollama::is_available().await;
            let mut results = Vec::with_capacity(files.len());

            for file in files {
                let analysis =
                    crate::task_engine::LocalTaskEngine::analyze_text(&file.name, &file.content)?;
                let (summary, note) = if ollama_available {
                    match crate::task_engine::LocalTaskEngine::process_document(
                        file.name.clone(),
                        file.content,
                        mode.clone(),
                        model.clone(),
                        None,
                    )
                    .await
                    {
                        Ok(document) => (
                            Some(document.summary),
                            "Analysed and summarised by local Ollama. No cloud transfer."
                                .to_string(),
                        ),
                        Err(error) => (
                            None,
                            format!("Analysis completed; local summary failed: {error}"),
                        ),
                    }
                } else {
                    (
                        None,
                        "Analysis only. Ollama is not available; no cloud service was contacted."
                            .to_string(),
                    )
                };

                results.push(FolderFileResult {
                    file_name: analysis.file_name,
                    size_bytes: analysis.size_bytes,
                    word_count: analysis.word_count,
                    line_count: analysis.line_count,
                    top_keywords: analysis.top_keywords,
                    summary,
                    note,
                });
            }

            Ok(FolderImportResult {
                cancelled: false,
                files: results,
                total_bytes,
            })
        },
    )
    .await
}

/// Reads a bounded, sorted set of supported direct-child text files without following links.
fn collect_selected_folder(folder: &Path) -> Result<Vec<SelectedTextFile>, String> {
    let folder_metadata = fs::symlink_metadata(folder)
        .map_err(|error| format!("Cannot inspect the selected folder: {error}"))?;
    if folder_metadata.file_type().is_symlink() || !folder_metadata.is_dir() {
        return Err("The selected location must be a real directory, not a symlink.".into());
    }

    let canonical_root = fs::canonicalize(folder)
        .map_err(|error| format!("Cannot resolve the selected folder: {error}"))?;
    let entries = fs::read_dir(&canonical_root)
        .map_err(|error| format!("Cannot read the selected folder: {error}"))?;
    let mut files = Vec::new();
    let mut total_bytes = 0usize;

    for entry in entries {
        let entry = entry.map_err(|error| format!("Cannot inspect a folder entry: {error}"))?;
        let file_type = entry.file_type()
            .map_err(|error| format!("Cannot inspect a folder entry: {error}"))?;

        if file_type.is_symlink() {
            return Err("The selected folder contains a symbolic link. Remove it and try again.".into());
        }
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path.extension().and_then(|value| value.to_str())
            .unwrap_or_default().to_ascii_lowercase();
        if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
            continue;
        }

        let canonical_file = fs::canonicalize(&path)
            .map_err(|error| format!("Cannot resolve selected file: {error}"))?;
        if canonical_file.parent() != Some(canonical_root.as_path()) {
            return Err("A selected file resolves outside the chosen folder.".into());
        }

        let metadata = fs::metadata(&canonical_file)
            .map_err(|error| format!("Cannot inspect selected file: {error}"))?;
        if !metadata.is_file() {
            return Err("Only regular text files can be imported.".into());
        }
        if metadata.len() > MAX_FILE_BYTES as u64 {
            return Err(format!("{} exceeds the 256 KiB per-file limit.", entry.file_name().to_string_lossy()));
        }

        let handle = File::open(&canonical_file)
            .map_err(|error| format!("Cannot open selected file: {error}"))?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        handle.take((MAX_FILE_BYTES + 1) as u64).read_to_end(&mut bytes)
            .map_err(|error| format!("Cannot read selected file: {error}"))?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err("A selected file grew beyond the 256 KiB limit while being read.".into());
        }

        total_bytes = total_bytes.checked_add(bytes.len())
            .ok_or_else(|| "Folder size overflow.".to_string())?;
        if total_bytes > MAX_TOTAL_BYTES {
            return Err("The selected folder exceeds the 1 MiB total import limit.".into());
        }
        let content = String::from_utf8(bytes)
            .map_err(|_| format!("{} is not valid UTF-8 text.", entry.file_name().to_string_lossy()))?;

        files.push(SelectedTextFile {
            name: entry.file_name().to_string_lossy().into_owned(),
            size_bytes: content.len(),
            content,
        });
        if files.len() > MAX_FILES {
            return Err("The selected folder contains more than 10 supported text files.".into());
        }
    }

    files.sort_by_cached_key(|file| file.name.to_lowercase());
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::{collect_selected_folder, MAX_FILE_BYTES, MAX_TOTAL_BYTES};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TestFolder(PathBuf);

    impl TestFolder {
        fn new() -> Self {
            let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ujani-folder-import-test-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create test folder");
            Self(path)
        }
    }

    impl Drop for TestFolder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reads_only_supported_immediate_text_files_in_sorted_order() {
        let folder = TestFolder::new();
        fs::write(folder.0.join("zeta.txt"), "last file").unwrap();
        fs::write(folder.0.join("alpha.md"), "first file").unwrap();
        fs::write(folder.0.join("ignored.exe"), "not imported").unwrap();
        fs::create_dir(folder.0.join("nested")).unwrap();
        fs::write(folder.0.join("nested").join("hidden.txt"), "not recursive").unwrap();

        let files = collect_selected_folder(&folder.0).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "alpha.md");
        assert_eq!(files[1].name, "zeta.txt");
    }

    #[test]
    fn enforces_ten_file_batch_limit() {
        let folder = TestFolder::new();
        for index in 0..11 {
            fs::write(folder.0.join(format!("file-{index}.txt")), "hello").unwrap();
        }
        assert!(collect_selected_folder(&folder.0).unwrap_err().contains("more than 10"));
    }

    #[test]
    fn rejects_oversized_file() {
        let folder = TestFolder::new();
        fs::write(folder.0.join("large.txt"), vec![b'x'; MAX_FILE_BYTES + 1]).unwrap();
        assert!(collect_selected_folder(&folder.0).unwrap_err().contains("256 KiB"));
    }

    #[test]
    fn enforces_total_batch_size_limit() {
        let folder = TestFolder::new();
        let payload = vec![b'x'; MAX_FILE_BYTES];
        for index in 0..5 {
            fs::write(folder.0.join(format!("file-{index}.txt")), &payload).unwrap();
        }
        assert!(MAX_TOTAL_BYTES < payload.len() * 5);
        assert!(collect_selected_folder(&folder.0).unwrap_err().contains("1 MiB"));
    }

    #[test]
    fn rejects_non_utf8_text() {
        let folder = TestFolder::new();
        fs::write(folder.0.join("invalid.txt"), [0xff, 0xfe, 0xfd]).unwrap();
        assert!(collect_selected_folder(&folder.0).unwrap_err().contains("UTF-8"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlinked_selected_root() {
        let folder = TestFolder::new();
        let link = folder.0.with_extension("link");
        std::os::unix::fs::symlink(&folder.0, &link).unwrap();

        let error = collect_selected_folder(&link).unwrap_err();
        assert!(error.contains("symlink"));
        let _ = fs::remove_file(link);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_entries_in_selected_folder() {
        let folder = TestFolder::new();
        let outside = folder.0.with_extension("outside.txt");
        fs::write(&outside, "outside").unwrap();
        std::os::unix::fs::symlink(&outside, folder.0.join("escape.txt")).unwrap();
        assert!(collect_selected_folder(&folder.0).unwrap_err().contains("symbolic link"));
        let _ = fs::remove_file(outside);
    }
}
