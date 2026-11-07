# UJANI Public AI — Privacy Receipt v2 and Managed Task Lifecycle

## Purpose and limits

Execution receipts are metadata-only provenance records. They bind a task identifier, operation, adapter/model identifiers, timestamps and SHA-256 digests of the exact input/output representations. Prompt, document, PDF, folder-file and model output contents are not written to the receipt ledger.

The local ledger is a SQLite hash chain. It is tamper-evident against accidental or partial row changes, but it is **not signed, externally anchored or independent attestation**. A local administrator who can replace the database can replace the whole chain.

## Receipt schema

- Schema v1 remains readable. When a v1 record has no network field, deserialization supplies explicit `unknown` observations; it does not invent zero-byte values.
- New receipts use schema v2 and include `network` with four independently classified observations:
  - `destinations`
  - `resolved_addresses`
  - `application_request_body_bytes`
  - `application_response_body_bytes`
- Every observation is one of:
  - `measured`: captured directly by the UJANI application at the stated boundary.
  - `inferred`: derived from configuration or resolver results; it is not proof of the actual connected peer.
  - `unknown`: not observed, with a reason. Unknown is never encoded as zero.
  - A zero byte count is accepted only as a `measured` observation; inferred byte counts must be positive.
- The BYOK cloud gateway measures the serialized HTTP request body length and the response body bytes read by the application. It records the configured destination host and the resolver addresses as inferred. These are application-payload measurements, **not total network traffic**: HTTP headers, TLS, transport framing, DNS packets, retries below the HTTP client and other processes/system traffic are excluded.
- Local Ollama tasks currently report network observations as unknown because UJANI does not run a system-wide socket monitor. This avoids falsely claiming that local inference generated zero network traffic.
- Model downloads are cancellable, bounded tasks but are not inference receipts. Their full registry/network transfer is not instrumented at this time.

## Managed task lifecycle

The shared managed-task path provides:
1. Validated task ID and duplicate-active rejection.
2. Memory admission reservation for inference/document work.
3. Durable receipt-slot reservation before execution.
4. A process-local cancellation token and task-progress events.
5. A hard wall-clock deadline, with cancellation/timeout/failure represented as distinct outcomes.
6. RAII cleanup of active task registration, memory reservation and unused receipt slots when a command returns, errors, is cancelled or unwinds.
7. Receipt persistence only after a successful, serializable result. A receipt-write failure is surfaced rather than silently reporting provenance success.

The fixed document workflow, selected text/document/PDF processing, translation and selected-folder import use the managed path. Folder import remains native-picker-only, direct-child-only, and bounded to 10 files, 256 KiB per file and 1 MiB total; its preflight scan also stops after 5,000 directory entries or 15 seconds. Model downloads require explicit confirmation, use cancellation cleanup and a 30-minute deadline, but are not written as inference receipts.

The BYOK path has its own approval, token reservation, cancellation, 95-second deadline and active-task RAII guard. Its conservative token reservation is intentionally not refunded on failure/cancellation.

The local file finder registers before opening its native picker, checks cancellation during metadata traversal and content inspection, and is bounded to 5,000 entries, 30 seconds for metadata traversal and 180 seconds for content inspection. The duplicate-file preview is also cancellable and bounded to 5,000 entries and 60 seconds. Dropping a Tauri command future cancels its shared token as well as removing the active task ID, allowing any cooperative `spawn_blocking` scan to stop.

## Verification

The core contract tests cover:
- legacy v1 receipt deserialization with unknown network evidence;
- v2 measured, inferred and unknown observations;
- rejection of malformed destinations and empty unknown reasons;
- existing digest, time-range, model-identifier and external-approval checks.

The desktop tests cover managed future success/failure, timeout and cancellation, active-task guard cleanup on scope exit/unwind, deterministic input-digest binding, and receipt-slot reservation release/expiry. Cross-platform CI remains the merge gate.
