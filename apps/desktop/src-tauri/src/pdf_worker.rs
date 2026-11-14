use std::io::{Read, Write};

const MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const OUTPUT_TRUNCATED_MARKER: &[u8] = b"UJANI_PDF_OUTPUT_TRUNCATED_V1\n";

/// Accept a PDF header only within the first 1 KiB, matching the native upload gate.
fn has_pdf_header(data: &[u8]) -> bool {
    let prefix = &data[..data.len().min(1024)];
    prefix.windows(5).any(|window| window == b"%PDF-")
}

/// Extracts text from a validated PDF while enforcing the worker's input-size ceiling.
fn extract_pdf_text(data: &[u8]) -> Result<String, ()> {
    if data.len() > MAX_INPUT_BYTES || !has_pdf_header(data) {
        return Err(());
    }
    pdf_extract::extract_text_from_mem(data).map_err(|_| ())
}

/// Reports whether the worker must mark its bounded output as incomplete.
fn output_was_truncated(text: &str) -> bool {
    text.len() > MAX_OUTPUT_BYTES
}

/// Truncates extracted text at the configured byte limit without splitting UTF-8 characters.
fn bounded_output(text: &str) -> &str {
    let mut end = text.len().min(MAX_OUTPUT_BYTES);
    while !text.is_char_boundary(end) { end -= 1; }
    &text[..end]
}


#[cfg(target_os = "linux")]
const SYS_FCHMODAT2: libc::c_long = 452;

/// Closes inherited non-stdio descriptors and denies path-based file access after
/// the dynamic loader has finished. The worker receives PDF bytes only on stdin.
#[cfg(target_os = "linux")]
fn install_linux_filesystem_deny_filter() -> Result<(), ()> {
    // A parser compromise must not inherit an open handle to another user file.
    // Linux 5.9+ close_range is required; failure stops extraction before stdin is read.
    let closed = unsafe { libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) };
    if closed != 0 {
        return Err(());
    }

    let expected_arch = match std::env::consts::ARCH {
        "x86_64" => 0xC000_003E, // AUDIT_ARCH_X86_64
        "aarch64" => 0xC000_00B7, // AUDIT_ARCH_AARCH64
        _ => return Err(()),
    };
    let mut denied: Vec<libc::c_long> = vec![
        libc::SYS_openat, libc::SYS_openat2,
        libc::SYS_open_by_handle_at, libc::SYS_name_to_handle_at,
        libc::SYS_newfstatat, libc::SYS_statx, libc::SYS_readlinkat,
        libc::SYS_getdents64, libc::SYS_faccessat, libc::SYS_faccessat2,
        libc::SYS_unlinkat, libc::SYS_renameat, libc::SYS_renameat2,
        libc::SYS_mkdirat, libc::SYS_linkat, libc::SYS_symlinkat,
        libc::SYS_mknodat, libc::SYS_fchmodat, libc::SYS_fchownat,
        // fchmodat2 is a separate path-based permission-change syscall.
        SYS_FCHMODAT2,
        libc::SYS_utimensat, libc::SYS_ftruncate, libc::SYS_fallocate,
        libc::SYS_io_uring_setup, libc::SYS_io_uring_enter, libc::SYS_io_uring_register,
        libc::SYS_pidfd_open, libc::SYS_pidfd_getfd,
        libc::SYS_process_vm_readv, libc::SYS_process_vm_writev,
        libc::SYS_ptrace, libc::SYS_execve, libc::SYS_execveat,
    ];
    #[cfg(target_arch = "x86_64")]
    denied.extend([
        libc::SYS_open, libc::SYS_creat, libc::SYS_stat,
        libc::SYS_lstat, libc::SYS_access, libc::SYS_readlink,
        libc::SYS_getdents,
        // Legacy x86_64 pathname mutation syscalls are separate from *at variants.
        libc::SYS_unlink, libc::SYS_rename, libc::SYS_truncate,
        libc::SYS_chmod, libc::SYS_chown, libc::SYS_lchown,
        libc::SYS_utime, libc::SYS_utimes, libc::SYS_mkdir,
        libc::SYS_rmdir, libc::SYS_link, libc::SYS_symlink,
        libc::SYS_mknod, libc::SYS_chroot, libc::SYS_mount,
        libc::SYS_umount2, libc::SYS_pivot_root,
    ]);

    let mut filter = Vec::<libc::sock_filter>::with_capacity(denied.len() * 2 + 7);
    filter.push(libc::sock_filter {
        code: (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16,
        jt: 0, jf: 0, k: 4,
    });
    filter.push(libc::sock_filter {
        code: (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16,
        jt: 1, jf: 0, k: expected_arch,
    });
    filter.push(libc::sock_filter {
        code: (libc::BPF_RET | libc::BPF_K) as u16,
        jt: 0, jf: 0, k: 0x8000_0000, // KILL_PROCESS on an unexpected syscall ABI
    });
    filter.push(libc::sock_filter {
        code: (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16,
        jt: 0, jf: 0, k: 0,
    });
    #[cfg(target_arch = "x86_64")]
    {
        filter.push(libc::sock_filter {
            code: (libc::BPF_JMP | 0x40 | libc::BPF_K) as u16,
            jt: 0, jf: 1, k: 0x4000_0000, // Reject the x32 syscall ABI
        });
        filter.push(libc::sock_filter {
            code: (libc::BPF_RET | libc::BPF_K) as u16,
            jt: 0, jf: 0, k: 0x8000_0000,
        });
    }
    for syscall in denied {
        filter.push(libc::sock_filter {
            code: (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16,
            jt: 0, jf: 1, k: syscall as u32,
        });
        filter.push(libc::sock_filter {
            code: (libc::BPF_RET | libc::BPF_K) as u16,
            jt: 0, jf: 0, k: libc::SECCOMP_RET_ERRNO | libc::EPERM as u32,
        });
    }
    filter.push(libc::sock_filter {
        code: (libc::BPF_RET | libc::BPF_K) as u16,
        jt: 0, jf: 0, k: libc::SECCOMP_RET_ALLOW,
    });

    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(());
    }
    let program = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_mut_ptr(),
    };
    if unsafe {
        libc::prctl(
            libc::PR_SET_SECCOMP,
            libc::SECCOMP_MODE_FILTER,
            &program as *const _ as libc::c_ulong,
            0,
            0,
        )
    } != 0 {
        return Err(());
    }
    Ok(())
}

/// Dedicated sidecar entry point. It reads one bounded PDF from stdin and writes
/// only bounded UTF-8 text to stdout; diagnostics never contain document content.
pub fn run() {
    // The parent already applied network/process restrictions before exec. This
    // second filter runs after the loader starts the worker but before PDF bytes
    // are read, so parser code cannot open arbitrary filesystem paths.
    #[cfg(target_os = "linux")]
    if install_linux_filesystem_deny_filter().is_err() {
        std::process::exit(6);
    }

    let mut data = Vec::new();
    if std::io::stdin().take((MAX_INPUT_BYTES + 1) as u64).read_to_end(&mut data).is_err()
        || data.len() > MAX_INPUT_BYTES {
        std::process::exit(2);
    }
    let text = match extract_pdf_text(&data) {
        Ok(text) => text,
        Err(()) => std::process::exit(3),
    };
    let truncated = output_was_truncated(&text);
    if std::io::stdout().write_all(bounded_output(&text).as_bytes()).is_err() {
        std::process::exit(4);
    }
    // Keep stdout as plain extracted text. A fixed metadata-only stderr marker
    // lets the parent distinguish a complete result from a bounded prefix.
    if truncated && std::io::stderr().write_all(OUTPUT_TRUNCATED_MARKER).is_err() {
        std::process::exit(5);
    }
}

#[cfg(test)]
mod tests {
    use super::{bounded_output, extract_pdf_text, has_pdf_header, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES};

    fn minimal_pdf_with_text() -> Vec<u8> {
        minimal_pdf_with_text_with_prefix(&[])
    }

    fn minimal_pdf_with_text_with_prefix(prefix: &[u8]) -> Vec<u8> {
        let stream = b"BT /F1 12 Tf 72 200 Td (UJANI worker test) Tj ET\n";
        let mut content = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
        content.extend_from_slice(stream);
        content.extend_from_slice(b"endstream");
        let objects = [
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
            content,
        ];
        let mut pdf = prefix.to_vec();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            pdf.extend_from_slice(object);
            pdf.extend_from_slice(b"\nendobj\n");
        }
        let xref_offset = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n", objects.len() + 1).as_bytes());
        pdf
    }

    #[test]
    fn extracts_text_from_a_valid_minimal_pdf() {
        let output = extract_pdf_text(&minimal_pdf_with_text()).unwrap();
        assert!(output.contains("UJANI worker test"));
    }

    #[test]
    fn accepts_pdf_header_within_first_kib_and_rejects_later_header() {
        let pdf = minimal_pdf_with_text();
        assert!(has_pdf_header(&pdf));
        assert!(extract_pdf_text(&pdf).unwrap().contains("UJANI worker test"));

        let offset_header = minimal_pdf_with_text_with_prefix(&vec![b' '; 512]);
        assert!(has_pdf_header(&offset_header));
        // Header preflight accepts this offset, but parser compatibility is separate.
        let too_late = minimal_pdf_with_text_with_prefix(&vec![b' '; 1024]);
        assert!(!has_pdf_header(&too_late));
        assert!(extract_pdf_text(&too_late).is_err());
    }

    #[test]
    fn rejects_input_over_the_worker_limit() {
        let mut input = vec![b'x'; MAX_INPUT_BYTES + 1];
        input[..5].copy_from_slice(b"%PDF-");
        assert!(extract_pdf_text(&input).is_err());
    }

    #[test]
    fn rejects_non_pdf_and_malformed_pdf_bytes() {
        assert!(extract_pdf_text(b"not a pdf").is_err());
        assert!(extract_pdf_text(b"%PDF-1.7\nnot a valid document").is_err());
    }

    #[test]
    fn output_truncation_is_signalled_only_above_the_limit() {
        assert!(!super::output_was_truncated(&"x".repeat(MAX_OUTPUT_BYTES)));
        assert!(super::output_was_truncated(&"x".repeat(MAX_OUTPUT_BYTES + 1)));
    }


    #[cfg(target_os = "linux")]
    #[test]
    fn worker_filesystem_filter_denies_path_open_and_closes_inherited_descriptors() {
        use std::{
            ffi::CString,
            io::Write,
            os::{fd::AsRawFd, unix::fs::PermissionsExt},
            process::Command,
            time::{SystemTime, UNIX_EPOCH},
        };

        const PROBE: &str = "UJANI_PDF_FILESYSTEM_FILTER_PROBE";
        const INHERITED_FD: &str = "UJANI_PDF_FILESYSTEM_FILTER_FD";
        const MUTATION_SOURCE: &str = "UJANI_PDF_FILESYSTEM_MUTATION_SOURCE";
        const MUTATION_TARGET: &str = "UJANI_PDF_FILESYSTEM_MUTATION_TARGET";
        const ORIGINAL_CONTENT: &[u8] = b"UJANI disposable filesystem probe";

        if std::env::var_os(PROBE).is_some() {
            let inherited_fd: i32 = std::env::var(INHERITED_FD)
                .expect("inherited descriptor number")
                .parse()
                .expect("valid descriptor number");
            let source = CString::new(
                std::env::var(MUTATION_SOURCE).expect("disposable source path"),
            ).expect("source path contains no NUL");
            assert!(
                unsafe { libc::fcntl(inherited_fd, libc::F_GETFD) } >= 0,
                "test descriptor must be inherited before confinement",
            );
            super::install_linux_filesystem_deny_filter()
                .expect("worker filesystem boundary must install");
            let inherited_flags = unsafe { libc::fcntl(inherited_fd, libc::F_GETFD) };
            let inherited_error = std::io::Error::last_os_error();
            assert_eq!(inherited_flags, -1);
            assert_eq!(inherited_error.raw_os_error(), Some(libc::EBADF));
            let error = std::fs::File::open("/etc/passwd")
                .expect_err("path-based reads must be denied");
            assert_eq!(error.raw_os_error(), Some(libc::EPERM));

            #[cfg(target_arch = "x86_64")]
            {
                let target = CString::new(
                    std::env::var(MUTATION_TARGET).expect("disposable target path"),
                ).expect("target path contains no NUL");
                assert_eq!(unsafe { libc::unlink(source.as_ptr()) }, -1);
                assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EPERM));
                assert_eq!(unsafe { libc::rename(source.as_ptr(), target.as_ptr()) }, -1);
                assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EPERM));
                assert_eq!(unsafe { libc::truncate(source.as_ptr(), 0) }, -1);
                assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EPERM));
            }

            // Linux syscall 452 is fchmodat2 on both supported worker ABIs.
            // Use a disposable file and request mode 0000; the filter must reject
            // the path-based permission change before it can alter the file.
            assert_eq!(
                unsafe {
                    libc::syscall(
                        super::SYS_FCHMODAT2,
                        libc::AT_FDCWD,
                        source.as_ptr(),
                        0o000u32,
                        0u32,
                    )
                },
                -1,
                "fchmodat2 must be denied by the worker filter",
            );
            assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EPERM));

            let parsed = super::extract_pdf_text(&minimal_pdf_with_text())
                .expect("in-memory PDF parsing must work after filesystem confinement");
            assert!(parsed.contains("UJANI worker test"));
            std::io::stdout()
                .write_all(b"UJANI_PDF_STDIO_REMAINS_AVAILABLE")
                .expect("stdout remains available");
            return;
        }

        let inherited = std::fs::File::open("/etc/passwd").expect("test fixture exists");
        let inherited_fd = unsafe { libc::fcntl(inherited.as_raw_fd(), libc::F_DUPFD, 200) };
        assert!(inherited_fd >= 200, "create a non-stdio inherited descriptor");

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!(
            "ujani-pdf-seccomp-{}-{nonce}",
            std::process::id(),
        ));
        std::fs::create_dir(&test_dir).expect("create disposable probe directory");
        let source_path = test_dir.join("source.txt");
        let target_path = test_dir.join("renamed.txt");
        std::fs::write(&source_path, ORIGINAL_CONTENT).expect("create disposable source file");
        std::fs::set_permissions(&source_path, std::fs::Permissions::from_mode(0o600))
            .expect("set deterministic disposable-file permissions");

        let output = Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "pdf_worker::tests::worker_filesystem_filter_denies_path_open_and_closes_inherited_descriptors",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(PROBE, "1")
            .env(INHERITED_FD, inherited_fd.to_string())
            .env(MUTATION_SOURCE, source_path.as_os_str())
            .env(MUTATION_TARGET, target_path.as_os_str())
            .output()
            .expect("run isolated filter probe");
        unsafe { libc::close(inherited_fd); }

        let marker_present = output.stdout.windows(b"UJANI_PDF_STDIO_REMAINS_AVAILABLE".len())
            .any(|window| window == b"UJANI_PDF_STDIO_REMAINS_AVAILABLE");
        let source_content = std::fs::read(&source_path).expect("source file remains readable");
        let source_mode = std::fs::metadata(&source_path)
            .expect("source metadata remains readable")
            .permissions()
            .mode()
            & 0o777;
        let target_exists = target_path.exists();
        std::fs::remove_dir_all(&test_dir).expect("remove disposable probe directory");

        assert!(
            output.status.success(),
            "probe failed; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(marker_present, "worker stdout remains available");
        assert_eq!(source_content, ORIGINAL_CONTENT, "truncate/rename/unlink must not alter the source");
        assert_eq!(source_mode, 0o600, "fchmodat2 must not change source permissions");
        assert!(!target_exists, "rename must not create the target");
    }

    #[test]
    fn output_limit_preserves_utf8_boundaries() {
        let text = format!("{}x", "م".repeat(MAX_OUTPUT_BYTES / 2 + 10));
        let output = bounded_output(&text);
        assert_eq!(output.len(), MAX_OUTPUT_BYTES);
        assert!(text.is_char_boundary(output.len()));
        let expected = "م".repeat(MAX_OUTPUT_BYTES / "م".len());
        assert_eq!(output, expected);
    }
}
