# Roadmap

This roadmap describes intended work, not completed features. Items are not production-ready until implemented, tested and documented.

## Phase 0 — foundations (current)
- [x] First-run workspace launcher with clear entry points for local AI chat, online BYOK AI, files, local setup, update center and explicitly unavailable Worker/Partner modes.
- [x] Bounded multi-turn local and online chat context with separate clear-chat controls and fresh exact-request cloud consent.
- [x] Define local-first product principles and explicit non-goals.
- [x] Add an initial typed Rust task/capability model and routing tests.
- [x] Document privacy, security and contribution expectations.
- [ ] Establish CI for formatting, linting, tests and dependency review.
- [ ] Agree a versioned task protocol shared with UJANI Enterprise AI.

## Phase 1 — desktop proof of concept
- [ ] Windows desktop shell (Tauri 2 + React/TypeScript).
- [x] Read-only initial inventory: OS, architecture, logical CPUs, total/available RAM and OS locale.
- [x] Initial GPU-use evidence from Ollama running-model VRAM telemetry; this does not replace full hardware/driver/runtime compatibility inventory.\n- [ ] Verified cross-platform GPU/NPU and model-runtime compatibility inventory (presence alone is insufficient).
- [x] Initial read-only capability registry with source, state and explanatory detail; GPU/NPU remain unknown until verified.
- [x] Allow-listed local task-plan preview with capability and coarse available-memory preflight; planning does not execute or reserve resources.
- [ ] Capability freshness/expiry, user permission metadata and runtime-specific accelerator compatibility.
- [x] Opt-in local SQLite task history with disabled-by-default recording, 30-day retention, 100-entry cap, per-entry deletion and clear-all controls; prompt/output encrypted with AES-256-GCM and key stored in OS credential vault (metadata remains plaintext).
- [x] First integrated local text-document pipeline: bounded analysis followed by Ollama summarisation.
- [x] Sequential, UTF-8-safe multi-section summarisation for documents up to 256 KiB, with bounded intermediate summaries and final synthesis; larger inputs require user splitting.
- [x] Optional second local-model step to translate document/PDF summaries into a user-selected language.
- [x] Registered PDF text extraction for user-selected PDFs (10 MiB input limit; scanned-PDF OCR not included).
- [ ] Image conversion and broader media metadata tools.
- [ ] OS-enforced Fortress isolation: Windows AppContainer/restricted token and firewall policy; macOS sandbox/entitlements; Linux namespaces/seccomp/Landlock where supported. Current app-level cloud blocking is not OS isolation.

## Phase 2 — local models and planning
- [x] Local translation operation through loopback Ollama (Fortress/Smart; bounded input).
- [x] Optional local translation stage in the integrated text/PDF summarisation workflow.
- [ ] Model catalogue with licence, checksum, source and compatibility metadata.
- [x] Integrate one local inference runtime behind a stable adapter (Ollama loopback adapter; prototype scope).
- [x] Initial registered local operation catalogue and conservative plan preview.
- [x] Versioned, bounded task graph schema with allow-listed operations, dependency validation, cycle rejection and deterministic topological planning (planning only; no general graph execution yet).
- [x] Execute one fixed, validated local text workflow (analyse → summarise → optional translate); arbitrary graph nodes are still not executable.
- [ ] Typed node input/output schemas and execution adapters for graph nodes.
- [ ] Verification and reproducible benchmark harness.

## Phase 3 — optional external assistance
- [x] User-configured OpenAI-compatible cloud gateway in Smart mode with approval bound to the exact endpoint, model, prompt and mode; API-key strings are zeroized in the native gateway where possible. Worker/Partner peer execution remains unavailable.
- [x] Native Fortress-mode rejection for cloud requests (application-level policy only).
- [x] Signed Worker/Partner task-envelope primitives with Ed25519 identity signatures, expiry, consent, allow-listed scopes, payload digest binding and in-memory replay checks (protocol foundation only).
- [ ] Authenticated peer enrollment, trusted-key lifecycle, encrypted peer transport, actual worker dispatch/execution and durable audit receipts. Until these exist, Worker/Partner are intentionally blocked from the BYOK cloud gateway.
- [ ] Per-subtask data minimisation and explicit approval receipts.
- [ ] Local-only mode tested with network disabled.
- [ ] Clear provenance for local, remote and deterministic-tool work.

## Phase 4 — hardening and community
- [x] Tauri updater plugin, startup update check, signed update verification and user-controlled install/restart UI.
- [x] Tagged multi-platform installer workflow for Windows, macOS Intel/Apple Silicon and Linux, with merged updater manifest and SHA-256 checksums.
- [x] Release pipeline gates packaging on frontend builds and Rust tests on Windows, macOS and Linux.
- [ ] Configure owner-held updater key pair in GitHub Actions secrets and publish/verify the first signed release.
- [ ] Configure Windows Authenticode and Apple Developer ID signing/notarization secrets before broad distribution.
- [ ] Verified GPU/NPU detection and model-runtime compatibility across Windows, macOS and Linux. Current GPU status becomes Available only when Ollama reports positive VRAM allocation for a running model; this is runtime evidence, not a full hardware/driver inventory. NPU remains Unknown.
- [ ] OS-enforced Fortress isolation: Windows AppContainer/restricted token + firewall policy; macOS sandbox profile/entitlements; Linux namespaces/seccomp/Landlock as supported. Current native Fortress policy blocks UJANI cloud requests but does not isolate the application process or its loopback Ollama traffic.
- [x] Signed Worker/Partner task-envelope protocol foundation: Ed25519 signatures, short-lived envelopes, exact task consent, allow-listed data scopes, payload digest binding and in-memory replay protection.
- [ ] Secure Worker/Partner peer gateway: authenticated device enrollment, trusted-key lifecycle, mutual TLS or equivalent secure transport, task dispatch/execution, durable replay protection, revocation, quotas, audit receipts and abuse controls.
- [ ] Threat-model review and platform-specific security testing for each capability before production claims.
- [ ] Independent security review, software bill of materials, accessibility and installer testing.

## Later
- [ ] Android and iOS capability adapters, subject to OS restrictions.
- [ ] Plugin/SDK interface with capability and permission boundaries.
- [ ] Optional integrations with UJANI Enterprise AI through a documented, versioned protocol.

## Release gates

No release should be described as secure, private-by-design in implementation, or production-ready solely because it follows this roadmap. Those claims require implementation evidence, tests and review.


## Resource scheduling and provenance — rebased implementation status (2026-11-06)

- [x] Add deterministic, fail-closed local scheduling policy contract for telemetry freshness, CPU/RAM budgets, verified GPU runtime/VRAM, thermal state, concurrency and background-work opt-in/idle/AC-power rules. This is policy-only; native telemetry, atomic reservations and dispatch are not implemented.
- [x] Add typed execution receipt contract with SHA-256 input/output digests, adapter/model identity, execution location, timestamps and external approval reference. Native adapter integration and durable persistence remain pending.
- [x] Add core unit/integration tests for scheduler decisions and receipt validation.
- [x] Add reproducible benchmark protocol documentation. Benchmark harness execution and retained measured results remain pending.
