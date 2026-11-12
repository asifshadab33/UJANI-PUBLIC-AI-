# UJANI Public AI — implementation status

**Status: early prototype / pre-alpha.** This document records the current verified implementation boundary so that the product does not claim capabilities that are still planned.

## Implemented

- Four-mode policy preview (Fortress, Smart, Worker, Partner) with registered operation checks, declared capability checks, memory preflight, explicit external-transfer approval and a capped Partner round count.
- Fortress never permits external data transfer. Smart selects local execution when its declared local requirements are met. Worker and Partner require explicit approval and a separate gateway capability.
- Real local inference through Ollama at `127.0.0.1:11434`; no remote Ollama endpoint or cloud fallback is configured.
- Bounded prompt/response handling and retry handling for transient local-service connection/time-out, throttling and server failures.
- Read-only OS CPU and memory reporting. GPU/NPU remain **unknown** until both hardware and a compatible compute runtime can be verified.
- User-selected text and PDF processing in memory, bounded document chunking, local summarisation and optional local translation.
- Bounded image-header inspection only; this is not image decoding, OCR or vision inference.
- Registered local operation and workflow-graph planning. A successful plan is not execution, resource reservation, filesystem permission or a sandbox.

## Lightweight media capability boundary

- **Documents:** selected supported text files and ordinary PDFs can be inspected within the documented file/text limits; the fixed local pipeline can summarise extracted text with Ollama and optionally translate the summary. Scanned-PDF OCR, complex-layout understanding and unbounded/very large document analysis are not implemented.
- **Images:** basic header/metadata inspection only. OCR, image-content understanding and vision question answering are not implemented.
- **Audio:** no local recording/transcription/speech-understanding pipeline is shipped.
- **Video:** no local scene-understanding, transcription or summarisation pipeline is shipped.
- These advanced media tasks are deliberately not implemented as bundled local model subsystems. The UI offers user-directed external AI websites for manual upload. UJANI does not upload media automatically; the current BYOK chat gateway accepts text prompts and does not transmit media attachments.
- This is a deliberate lightweight boundary, not a promise that the above capabilities are queued for local implementation. Any future native support requires a separate cost/complexity, resource and privacy review.

## Important policy distinctions

- Fortress means **no external task-data transfer**. Ollama is contacted over local loopback HTTP; this is local IPC/network communication and does not provide OS-wide network isolation.
- A mode selection is not consent. An explicit approval checkbox is required before a future external transfer can be considered.
- The router and resource scheduler are policy evaluators, not general executors. The native task engine invokes the local Ollama adapter; the separate BYOK gateway sends only explicitly approved Smart-mode requests to a user-configured provider.
- A model's self-reported confidence must never grant external-transfer permission. If a local model cannot complete a task, the current product should explain the limitation and ask the user before any future external gateway is used.
- Worker and Partner remain policy previews only. BYOK Smart-mode cloud requests are implemented through a bounded native gateway with a local daily token reservation and local execution receipts; provider-side billing controls, configurable monetary budgets, encrypted peer transport, dispatch and cross-device execution are not implemented.

## Deferred deliberately

1. **Cloud gateway hardening:** the BYOK gateway now validates requests before reserving a local daily token allowance and writes local hash-chained receipts after successful calls. Provider-specific integration tests, exact provider usage reconciliation, user-configurable monetary spend limits, field-level data minimisation and externally anchored audit evidence remain required before production.
2. **Persistent history / SQLite:** optional SQLite history exists with AES-256-GCM encrypted prompt/output fields and row/column authenticated data; the local receipt ledger is hash-chained and capped at 10,000 entries. The saved-history panel now supports user-confirmed export of the currently loaded entries (up to 100) to versioned JSON. The export is plaintext, is not encrypted by UJANI, and is not removed when in-app history is deleted; users must protect exported copies. Remaining gaps include key rotation/recovery, encrypted backup/restore, concurrent key initialization and end-to-end migration verification. The receipt ledger is local, unsigned and not externally anchored.
3. **GPU/NPU detection:** do not infer accelerator support from hardware presence alone. Verify driver/runtime compatibility and report unknown when uncertain.
4. **Full image processing:** add decoding/OCR/vision only with pixel, dimension, memory and decompression-bomb limits.
5. **Filesystem sandbox:** current file tools accept user-selected bytes from the UI and do not open arbitrary paths. A future folder tool must use OS-granted scoped access and canonical-path checks; never accept paths or shell commands generated by a model.
6. **Distributed execution:** device identity, native-confirmed peer enrollment/revocation, recipient-bound domain-separated protocol-v2 signatures with strict verification, and persistent replay nonce claims exist in the current remediation branch. Encrypted authenticated transport, dispatch, remote execution, peer resource budgets, organisation enrollment and remote receipt exchange remain unimplemented; Worker/Partner stay blocked.

## Verification

From the repository root:

```bash
cargo test --workspace
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
```

From `apps/desktop`:

```bash
npm ci
npm run build
```

A successful CI run verifies compilation/tests in its runner. It does not prove that Ollama is installed on a user's computer, that a model is downloaded, or that native inference works on every target device. For a manual smoke test, install Ollama from [the official download page](https://ollama.com/download), run `ollama pull llama3.2`, start the service, and test Fortress with a non-sensitive prompt.

## Current next milestone

Stabilise and test the local Fortress/Smart workflow on Windows, macOS and Linux, including Ollama unavailable, model missing, timeout, oversized prompt/response and low-memory cases. Do not start cloud escalation until the consent and gateway controls above are implemented.


## 2026-11-06 implementation update

- Added advisory installed-model fit estimates using Ollama-reported disk sizes and current OS-reported available RAM. The estimate uses model disk size × 2 + 512 MiB and reserves 25% of currently available RAM. It is not a guarantee: quantisation, context length, runtime and accelerator offload affect actual memory. The app never changes the selected model automatically.
- Added a native command that validates a versioned, allow-listed three-step document graph before executing the existing bounded local document pipeline. Only analyse → summarise → optional translate is executable; arbitrary graphs are never interpreted as code.
- The desktop UI now presents model fit estimates, lets the user explicitly choose the suggested installed model, and executes the fixed validated document workflow using already-selected text.
- GPU/NPU runtime verification, full image decoding/OCR, signed or externally anchored receipt verification, real distributed execution and production-grade cloud usage/cost controls remain deferred. Cross-platform native testing is still required.


## 2026-11-06 reliability follow-up

- The model-advice panel remains visible while estimates are loading and provides a retry action even when the initial recommendation request fails. Failures are shown to the user rather than silently hiding the panel.
- Added core tests for the exact advisory-memory boundary, deterministic tie ordering and an empty installed-model catalogue.
- The estimate remains advisory only. It is not used as a hard execution gate because disk size cannot reliably predict memory use across quantisation, context sizes, runtime configuration and accelerator offload.

## 2026-11-06 selected-folder import update

- Added a folder picker for user-selected batches of supported text files. The current batch is limited to 10 files, 256 KiB per file and 1 MiB total.
- Selected file bytes are read by the frontend picker and passed to existing native analysis/local summarisation commands one file at a time. Summaries use the selected local Ollama model only; no cloud transfer occurs.
- This is **not a persistent folder sandbox**: UJANI does not store an approved directory path, enumerate a directory through native filesystem APIs, or provide a reusable folder grant. The operating-system picker supplies the selected file objects; the app does not accept arbitrary paths or follow symlinks itself.
- Folder import is limited to supported text formats. PDF remains available through the existing single-file PDF workflow.
- A future native folder workflow must use an OS-scoped permission/grant, canonical path containment, symlink-safe traversal, explicit revocation and audit events before it can be described as a sandbox.

## Native folder import and cross-platform checks — 2026-11-06

- Replaced the browser `webkitdirectory` picker with the Tauri native folder dialog. The webview does not submit or construct a filesystem path.
- Added `folder_import.rs`: native command opens the OS picker, canonicalizes the selected directory, reads only direct regular-file children, rejects symlink entries and symlink roots, validates containment, accepts a fixed text-extension allow-list, enforces 10-file / 256 KiB-per-file / 1 MiB-total limits, validates UTF-8, and sorts results deterministically.
- Folder analysis runs locally. If a selected local Ollama model is available, each accepted file is summarised locally; otherwise analysis-only results are returned. No cloud transfer is implemented.
- Added native unit tests for allow-list filtering, non-recursive behaviour, deterministic ordering, file-count limit, per-file size, total size, invalid UTF-8, and symlink rejection on Unix.
- CI now runs the desktop Rust compile check and native unit tests on Ubuntu, Windows and macOS, in addition to core tests and the frontend build.
- The native OS dialog itself requires interactive desktop UI and is not automated in headless CI. It still needs manual smoke testing on Windows, macOS and Linux.
- This is a one-time picker workflow, not a persistent folder grant or a complete OS sandbox. The selected folder path is not stored. The implementation uses canonical-path and symlink checks, but does not claim race-free protection against a hostile process concurrently replacing filesystem entries.


## 2026-11-06 security follow-up

- Native IPC inputs now have explicit size/character bounds for task identifiers, route/capability requests, workflow identifiers and cancellation identifiers.
- The unused frontend `dialog:default` permission was removed. Native application-owned dialogs remain in use.
- History key bytes, encoded key material and the caller-owned base64 decode buffer use zeroizing storage. This reduces residual memory exposure but cannot guarantee erasure of copies inside dependencies, allocators or the OS credential provider.
- Optional cargo-fuzz targets exist for image-header and PDF-text parsers. A completed fuzz campaign is not claimed.
- The BYOK gateway chooses the completion-token parameter by recognised model family, including custom endpoints, with an explicit override for provider deployment aliases. This is a compatibility heuristic; live provider integration tests remain required.


## Rebased resource scheduling and provenance update — 2026-11-06

- The core now includes a deterministic resource-scheduling policy contract. It fails closed on stale/unknown telemetry, respects CPU/RAM/concurrency and thermal constraints, requires verified GPU runtime plus VRAM, and requires explicit owner opt-in/idle/AC-power policy for background work.
- The core now includes a typed execution receipt contract with exact-byte SHA-256 fields, adapter/model provenance, execution location, timestamps and external approval reference.
- At the time of this 2026-11-06 update, these additions were policy/schema foundations only; native telemetry, atomic reservation, task dispatch, durable receipt storage, signed receipt verification and real distributed execution were not yet implemented. The 2026-11-07 native enforcement update below supersedes the receipt-storage status.
- A reproducible benchmark protocol is documented; no benchmark campaign or measured performance claim is asserted.


## Native enforcement and audit update — 2026-11-07

- **Local admission control:** chat, selected-text summary, text/PDF document workflows, translation and folder import acquire an atomic process-local reservation. UJANI allows one local AI pipeline at a time and requires at least 1 GiB available RAM at admission. This is not an OS reservation and cannot prevent other applications from consuming RAM.
- **PDF worker limits:** the parser runs as a child process. Unix builds apply CPU, address-space, file-descriptor, file-size and core-dump limits; Linux also applies no_new_privs and seccomp denial of network, process/thread-creation and namespace syscalls; macOS uses a deny-network/file-write sandbox-exec profile and fails closed if unavailable. Windows builds use a Job Object with 768 MiB process-memory, 45-second CPU, one-active-process and kill-on-close limits. These are worker-level restrictions only: full filesystem isolation and full-application Fortress isolation are not implemented.
- **Execution receipts:** successful local chat, selected-text summaries, document/PDF pipelines, local translation and explicitly approved BYOK chat calls persist a domain-separated SHA-256 digest of the task inputs plus a digest of the serialized native result, task/adapter/model metadata and timestamps in a SQLite hash chain. Source text and PDF bytes are never written to the receipt ledger. The ledger is capped at 10,000 entries and exposes a verification command. It is not signed or anchored outside the local database; local database replacement/recreation cannot be detected by this mechanism alone. Model version is recorded as unknown where Ollama does not expose a verified digest.
- **Device identity and peer trust:** an Ed25519 device key is generated and stored in the OS credential vault; only the public key and its SHA-256 fingerprint are exposed. A native-confirmed rotation replaces the key and requires remote peers to re-enrol the new fingerprint. Peer enrollment requires a native OS confirmation dialog, peer revocation is persisted, and signed envelope verification checks registered operations, signature, payload digest, expiry, consent and a durable replay nonce. There is still no encrypted peer transport, dispatch or remote execution.
- **Cloud budget:** BYOK requests reserve a conservative maximum of 8,192 completion tokens plus an input estimate of one token per UTF-8 byte against a 250,000-token UTC-day local budget before network send. Failed/cancelled requests are not refunded. This can over-reserve and is not a provider-side billing limit; deleting local app data can reset the local counter.
- **GPU/NPU:** GPU use is classified only when Ollama reports positive VRAM allocation for a running model. Zero VRAM is not proof of CPU execution (integrated GPUs and incomplete telemetry exist); only positive Ollama VRAM telemetry is recorded as LocalGpu. All other placements are local_unverified. Driver compatibility, accelerator performance and NPU support remain unknown.
- **Installer size:** a cross-platform installer-size workflow now builds Windows MSI/NSIS, macOS DMG and Linux DEB/AppImage packages and records exact bytes/MiB in GitHub Actions summaries. Exact package sizes are not yet available until that workflow completes on the current code.


## Saved history export

The saved-history panel can export the currently loaded entries (up to the 100-entry retention limit) as a versioned JSON file after an explicit confirmation warning. The export contains decrypted prompts and results in plaintext; users must protect the downloaded file themselves. Deleting or clearing in-app history does not delete previously exported copies.


## Privacy receipt UI update

The desktop UI can verify the local receipt hash chain and display up to 10 of the latest 50 metadata-only records. Receipt creation is wired to supported local chat, approved cloud chat, selected-text summaries, selected-document/PDF commands that use the managed-task wrapper, and local translation. The `execute_document_workflow` and selected-folder import (`process_selected_folder`) paths do not currently create receipts; model-download operations are also uncovered. Local document-task receipts bind the operation, selected model and task inputs in a domain-separated digest; the result digest covers the serialized native result. The listing command refuses to return entries if full ledger verification fails. It displays adapter/model/location, timestamps, input/output SHA-256 digests and ledger record hashes; it does not expose prompts or outputs. The chain is not digitally signed or externally anchored, so it must not be described as a tamper-proof independent audit log.


## Audit remediation update — cancellable local tasks

The document/PDF and translation commands now share the native task registry used by local chat and model downloads. Each accepts a bounded task identifier, rejects duplicate active IDs, emits lifecycle events, observes explicit cancellation, and applies a 240-second overall time budget. The desktop UI passes IDs and exposes cancellation buttons. The shared wrapper also acquires the native process-local memory reservation before work begins. Cancellation is cooperative at async boundaries; CI and platform smoke tests are required before claiming verified behaviour.


## 2026-11-10 release-readiness update

- The current product remains pre-alpha. The uploaded Android ARM64 Actions artifact archive measured 16,718,772 bytes; this is not the raw APK file size. The workflow did not report the uncompressed APK size, so the reduction from the first 196 MB debug build is not verified. PR #178 adds raw-size reporting by disabling Rust debug symbols only for the Android CI development profile. PR #176 was merged to `main` as `e5764c6162d775ed759ceeac3f082d6f8475f988`.
- On the exact PR head, Android ARM64 compatibility, Rust core tests, Rust lint, React TypeScript build, Windows/macOS/Linux Tauri native test jobs, Rust dependency audit, CodeQL and installer-size audit all passed. The installer-size workflow also produced unsigned Windows, macOS and Linux preview artifacts. Post-merge CI and CodeQL for the new main commit were still running at the time of this update.
- This evidence verifies CI builds and tests only. No physical Android installation, device launch, upgrade/uninstall, real Ollama inference on tester hardware, or minimum-OS compatibility certification is claimed.
- Remaining release blockers include OS-enforced Fortress isolation, platform-verified PDF worker containment, physical-device smoke tests, signed/provenanced release artifacts and SBOM review, independent security assessment, provider-specific live cloud tests and the outstanding data-lifecycle checks described above.


### 2026-11-10 everyday-tools verification

- The native `everyday_tools` module and React `EverydayTools` panel are present and wired through the Tauri invoke handler.
- Device-local date/time is displayed without a network request.
- Read-only duplicate-file preview is implemented for a user-selected desktop folder. It groups same-size candidates and verifies content with SHA-256; traversal, file size, total hashed bytes, duplicate-group count and task duration are bounded. Symlinks/reparse points are skipped/rejected by the shared filesystem helpers. The feature never deletes, moves, renames or overwrites files.
- Android currently returns an explicit unsupported result for folder duplicate scanning because Storage Access Framework integration is not yet implemented. Do not claim duplicate scanning works on Android.
- This closes the initial Phase 1 slice of issue #44 for desktop only; remaining OS-backed utilities and the wider Everyday Tools roadmap in issue #46 remain open.
