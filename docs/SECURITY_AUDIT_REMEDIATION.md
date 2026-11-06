# Security audit remediation — Ollama, history and PDF worker

## Scope

This change set addresses three reported release-blocking concerns in the native desktop application. It is a remediation record, not a third-party certification or a claim that all security properties of the application have been independently verified.

## A. Local Ollama SSRF / proxy leakage

The shared Ollama HTTP client is built with both:

- `redirect(reqwest::redirect::Policy::none())`: a local service response cannot redirect a prompt-bearing request to another host.
- `no_proxy()`: proxy environment variables are not consulted for this client.

The same builder is used by the normal local client and the model-pull client. A regression test starts a local HTTP redirector and a second listener; it asserts that the response remains HTTP 302 and the redirect target receives no request.

The Ollama endpoint remains fixed to `http://127.0.0.1:11434`; user-supplied endpoints are not accepted by this adapter.

## B. History ciphertext swapping

Current history encryption uses AES-256-GCM with a fresh nonce and associated authenticated data derived from the row ID and field name (`prompt` or `output`). Normal reads accept only the current `ujani:v2:` format and verify the same row/column AAD used at encryption. Swapping a ciphertext between rows or between prompt/output fields therefore fails authentication.

A one-time migration path handles plaintext rows and legacy `ujani:v1:` ciphertexts. The database records encryption version 2 in `history_settings`; after that marker is committed, legacy v1 values are no longer automatically decrypted or migrated on every read. A v1 value injected after migration is treated as unreadable.

**Important historical limitation:** legacy v1 ciphertext did not bind row ID or column. If a valid v1 ciphertext had already been swapped before the first v2 migration, cryptography cannot determine its original row/field after the fact. The migration can preserve and re-encrypt the value it finds, but cannot prove its historical association. Users with a suspected prior database tampering event should clear old history rather than rely on a migration to repair provenance. New v2 records are row/column bound.

## C. PDF extraction cancellation and isolation

PDF parsing runs in a dedicated child process, not a Tokio blocking-pool thread. The child is launched with `kill_on_drop(true)`, receives bounded PDF bytes over stdin, and returns bounded UTF-8 output over stdout.

The parent:
- enforces a 10 MiB input cap and 1 MiB output cap;
- reads and writes the child pipes concurrently to avoid pipe deadlocks;
- applies a 45-second hard extraction timeout;
- drops the child process on timeout, I/O failure or cancellation.

The native PDF command now registers a task cancellation token. The UI exposes a Cancel PDF processing action. Cancelling the command drops the in-flight PDF pipeline; during extraction this drops the child handle and terminates the worker. A separate 180-second overall PDF pipeline time budget also applies.

This is process isolation and bounded execution, not a full OS sandbox. The worker still runs with the desktop application's user privileges. Stronger OS sandboxing, seccomp/AppContainer/sandbox-exec profiles and platform-specific filesystem/network restrictions remain future hardening work.

## Verification added

- Ollama local HTTP redirect regression test.
- History row/column AAD swap tests.
- Legacy v1 one-time migration boundary test.
- Existing PDF worker timeout and output-bound enforcement retained.
- UI cancellation is wired through the native task registry.

## Release gates still required

Run the complete Rust workspace tests, Tauri native test matrix on Windows/macOS/Linux, React/TypeScript build, and CodeQL on the final PR head. Review test output rather than treating queued/in-progress checks as passed. Perform a packaged-app test for PDF cancellation on each supported OS. Do not describe UJANI as independently audited, formally sandboxed, or absolutely private on the basis of this patch alone.
