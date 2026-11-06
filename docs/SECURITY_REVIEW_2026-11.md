# Security review: 2026-11 audit follow-up

This document records the code-level responses to the 2026-11-06 external audit.
It is not a penetration-test report or a production-security certification.

## Implemented on the audit branch

- The Tauri command surface is explicitly enumerated in `generate_handler!`. The backend does not register a generic shell, arbitrary script execution, or arbitrary filesystem-path command.
- Removed the unused frontend `dialog:default` capability. The application retains native Rust-side dialogs for its own confirmation and folder-selection flows; this change does not grant the frontend a general dialog-plugin IPC permission.
- Added length and character validation for task-route identifiers and capability lists, workflow node/dependency identifiers and cancellation task IDs.
- Local history encryption key bytes, the decoded key buffer and the encoded key string are now held in zeroizing wrappers. This reduces residual key material in application memory; it cannot guarantee removal of compiler/runtime copies or OS credential-vault copies.
- Added optional cargo-fuzz targets for the image-header inspector and PDF parser. They are harnesses only; no fuzz campaign is claimed as completed.
- Added a cloud request compatibility selector: automatic mode uses `max_completion_tokens` and a larger 8,192-token budget for recognised GPT-5/o1/o3/o4 model prefixes on any configured HTTPS endpoint; standard model families retain `max_tokens` and a 2,048-token budget. A user-visible explicit override supports custom provider/Azure deployment aliases. The selected field is included in the per-task approval snapshot. Provider support still varies, so live integration tests are required.

## Tauri IPC review

The current native command handler exposes only application-specific commands for policy preview, capability/model metadata, bounded local analysis, Ollama tasks, selected-file processing, cancellation, and opt-in history. There is no shell plugin dependency and no command that accepts an arbitrary filesystem path to read or write.

Input checks are layered: command-level limits apply to route/workflow/cancellation identifiers; core parsers and task-engine functions enforce file, PDF, image, text, model and workflow bounds; the cloud gateway validates endpoint, DNS resolution, consent, key, prompt and response sizes.

The Tauri v2 capability file still includes `core:default`, `updater:default` and `process:allow-restart`. The updater and restart permissions should be reviewed against the final updater UX before release. Custom application commands are explicitly exposed through `generate_handler!`; do not treat plugin capability declarations as a replacement for authorization and validation inside those Rust commands.

## Not implemented / still release-blocking

- OS-enforced Fortress sandboxing and process/network isolation on Linux, macOS and Windows.
- OS resource limits for PDF child processes (CPU, memory, file descriptors); child-process separation and time/output limits are not a complete sandbox.
- TPM/Secure Enclave device attestation, enrollment, rotation and revocation.
- Persistent peer replay protection, peer key enrollment and authenticated transport.
- A real distributed scheduler, durable execution receipts, and atomic cross-process resource reservations.
- A completed fuzz campaign, third-party penetration test, SBOM review and signed reproducible-build process.
- Cross-provider live tests for cloud token parameter compatibility, Azure/custom deployment aliases, billing/usage reporting and granular consent UX review.

## Release posture

Fortress currently means that UJANI blocks its own external/cloud routes at the application policy boundary. It must not be marketed as OS-enforced isolation until a supported sandbox adapter is installed, verified at runtime, and tested against escape and network-egress cases.


## Cloud provider token parameter follow-up

The native gateway accepts only three preference values: `auto`, `max_tokens` and
`max_completion_tokens`. Automatic selection is based on a recognised model
family prefix rather than the provider hostname, so compatible reasoning-model
aliases on Azure or other custom endpoints are not forced onto the standard
field. The UI exposes the explicit override for deployment aliases that do not
include a recognisable model family. Unknown preference values fail closed.
Recognised reasoning families receive an 8,192-token budget; standard models
receive 2,048 tokens. These are application request limits, not guarantees of
provider output length or support. The preference is bound to the exact
per-request consent snapshot. Integration tests against real provider contracts
remain a release gate.
