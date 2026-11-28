# Public AI native IPC authorization audit

Status: implementation checklist; not a security sign-off. This document deliberately separates the global Tauri invoke filter from command-specific authorization.

## Current boundary observed on main

The current `guard_ipc_invoke` performs three first-layer checks: command has a registered policy class, caller is the main webview at an approved packaged/development origin, and peer-preview commands are limited to explicitly enabled debug builds. This is useful defense in depth, but the policy enum is not itself a capability grant. A command classified as `LocalMutation` or `NativeConsent` is not thereby authorized to access a path, mutate persistent state, start a model download, or transfer data.

The command inventory test checks a hand-maintained list against `ipc_command_policy`; it does not independently prove that every Tauri registration is present in that list or that every handler enforces its own scope and consent.

## Required command-level review

For each registered command, reviewers must record:

| Field | Required evidence |
|---|---|
| Caller and entry point | Tauri command registration and all alternate/native invocation paths |
| Data scope | Exact selected file/folder, local database row set, model ID, or provider scope |
| Authorization | Shared capability/policy API invoked by the handler, not UI visibility or caller booleans |
| Consent | Native confirmation or persisted, revocable, task-scoped grant where the operation requires it |
| Side effects | Filesystem, SQLite, keyring, process, model pull, network, OS integration |
| Ordering | Validation, authorization and consent must precede receipt reservation where reservation itself is a durable write; all other durable/external effects must follow required receipt reservation |
| Cancellation | Cancellation checked before work and propagated into blocking/async operations; bounded cleanup |
| Limits | Input/output bytes, time, concurrency, memory, recursion and result counts |
| Failure mode | Missing, stale, corrupt or unavailable policy state fails closed |
| Receipt | Stable operation ID, intent/scope, outcome and timestamps; no prompts, file contents, credentials, secrets or unnecessary paths |

## First high-risk handler batch

Review these handler groups independently and add negative tests before changing implementation:

1. Filesystem: `process_selected_folder`, `find_local_files`, `find_duplicate_files`, `find_large_files`, `find_recent_files`. Prove selected-root binding, canonical identity, symlink/junction/reparse handling, race-safe open, cancellation and no traversal before authorization.
2. Document/PDF: `execute_document_workflow`, `process_selected_document`, `process_selected_pdf`, `analyze_text_file`, `summarize_selected_text`, `translate_text_local`. Prove model/input validation, capability checks, worker resource limits and cancellation before decode/inference.
3. Model runtime: `pull_ollama_model`, `execute_local_ai_task`, `run_local_model_task`. Prove explicit user intent for downloads, model ID/host validation, redirect/proxy restrictions, bounded time/body, task cancellation and truthful mock-vs-real inference status.
4. External transfer: `execute_cloud_ai_task`. Prove data classification, exact recipient disclosure, task-scoped approval and no request construction before approval.
5. Persistent state: history, cache, reminders, cloud budget, trusted-peer/device identity. Prove receipt ordering, atomic transaction behavior, key-vault failure handling, explicit user confirmation for sensitive mutations and metadata minimization.

## Test requirements

- Add table-driven tests for every registered command and fail when a new registration is not represented.
- For every sensitive handler, test missing consent, stale/revoked capability, malformed or out-of-scope path, policy-store error, receipt-capacity failure and cancellation.
- Assert denial occurs before the observable side effect (filesystem write/open, SQLite mutation, keyring mutation, model pull or HTTP request), not merely that an error is returned.
- Exercise both normal UI invocation and any alternate/native entry point.
- Keep source-order guards as regression tripwires only; pair them with behavioral tests. A source marker is not proof of runtime authorization.
- Record exact commit, platform, test command, result and evidence. Do not label a test as device-verified when it ran only in hosted CI.

## Merge and release gates

- Security-sensitive changes require independent review and exact-head CI.
- Do not merge based only on a green source-order checker.
- Keep peer-preview commands unavailable in production builds until a separate threat review authorizes them.
- Keep cloud transfer disabled unless the user's explicit task-scoped approval is valid.
- Do not claim issue #120 complete until the command matrix, denial-before-side-effect tests, receipt coverage and alternate invocation review are all complete.

## Known coordination constraints

- PRs #484 and #486 both touch `scripts/check_sensitive_command_receipts.py` and the native command security map. Rebase/merge them serially and rerun the combined exact-head checks; do not stack stale copies of these shared files.
- PR #487 changes `history.rs` and the same receipt guard/map. Keep it isolated until its history regression and independent review are complete.
- This audit checklist is additive and does not modify runtime authorization behavior.
