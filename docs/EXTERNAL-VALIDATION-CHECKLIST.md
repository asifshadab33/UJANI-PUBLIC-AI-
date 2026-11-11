# UJANI Public AI — external validation checklist

Status: prepared for an independent test team. This checklist is not evidence that any test has passed. Record the exact application commit, installer checksum, OS build, device model and tester for every run.

## A. Filesystem boundary and local file handling

- [ ] Verify the selected root is not a symlink/reparse point.
- [ ] Verify relative paths, parent traversal, outside-root paths and directory targets are rejected.
- [ ] Verify final symlinks and Windows junction/reparse ancestors are rejected.
- [ ] Run a concurrent rename/replacement stress test against selected-root ancestors and final components on Windows, macOS and Linux.
- [ ] Confirm the opened file handle—not a later path re-resolution—is the object actually read and hashed.
- [ ] Confirm cancellation and permission errors do not leave partial output or hang the UI.

**Known limitation:** Unix handle-relative traversal exists in the current implementation branch. Windows currently uses canonical containment plus final-component no-follow checks; this does not close the parent-directory replacement race. Do not approve the Windows race-resistance gate until a handle-relative implementation and adversarial Windows test pass.

## B. PDF worker reliability and confinement

- [ ] Test valid, malformed, truncated, encrypted, scanned/image-only and oversized PDFs.
- [ ] Test 10 MiB input and 1 MiB output boundaries, including UTF-8 truncation.
- [ ] Test the 30-second upload decode limit and 45-second worker timeout.
- [ ] Cancel during preparation, input transfer, extraction, output reading and downstream summarisation.
- [ ] Verify the child process exits after timeout/cancellation and no orphan remains.
- [ ] Verify CPU, memory, file descriptor, file growth and process-count limits on each OS.
- [ ] Verify network and filesystem isolation independently; resource limits are not a sandbox.

**Known limitation:** Windows Job Objects do not deny network or filesystem reads. macOS uses deprecated `sandbox-exec`; Linux uses seccomp controls. These controls require independent platform verification.

## C. Privacy, history, cache and execution routing

- [ ] Confirm history is disabled by default and no OS credential is created until explicit opt-in.
- [ ] Confirm prompt/output ciphertext fails authentication when row ID, column or any AAD metadata changes.
- [ ] Confirm legacy history migration does not leave plaintext sibling fields after a corrupt field is encountered.
- [ ] Confirm cache is disabled by default, retention choices work, negative results expire, and clearing/revoking a scope removes associated entries.
- [ ] Inspect SQLite files and logs for plaintext prompts, file contents, tokens or secrets.
- [ ] Verify Fortress never sends task content outside the device.
- [ ] Verify Smart remains local-first and external assistance requires explicit approval.
- [ ] Verify cancellation, progress, errors and execution receipts are visible and truthful.

## D. Desktop and mobile workflow reliability

- [ ] Test navigation, back/forward, repeated link clicks, window resize and app resume.
- [ ] Test file picker cancellation, permission denial, large folder traversal and duplicate scanning.
- [ ] Test Ollama unavailable, model missing, model loading, inference timeout and service restart.
- [ ] Confirm Android reports desktop-only features unavailable rather than silently failing.
- [ ] Confirm no background scan or compute starts without a visible user action.

## E. Security and release integrity

- [ ] Run dependency audit and CodeQL against the exact candidate commit.
- [ ] Verify installer signatures, checksums, SBOM and provenance when release signing is configured.
- [ ] Test install, first launch, upgrade, rollback, uninstall and clean reinstall.
- [ ] Verify sidecar placement and execution permissions after installation and upgrade.
- [ ] Record unsupported OS versions and architectures; do not infer compatibility from CI runners.

## F. Evidence record

For each test record: test ID, date/time, tester, device model, OS version/build, app commit SHA, installer SHA-256, steps, expected result, actual result, logs with secrets removed, pass/fail, severity and linked issue/PR. A CI pass is not a substitute for this device record.

## Release decision

Do not classify the build as production-ready or suitable for sensitive production data until P0 security, installation, signing and independent-review gates have documented evidence. Keep remote/cloud routing and OS-isolation claims limited to the verified behavior of the tested build.
