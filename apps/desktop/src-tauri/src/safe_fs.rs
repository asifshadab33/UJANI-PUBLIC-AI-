//! Shared hardened file-opening primitives for user-selected local workflows.
//!
//! This module centralizes bounded no-follow directory traversal and platform-specific
//! file opening; callers retain extension, content, size and task-specific error policy.
use std::{io, path::Path, time::{Duration, Instant, UNIX_EPOCH}};
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



/// Opens a file beneath a canonical selected root using handle-relative, no-follow
/// traversal. On Unix every ancestor is opened with O_DIRECTORY|O_NOFOLLOW and the
/// final component with O_NOFOLLOW, so replacing a path component with a symlink
/// cannot redirect a later file read outside the selected directory.
#[cfg(unix)]
pub(crate) fn open_regular_file_under_root(
    root: &Path,
    path: &Path,
) -> io::Result<std::fs::File> {
    use std::{
        ffi::CString,
        os::fd::{AsRawFd, FromRawFd},
        os::unix::ffi::OsStrExt,
    };

    fn open_at(
        parent: &std::fs::File,
        name: &std::ffi::OsStr,
        flags: i32,
    ) -> io::Result<std::fs::File> {
        let name = CString::new(name.as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path component contains NUL"))?;
        // SAFETY: parent is a live directory descriptor; name is NUL-terminated;
        // openat returns a new owned descriptor or -1 and does not retain pointers.
        let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fd is a fresh descriptor returned by openat and ownership is transferred.
        Ok(unsafe { std::fs::File::from_raw_fd(fd) })
    }

    if !root.is_absolute() || !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "selected root and file path must be absolute",
        ));
    }
    let relative = path.strip_prefix(root)
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "file path is outside selected root",
            )
        })?;
    let mut components = root
        .components()
        .chain(relative.components())
        .filter_map(|component| {
        match component {
            std::path::Component::RootDir => None,
            std::path::Component::Normal(name) => Some(Ok(name)),
            _ => Some(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "non-normal path component rejected",
            ))),
        }
    }).peekable();

    let mut directory = std::fs::File::open("/")?;
    while let Some(component) = components.next() {
        let name = component?;
        let final_component = components.peek().is_none();
        let flags = if final_component {
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK
        } else {
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY
        };
        let opened = open_at(&directory, name, flags)?;
        if final_component {
            let metadata = opened.metadata()?;
            if !metadata.is_file() || is_reparse_point(&metadata) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "selected path is not a regular file",
                ));
            }
            return Ok(opened);
        }
        if !opened.metadata()?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "selected path ancestor is not a directory",
            ));
        }
        directory = opened;
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "selected path does not name a file",
    ))
}

/// Non-Unix fallback retains final-component no-follow checks and verifies the
/// canonical path is inside the selected root. Windows handle-relative traversal
/// remains a separate hardening task; this fallback does not claim to close its race.
#[cfg(not(unix))]
pub(crate) fn open_regular_file_under_root(
    root: &Path,
    path: &Path,
) -> io::Result<std::fs::File> {
    let canonical_root = std::fs::canonicalize(root)?;
    let canonical_path = std::fs::canonicalize(path)?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "file path is outside selected root",
        ));
    }
    open_regular_file(&canonical_path)
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
    fn handle_relative_open_rejects_symlinked_ancestor() {
        use super::open_regular_file_under_root;
        use std::os::unix::fs::symlink;
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-safe-fs-ancestor-{id}"));
        let selected = root.join("selected");
        let outside = root.join("outside");
        fs::create_dir_all(selected.join("nested")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), b"outside secret").unwrap();
        fs::write(selected.join("nested").join("data.txt"), b"inside").unwrap();
        let canonical_root = fs::canonicalize(&selected).unwrap();
        let target = canonical_root.join("nested").join("data.txt");
        assert_eq!(
            open_regular_file_under_root(&canonical_root, &target)
                .unwrap()
                .metadata()
                .unwrap()
                .len(),
            6
        );

        fs::remove_dir_all(canonical_root.join("nested")).unwrap();
        symlink(&outside, canonical_root.join("nested")).unwrap();
        assert!(open_regular_file_under_root(&canonical_root, &target).is_err());
        fs::remove_dir_all(root).unwrap();
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
    /// Optional predicate applied before opening regular files.
    pub file_filter: Option<fn(&Path) -> bool>,
    /// Optional total traversal time budget. Expiry returns a truncated partial result.
    pub max_duration: Option<Duration>,
}

#[derive(Debug, Clone)]
pub(crate) struct WalkFile {
    pub path: PathBuf,
    pub name: OsString,
    pub size_bytes: u64,
    pub modified_unix_ms: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct WalkResult {
    pub root: PathBuf,
    pub files: Vec<WalkFile>,
    /// Number of regular files encountered, including entries later rejected by a filter/open.
    pub files_examined: usize,
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
    let started = Instant::now();
    let mut stack = vec![(root.clone(), 0usize)];
    let mut files = Vec::new();
    let mut files_examined = 0usize;
    let mut entries_seen = 0usize;
    let mut truncated = false;

    while let Some((directory, depth)) = stack.pop() {
        if cancellation.is_cancelled() {
            return Err("Directory traversal cancelled by user.".into());
        }
        if options.max_duration.is_some_and(|limit| started.elapsed() >= limit) {
            truncated = true;
            break;
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
            if options.max_duration.is_some_and(|limit| started.elapsed() >= limit) {
                truncated = true;
                break;
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
            files_examined = files_examined.saturating_add(1);
            if options.file_filter.is_some_and(|filter| !filter(&path)) {
                continue;
            }
            // Open the final path without following links before canonicalizing it.
            // This closes the final-component symlink/reparse race at the file boundary.
            let handle = match open_regular_file(&path) {
                Ok(handle) => handle,
                Err(error) if options.errors == WalkErrorPolicy::Reject || options.links == LinkPolicy::Reject => {
                    return Err(format!("Cannot safely open selected file: {error}"));
                }
                Err(_) => { truncated = true; continue; }
            };
            let opened_metadata = match handle.metadata() {
                Ok(metadata) if metadata.is_file() && !is_reparse_point(&metadata) => metadata,
                _ if options.links == LinkPolicy::Reject => {
                    return Err("A selected file is not a regular non-reparse file.".into());
                }
                _ => { truncated = true; continue; }
            };
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
            let modified_unix_ms = opened_metadata.modified().ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64);
            files.push(WalkFile {
                path: canonical_file,
                name: entry.file_name(),
                size_bytes: opened_metadata.len(),
                modified_unix_ms,
            });
        }
        if entries_seen > options.max_entries
            || options.max_duration.is_some_and(|limit| started.elapsed() >= limit)
        {
            if options.max_duration.is_some_and(|limit| started.elapsed() >= limit) {
                truncated = true;
            }
            break;
        }
    }

    Ok(WalkResult { root, files, files_examined, truncated })
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
            file_filter: None,
            max_duration: None,
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
    fn shared_walker_filters_before_opening_files() {
        let folder = root("filter");
        fs::write(folder.join("keep.txt"), "keep").unwrap();
        fs::write(folder.join("ignore.exe"), "ignore").unwrap();
        let mut configured = options(0, 20, LinkPolicy::Skip);
        configured.file_filter = Some(|path| path.extension().is_some_and(|ext| ext == "txt"));
        let walked = walk_directory(&folder, configured, &CancellationToken::new()).unwrap();
        assert_eq!(walked.files.len(), 1);
        assert_eq!(walked.files[0].name, "keep.txt");
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn shared_walker_marks_expired_time_budget_as_truncated() {
        let folder = root("time-budget");
        fs::write(folder.join("file.txt"), "data").unwrap();
        let mut configured = options(2, 20, LinkPolicy::Skip);
        configured.max_duration = Some(std::time::Duration::ZERO);
        let walked = walk_directory(&folder, configured, &CancellationToken::new()).unwrap();
        assert!(walked.truncated);
        assert!(walked.files.is_empty());
        assert_eq!(walked.files_examined, 0);
        fs::remove_dir_all(folder).unwrap();
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


// Regression coverage for the caller-visible root/path containment contract.
#[cfg(test)]
mod selected_root_open_boundary_tests {
    use super::open_regular_file_under_root;
    use std::{fs, path::Path, time::{SystemTime, UNIX_EPOCH}};

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("ujani-open-boundary-{label}-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn rejects_relative_paths_outside_root_and_directory_targets() {
        let parent = temp_root("containment");
        let selected = parent.join("selected");
        fs::create_dir_all(&selected).unwrap();
        let inside = selected.join("inside.txt");
        let outside = parent.join("outside.txt");
        fs::write(&inside, b"inside").unwrap();
        fs::write(&outside, b"outside").unwrap();
        let root = fs::canonicalize(&selected).unwrap();

        assert!(open_regular_file_under_root(&root, Path::new("relative.txt")).is_err());
        assert!(open_regular_file_under_root(&root, &outside).is_err());
        assert!(open_regular_file_under_root(&root, &root).is_err());
        let mut opened = open_regular_file_under_root(&root, &inside).unwrap();
        use std::io::Read;
        let mut contents = Vec::new();
        opened.read_to_end(&mut contents).unwrap();
        assert_eq!(contents, b"inside");
        fs::remove_dir_all(parent).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_final_symlink_and_symlinked_root_component() {
        use std::os::unix::fs::symlink;
        let parent = temp_root("links");
        let selected = parent.join("selected");
        fs::create_dir_all(&selected).unwrap();
        let target = parent.join("secret.txt");
        fs::write(&target, b"secret").unwrap();

        let final_link = selected.join("final-link.txt");
        symlink(&target, &final_link).unwrap();
        let canonical_selected = fs::canonicalize(&selected).unwrap();
        assert!(open_regular_file_under_root(&canonical_selected, &final_link).is_err());

        let root_link = parent.join("selected-link");
        symlink(&selected, &root_link).unwrap();
        let path_through_link = root_link.join("ordinary.txt");
        fs::write(selected.join("ordinary.txt"), b"ordinary").unwrap();
        assert!(open_regular_file_under_root(&root_link, &path_through_link).is_err());

        fs::remove_dir_all(parent).unwrap();
    }
}
