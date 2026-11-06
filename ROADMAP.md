# Roadmap

This roadmap describes intended work, not completed features. Items are not production-ready until implemented, tested and documented.

## Phase 0 — foundations (current)
- [x] Define local-first product principles and explicit non-goals.
- [x] Add an initial typed Rust task/capability model and routing tests.
- [x] Document privacy, security and contribution expectations.
- [ ] Establish CI for formatting, linting, tests and dependency review.
- [ ] Agree a versioned task protocol shared with UJANI Enterprise AI.

## Phase 1 — desktop proof of concept
- [ ] Windows desktop shell (Tauri 2 + React/TypeScript).
- [x] Read-only initial inventory: OS, architecture, logical CPUs, total/available RAM and OS locale.
- [ ] Verified GPU/NPU and model-runtime compatibility inventory across supported operating systems (presence alone is insufficient).
- [x] Initial read-only capability registry with source, state and explanatory detail; GPU/NPU remain unknown until verified.
- [x] Allow-listed local task-plan preview with capability and coarse available-memory preflight; planning does not execute or reserve resources.
- [ ] Capability freshness/expiry, user permission metadata and runtime-specific accelerator compatibility.
- [x] Opt-in local SQLite task history with disabled-by-default recording, 30-day retention, 100-entry cap, per-entry deletion and clear-all controls; prompt/output encrypted with AES-256-GCM and key stored in OS credential vault (metadata remains plaintext).
- [x] First integrated local text-document pipeline: bounded analysis followed by Ollama summarisation.
- [x] Sequential, UTF-8-safe multi-section summarisation for documents up to 256 KiB, with bounded intermediate summaries and final synthesis; larger inputs require user splitting.
- [x] Optional second local-model step to translate document/PDF summaries into a user-selected language.
- [x] Registered PDF text extraction for user-selected PDFs (10 MiB input limit; scanned-PDF OCR not included).
- [ ] Image conversion and broader media metadata tools.
- [ ] OS-enforced Fortress isolation: Windows AppContainer/restricted token and firewall policy; macOS sandbox/entitlements; Linux namespaces/seccomp/Landlock where supported. Fail closed if isolation cannot be enforced.

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
- [ ] Per-subtask cloud assistance interface.
- [ ] Data minimisation and explicit approval policies.
- [ ] Local-only mode tested with network disabled.
- [ ] Clear provenance for local, remote and deterministic-tool work.

## Phase 4 — hardening and community\n- [x] Tauri updater plugin, startup update check, signed update verification and user-controlled install/restart UI.\n- [x] Tagged multi-platform installer workflow for Windows, macOS Intel/Apple Silicon and Linux, with merged updater manifest.\n- [ ] Configure owner-held updater key pair in GitHub Actions secrets and publish/verify the first signed release.
- [x] Configure signed Tauri updater artifacts and a GitHub Releases update endpoint (requires owner-generated signing keys in GitHub Actions secrets before a signed release can be built).
- [x] Add tagged-release installer pipeline for Windows, macOS and Linux, including signed update manifest generation.
- [ ] Generate and pin the production updater key pair; publish the first signed release and verify update/install/rollback behavior on clean machines.
- [ ] Verified GPU/NPU detection and model-runtime compatibility across Windows, macOS and Linux; report driver/runtime versions and confidence, never infer compatibility from device presence alone.
- [ ] OS-enforced Fortress isolation: Windows AppContainer/restricted token + firewall policy; macOS sandbox profile/entitlements; Linux namespaces/seccomp/landlock as supported. Deny-by-default, test escape boundaries and fail closed when enforcement is unavailable.
- [ ] Secure Worker/Partner peer gateway: authenticated device enrollment, mutual TLS or signed identities, short-lived signed task envelopes, explicit per-task data scopes and consent, replay protection, encrypted transport, resource quotas, audit receipts, revocation and abuse controls.
- [ ] Threat-model review and platform-specific security testing for each of the three capabilities before claiming production support.
- [ ] Threat model and independent security review.
- [ ] Signed releases and software bill of materials.
- [ ] Accessibility, internationalisation and installer testing.
- [ ] Public benchmark methodology and issue triage process.

## Later
- [ ] Android and iOS capability adapters, subject to OS restrictions.
- [ ] Plugin/SDK interface with capability and permission boundaries.
- [ ] Optional integrations with UJANI Enterprise AI through a documented, versioned protocol.

## Release gates

No release should be described as secure, private-by-design in implementation, or production-ready solely because it follows this roadmap. Those claims require implementation evidence, tests and review.
