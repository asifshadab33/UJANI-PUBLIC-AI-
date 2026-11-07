# PDF worker confinement: platform evidence and limitations

The PDF parser handles untrusted bytes in a child process. The controls below apply to that worker only; they do not sandbox the complete UJANI application.

## Linux

- The child applies resource limits before executing the worker: 45 seconds CPU, 768 MiB address space, 16 file descriptors, zero file growth and zero core dumps.
- It sets `no_new_privs` and installs seccomp before the worker executable runs.
- The seccomp filter fails closed on an unexpected syscall architecture and denies network socket operations, process/thread creation and namespace changes.
- The filter includes the x86_64 x32 ABI rejection. Supported architecture claims are limited to the architectures covered by the build/test matrix.

## macOS

- The child applies the same Unix resource limits and launches through `/usr/bin/sandbox-exec` with a profile denying network operations and file writes.
- Apple has deprecated `sandbox-exec`; UJANI currently uses it because a tested replacement is not available in this implementation. If the executable is absent or launch fails, PDF extraction fails closed.
- This worker profile is not a full application sandbox. Current CI builds/tests do not constitute independent validation on every Intel and Apple Silicon hardware generation.

## Windows

- The child is assigned to a Job Object with kill-on-close, one active process, a 45-second process CPU limit and a 768 MiB process-memory limit.
- Windows Job Object assignment occurs immediately after process creation, not atomically with creation. UJANI mitigates this interval by withholding all untrusted PDF bytes until assignment succeeds: the worker blocks reading stdin, and the parent only takes/writes stdin after successful assignment. If assignment fails, the parent kills the child before sending the bytes.
- The launcher process can execute before assignment, so this is a mitigation, not an atomic containment guarantee.
- The Job Object does not itself deny network access or arbitrary file writes. This build does not claim Windows worker network/file-system isolation; those capabilities remain unknown/unprovided pending a supported OS sandbox or restricted-token design and direct tests.

## Cross-platform lifecycle

- Parent cancellation, the 45-second extraction timeout and the outer managed-task timeout drop/kill the child process. The child is configured with kill-on-drop.
- Input is capped at 10 MiB and output at 1 MiB. Worker output is accepted only after successful child exit and UTF-8 validation.
- PDF extraction errors and output-limit failures are returned as errors; the parent does not silently treat them as empty extracted text.
- Resource limits and worker confinement do not prove absence of all side effects, do not attest the whole process tree, and do not replace OS-level application sandboxing.

## Evidence status

GitHub Actions validates compilation, Rust tests, CodeQL and installer builds on its Linux, Windows and macOS runners. Those checks are useful regression evidence, not a substitute for hands-on tests on separate macOS Intel/Apple Silicon devices or a security assessment of the deployed OS versions. Unsupported restrictions must remain labelled unknown rather than inferred from a successful build.
