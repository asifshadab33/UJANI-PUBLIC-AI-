//! Shared hardened file-opening primitives for user-selected local workflows.
//!
//! Callers retain their own scope, traversal, extension, size and cancellation policy.
//! This module centralizes the platform-specific no-follow open and handle validation.
use std::{fs, io, ffi::OsString, path::{Path, PathBuf}};
#[cfg(any(unix, windows))]
use std::fs::OpenOptions;

/// Opens a regular file without following its final symlink/reparse point.
pub(crate) fn open_regular_file(path: &Path) -> io::Result<std::fs::File> {
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new().read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)?
    };
    #[cfg(windows)]
    let file = {
        use std::os::windows::fs::OpenOptionsExt;
        // Open the reparse point itself so handle metadata can reject it.
        OpenOptions::new().read(true).custom_flags(0x0020_0000).open(path)?
    };
    #[cfg(not(any(unix, windows)))]
    let file = std::fs::File::open(path)?;

    let metadata = file.metadata()?;
    if !metadata.is_file() || is_reparse_point(&metadata) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            "The selected path is not a regular non-reparse file."));
    }
    Ok(file)
}

/// Whether links encountered during a selected-folder walk are rejected or skipped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LinkPolicy { Reject, Skip }

/// Whether non-root traversal errors abort the walk or produce a truncated result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WalkErrorPolicy { Strict, BestEffort }

#[derive(Clone, Copy, Debug)]
pub(crate) struct WalkOptions {
    /// Root is depth zero; zero visits immediate children only.
    pub max_depth: usize,
    pub max_entries: usize,
    pub link_policy: LinkPolicy,
    pub error_policy: WalkErrorPolicy,
}

pub(crate) struct WalkEntry {
    pub path: PathBuf,
    pub name: OsString,
    pub is_file: bool,
}

pub(crate) struct WalkReport {
    pub root: PathBuf,
    pub entries: Vec<WalkEntry>,
    pub truncated: bool,
}

/// One bounded traversal implementation shared by local search and folder import.
/// The caller supplies cancellation and retains its own file-type, extension,
/// content-size and authorization policy. Returned file paths are canonical and
/// remain subject to open_regular_file before any content is read.
pub(crate) fn walk_directory<F>(
    selected_root: &Path,
    options: WalkOptions,
    mut is_cancelled: F,
) -> Result<WalkReport, String>
where
    F: FnMut() -> bool,
{
    if options.max_entries == 0 {
        return Err("Directory traversal entry limit must be greater than zero.".into());
    }
    let root_metadata = fs::symlink_metadata(selected_root)
        .map_err(|error| format!("Cannot inspect selected folder: {error}"))?;
    if is_reparse_point(&root_metadata) || !root_metadata.is_dir() {
        return Err("Choose a real directory, not a symbolic link or reparse point.".into());
    }
    let root = fs::canonicalize(selected_root)
        .map_err(|error| format!("Cannot resolve selected folder: {error}"))?;
    let mut stack = vec![(root.clone(), 0usize)];
    let mut entries = Vec::new();
    let mut entries_seen = 0usize;
    let mut truncated = false;

    while let Some((directory, depth)) = stack.pop() {
        if is_cancelled() {
            return Err("Directory traversal cancelled by user.".into());
        }
        let directory_metadata = match fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) => {
                if options.error_policy == WalkErrorPolicy::Strict {
                    return Err(format!("Cannot inspect a selected directory: {error}"));
                }
                truncated = true;
                continue;
            }
        };
        if is_reparse_point(&directory_metadata) || !directory_metadata.is_dir() {
            if options.link_policy == LinkPolicy::Reject {
                return Err("Selected folder contains a symbolic link or reparse point.".into());
            }
            truncated = true;
            continue;
        }
        let canonical_directory = match fs::canonicalize(&directory) {
            Ok(path) if path.starts_with(&root) => path,
            Ok(_) => {
                if options.error_policy == WalkErrorPolicy::Strict {
                    return Err("A directory resolves outside the selected folder.".into());
                }
                truncated = true;
                continue;
            }
            Err(error) => {
                if options.error_policy == WalkErrorPolicy::Strict {
                    return Err(format!("Cannot resolve a selected directory: {error}"));
                }
                truncated = true;
                continue;
            }
        };
        let iterator = match fs::read_dir(&canonical_directory) {
            Ok(iterator) => iterator,
            Err(error) if depth == 0 => return Err(format!("Cannot read selected folder: {error}")),
            Err(error) if options.error_policy == WalkErrorPolicy::Strict => {
                return Err(format!("Cannot read a selected subdirectory: {error}"));
            }
            Err(_) => {
                truncated = true;
                continue;
            }
        };

        for item in iterator {
            if is_cancelled() {
                return Err("Directory traversal cancelled by user.".into());
            }
            let item = match item {
                Ok(item) => item,
                Err(error) if options.error_policy == WalkErrorPolicy::Strict => {
                    return Err(format!("Cannot inspect a folder entry: {error}"));
                }
                Err(_) => {
                    truncated = true;
                    continue;
                }
            };
            entries_seen = entries_seen.saturating_add(1);
            if entries_seen > options.max_entries {
                if options.error_policy == WalkErrorPolicy::Strict {
                    return Err("Selected folder exceeds the bounded directory-entry limit.".into());
                }
                truncated = true;
                break;
            }
            let path = item.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if options.error_policy == WalkErrorPolicy::Strict => {
                    return Err(format!("Cannot inspect a folder entry: {error}"));
                }
                Err(_) => {
                    truncated = true;
                    continue;
                }
            };
            if is_reparse_point(&metadata) {
                if options.link_policy == LinkPolicy::Reject {
                    return Err("Selected folder contains a symbolic link or reparse point.".into());
                }
                continue;
            }
            let file_type = metadata.file_type();
            if file_type.is_dir() {
                entries.push(WalkEntry {
                    path: path.clone(), name: item.file_name(), is_file: false,
                });
                if depth < options.max_depth {
                    stack.push((path, depth + 1));
                } else if options.max_depth > 0 {
                    truncated = true;
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let canonical_file = match fs::canonicalize(&path) {
                Ok(path) if path.starts_with(&root) => path,
                Ok(_) => {
                    if options.error_policy == WalkErrorPolicy::Strict {
                        return Err("A selected file resolves outside the chosen folder.".into());
                    }
                    truncated = true;
                    continue;
                }
                Err(error) if options.error_policy == WalkErrorPolicy::Strict => {
                    return Err(format!("Cannot resolve selected file: {error}"));
                }
                Err(_) => {
                    truncated = true;
                    continue;
                }
            };
            entries.push(WalkEntry {
                path: canonical_file, name: item.file_name(), is_file: true,
            });
        }
        if entries_seen > options.max_entries {
            break;
        }
    }
    Ok(WalkReport { root, entries, truncated })
}

/// Detects links and Windows reparse points from filesystem metadata.
pub(crate) fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    { metadata.file_type().is_symlink() }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(any(unix, windows)))]
    { metadata.file_type().is_symlink() }
}

#[cfg(test)]
mod tests {
    use super::open_regular_file;
    #[cfg(unix)]
    use super::is_reparse_point;
    use std::{fs, io::Read, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn shared_walker_honors_depth_and_entry_limits() {
        use super::{walk_directory, LinkPolicy, WalkErrorPolicy, WalkOptions};
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-walker-limits-{id}"));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("top.txt"), b"top").unwrap();
        fs::write(root.join("nested").join("child.txt"), b"child").unwrap();
        let direct = walk_directory(&root, WalkOptions {
            max_depth: 0, max_entries: 8, link_policy: LinkPolicy::Skip,
            error_policy: WalkErrorPolicy::Strict,
        }, || false).unwrap();
        assert_eq!(direct.entries.iter().filter(|entry| entry.is_file).count(), 1);
        assert!(!direct.truncated);
        let bounded = walk_directory(&root, WalkOptions {
            max_depth: 2, max_entries: 1, link_policy: LinkPolicy::Skip,
            error_policy: WalkErrorPolicy::BestEffort,
        }, || false).unwrap();
        assert!(bounded.truncated);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn shared_walker_skips_or_rejects_links_by_caller_policy() {
        use super::{walk_directory, LinkPolicy, WalkErrorPolicy, WalkOptions};
        use std::os::unix::fs::symlink;
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-walker-links-{id}"));
        let outside = root.with_extension("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), b"secret").unwrap();
        symlink(outside.join("secret.txt"), root.join("link.txt")).unwrap();
        let skipped = walk_directory(&root, WalkOptions {
            max_depth: 0, max_entries: 8, link_policy: LinkPolicy::Skip,
            error_policy: WalkErrorPolicy::BestEffort,
        }, || false).unwrap();
        assert!(skipped.entries.is_empty());
        assert!(walk_directory(&root, WalkOptions {
            max_depth: 0, max_entries: 8, link_policy: LinkPolicy::Reject,
            error_policy: WalkErrorPolicy::Strict,
        }, || false).is_err());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn shared_walker_honors_cancellation() {
        use super::{walk_directory, LinkPolicy, WalkErrorPolicy, WalkOptions};
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-walker-cancel-{id}"));
        fs::create_dir_all(&root).unwrap();
        let result = walk_directory(&root, WalkOptions {
            max_depth: 0, max_entries: 8, link_policy: LinkPolicy::Skip,
            error_policy: WalkErrorPolicy::BestEffort,
        }, || true);
        assert!(result.unwrap_err().contains("cancelled"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn opens_regular_file_and_rejects_directory() {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-safe-fs-{id}"));
        fs::create_dir_all(&root).unwrap();
        let file = root.join("data.txt");
        fs::write(&file, b"local data").unwrap();
        let mut handle = open_regular_file(&file).unwrap();
        let mut contents = Vec::new();
        handle.read_to_end(&mut contents).unwrap();
        assert_eq!(contents, b"local data");
        assert!(open_regular_file(&root).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_final_symlink_without_reading_its_target() {
        use std::os::unix::fs::symlink;
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-safe-fs-link-{id}"));
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.txt");
        let link = root.join("link.txt");
        fs::write(&target, b"secret").unwrap();
        symlink(&target, &link).unwrap();
        assert!(open_regular_file(&link).is_err());
        assert!(is_reparse_point(&fs::symlink_metadata(&link).unwrap()));
        let _ = fs::remove_dir_all(root);
    }
}
