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
    use super::{is_reparse_point, open_regular_file};
    use std::{fs, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn opens_regular_file_and_rejects_directory() {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ujani-safe-fs-{id}"));
        fs::create_dir_all(&root).unwrap();
        let file = root.join("data.txt");
        fs::write(&file, b"local data").unwrap();
        assert_eq!(fs::read(open_regular_file(&file).unwrap()).unwrap(), b"local data");
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
