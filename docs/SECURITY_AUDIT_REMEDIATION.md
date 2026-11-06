# Security audit remediation — Ollama, history and PDF worker

## Scope

This change set addresses three reported release-blocking concerns in the native desktop application. It is a remediation record, not a third-party certification or a claim that all security properties of the application have been independently verified.

## A. Local Ollama SSRF / proxy leakage

The shared Ollama HTTP client is built with both:

- `redirect(reqwest::redirect::Policy::none())`: a local service response cannot redirect a prompt-bearing request to another host.
- `no_proxy()`: proxy environment variables are not consulted for this client.

The same builder is used by the normal local client and the model-pull client. Regression tests start a local HTTP redirector and a second listener, and set proxy environment variables against a local target; they assert that the redirect is not followed and the configured proxy receives no local request.

The Ollama endpoint remains fixed to `http://127.0.0.1:11434`; user-supplied endpoints are not accepted by this adapter.

## B. History ciphertext swapping

Current history encryption uses AES-256-GCM with a fresh nonce and associated authenticated data derived from the row ID and field name (`prompt` or `output`). The 256-bit history key is generated from the operating system cryptographic random source and stored only in the OS credential vault; tests generate independent random keys rather than using fixed key material. Normal reads accept only the current `ujani:v2:` format and verify the same row/column AAD used at encryption. Swapping a ciphertext between rows or between prompt/output fields therefore fails authentication.

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

- Ollama local HTTP redirect and Unix proxy-environment regression tests.
- History row/column AAD swap tests.
- Legacy v1 one-time migration boundary test.
- Existing PDF worker timeout and output-bound enforcement retained.
- UI cancellation is wired through the native task registry.

## Release gates still required

Run the complete Rust workspace tests, Tauri native test matrix on Windows/macOS/Linux, React/TypeScript build, and CodeQL on the final PR head. Review test output rather than treating queued/in-progress checks as passed. Perform a packaged-app test for PDF cancellation on each supported OS. Do not describe UJANI as independently audited, formally sandboxed, or absolutely private on the basis of this patch alone.

## D. CI supply-chain and workflow permissions

The repository's CI, CodeQL, preview-installer and signed-release workflows now pin third-party GitHub Actions to full commit SHAs resolved from their upstream repositories during this remediation. Workflow token permissions are read-only by default; release write permissions are scoped to the jobs that upload and publish release assets. This reduces tag-movement risk and limits the blast radius of a compromised workflow step.

**Dependency reproducibility remains open:** this repository currently has no committed npm package-lock.json and no committed desktop Cargo.lock. CI and release workflows still resolve semver-compatible dependencies at install time. Generate and review lockfiles using the supported npm and Cargo toolchains, commit them, and switch frontend CI/release installs to npm ci before treating dependency resolution as reproducible. Do not hand-author lockfiles.

## E. Tauri updater and desktop permission release gates

The checked-in Tauri updater public key is intentionally empty. The signed release workflow requires the TAURI_UPDATER_PUBLIC_KEY and TAURI_SIGNING_PRIVATE_KEY secrets and injects the public key into the build configuration only for the release build. Until the maintainer configures and verifies those secrets and completes a signed update/install test, updater distribution must not be represented as production-ready.

The current default Tauri capability grants the default core, dialog and updater plugin permission sets. Before public distribution, replace broad plugin defaults with the minimum specific commands required by the UI, verify the generated Tauri permission schema against the exact installed plugin versions, and test that unneeded dialog/updater commands are denied. Keep the current restrictive content security policy unless a documented feature requires a narrowly scoped addition.

## F. Product-security boundaries not yet implemented

- The resource scheduler is a deterministic policy contract; it is not yet the authoritative scheduler for every task adapter and does not reserve or release live hardware resources.
- The execution receipt type is a provenance contract; durable receipt storage and independent output verification are not implemented.
- The built-in model adapter is a clearly labelled deterministic mock, not an AI model. Actual local inference in the desktop app is performed through the explicitly selected Ollama service.
- PDF extraction uses a child process but is not an OS sandbox and has no platform-specific memory/CPU quotas yet.
- Folder import and every native long-running task still need a common cancellable task runner and per-operation budgets.
- Worker/Partner peer networking, enrollment, trust/reputation and remote execution are not implemented.

These are product and release gates, not claims that the current patch closes them.