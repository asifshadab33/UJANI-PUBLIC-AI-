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

1. **Security provider status — code merged, device verification pending:** PR #121 merged on 2026-11-09. `get_security_provider_status` now validates the provider IDs and obtains a fresh, scope-specific native confirmation before calling the read-only OS adapter; denial, cancellation, timeout and channel closure fail closed. The Windows Security Center adapter still needs a real Windows device smoke test. This closes that command's native-consent gap, not the full command audit.
2. **Model download — code merged, receipt coverage pending:** PR #121 merged on 2026-11-09. `pull_ollama_model` validates Smart-only mode, the local Ollama model identifier and task ID; registers cancellation before the native prompt; and fails closed on timeout, cancellation or denial. Model downloads are still not covered by the metadata-only execution receipt ledger, so receipt coverage remains an audit item.
3. **Cloud execution:** verify that every request has task-scoped approval, recipient/data disclosure, policy evaluation, bounded payloads, cancellation and a receipt before any external request. A UI-only approval is insufficient.
4. **Filesystem operations:** verify user-selected scope, path canonicalisation, symlink/reparse-point handling, bounded traversal, cancellation, output limits and no OS-wide scan.
5. **Persistent state:** verify policy gates and receipts for history/cache settings and mutations, document workflows, folder imports, cloud-budget changes and peer-trust changes. Receipts must not contain prompts, document contents, credentials or other secrets.
6. **Peer preview:** verify feature gating at compile time and runtime; no default-build registration, implicit peer trust, or remote execution.
7. **Release guard — partial:** `scripts/check_native_command_inventory.py` is wired into `.github/workflows/ci.yml` and currently verifies parity between the handler and this inventory (42 default commands plus 7 peer-preview commands). This catches an unlisted command but does not require a capability/policy mapping, receipt mapping or denial-before-side-effect test. Extend the guard to require those fields for sensitive commands; keep issue #120 open.

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
