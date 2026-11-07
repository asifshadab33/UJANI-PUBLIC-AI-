# UJANI consolidated implementation backlog — 7 November 2026

This is an implementation and release-planning record for the Public AI and Enterprise AI repositories, reconciled against the current repository roadmaps, implementation-status documents, open issues, pull requests and CI evidence on 7 November 2026.

**Product status remains pre-alpha / prototype.** This document is not a security certification. A merged change or green hosted CI does not prove real-device behavior or production security.

## Operating direction

- Continue implementation without waiting for external tester feedback. No further tester report is expected or required for the current roadmap.
- Prioritise reliability, privacy boundaries, cancellation, bounded resource use and honest capability descriptions over feature count.
- Keep Public AI and Enterprise AI independently governed until a separately versioned, tested interoperability protocol exists.
- Keep Worker/Partner and remote execution disabled. Do not enable distributed execution based only on protocol primitives or a passing unit test.
- Do not weaken CI, suppress dependency advisories, or merge security-sensitive work while required checks are failing or unexplained.
- Treat user-selected scope, explicit approval and read-only previews as defaults for filesystem operations.

## Verified recent delivery

### Public AI — merged to main

- PR #59 — reduced PDF upload IPC expansion by using bounded base64 transfer; added native encoded/decoded size checks; made same-file retries possible after errors; clarified that image inspection is metadata-only. Merge commit: `813b85370b36deecf51f57cabd7cd3f3918c5422`.
- PR #57 — receipt ledger archival and rotation.
- PR #55 — bounded read-only large-file finder.
- PR #53 — browser-preview local-clock correction.
- PR #52 — everyday-tools delivery record.
- Earlier merged work includes encrypted validated local search cache, local file finder, managed folder import cancellation/receipts, PDF dependency upgrade, task registry cleanup, receipt coverage foundations, SBOM/provenance workflow and locked dependency builds.

### Public AI — active pull requests

- PR #60 — native OS confirmation for every cloud transfer and recovery of completed inference results when receipt persistence fails. The branch has a follow-up head commit; CI has restarted. Earlier lint output exposed dead-code and large-error-type warnings; the current head removes the unused wrapper and boxes the recoverable error. Re-check the current head's full CI before merge. Receipt retry remains process-local and not crash-durable; receipt chain remains unsigned and without external anchoring.
- PR #58 — peer IPC disabled by default and synchronous dialogs replaced with asynchronous native confirmation. Do not merge until the current head's full CI and CodeQL status are understood. An earlier CodeQL run reported a critical alert; a later run was neutral because the base CodeQL configuration could not be reconciled. Confirm the alert and configuration state rather than treating neutral as a pass.

Closed-but-unmerged PRs that duplicate later merged work should be reviewed for branch cleanup only after confirming their commits are not unique and not referenced by another open PR. Do not delete branches solely because a PR is closed.

## Priority backlog — Public AI

### P0 — stability and safe alpha foundation

1. **Complete PR #60.** Require current-head Rust formatting/lint, native tests on Linux/macOS/Windows, frontend build, dependency audit, CodeQL, workflow lint and installer builds. Review the native consent wording and failure/decline behavior. Confirm completed model results are still returned when receipt storage fails, warnings remain visible, and retries do not duplicate receipts or lose the reserved slot.
2. **Resolve PR #58 security gate.** Establish the exact CodeQL alert and whether it is a real issue or a stale/base-configuration artifact. Ensure default builds do not expose peer identity, enrollment, revocation or envelope IPC. Validate both default and opt-in preview feature builds. Do not expose Worker/Partner UI or remote execution.
3. **Reconcile the remaining dependency/security advisories.** Keep locked Rust/npm dependencies and the audit gate. Trace every denied advisory to its dependency path; upgrade or document a narrow, reviewed exception. Do not silence the audit.
4. **Re-run crash/hang regression coverage.** The PDF IPC expansion was a credible renderer-stall cause and is now fixed in PR #59. Add bounded tests for maximum-size PDFs, malformed and empty PDFs, repeated same-file selection, cancellation, native decode errors and UI responsiveness. The user has elected not to collect external tester feedback now; use CI and code-level verification until a later product-test phase.
5. **Audit all long-running Tauri commands.** Every operation must have input bounds, a managed task ID, cancellation, timeout/deadline, concurrency and memory limits, progress/error state, cleanup on every early return, and no UI-thread blocking.
6. **Close provenance coverage gaps.** Add receipts for the managed document workflow path (`execute_document_workflow`), selected-folder import (`process_selected_folder`) and model-download operations where appropriate. Keep receipt fields measured/inferred/unknown distinct. Verify that receipt failure never silently changes a completed inference into a failed user result.
7. **Complete release hygiene.** Configure owner-held updater signing keys and verify a signed tagged release. Keep Authenticode and Apple Developer ID signing/notarization as broad-distribution gates. Verify rollback and recovery before public distribution.

### P1 — local-first product completeness

8. **Local search/cache semantics.** Preserve encrypted, validated local search history/cache. On repeated identical requests, check for a valid cached result first; verify query, scope, index/version, permissions, freshness/TTL and source metadata before reuse. Invalidate on file changes or scope changes. Show cached age and allow refresh/clear. Never treat a prior result as proof that the file still exists or is still accessible.
9. **Everyday local computer workspace.** Continue the existing bounded, read-only selected-folder tools: recent files, empty folders, content search for supported formats, document metadata and image metadata. Add preview-only sorting/organisation next. Any move/rename/archive extraction requires explicit preview and confirmation, no-clobber defaults, path traversal/symlink defenses, cancellation and rollback manifest. Never auto-delete or add shell/model-generated commands.
10. **Screenshot and image understanding.** Keep current metadata-only capability labels. A screenshot-to-chat/vision attachment pipeline is a separate feature requiring explicit user selection, pixel/dimension/decompression limits, memory bounds, privacy consent and tests. Do not imply screenshots are sent to a model until implemented.
11. **PDF and media robustness.** Scanned-PDF OCR, image decoding and vision inference remain separate work. Add only with worker isolation, decompression-bomb defenses, output bounds and cross-platform validation.
12. **Model/runtime catalogue.** Add licence, source, checksum, supported architecture, quantisation and runtime compatibility metadata. Keep GPU/NPU unknown until driver/runtime capability is actually verified; Ollama VRAM telemetry is not a complete hardware inventory.
13. **Task graph execution.** Add typed node input/output schemas, execution adapters and safe execution boundaries only after the current fixed workflow is stable. Planning success must not be represented as execution or resource reservation.
14. **Resource-aware scheduling.** Move beyond policy-only scheduler contracts only when native telemetry freshness, atomic resource reservation, owner opt-in, thermal/AC/idle conditions, fairness, cancellation and actual dispatch can be enforced and tested.
15. **Benchmarks and impact claims.** Implement repeatable workload-normalised latency, memory, energy and quality measurements. Do not claim energy/carbon savings from theoretical scheduler policies or model estimates.

### P0 — Public AI release security gates

- OS-enforced application isolation on supported Windows, macOS and Linux configurations (network egress, filesystem, child processes, CPU/memory/file descriptors); current Fortress checks and PDF-worker restrictions are not a full application sandbox.
- Local-only mode verification with network disabled, including no cloud fallback and clear failure messaging.
- Independent threat modelling/security review and platform-specific real-device validation.
- History lifecycle: migration, encrypted backup/restore, key rotation/recovery, concurrent key initialization, retention and deletion verification. Exported JSON is plaintext and outside in-app deletion/encryption controls.
- Receipt archive integrity: signed or externally anchored checkpoint, anti-rollback protection, archive export/retention and recovery tests. Current SHA-256 chain is local and unkeyed.
- Publisher signing/notarization, updater key custody/rotation, signed release verification, SBOM and tested rollback.

## Priority backlog — Enterprise AI

Enterprise AI is a separate organisation-control-plane project. Its current policy and reservation primitives are not a deployable authenticated device-trust service or a remote-execution platform.

### P0 — issue #20: trusted device and authenticated lease control plane

1. Implement a durable authoritative device registry with tenant binding, lifecycle state, enrollment/approval evidence, key IDs, monotonic key generations, rotation history and auditable ownership.
2. Bootstrap administrator trust from protected deployment keys (HSM/KMS/TPM or controlled secret manager). Never accept caller-supplied administrator keys as trust anchors.
3. Implement challenge-based enrollment requiring device proof-of-possession and administrator approval; implement rotation and revocation that invalidate old generations and active leases.
4. Authenticate every lease acquire/renew/release/cancel request and bind it to tenant, device, key generation, workload, exact manifest digest, resource limits, policy and expiry.
5. Add signed canonical request envelopes, method/path/body binding, request IDs, strict expiry/clock-skew checks and replay protection across restarts and service instances.
6. Protect durable ledger integrity with a keyed MAC/signature whose key is outside the writable ledger directory; add anti-rollback checkpointing and key recovery/rotation.
7. Add fail-closed tests for cross-tenant access, revoked/stale devices, forged admin keys, replay, clock skew, tampering, concurrent reservations, service restart and partial failure.
8. Keep dispatch and sandbox execution out of scope until control-plane authorization is independently reviewed.

### P1 — Enterprise platform readiness

- Complete the API/service contract and production identity provider integration; define tenant/admin RBAC and operational audit.
- Integrate device trust with the durable reservation ledger transactionally and idempotently.
- Add multi-instance consistency, disaster recovery, backup restore and external monotonic anchoring.
- Implement worker-side verification of signed assignments before any future execution.
- Enforce OS sandboxing, scoped mounts, egress allow-lists, CPU/RAM/disk/network/time quotas, owner pause/opt-out and emergency stop.
- Add heartbeat, lease expiry, checkpoint, cancellation and bounded retry behavior.
- Validate output provenance and false-result detection.
- Measure incremental energy and cost per equivalent completed workload before any efficiency claim.
- Complete independent threat-model review and security testing before a pilot.

## Personal-to-Enterprise integration

- Publish a separately versioned, byte-exact interoperable task, approval, assignment and receipt protocol.
- Add cross-language test vectors, canonical encoding, parser limits, version negotiation and downgrade rejection.
- Bind user consent, data scopes, recipient identity, payload digests, expiry and policy version across both sides.
- Do not treat Public AI peer-envelope v2 and Enterprise administrator-approval claims as wire-compatible without an explicit adapter and verified contract.
- Do not enable Worker/Partner, public compute marketplace or remote workload execution until identity, authorization, transport, sandbox, audit and rollback gates all pass.

## Working sequence

1. Merge PR #59 (completed).
2. Complete current-head CI and security review for PR #60.
3. Resolve PR #58 CodeQL/base-configuration issue and complete both feature configurations.
4. Reconcile all duplicate closed PR branches and stale issues without deleting unique work.
5. Finish Public AI cancellation/receipt coverage and cache correctness.
6. Advance Public AI local computer workspace in bounded read-only increments.
7. Implement Enterprise issue #20 trust/lease control plane and its tests.
8. Only then design and verify the cross-repository protocol and any controlled pilot.

## Explicit non-goals for this phase

- No external tester feedback is assumed or awaited.
- No automatic file deletion or unrestricted disk scan.
- No shell execution or model-generated OS commands.
- No automatic cloud fallback.
- No enabled Worker/Partner execution, remote dispatch or public compute marketplace.
- No production/security/compliance/energy-saving claims without the evidence described above.
