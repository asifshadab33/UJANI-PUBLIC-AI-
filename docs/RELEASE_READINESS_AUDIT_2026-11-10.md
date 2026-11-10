# UJANI Public AI — release-readiness and liability audit

**Audit date:** 2026-11-10  
**Repository:** `asifshadab33/UJANI-PUBLIC-AI-`  
**Scope:** repository metadata, current main-branch documentation, selected native Rust security boundaries, recent pull-request evidence and hosted CI artifacts.  
**Limit:** this is a maintainer-level code/documentation review, not an independent penetration test, legal opinion, privacy certification or exhaustive line-by-line audit.

## Verified repository and build state

- PR #176 (Android debug-symbol reduction) merged to `main` as `e5764c6162d775ed759ceeac3f082d6f8475f988`.
- Exact PR-head jobs passed: Rust core tests, Rust lint, React TypeScript build, Tauri native tests on Windows/macOS/Linux, Rust dependency audit, CodeQL and Android ARM64 compatibility.
- The uploaded Android ARM64 Actions artifact archive measured **16,718,772 bytes**. This is the compressed artifact archive, not the raw APK file size. The workflow did not report the uncompressed APK size, so the reduction from the previously recorded 196 MB initial APK is not verified. PR #178 adds a raw APK size measurement to the workflow summary.
- The installer-size workflow produced unsigned preview artifacts for Windows, macOS and Linux.
- The post-merge CI and CodeQL workflows for the merge commit were still running at the time of this audit. The documentation commits that follow this report require their own fresh main-branch checks.
- No open PRs were present at the time of the repository check.

## Implemented code paths inspected

- `cloud_gateway.rs`: HTTPS-only BYOK gateway, bounded request/response handling, endpoint validation, explicit approval integration, and disabled redirects/proxy behavior (see module implementation and tests for exact guarantees).
- `native_consent.rs`: native Yes/No confirmation with cancellation, timeout and channel-closure failure paths returning false.
- `safe_fs.rs`: no-follow file opening and bounded directory traversal helpers; Unix symlink rejection tests are present.
- `pdf_worker.rs`: dedicated stdin/stdout PDF extraction sidecar with 10 MiB input and 1 MiB output bounds. These bounds do not themselves establish OS sandboxing.
- `security_store.rs`: local SQLite security metadata store for hash-chained receipts, replay claims and local cloud-token reservations. Local hash chains are tamper-evident only within their trust boundary; they are not externally anchored evidence.
- `task_engine.rs`, `local_search.rs`, `folder_import.rs`, `model_inventory.rs` and `ollama.rs` are present in the current native source tree. Their existence does not imply every future planner, media search, connector or distributed feature is complete.

## Findings and release gates

| Priority | Finding | Required evidence / action |
|---|---|---|
| P0 | Fortress is an application-level routing/data-transfer policy, not OS-wide network isolation. | Implement and test OS-enforced network/process/filesystem isolation on each supported platform before claiming sandbox or air-gap security. |
| P0 | Physical Android and desktop installation behavior is unverified. | Install, launch, upgrade, cancel/uninstall and relaunch on representative real devices; record OS/device/build identifiers and results. |
| P0 | Unsigned preview installers are not release-grade signed packages. | Configure protected signing keys/certificates, verify signatures/notarization, publish checksums, SBOM and provenance, and exercise rollback/update recovery. |
| P0 | The PDF sidecar's byte limits are not a complete sandbox. | Verify CPU, memory, process, filesystem, network and termination controls for the worker on every supported OS. |
| P1 | BYOK provider compatibility and billing/usage semantics vary. | Add provider-specific contract tests and verify reported usage, token-field behavior, endpoint/DNS protections and consent snapshots against real test tenants. |
| P1 | Local receipts and history have lifecycle limitations. | Complete key rotation/recovery, encrypted backup/restore, concurrent initialization and externally anchored provenance; clearly warn that JSON exports are plaintext and outside in-app deletion. |
| P1 | GPU/NPU and storage capabilities are not universally known. | Keep unknown values unknown until supported OS/runtime evidence is available; test real hardware and drivers before scheduling on accelerators. |
| P1 | The full personal-intelligence/connectors and media-search roadmap is not complete. | The desktop local clock and read-only SHA-256 duplicate-file preview are implemented (issue #44 Phase 1); folder duplicate scanning is not available in the Android preview. Consent-based calendar/mail/notes/reminder connectors, video indexing/search, OCR/vision, richer organisation tools and automatic multi-provider task decomposition remain gated until implemented and tested. |
| P2 | Support matrix is a proposed baseline, not compatibility certification. | Test the documented minimum OS versions and supported architectures before advertising support. |

## Privacy, legal and operational liabilities

- Keep the product local-first. No task payload should leave the device without the exact task's external-provider approval.
- Never describe a policy preview, capability declaration, signed assertion or CI pass as a security sandbox, hardware attestation or independent verification.
- Do not claim energy, cost, carbon or data-centre reduction without equivalent-workload measurements that include local, network, retry and cloud energy.
- The project is GPL-3.0. Preserve notices and comply with the licence when redistributing modified binaries.
- Existing legal notices are informational and do not replace jurisdiction-specific privacy, consumer, export, employment, data-protection or product-liability advice. Obtain qualified legal review before public distribution or enterprise deployment.
- Avoid collecting employee activity, prompts, raw file contents or secrets for telemetry. Any future analytics must be documented, minimised, opt-in where required and governed by retention/deletion controls.

## Release decision

**Do not label this build production-ready or suitable for sensitive production data.** It is a pre-alpha controlled preview. Hosted CI evidence supports further controlled testing, but physical-device acceptance, OS isolation, signed release provenance, provider validation and independent security review remain release gates.

## Evidence links

- [PR #176](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/176)
- [Android ARM64 compatibility workflow](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/42820535509)
- [Exact-head CI workflow](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/42820535473)
- [Exact-head CodeQL workflow](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/42820535608)
- [Unsigned installer-size workflow](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/42820535574)


### Additional everyday-tools check

The local date/time panel and selected-folder read-only duplicate preview are implemented and registered in the desktop Tauri IPC handler. The duplicate scanner is bounded (depth 6, 5,000 entries, 64 MiB/file, 512 MiB total hash input, 60 seconds) and hashes same-size candidates with SHA-256. It does not mutate files. Android explicitly reports this folder scan as unavailable pending Storage Access Framework support.
