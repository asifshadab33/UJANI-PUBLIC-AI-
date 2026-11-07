# UJANI project status and release gates

**Project status: pre-alpha / prototype.** This file consolidates the verified implementation boundary and outstanding work across UJANI Public AI and UJANI Enterprise AI as of 6 November 2026. It is a project planning record, not a security certification.

## Product architecture

- **UJANI Public AI** is the personal-device application: local-first assistance, user-controlled local models, bounded local tools and explicitly approved BYOK cloud requests.
- **UJANI Enterprise AI** is the organisation-controlled policy and fleet foundation: administrator approval, workload policy contracts and future organisation-level device/workload governance.
- The two repositories remain independently governed. A future Personal-to-Enterprise bridge must use a separately versioned, interoperable protocol; the existing Public task-envelope and Enterprise administrator-approval formats are not wire-compatible.

## Implemented foundations

### Public AI

- Tauri desktop application with Rust core and React/TypeScript UI.
- Local Ollama inference bound to loopback, bounded task inputs/outputs, cancellation and task concurrency controls.
- User-selected text/PDF/image-header processing, fixed local document pipeline and native selected-folder import with bounded file counts/sizes.
- Local encrypted history with AES-GCM v2 row/column authenticated data and migration support.
- BYOK OpenAI-compatible HTTPS gateway with explicit per-task consent, endpoint/DNS checks, redirect/proxy restrictions, response bounds and local-only Fortress policy.
- Native boundaries currently validate routing through `router::evaluate_mode`, validate BYOK requests through `cloud_gateway::validate_request`, and validate command/task identifiers. The separate `safety_policy` and `policy_gate` modules are not yet universal gates on every native command.
- Versioned workflow planning, resource-aware scheduler policy contracts and typed execution receipt contracts.
- Peer-envelope v2 uses recipient-bound, domain-separated Ed25519 signatures with strict verification; the desktop also has OS-vault device identity, native-confirmed peer trust/revocation and persistent nonce claims. There is still no encrypted peer transport, remote dispatch or remote execution.
- Native IPC input bounds, history-key zeroization improvements, optional parser fuzz harnesses and pinned CI dependencies.
- These foundations do not establish a full OS sandbox, hardware attestation, distributed execution or independent verification of model output.

### Enterprise AI

- Signed administrator approval and policy-validation foundations.
- Ed25519 approval claims bound to workload/device/class/resource limits and expiry.
- In-memory reservation leases, expiry/renewal handling, replay protection within process lifetime and overflow-safe resource accounting.
- Strict, versioned workload manifest input contract and JSON Schema, with validation tests.
- The workload manifest is not an execution grant. Production identity enrollment, durable state and a complete control plane remain future work.

## Critical release gates — not yet implemented or independently verified

### P0 — before describing either product as production-ready

1. **OS-enforced isolation:** implement and test supported Linux, macOS and Windows sandbox adapters. Fortress is currently an application policy boundary, not OS-enforced network/process isolation. Validate filesystem, network-egress, child-process, CPU, memory and file-descriptor restrictions.
2. **PDF worker containment:** OS-specific worker restrictions are implemented in the current remediation branch (Unix resource limits and Linux network syscall filtering, macOS deny-network/file-write profile, Windows Job Object). Validate launch, cancellation, termination and cleanup on all supported OSes; these worker restrictions are not a full application sandbox.
3. **Security verification:** commission independent threat modelling and penetration testing; review dependencies/SBOM, signing, update provenance, secrets, IPC, local services, cloud gateway, storage and installer/release pipeline.
4. **Release engineering:** establish signed reproducible builds, provenance/attestations, protected release approvals, artifact verification, rollback and a tested update/recovery process.
5. **Data lifecycle:** verify history migration, backup/restore, user-visible export/deletion, key rotation/recovery, retention enforcement and concurrent key initialization across supported OS credential stores.

### P1 — required for real distributed Personal-to-Enterprise execution

1. **Interoperable protocol:** publish byte-exact canonical envelope and receipt schemas, cross-language test vectors, strict parser limits, version negotiation and compatibility policy.
2. **Identity and key lifecycle:** authenticate sender and organisation enrollment out of band; bind keys to sender, organisation, permitted audience and validity; support rotation, revocation and expiry.
3. **Confidential transport:** authenticated TLS on every hop or equivalent end-to-end authenticated encryption for payloads and results. Signatures and digests alone do not provide confidentiality.
4. **Durable replay protection:** persistent, transactional, restart-safe nonce/task-ID store; test concurrent duplicate requests and crash recovery.
5. **Receiver-side authorization:** verify signatures and exact payload digests before parsing/execution; re-evaluate current organisation policy, user consent, device status, scopes, quotas and resource limits immediately before dispatch.
6. **Result provenance:** sign receipts; bind worker and organisation identity; verify task ID and input digest; hash exact returned output bytes and compare to the signed output digest before accepting results.
7. **Authenticated transport/API:** build the Enterprise control-plane API, device agent, mutual authentication, authorization, audit logs, rate limiting, idempotency, failure recovery and tenant isolation.
8. **Durable reservations:** replace in-memory-only reservation/replay state with a transactional store and crash-safe leases, accounting, expiry, reconciliation and idempotency.

### P1 — actual resource-aware execution

1. Wire live, timestamped CPU/RAM/thermal/AC/battery/accelerator telemetry into scheduler snapshots. Reject stale or unverified measurements.
2. Verify GPU/NPU runtime, driver, device identity and usable VRAM; implement safe CPU fallback and tests on real hardware. Hardware presence alone is not proof of compatibility.
3. Enforce atomic resource reservations, concurrency limits, release-on-cancel/failure, queue fairness, priorities and back-pressure. The current scheduler is a pure policy contract, not a dispatcher.
4. Implement model-specific memory estimates using quantisation, context length, runtime and accelerator offload; keep estimates advisory until validated against measurements.
5. Persist execution receipts and benchmark evidence. Verify exact input/output bytes and avoid treating a model's self-reported quality as independent verification.

### P2 — quality, product and operations

1. Run and retain cargo-fuzz campaigns for PDF and image parsers; add regression fixtures for every crash.
2. Add integration tests against real OpenAI-compatible providers, including Azure OpenAI and custom deployment aliases, auth headers, token-field selection, errors, usage/cost reporting and cancellation. Current token-family detection is a compatibility heuristic, not a provider guarantee.
3. Add user-configurable spending limits, per-task cost estimates, provider data-retention disclosure and granular field-level consent.
4. Complete GPU/NPU compatibility inventory and native smoke tests on supported Windows/macOS/Linux hardware.
5. Establish reproducible benchmarks for latency, output quality, RAM/VRAM, energy and failures. Label energy sources as direct meter, OS estimate or modelled estimate.
6. Add full image decode/OCR/vision only with pixel, dimension, memory and decompression-bomb limits.
7. Add a real mobile client only after desktop contracts and threat model stabilise.
8. Add enterprise tenant management, device enrollment, policy administration, quotas, audit export, admin UX, operational monitoring and incident response.

## Current known limitations and non-claims

- Worker and Partner are not general distributed execution modes; no peer transport or device-to-device dispatch is implemented.
- The Public scheduler does not collect telemetry, reserve resources, dispatch work or guarantee capacity.
- The execution receipt type is a contract; there is no durable receipt ledger or universal cryptographic attestation of execution.
- Peer protocol v2 plus local device identity, native-confirmed trust/revocation and persistent nonce claims do not provide encrypted/authenticated network transport, remote dispatch, organisation-level identity, or remote execution.
- Enterprise in-memory leases/replay protection are not durable across process restarts.
- A workload manifest validates requested input only; it does not authenticate a sender, authorize a workload or execute it.
- Parser fuzz targets exist, but no completed fuzz campaign is claimed.
- CI compilation and unit tests do not substitute for native desktop smoke tests, hardware tests or independent security testing.

## Suggested implementation order

1. Stabilise and merge the current Public security remediation only after all CI and CodeQL checks pass.
2. Rework Public PR #14 against the current `main` base, resolve conflicts, and make all three native platform checks pass before merging its resource-aware and runtime-hardening work.
3. Build the Enterprise durable identity, approval, reservation and replay store before enabling any external worker.
4. Freeze the shared Personal-to-Enterprise protocol and publish cross-language test vectors before implementing transport.
5. Implement OS isolation and PDF worker limits before production security claims.
6. Add real hardware telemetry, resource reservations and measured benchmark/receipt storage before claiming distributed optimisation.

## Verification checklist

- [ ] Rust core tests and clippy on locked dependencies.
- [ ] Native Tauri compile/tests on Windows, macOS and Linux.
- [ ] Frontend TypeScript build and dependency audit.
- [ ] CodeQL and dependency/SBOM review.
- [ ] Manual smoke tests with Ollama absent, model missing, cancellation, timeout, low memory, corrupted history and folder-picker edge cases.
- [ ] Cloud integration tests using provider test accounts and explicit spend caps.
- [ ] Sandbox escape and network-egress tests on each supported OS.
- [ ] Cross-language protocol vectors, restart/replay/concurrency tests and signed receipt verification.
- [ ] Independent penetration test and release-signing verification.
