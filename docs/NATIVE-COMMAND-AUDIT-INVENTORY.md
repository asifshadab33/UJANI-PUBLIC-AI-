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

`get_ollama_status` reads OS CPU/RAM information and contacts only `127.0.0.1:11434`. It uses bounded `/api/tags` (maximum 512 installed models; existing 2 MiB response ceiling), `/api/show` (maximum 32 models, four concurrent requests, three seconds per request and an eight-second aggregate deadline), `/api/version` (two seconds) and `/api/ps` (three seconds). Redirects and environment proxies are disabled by the existing Ollama client. The command does not send prompts, files, paths or credentials; it does not start/stop a runtime, download a model, change selection or expose a listening endpoint. GPU evidence means only Ollama-reported active model VRAM allocation; total VRAM, NPU and storage remain unknown. The separate `get_model_recommendation` command also uses runtime-reported chat capability and an advisory RAM-fit estimate; it does not infer reasoning, coding or language quality.

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
2. **Model download:** PR #121 also moved task registration before native consent and added a bounded consent timeout/cancellation path. Device allow/deny/timeout/cancel-during-dialog smoke tests remain outstanding; HTTP cancellation must still be verified against the actual Ollama request on supported devices.
3. **Cloud execution:** verify that every request has task-scoped approval, recipient/data disclosure, policy evaluation, bounded payloads, cancellation and a receipt before any external request. A UI-only approval is insufficient.
4. **Filesystem operations:** verify user-selected scope, path canonicalisation, symlink/reparse-point handling, bounded traversal, cancellation, output limits and no OS-wide scan.

### Current filesystem race boundary — release blocker

The shared walker merged in PR #160 improves final-component handling: Unix opens regular files with `O_NOFOLLOW`; Windows opens the final reparse point for handle-metadata rejection; consumers then read from the validated file handle. This does **not** yet prove parent-directory-chain safety. The current traversal still uses canonicalised path strings with `read_dir`, and a later file open is path-based. An attacker able to mutate a selected tree concurrently may replace an ancestor directory between enumeration/canonicalisation and open; final-component `O_NOFOLLOW` alone does not pin every parent component. Keep this P0 open until traversal is handle-relative (or fails closed where it cannot be made safe) on Linux, macOS and Windows. Required adversarial tests must swap ancestor symlinks/reparse points between enumeration and open, verify the opened handle remains beneath the selected root, and prove no outside file is read. Do not treat PR #160's final-component checks as closing the full traversal/TOCTOU requirement.

### PDF worker isolation — configured, not device-verified

The sidecar launcher configures OS controls (Unix resource limits, Linux `no_new_privs`/seccomp, macOS `sandbox-exec`, Windows Job Object memory/CPU/kill-on-close). On Windows, Job Object assignment occurs after the launcher starts, although the parent withholds untrusted PDF bytes until assignment succeeds; this is a reduced race window, not atomic process creation. These controls constrain the PDF worker, not the full application. The capability snapshot reports `pdf.worker.resource_limits` as **Unknown** until a non-destructive enforcement probe verifies the real process limits. More importantly, no platform currently has a deny-by-default filesystem-read boundary: the worker retains the current user's file-read permissions, so a parser compromise may read accessible files. The snapshot reports `pdf.worker.filesystem_isolation` as **Unavailable**. Linux seccomp and macOS `sandbox-exec` configure network denial but remain **Unknown** pending device verification; Windows Job Objects do not block network, so `pdf.worker.network_isolation` is **Unavailable** there. Do not treat source configuration or a green compile as proof of runtime enforcement or full sandboxing.
5. **Persistent state:** verify policy gates and receipts for history/cache settings and mutations, document workflows, folder imports, cloud-budget changes and peer-trust changes. Receipts must not contain prompts, document contents, credentials or other secrets.
6. **Peer preview:** verify feature gating at compile time and runtime; no default-build registration, implicit peer trust, or remote execution.
7. **Release guard — partial:** `scripts/check_native_command_inventory.py` is wired into `.github/workflows/ci.yml` and verifies parity between the handler and this inventory (42 default commands plus 7 peer-preview commands). This catches an unlisted command but does not require a capability/policy mapping, receipt mapping or denial-before-side-effect test. Extend the guard to require those fields for sensitive commands; keep issue #120 open.

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
