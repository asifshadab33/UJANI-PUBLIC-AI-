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

#[cfg(windows)]
mod windows_handle {
    use super::{is_reparse_point, OpenOptions};
    use std::{
        ffi::{c_void, OsStr},
        io,
        mem::size_of,
        os::windows::{
            ffi::OsStrExt,
            fs::OpenOptionsExt,
            io::{AsRawHandle, FromRawHandle},
        },
        path::{Component, Path, Prefix},
        ptr,
    };

    use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;

    // GetDriveTypeW values from the Windows SDK (WinBase.h). windows-sys 0.61
    // exposes the function in Storage::FileSystem but not these WinBase constants.
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_RAMDISK: u32 = 6;

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *mut u16,
    }

    #[repr(C)]
    struct ObjectAttributes {
        length: u32,
        root_directory: *mut c_void,
        object_name: *mut UnicodeString,
        attributes: u32,
        security_descriptor: *mut c_void,
        security_quality_of_service: *mut c_void,
    }

    // IO_STATUS_BLOCK is pointer-aligned on 64-bit Windows; repr(C) supplies
    // the required padding between Status and Information.
    #[repr(C)]
    struct IoStatusBlock {
        status: i32,
        information: usize,
    }

    #[link(name = "ntdll")]
    extern "system" {
        #[link_name = "NtCreateFile"]
        fn nt_create_file(
            file_handle: *mut *mut c_void,
            desired_access: u32,
            object_attributes: *mut ObjectAttributes,
            io_status_block: *mut IoStatusBlock,
            allocation_size: *mut i64,
            file_attributes: u32,
            share_access: u32,
            create_disposition: u32,
            create_options: u32,
            ea_buffer: *mut c_void,
            ea_length: u32,
        ) -> i32;
        #[link_name = "RtlNtStatusToDosError"]
        fn rtl_nt_status_to_dos_error(status: i32) -> u32;
    }

    const OBJ_CASE_INSENSITIVE: u32 = 0x0000_0040;
    const FILE_TRAVERSE: u32 = 0x0000_0020;
    const FILE_READ_DATA: u32 = 0x0000_0001;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const FILE_SHARE_READ_WRITE_DELETE: u32 = 0x0000_0007;
    const FILE_OPEN: u32 = 0x0000_0001;
    const FILE_DIRECTORY_FILE: u32 = 0x0000_0001;
    const FILE_NON_DIRECTORY_FILE: u32 = 0x0000_0040;
    const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x0000_0020;
    const FILE_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

    fn invalid_path(message: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, message)
    }

    fn verify_directory_handle(handle: &std::fs::File) -> io::Result<()> {
        let metadata = handle.metadata()?;
        if !metadata.is_dir() || is_reparse_point(&metadata) {
            return Err(invalid_path(
                "A selected directory component is not a real directory.",
            ));
        }
        Ok(())
    }

    /// Opens one child relative to an already-open directory handle. Each child is
    /// opened as the reparse point itself, never by following its target.
    fn open_relative_component(
        parent: &std::fs::File,
        name: &OsStr,
        directory: bool,
    ) -> io::Result<std::fs::File> {
        let mut wide: Vec<u16> = name.encode_wide().collect();
        if wide.is_empty()
            || wide.iter().any(|unit| {
                *unit == 0
                    || *unit == b'/' as u16
                    || *unit == b'\\' as u16
                    || *unit == b':' as u16
            })
        {
            return Err(invalid_path("Invalid Windows path component."));
        }
        let byte_length = wide
            .len()
            .checked_mul(size_of::<u16>())
            .and_then(|length| u16::try_from(length).ok())
            .ok_or_else(|| invalid_path("Windows path component is too long."))?;
        let mut object_name = UnicodeString {
            length: byte_length,
            maximum_length: byte_length,
            buffer: wide.as_mut_ptr(),
        };
        let mut attributes = ObjectAttributes {
            length: size_of::<ObjectAttributes>() as u32,
            root_directory: parent.as_raw_handle() as *mut c_void,
            object_name: &mut object_name,
            attributes: OBJ_CASE_INSENSITIVE,
            security_descriptor: ptr::null_mut(),
            security_quality_of_service: ptr::null_mut(),
        };
        let mut io_status = IoStatusBlock {
            status: 0,
            information: 0,
        };
        let mut handle = ptr::null_mut::<c_void>();
        let access = if directory {
            FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE
        } else {
            FILE_READ_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE
        };
        let options = FILE_SYNCHRONOUS_IO_NONALERT
            | FILE_OPEN_REPARSE_POINT
            | if directory {
                FILE_DIRECTORY_FILE
            } else {
                FILE_NON_DIRECTORY_FILE
            };

        // SAFETY: the parent handle remains live for the call; OBJECT_ATTRIBUTES
        // and UNICODE_STRING point to owned, correctly sized storage; NtCreateFile
        // returns a new handle which is transferred to std::fs::File below.
        let status = unsafe {
            nt_create_file(
                &mut handle,
                access,
                &mut attributes,
                &mut io_status,
                ptr::null_mut(),
                0,
                FILE_SHARE_READ_WRITE_DELETE,
                FILE_OPEN,
                options,
                ptr::null_mut(),
                0,
            )
        };
        if status < 0 {
            // SAFETY: RtlNtStatusToDosError accepts every NTSTATUS value.
            let win32_error = unsafe { rtl_nt_status_to_dos_error(status) };
            return Err(io::Error::from_raw_os_error(win32_error as i32));
        }
        if handle.is_null() {
            return Err(io::Error::other("Windows returned an empty file handle."));
        }

        // SAFETY: NtCreateFile succeeded and ownership of this fresh handle is
        // transferred exactly once to std::fs::File.
        Ok(unsafe { std::fs::File::from_raw_handle(handle) })
    }

    /// Opens the canonical selected root from its local drive root, pinning every
    /// ancestor by handle and rejecting any reparse point encountered on the way.
    /// UNC/network roots and non-drive device namespaces are rejected fail-closed.
    pub(super) fn open_directory_path_no_reparse(path: &Path) -> io::Result<std::fs::File> {
        let mut components = path.components();
        let drive = match components.next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Only local drive paths are supported for safe folder access.",
                    ));
                }
            },
            _ => return Err(invalid_path("Selected root must be an absolute drive path.")),
        };
        if !matches!(components.next(), Some(Component::RootDir)) {
            return Err(invalid_path("Selected root must be an absolute drive path."));
        }

        let drive_root = format!("\\\\?\\{}:\\", drive as char);
        let drive_type_path = format!("{}:\\", drive as char);
        let drive_type_wide: Vec<u16> = drive_type_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: drive_type_wide is NUL-terminated and remains alive for the call.
        let drive_type = unsafe { GetDriveTypeW(drive_type_wide.as_ptr()) };
        if drive_type != DRIVE_FIXED
            && drive_type != DRIVE_REMOVABLE
            && drive_type != DRIVE_RAMDISK
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Safe folder access is limited to local fixed, removable or RAM drives.",
            ));
        }

        let mut directory = OpenOptions::new()
            .access_mode(FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
            .share_mode(FILE_SHARE_READ_WRITE_DELETE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(drive_root)?;
        verify_directory_handle(&directory)?;

        for component in components {
            let name = match component {
                Component::Normal(name) => name,
                _ => return Err(invalid_path("Non-normal Windows path component rejected.")),
            };
            let child = open_relative_component(&directory, name, true)?;
            verify_directory_handle(&child)?;
            directory = child;
        }
        Ok(directory)
    }

    /// Resolves a relative path only through pinned directory handles. No path-based
    /// canonicalization or reopening occurs after the selected root is pinned.
    pub(super) fn open_relative_path_from_directory(
        mut directory: std::fs::File,
        relative: &Path,
    ) -> io::Result<std::fs::File> {
        let mut components = relative.components().peekable();
        while let Some(component) = components.next() {
            let name = match component {
                Component::Normal(name) => name,
                _ => return Err(invalid_path("Non-normal relative path component rejected.")),
            };
            let final_component = components.peek().is_none();
            let opened = open_relative_component(&directory, name, !final_component)?;
            if final_component {
                let metadata = opened.metadata()?;
                if !metadata.is_file() || is_reparse_point(&metadata) {
                    return Err(invalid_path(
                        "Selected path is not a regular non-reparse file.",
                    ));
                }
                return Ok(opened);
            }
            verify_directory_handle(&opened)?;
            directory = opened;
        }
        Err(invalid_path("Selected path does not name a file."))
    }

    pub(super) fn open_regular_file_under_root(
        root: &Path,
        path: &Path,
    ) -> io::Result<std::fs::File> {
        if !root.is_absolute() || !path.is_absolute() {
            return Err(invalid_path(
                "Selected root and file path must be absolute.",
            ));
        }
        let relative = path.strip_prefix(root).map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "File path is outside selected root.",
            )
        })?;
        let directory = open_directory_path_no_reparse(root)?;
        open_relative_path_from_directory(directory, relative)
    }
}

#[cfg(windows)]
pub(crate) fn open_regular_file_under_root(
    root: &Path,
    path: &Path,
) -> io::Result<std::fs::File> {
    windows_handle::open_regular_file_under_root(root, path)
}

/// Non-Unix/non-Windows fallback retains final-component no-follow checks and verifies
/// canonical containment. Windows uses native handle-relative traversal above.
#[cfg(not(any(unix, windows)))]
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
    fn handle_relative_open_rejects_selected_root_replaced_by_symlink() {
        use super::open_regular_file_under_root;
        use std::os::unix::fs::symlink;
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let parent = std::env::temp_dir().join(format!("ujani-safe-fs-root-swap-{id}"));
        let selected = parent.join("selected");
        let outside = parent.join("outside");
        fs::create_dir_all(&selected).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(selected.join("data.txt"), b"inside").unwrap();
        fs::write(outside.join("data.txt"), b"outside secret").unwrap();

        // Model an attacker replacing the selected root after enumeration but
        // before the importer reopens the canonical child path.
        let canonical_root = fs::canonicalize(&selected).unwrap();
        let target = canonical_root.join("data.txt");
        fs::remove_dir_all(&selected).unwrap();
        symlink(&outside, &selected).unwrap();

        assert!(open_regular_file_under_root(&canonical_root, &target).is_err());
        fs::remove_file(&selected).unwrap();
        fs::remove_dir_all(parent).unwrap();
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
        fs::write(selected.join("inside.txt"), b"inside").unwrap();
        let outside = parent.join("outside.txt");
        fs::write(&outside, b"outside").unwrap();
        let root = fs::canonicalize(&selected).unwrap();
        // Build the in-root target from the canonical root: temp directories may
        // be exposed through a symlinked prefix (for example /tmp on macOS).
        let inside = root.join("inside.txt");

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

    #[cfg(windows)]
    #[test]
    fn opens_selected_file_with_traverse_only_directory_handles() {
        use super::windows_handle::{open_directory_path_no_reparse, open_relative_path_from_directory};
        use std::io::Read;
        let parent = temp_root("traverse-without-list");
        let selected = parent.join("selected");
        fs::create_dir_all(&selected).unwrap();
        fs::write(selected.join("data.txt"), b"traverse-only").unwrap();

        let root = fs::canonicalize(&selected).unwrap();
        let pinned = open_directory_path_no_reparse(&root).unwrap();
        let mut opened = open_relative_path_from_directory(pinned, Path::new("data.txt")).unwrap();
        let mut contents = Vec::new();
        opened.read_to_end(&mut contents).unwrap();
        assert_eq!(contents, b"traverse-only");
        fs::remove_dir_all(parent).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn rejects_junctioned_ancestor_that_resolves_outside_selected_root() {
        use super::open_regular_file_under_root;
        use std::{process::Command, time::{SystemTime, UNIX_EPOCH}};
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let base = std::env::temp_dir().join(format!("ujani-safe-fs-junction-{id}"));
        let selected = base.join("selected");
        let outside = base.join("outside");
        let junction = selected.join("nested");
        fs::create_dir_all(&selected).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), b"outside secret").unwrap();

        let output = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside)
            .output()
            .expect("Windows cmd must be available to create a test junction");
        assert!(
            output.status.success(),
            "could not create test junction: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let root = fs::canonicalize(&selected).unwrap();
        let target = root.join("nested").join("secret.txt");
        assert!(
            open_regular_file_under_root(&root, &target).is_err(),
            "a path traversing a junction outside the selected root must be rejected"
        );
        fs::remove_dir(&junction).unwrap();
        fs::remove_dir_all(base).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn rejects_ancestor_junction_replacement_after_selected_root_is_pinned() {
        use super::windows_handle::{open_directory_path_no_reparse, open_relative_path_from_directory};
        use std::{process::Command, time::{SystemTime, UNIX_EPOCH}};
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let base = std::env::temp_dir().join(format!("ujani-safe-fs-pinned-root-{id}"));
        let selected = base.join("selected");
        let outside = base.join("outside");
        let nested = selected.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), b"outside secret").unwrap();
        fs::write(nested.join("inside.txt"), b"inside").unwrap();

        let root = fs::canonicalize(&selected).unwrap();
        let pinned_root = open_directory_path_no_reparse(&root).unwrap();

        fs::remove_file(nested.join("inside.txt")).unwrap();
        fs::remove_dir(&nested).unwrap();
        let output = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&nested)
            .arg(&outside)
            .output()
            .expect("Windows cmd must be available to create a test junction");
        assert!(
            output.status.success(),
            "could not create test junction: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let relative = Path::new("nested").join("secret.txt");
        assert!(
            open_relative_path_from_directory(pinned_root, &relative).is_err(),
            "a child junction installed after root selection must not redirect a handle-relative open"
        );
        fs::remove_dir(&nested).unwrap();
        fs::remove_dir_all(base).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn rejects_selected_root_replaced_by_junction_after_canonicalization() {
        use std::{process::Command, time::{SystemTime, UNIX_EPOCH}};
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let base = std::env::temp_dir().join(format!("ujani-safe-fs-root-junction-{id}"));
        let selected = base.join("selected");
        let outside = base.join("outside");
        fs::create_dir_all(&selected).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), b"outside secret").unwrap();

        let root = fs::canonicalize(&selected).unwrap();
        fs::remove_dir(&selected).unwrap();
        let output = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&selected)
            .arg(&outside)
            .output()
            .expect("Windows cmd must be available to create a test junction");
        assert!(
            output.status.success(),
            "could not create test junction: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let target = root.join("secret.txt");
        assert!(
            open_regular_file_under_root(&root, &target).is_err(),
            "a selected root replaced by a junction must be rejected"
        );
        fs::remove_dir(&selected).unwrap();
        fs::remove_dir_all(base).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn rejects_unc_device_and_drive_relative_roots_before_opening() {
        use super::windows_handle::open_directory_path_no_reparse;

        // These paths must be rejected by prefix parsing before any filesystem
        // handle is opened. Folder import deliberately supports only local drives.
        let unsupported = [
            r"\\server\share\folder",
            r"\\?\UNC\server\share\folder",
            r"\\.\PhysicalDrive0",
            r"C:relative\folder",
        ];
        for value in unsupported {
            assert!(
                open_directory_path_no_reparse(Path::new(value)).is_err(),
                "unsupported Windows root must fail closed: {value}"
            );
        }
    }

}
