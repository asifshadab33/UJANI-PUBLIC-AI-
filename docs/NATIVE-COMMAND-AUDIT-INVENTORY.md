# Native IPC Command Inventory — UJANI Public AI

Status: **inventory baseline; security audit in progress**  
Scope: Tauri command surface registered by `apps/desktop/src-tauri/src/lib.rs` on the `main` branch, plus the peer-preview command extension.

> This document is not a declaration that every command is already authorized correctly. It records the IPC inventory and identifies the review obligations. Do not treat the presence of a UI control, hidden menu item, or caller-supplied flag as an authorization boundary. Sensitive side effects must be gated in native code.

## Inventory source and count

The current `ujani_invoke_handler!` registration contains **42 statically named command entries**, plus a feature-supplied peer-command expansion. The main native source also contains 23 directly annotated command functions; module-level commands are registered by qualified module path. The inventory must be refreshed whenever the handler or command modules change.

## Registered command inventory

| Area | Registered command |
|---|---|
| Capability and planning | `get_capability_snapshot` |
| Capability and planning | `get_tool_catalogue` |
| Capability and planning | `plan_local_operation` |
| Capability and planning | `plan_task_workflow` |
| Capability and planning | `get_model_recommendation` |
| Capability and planning | `get_ollama_status` |
| Capability and planning | `get_device_info` |
| Capability and planning | `list_local_models` |
| Capability and planning | `evaluate_task_route` |
| Local runtime / model | `pull_ollama_model` |
| Local runtime / model | `execute_local_ai_task` |
| Local runtime / model | `run_local_model_task` |
| Local runtime / model | `run_local_task` |
| Local runtime / model | `cancel_active_task` |
| Documents and text | `execute_document_workflow` |
| Documents and text | `process_selected_document` |
| Documents and text | `process_selected_pdf` |
| Documents and text | `analyze_text_file` |
| Documents and text | `summarize_selected_text` |
| Documents and text | `translate_text_local` |
| Documents and images | `inspect_image_file` |
| Cloud | `execute_cloud_ai_task` |
| Filesystem | `folder_import::process_selected_folder` |
| Filesystem | `local_search::find_local_files` |
| Filesystem | `everyday_tools::find_duplicate_files` |
| Filesystem | `file_insights::find_large_files` |
| OS information | `everyday_tools::get_local_clock` |
| Search cache | `search_cache::get_local_search_cache_settings` |
| Search cache | `search_cache::list_recent_local_searches` |
| Search cache | `search_cache::set_local_search_cache_settings` |
| Search cache | `search_cache::clear_local_search_cache` |
| Task history | `history::get_task_history_settings` |
| Task history | `history::set_task_history_enabled` |
| Task history | `history::save_task_history` |
| Task history | `history::list_task_history` |
| Task history | `history::delete_task_history` |
| Task history | `history::clear_task_history` |
| Receipts and cloud budget | `security_store::verify_execution_receipt_ledger` |
| Receipts and cloud budget | `security_store::list_execution_receipts` |
| Receipts and cloud budget | `security_store::get_cloud_budget_status` |
| Receipts and cloud budget | `security_store::set_cloud_budget_limit` |
| Security Centre | `get_security_provider_status` |

### Read-only local model discovery boundary

`get_ollama_status` reads OS CPU/RAM information and contacts only `127.0.0.1:11434`. It uses bounded `/api/tags` (maximum 512 installed models; existing 2 MiB response ceiling), `/api/show` (maximum 32 models, four concurrent requests, three seconds per request and an eight-second aggregate deadline), `/api/version` (two seconds) and `/api/ps` (three seconds). Redirects and environment proxies are disabled by the existing Ollama client. The command does not send prompts, files, paths or credentials; it does not start/stop a runtime or expose a listening endpoint. The separate `get_model_recommendation` command requires runtime-reported chat capability and context capacity at least equal to the configured Ollama `num_ctx`, plus an advisory RAM-fit estimate; it does not infer reasoning, coding or language quality. Current main uses `NUM_CTX=4096`; the hardware-aware model guidance and bounded context change are already present. The `model_runtime.rs` adapter remains a deterministic test mock, not real inference. GPU evidence means only Ollama-reported active model VRAM allocation; total VRAM, NPU and storage remain unknown.

### Conditional peer-preview surface

The handler macro accepts seven additional commands only when the opt-in `peer-preview` Cargo feature is enabled. They are not registered in normal builds:

| Area | Conditional command (peer-preview only) |
|---|---|
| Peer trust | `security_store::enroll_trusted_peer` |
| Peer trust | `security_store::revoke_trusted_peer` |
| Peer trust | `security_store::list_trusted_peers` |
| Peer envelope | `security_store::verify_and_claim_peer_envelope` |
| Device identity | `device_identity::get_or_create_device_identity` |
| Device identity | `device_identity::rotate_device_identity` |
| Device identity | `device_identity::sign_peer_task_envelope` |

These commands must remain unavailable in normal builds unless the opt-in feature is deliberately enabled. Their authorization, replay, persistence, revocation and receipt requirements remain part of the audit.

## High-priority findings to close

1. **Security provider status:** PR #121 merged on 2026-11-09 and adds fresh native, scope-specific confirmation with fail-closed handling. Its recorded CI/CodeQL/installer checks passed on head `eeabbb9ec18b4d656791a97dd5976e6327ec9df9`; Windows Security Center device smoke validation is still outstanding. This closes that targeted bypass, not the full command audit in #120.
2. **Model download:** Current main's `pull_ollama_model` now fails closed before any download or consent prompt because `model_download_preflight_ready()` is deliberately false. UJANI cannot independently verify the exact upstream artifact digest, licence, download size, or the actual model-store path and free space used by the Ollama daemon. The UI permits a candidate name for manual setup only; users may install through their trusted Ollama interface and then refresh UJANI's local inventory. Do not re-enable UJANI-managed downloads until artifact provenance and daemon-store disk preflight are verifiable. The Ollama adapter still bounds requests, progress, cancellation and response handling; these controls do not prove that the daemon immediately aborts downstream registry traffic. Supported-platform allow/deny/timeout/cancel-dialog and daemon-stop behavior remain device-validation items.
3. **Cloud execution — partial policy wiring:** the BYOK cloud command now requires an explicit user-declared data class (`public`, `internal`, `confidential`, or `restricted`) and calls the shared `policy_gate::evaluate_external_transfer` before native confirmation or network dispatch. Unknown/unclassified values fail closed; confidential/restricted values are denied by the generic gateway even if the user checks approval; public/internal still require task-scoped approval and the native confirmation dialog. The selected class is included in the UI approval snapshot, native confirmation text and approval-ID digest. This is a user declaration, not automatic content classification or DLP; the native dialog exposes the declared class and prompt preview so the user can correct or reject it. Other native command routes still need their own complete mappings and negative tests.
4. **Filesystem operations:** verify user-selected scope, path canonicalisation, symlink/reparse-point handling, bounded traversal, cancellation, output limits and no OS-wide scan.

### Current filesystem race boundary — Windows release blocker

PR #231 merged the selected-root-safe reopen path and a regression for replacing the selected root with a symlink after enumeration. `folder_import.rs` now calls `safe_fs::open_regular_file_under_root`. On Linux and macOS (`cfg(unix)`), this helper starts from `/` and opens every ancestor with handle-relative `openat`, `O_DIRECTORY|O_NOFOLLOW`, and the final file with `O_NOFOLLOW`; tests reject symlinked ancestors and selected-root replacement. This closes the documented path-component replacement route for those Unix builds, subject to platform device verification.

PR #255 adds a Windows-specific handle-relative implementation in `safe_fs.rs`: it opens the local drive root and each selected-root ancestor as a handle, rejects reparse points, and resolves each child through its pinned parent handle with `NtCreateFile` and `FILE_OPEN_REPARSE_POINT`. UNC paths, mapped network drives and unsupported device namespaces fail closed rather than falling back to path reopening. Windows regression tests cover a selected root replaced by a junction and an ancestor junction installed after the root handle is pinned. This change is not yet release evidence: exact-head Windows/Linux/macOS CI and physical Windows junction/reparse-point tests must pass before closing the P0 gate. The change does not claim to solve every filesystem race or other Windows filesystem API.
### PDF worker isolation — configured, not device-verified

The sidecar launcher configures OS controls (Unix resource limits, Linux `no_new_privs`/seccomp, macOS `sandbox-exec`, Windows Job Object memory/CPU/kill-on-close). On Windows, Job Object assignment occurs after the launcher starts, although the parent withholds untrusted PDF bytes until assignment succeeds; this is a reduced race window, not atomic process creation. These controls constrain the PDF worker, not the full application. The capability snapshot reports `pdf.worker.resource_limits` as **Unknown** until a non-destructive enforcement probe verifies the real process limits. More importantly, no platform currently has a deny-by-default filesystem-read boundary: the worker retains the current user's file-read permissions, so a parser compromise may read accessible files. The snapshot reports `pdf.worker.filesystem_isolation` as **Unavailable**. Linux seccomp and macOS `sandbox-exec` configure network denial but remain **Unknown** pending device verification; Windows Job Objects do not block network, so `pdf.worker.network_isolation` is **Unavailable** there. Do not treat source configuration or a green compile as proof of runtime enforcement or full sandboxing.
5. **Persistent state and receipts:** The per-command policy/approval and denial-before-side-effect audit remains open. The cloud-budget ceiling still requires fresh native confirmation for increases and rechecks the approved previous limit and exact requested new limit inside the transaction; decreases do not require elevation. Public PR #231 merged at `08f93d63b11f15b6084363f142ec8f60410527aa`; its exact head `3732706b34297720951ff21f14eb9ecca3ffc310` passed CI (`43188378713`), CodeQL (`43188378691`), Android compatibility (`43188378504`), installer-size audit (`43188378814`), unsigned preview installer (`43188378749`) and security-sensitive change watch (`43210074685`). It adds schema-v3 metadata-only outcome receipts for managed-task success/failure/cancellation/timeout, cloud attempts, model downloads, history/cache mutations, cloud-budget changes and peer enrollment/revocation/replay claims; it reserves receipt capacity before wrapped mutations. Device-identity create/rotation is serialized through SQLite `BEGIN IMMEDIATE` and rechecks the approved fingerprint under lock. `scripts/check_sensitive_command_receipts.py` is a static CI regression guard, not proof of runtime authorization or every denial-before-side-effect path. SQLite does not make OS credential-vault writes and receipt persistence one atomic transaction, and no hardware-backed identity attestation is claimed. History uses AES-256-GCM v3 with AAD bound to row ID, column, mode, model, kind and creation time; tests cover ciphertext/metadata tampering and legacy v1/v2 migration. Search cache uses AES-GCM AAD scoped to selected-folder ID, request key and field kind; paths, queries, inventory and payload are encrypted, caching is disabled by default, retention defaults to 30 days, and bounds are 50 entries/10 MiB. Cache hits are hints and callers must re-open files. History/cache directories and SQLite files receive restrictive 0700/0600 permissions on Unix; Windows ACL behavior and any history-export path still require verification. Receipts must not contain prompts, document contents, credentials or other secrets.
6. **Peer preview:** verify feature gating at compile time and runtime; no default-build registration, implicit peer trust, or remote execution.
7. **Release guard — partial:** `scripts/check_native_command_inventory.py` verifies parity between the handler and this inventory (42 default commands plus 7 peer-preview commands). PR #231 adds `scripts/check_sensitive_command_receipts.py`, which statically checks that the currently identified persistent mutation paths retain receipt-wiring calls. The latter does not prove runtime authorization, receipt persistence success, denial-before-side-effect behavior or automatic classification of every newly added command. Keep issue #120 open until machine-readable capability/policy mappings and per-command negative tests are enforced.

## Required per-command audit fields

For every registered command, complete a row with:

- caller/UI entry point and whether any non-UI IPC caller can invoke it;
- capability identifier and native policy gate;
- data read, data written, data classification and exact user-selected scope;
- filesystem, network, process, OS API and persistence side effects;
- consent surface, consent lifetime, recipient disclosure and revocation semantics;
- cancellation points, timeouts, concurrency limits, input/output bounds and resource budgets;
- failure behaviour (must fail closed before side effects);
- execution receipt/audit event, stable operation ID and secret/content minimisation;
- unit and integration tests, including denial-before-side-effect tests.

## Release rule

This inventory is a starting point, **not completion of issue #120**. Do not close the native-command audit or declare Public AI release-ready until all registered and conditional commands have complete mappings, sensitive commands enforce shared native authorization, required receipts are covered, negative tests pass, and CI rejects unreviewed command additions.
