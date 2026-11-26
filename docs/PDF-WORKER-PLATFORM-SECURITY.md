# PDF worker confinement: platform evidence and limitations

The PDF parser handles untrusted bytes in a child process. These controls apply to that worker only; they do not sandbox the complete UJANI application. UJANI's PDF feature is text extraction only. It does not execute PDF JavaScript, launch embedded files, follow document links, or fetch remote resources.

## Security design requirements

Treat every PDF as hostile input, regardless of extension or declared MIME type.

- Validate the selected file and enforce the 10 MiB input cap before expensive decoding or parser startup.
- Run parsing out of process. Do not fall back to in-process parsing when the worker or a required OS control is missing.
- Deny worker network access and access to files other than the explicitly supplied input channel. Do not rely on the PDF parser's own behavior to enforce this.
- Apply CPU, memory/address-space, process, descriptor, temporary-storage and output limits. Keep stdout/stderr draining bounded so a noisy worker cannot deadlock the parent or exhaust memory.
- Bound wall-clock time; propagate cancellation; terminate the child on timeout, cancellation, malformed output or parent failure. Do not release concurrency permits while blocking parser work is still running.
- Accept output only after successful worker exit and strict output validation. Truncated extraction must be reported as incomplete and must never be cached as a complete negative search result.
- Keep diagnostics metadata-only and bounded. Never log document contents, extracted text, credentials or private paths.
- Re-enable a platform only after native isolation tests, negative tests, installed-package tests and a supported OS-version/architecture matrix demonstrate the required boundary.

## Linux — current supported worker path, subject to device verification

- The child applies resource limits before executing the worker: 45 seconds CPU, 768 MiB address space, 16 file descriptors, zero file growth and zero core dumps.
- The parent sets `no_new_privs` and installs seccomp before exec to deny network socket operations, process/thread creation and namespace changes. The filter fails closed on an unexpected syscall architecture and rejects the x86_64 x32 ABI.
- After the dynamic loader has started the worker, but before it reads PDF bytes, the worker closes every inherited descriptor above stdin/stdout/stderr with `close_range` and stacks a second seccomp filter. It denies file opens, directory enumeration, file mutation, `io_uring`, pidfd descriptor transfer, process-memory access and exec syscalls.
- The parent pre-exec denylist and worker-local second filter both deny the separate `fchmodat2` permission-change syscall. Regression probes verify denied operations return `EPERM` and leave test files unchanged.
- The worker receives untrusted PDF bytes only through stdin and returns bounded text through stdout; stderr carries only the fixed truncation marker. If descriptor closure or filter installation fails, the worker exits before reading the PDF.
- Automated subprocess regressions cover inherited descriptor closure, denied path reads and path mutation attempts. This is worker-level confinement, not a full application sandbox.

**Evidence limitation:** Linux CI and subprocess tests do not prove that every deployed kernel, architecture or installed package enforces the intended boundary. Keep runtime capability status unknown until the supported-device probe and installed-package matrix pass.

## Windows — PDF extraction disabled

The previous Job Object configuration provided process/resource limits, but it did not deny arbitrary filesystem access or network access. A process may begin executing before Job Object assignment is complete. Withholding PDF bytes during that interval is a useful mitigation, not a complete isolation boundary.

The current build therefore fails closed and does not start the PDF parser on Windows. Do not re-enable it based only on Job Objects, restricted UI visibility or successful compilation. A replacement must use a supported OS-enforced design that denies filesystem and network access, applies resource limits, closes unintended inherited handles, and has negative tests for escape and unauthorized access.

## macOS — PDF extraction disabled

The previous worker profile used `/usr/bin/sandbox-exec`. Apple has deprecated this interface, and the existing profile did not establish the required denial of arbitrary file reads. It is not an acceptable production security boundary for untrusted PDF parsing.

The current build therefore fails closed and does not start the PDF parser on macOS. A replacement should use a supported macOS sandbox architecture—such as a separately configured sandboxed helper/XPC service with least-privilege entitlements—and prove that the helper cannot access unselected files or the network. The helper must receive only the user-selected document through an explicitly granted file capability. Test both Intel and Apple Silicon, supported OS versions, signed/installed packages, cancellation and recovery.

## Android

The desktop PDF worker path is not supported on Android. Do not infer support from shared Rust code or a successful Android compatibility build.

## Cross-platform lifecycle and output contract

- Parent cancellation, the 45-second extraction timeout and the outer managed-task timeout must terminate the worker. Child lifetime and cleanup must be bounded.
- Input is capped at 10 MiB and output at 1 MiB. Worker output is accepted only after successful child exit and UTF-8 validation.
- Worker stderr retention is bounded to 4 KiB and preserves the fixed `UJANI_PDF_OUTPUT_TRUNCATED_V1` marker.
- PDF extraction errors and output-limit failures are returned as explicit errors/incomplete results; they are not silently converted into empty extracted text or complete search misses.
- A started blocking decoder thread cannot be forcibly aborted by Tokio. Its concurrency permit must remain held until the thread actually exits.
- Missing workers, missing controls, failed sandbox setup, unexpected exit, timeout and cancellation must fail closed. There is no in-process parser fallback and no silent cloud fallback.

## Re-enable criteria for each disabled platform

A platform may be re-enabled only when all of the following are demonstrated on the exact release candidate:

1. OS-enforced filesystem and network denial for the parser process, including denial of unselected paths and inherited-handle abuse.
2. CPU, memory, process, descriptor, temporary-storage, input, output and wall-clock bounds.
3. Negative tests for unauthorized file reads/writes, network access, process creation and sandbox setup failure.
4. Cancellation, timeout, crash, retry and cleanup tests, including proof that no orphan worker remains.
5. Installed-package tests on every claimed OS version and architecture.
6. Clear UI state and recovery behavior for unsupported platforms and malformed/oversized/encrypted PDFs.
7. Independent review of the platform boundary and a recorded test matrix with package checksum, OS build, architecture and sanitized evidence.

A green hosted build is necessary but is not proof of installed-device isolation. Unsupported, untested or unknown controls must remain unavailable rather than being described as secure.
