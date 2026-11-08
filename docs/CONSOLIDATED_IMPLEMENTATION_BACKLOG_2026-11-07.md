# UJANI consolidated implementation backlog — 8 November 2026

This is an implementation and release-planning record for the Public AI and Enterprise AI repositories, reconciled against the current repository roadmaps, implementation-status documents, open issues, pull requests and CI evidence on 8 November 2026.

**Product status remains pre-alpha / prototype.** This document is not a security certification. A merged change or green hosted CI does not prove real-device behavior or production security.

## Operating direction

- Continue engineering independently; no external tester report is required to proceed with code work. Manual device-test evidence remains mandatory for release-critical install, cancellation, privacy, permission and stability gates.
- Prioritise reliability, privacy boundaries, cancellation, bounded resource use and honest capability descriptions over feature count.
- Keep Public AI and Enterprise AI independently governed until a separately versioned, tested interoperability protocol exists.
- Keep Worker/Partner and remote execution disabled. Do not enable distributed execution based only on protocol primitives or a passing unit test.
- Do not weaken CI, suppress dependency advisories, or merge security-sensitive work while required checks are failing or unexplained.
- Treat user-selected scope, explicit approval and read-only previews as defaults for filesystem operations.

## Verified recent delivery

### Public AI — merged to main

- PR #65 — unsigned cross-platform preview installer artifacts, size audit, and SHA-256 manifests. These remain preview-only artifacts, not production-signed packages.
- PR #69 — makes reducing unnecessary data-centre dependence and energy use a primary project objective. This is a policy/roadmap commitment, not measured energy savings or an enabled distributed scheduler.
- PR #71 — functional workspace sidebar navigation and saved-history loading improvements. Hosted checks passed; real desktop/mobile navigation smoke tests remain.
- Earlier merged work includes bounded PDF IPC, encrypted opt-in search cache/history, local file search, selected-folder managed tasks, receipt recovery, peer IPC disabled by default, and local Everyday Tools (device clock, duplicate-file preview, and large-file finder).

### Enterprise AI — merged to main

- PR #37 — energy-aware placement policy foundation. It does not yet measure Wh per equivalent task or dispatch workloads through a production scheduler.
- PR #45 — Collective Defence v1 privacy-minimised event schema, CI contract validator, and event-boundary documentation. This is a contract and policy foundation, not a live sensor, ingestion service, correlation engine, or response system.

### Active pull requests and current validation

- PR #66 — merged into `main`; this consolidated backlog and status reconciliation are now part of the default branch.
- PR #68 — platform support policy plus release-workflow gates. It requires Windows Authenticode signing and verification, macOS Developer ID signing/notarization credentials and artifact checks, plus a `production-release` environment. Repository administrators must configure required reviewers and disable self-review for that environment. Manual minimum-OS install/launch and provenance/SBOM review remain human release gates. Current-head CI/CodeQL/installer checks are queued/running on the refreshed branch.
- PR #72 — PDF cancellation hardening: abortable webview FileReader, native task registration before decode, bounded single-worker PDF decoding, observable decode_started/decode_finished events, and bounded pending cancellation across the FileReader-to-native handoff. A deterministic native worker-barrier regression now proves cancellation returns while a started blocking worker retains the decoder permit, rejects overlap, and permits retry only after exit. Current-head CI/CodeQL/installer checks are running; real-device FileReader cancellation, normal 10 MiB upload, malformed/empty PDF and same-file retry evidence remain NOT RUN.
- PR #74 — merged to `main`; the manual smoke-test matrix covers install/launch, PDF cancellation/retry, image metadata inspection, navigation/freeze recovery, privacy/history, cloud egress and cross-platform evidence. It explicitly marks screenshot capture/chat attachment and vision inference NOT IMPLEMENTED; it is a test procedure only, and no manual device results are claimed.
- PR #41 in Enterprise AI remains a draft design specification for durable consent authority, now rebased on current main and updated to distinguish the in-memory reference backend from production persistence.

PR #40, #43 and #44 were closed as superseded: the dual-consent primitive is already on Enterprise main through merged PR #42, and the consolidated event contract is on main through merged PR #45. Do not delete their branches until confirming they are not referenced by another open PR.

## Priority backlog — Public AI

### P0 — stability and safe alpha foundation

1. **Keep merged PR #64 regression-safe.** Its native cloud-consent and bounded receipt-recovery changes are on `main`; completed model results are preserved when receipt storage fails, with a visible warning and a bounded process-local retry queue. The queue is not crash-durable, and the local receipt chain is not externally anchored.
2. **Keep peer IPC disabled by default.** PR #58 is merged. Confirm default builds do not register peer identity, enrollment, revocation or envelope IPC; compile the opt-in preview feature separately. Do not expose Worker/Partner execution or remote dispatch.
3. **Reconcile the remaining dependency/security advisories — release blocker tracked in [issue #73](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/73).** The latest inspected desktop lockfile reports six unresolved RustSec warnings:
   - `glib 0.18.5` — `RUSTSEC-2024-0429` (unsound iterator implementation).
   - `proc-macro-error 1.0.4` — `RUSTSEC-2024-0370` (unmaintained).
   - `ttf-parser 0.25.1` — `RUSTSEC-2026-0192` (unmaintained).
   - `zbus_names 3.0.0` — `RUSTSEC-2026-0371` (unmaintained).
   - `zvariant 4.2.0` — `RUSTSEC-2026-0370` (unmaintained).
   - `zvariant_derive 4.2.0` — `RUSTSEC-2026-0372` (unmaintained).
   
   A successful audit job that reports warnings is not a clean dependency posture. Trace each transitive path and require an individual compatible upgrade or a documented, time-bounded blocker with an owner. Do not add blanket `ignore` entries or weaken the audit gate.
4. **Continue crash/hang regression coverage.** PR #59 replaced large JSON-number IPC payloads with bounded base64 transfer and enabled same-file retries. PR #64 added empty-file rejection, bounded base64 decoding on the blocking pool, and a maximum-size decode-thread regression test. PR #72 remains open pending current-head CI/CodeQL/installer results. Its branch aborts FileReader, registers before native decode, queues bounded pre-registration PDF cancellation, emits decode_started/decode_finished events, and holds a global single-worker semaphore until any started blocking decode exits. Native unit tests now cover both pre-registration cancellation consumption and cancellation/retry while the blocking worker is held by a deterministic barrier. Real-device FileReader cancellation, malformed/empty/large PDF, same-file retry and UI responsiveness evidence remain NOT RUN. Use CI and code-level verification until the device-test phase.
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

### Current Collective Defence and energy status

- Dual consent is still mandatory: tenant administrator opt-in and separate individual opt-in for the exact principal/device. The signed policy primitive and MemoryConsentAuthority reference backend exist on main; the memory backend is process-local, non-durable, and not rollback-resistant.
- PR #41 documents the durable authority design. Production database transactions, an external monotonic anchor, protected admin/user key custody, authenticated control-plane APIs, and durable revocation are still outstanding under issue #20 and issue #39.
- PR #45 merged the v1 event contract and schema CI validation. A read-only endpoint sensor, authenticated event ingestion, durable event store, cross-device correlation, analyst dashboard, and controlled multi-endpoint lab validation remain outstanding under issues #38 and #35.
- PR #37 merged energy-aware placement policy only. Hardware energy measurement, workload-normalised quality/latency comparison, resource-aware runtime scheduling, and a controlled device pilot remain outstanding.
- Tenant-isolated fleet/cache work (#26), Control Plane UI (#21), and read-only fleet operations (#23) remain blocked on trusted enrollment, authenticated leases, and server-side tenant/RBAC enforcement (#20).
- Keep remote execution, Worker/Partner dispatch, automated containment, and cross-tenant sharing disabled until the corresponding trust, consent, receiver-side authorization, sandboxing, audit, and independent-review gates pass.

Enterprise AI is a separate organisation-control-plane project. Its current policy and reservation primitives are not a deployable authenticated device-trust service or a remote-execution platform.

### P0 — issue #20: trusted device and authenticated lease control plane

1. Durable device registry primitives, tenant binding, lifecycle state, key IDs, monotonic device-key generations, signed admin enrollment/rotation/revocation and an external-anchor contract are implemented. The adapter reloads state under an OS lock and authenticates snapshots with an injected HMAC key; production host/key-provider and external anchor integration remain outstanding.
2. Bootstrap administrator trust from protected deployment keys (HSM/KMS/TPM or controlled secret manager). Never accept caller-supplied administrator keys as trust anchors. Production key custody and admin-key lifecycle service remain outstanding.
3. Device proof-of-possession and administrator approval, rotation and revocation primitives are implemented; service/API integration and transactional lease invalidation on revocation remain outstanding.
4. PR #33 merged: device requests are signed over the exact raw body digest, body size is capped at 1 MiB, and the request nonce is not consumed on body-digest mismatch. Durable registry tests cover replay across restart and concurrent handles. This is authenticated-request groundwork, not an authenticated lease service. A follow-up main-branch regression test now verifies that administrator revocation survives restart and rejects subsequent requests signed by the revoked device; its CI is pending.
5. Authenticate every lease acquire/renew/release/cancel request and atomically bind it to tenant, device, key generation, workload, exact manifest digest, resource limits, policy and expiry. This is still open; the current durable ledger lease-token primitives are not yet integrated with the authenticated device registry.
6. Add signed canonical request envelopes, method/path/body binding, request IDs, strict expiry/clock-skew checks and replay protection across restarts and service instances. Device-request primitives exist; production HTTP/API adapter and shared transaction boundary remain outstanding.
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

1. Keep merged Public AI privacy, history, peer-IPC-off, bounded PDF and receipt-recovery guarantees regression-tested.
2. Finish current-head CI/CodeQL/installer validation for PRs #68, #72 and #74; merge only when checks pass and review threads are resolved.
3. Execute the manual device matrix on Windows, macOS, Linux and narrow/mobile viewports. Do not mark NOT RUN/BLOCKED cases as passed.
4. Resolve issue #73 dependency advisories individually; do not suppress warnings or hand-edit the lockfile.
5. Finish PDF malformed/encrypted/large-input and image-metadata failure-path tests; close the remaining application freeze/navigation regressions. Screenshot-to-chat/vision remains a separate unimplemented feature with its own local-first privacy and resource-safety requirements.
6. Expand Public AI's bounded, read-only local workspace (recent/empty/content search and safe preview workflows) without unrestricted disk scans or automatic mutations.
7. Implement and independently review Enterprise durable trust and dual-consent persistence, external monotonic anchoring, authenticated leases, and receiver-side authorization.
8. Build read-only Collective Defence sensor/ingestion and explainable correlation only after trust/consent are durable; keep response actions disabled.
9. Turn the energy policy into measured workload-normalised energy telemetry and a consent-aware scheduler; no savings claims before equivalent-task measurements.
10. Only then design and verify cross-repository interoperability and a controlled multi-device pilot.

## Explicit non-goals for this phase

- No external tester feedback is assumed or awaited.
- No automatic file deletion or unrestricted disk scan.
- No shell execution or model-generated OS commands.
- No automatic cloud fallback.
- No enabled Worker/Partner execution, remote dispatch or public compute marketplace.
- No production/security/compliance/energy-saving claims without the evidence described above.
