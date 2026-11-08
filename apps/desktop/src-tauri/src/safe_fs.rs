//! Shared hardened file-opening primitives for user-selected local workflows.
//!
//! Callers retain their own scope, traversal, extension, size and cancellation policy.
//! This module centralizes the platform-specific no-follow open and handle validation.
use std::{io, path::Path};
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


use std::ffi::OsString;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

/// Whether encountered links/reparse points should be skipped or reject the selected folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkPolicy {
    Skip,
    Reject,
}

/// How traversal should handle unreadable entries and paths that escape the selected root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalkErrorPolicy {
    SkipAndMarkTruncated,
    Reject,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WalkOptions {
    /// Maximum directory depth below the selected root. Zero means immediate children only.
    pub max_depth: usize,
    /// Maximum total directory entries examined across the traversal.
    pub max_entries: usize,
    /// Whether to descend into child directories.
    pub recursive: bool,
    pub links: LinkPolicy,
    pub errors: WalkErrorPolicy,
}

#[derive(Debug, Clone)]
pub(crate) struct WalkFile {
    pub path: PathBuf,
    pub name: OsString,
    pub size_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct WalkResult {
    pub root: PathBuf,
    pub files: Vec<WalkFile>,
    pub truncated: bool,
}

/// Shared bounded directory walker. It never follows an encountered symbolic link or
/// reparse point intentionally, confines canonical paths to the selected root, and
/// leaves extension, size, content, hashing and cancellation-result policy to callers.
pub(crate) fn walk_directory(
    selected_root: &Path,
    options: WalkOptions,
    cancellation: &CancellationToken,
) -> Result<WalkResult, String> {
    let root_metadata = std::fs::symlink_metadata(selected_root)
        .map_err(|error| format!("Cannot inspect selected folder: {error}"))?;
    if is_reparse_point(&root_metadata) || !root_metadata.is_dir() {
        return Err("Choose a real directory, not a symbolic link or reparse point.".into());
    }
    let root = std::fs::canonicalize(selected_root)
        .map_err(|error| format!("Cannot resolve selected folder: {error}"))?;
    let mut stack = vec![(root.clone(), 0usize)];
    let mut files = Vec::new();
    let mut entries_seen = 0usize;
    let mut truncated = false;

    while let Some((directory, depth)) = stack.pop() {
        if cancellation.is_cancelled() {
            return Err("Directory traversal cancelled by user.".into());
        }
        let directory_metadata = match std::fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if options.errors == WalkErrorPolicy::Reject => {
                return Err(format!("Cannot inspect directory: {error}"));
            }
            Err(_) => { truncated = true; continue; }
        };
        if is_reparse_point(&directory_metadata) || !directory_metadata.is_dir() {
            if options.links == LinkPolicy::Reject {
                return Err("The selected folder contains a symbolic link or reparse point.".into());
            }
            continue;
        }
        let canonical_directory = match std::fs::canonicalize(&directory) {
            Ok(path) if path.starts_with(&root) => path,
            Ok(_) if options.errors == WalkErrorPolicy::Reject => {
                return Err("A directory resolves outside the selected folder.".into());
            }
            Err(error) if options.errors == WalkErrorPolicy::Reject => {
                return Err(format!("Cannot resolve a selected directory: {error}"));
            }
            _ => { truncated = true; continue; }
        };
        let iterator = match std::fs::read_dir(&canonical_directory) {
            Ok(entries) => entries,
            Err(error) if options.errors == WalkErrorPolicy::Reject || depth == 0 => {
                return Err(format!("Cannot read selected folder: {error}"));
            }
            Err(_) => { truncated = true; continue; }
        };

        for entry in iterator {
            if cancellation.is_cancelled() {
                return Err("Directory traversal cancelled by user.".into());
            }
            entries_seen = entries_seen.saturating_add(1);
            if entries_seen > options.max_entries {
                truncated = true;
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) if options.errors == WalkErrorPolicy::Reject => {
                    return Err(format!("Cannot inspect a folder entry: {error}"));
                }
                Err(_) => { truncated = true; continue; }
            };
            let path = entry.path();
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if options.errors == WalkErrorPolicy::Reject => {
                    return Err(format!("Cannot inspect a folder entry: {error}"));
                }
                Err(_) => { truncated = true; continue; }
            };
            if is_reparse_point(&metadata) {
                if options.links == LinkPolicy::Reject {
                    return Err("The selected folder contains a symbolic link or reparse point. Remove it and try again.".into());
                }
                continue;
            }
            if metadata.is_dir() {
                if options.recursive && depth < options.max_depth {
                    let canonical_child = match std::fs::canonicalize(&path) {
                        Ok(child) if child.starts_with(&root) => child,
                        Ok(_) if options.errors == WalkErrorPolicy::Reject => {
                            return Err("A child directory resolves outside the selected folder.".into());
                        }
                        Err(error) if options.errors == WalkErrorPolicy::Reject => {
                            return Err(format!("Cannot resolve child directory: {error}"));
                        }
                        _ => { truncated = true; continue; }
                    };
                    stack.push((canonical_child, depth + 1));
                } else if options.recursive && depth >= options.max_depth {
                    truncated = true;
                }
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            let canonical_file = match std::fs::canonicalize(&path) {
                Ok(file) if file.starts_with(&root) => file,
                Ok(_) if options.errors == WalkErrorPolicy::Reject => {
                    return Err("A selected file resolves outside the chosen folder.".into());
                }
                Err(error) if options.errors == WalkErrorPolicy::Reject => {
                    return Err(format!("Cannot resolve selected file: {error}"));
                }
                _ => { truncated = true; continue; }
            };
            let canonical_metadata = match std::fs::symlink_metadata(&canonical_file) {
                Ok(metadata) => metadata,
                Err(error) if options.errors == WalkErrorPolicy::Reject => {
                    return Err(format!("Cannot inspect selected file: {error}"));
                }
                Err(_) => { truncated = true; continue; }
            };
            if is_reparse_point(&canonical_metadata) || !canonical_metadata.is_file() {
                if options.links == LinkPolicy::Reject {
                    return Err("A selected file is not a regular non-reparse file.".into());
                }
                truncated = true;
                continue;
            }
            files.push(WalkFile {
                path: canonical_file,
                name: entry.file_name(),
                size_bytes: canonical_metadata.len(),
            });
        }
        if entries_seen > options.max_entries {
            break;
        }
    }

    Ok(WalkResult { root, files, truncated })
}


#[cfg(test)]
mod walk_tests {
    use super::{walk_directory, LinkPolicy, WalkErrorPolicy, WalkOptions};
    use std::{fs, time::{SystemTime, UNIX_EPOCH}};
    use tokio_util::sync::CancellationToken;

    fn root(label: &str) -> std::path::PathBuf {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("ujani-walk-{label}-{id}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn options(max_depth: usize, max_entries: usize, links: LinkPolicy) -> WalkOptions {
        WalkOptions {
            max_depth,
            max_entries,
            recursive: true,
            links,
            errors: WalkErrorPolicy::SkipAndMarkTruncated,
        }
    }

    #[test]
    fn shared_walker_bounds_depth_and_entry_count() {
        let folder = root("bounds");
        fs::write(folder.join("root.txt"), "root").unwrap();
        fs::create_dir_all(folder.join("one").join("two")).unwrap();
        fs::write(folder.join("one").join("nested.txt"), "nested").unwrap();
        fs::write(folder.join("one").join("two").join("deep.txt"), "deep").unwrap();

        let walked = walk_directory(&folder, options(1, 20, LinkPolicy::Skip), &CancellationToken::new()).unwrap();
        assert_eq!(walked.files.len(), 2);
        assert!(walked.truncated);

        let limited = walk_directory(&folder, options(6, 1, LinkPolicy::Skip), &CancellationToken::new()).unwrap();
        assert!(limited.truncated);
        assert!(limited.files.len() <= 1);
        fs::remove_dir_all(folder).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn shared_walker_skips_links_or_rejects_them_under_strict_policy() {
        use std::os::unix::fs::symlink;
        let folder = root("links");
        let outside = root("outside");
        fs::write(outside.join("secret.txt"), "secret").unwrap();
        symlink(outside.join("secret.txt"), folder.join("linked.txt")).unwrap();

        let skipped = walk_directory(&folder, options(2, 20, LinkPolicy::Skip), &CancellationToken::new()).unwrap();
        assert!(skipped.files.is_empty());
        assert!(!skipped.truncated);

        let rejected = walk_directory(&folder, options(0, 20, LinkPolicy::Reject), &CancellationToken::new());
        assert!(rejected.is_err());
        fs::remove_dir_all(folder).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn shared_walker_observes_cancellation_before_traversal() {
        let folder = root("cancel");
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        assert!(walk_directory(&folder, options(2, 20, LinkPolicy::Skip), &cancellation).is_err());
        fs::remove_dir_all(folder).unwrap();
    }
}
