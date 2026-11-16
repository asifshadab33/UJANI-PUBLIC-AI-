# UJANI project status and release gates

**Project status: pre-alpha / prototype.** This file consolidates the verified implementation boundary and outstanding work across UJANI Public AI and UJANI Enterprise AI as of 16 November 2026. It is a project planning record, not a security certification.

## Product architecture

- **UJANI Public AI** is the personal-device application: local-first assistance, user-controlled local models, bounded local tools and explicitly approved BYOK cloud requests.
- **UJANI Enterprise AI** is the organisation-controlled policy and fleet foundation: administrator approval, workload policy contracts and future organisation-level device/workload governance.
- The two repositories remain independently governed. A shared Personal-to-Enterprise task-contract v1 now exists in both repositories: versioned schemas, shared consent-binding/model-inventory vectors, strict validators and native Rust decoders/verifiers. It is a protocol boundary only; it does not provide authenticated transport, enrollment-to-key binding, atomic policy/consent/lease admission, dispatch or execution.

## Implemented foundations

### Public AI

- Tauri desktop application with Rust core and React/TypeScript UI.
- Local Ollama inference bound to loopback, bounded task inputs/outputs, cancellation and task concurrency controls.
- User-selected text/PDF/image-header processing, fixed local document pipeline and native selected-folder import with bounded file counts/sizes.
- Optional local task history uses AES-256-GCM v3 with row/column and metadata-bound authenticated data; legacy v1/v2 records are supported for migration. Recording remains disabled by default and requires explicit user opt-in. If the OS credential vault is unavailable, encrypted history remains unavailable rather than falling back to plaintext.
- BYOK OpenAI-compatible HTTPS gateway with explicit per-task consent, endpoint/DNS checks, redirect/proxy restrictions, response bounds and local-only Fortress policy.
- Native boundaries currently validate routing through `router::evaluate_mode`, validate BYOK requests through `cloud_gateway::validate_request`, and validate command/task identifiers. The separate `safety_policy` and `policy_gate` modules are not yet universal gates on every native command.
- Versioned workflow planning, resource-aware scheduler policy contracts and typed execution receipt contracts.
- Peer-envelope v2 uses recipient-bound, domain-separated Ed25519 signatures with strict verification; the desktop also has OS-vault device identity, native-confirmed peer trust/revocation and persistent nonce claims. There is still no encrypted peer transport, remote dispatch or remote execution.
- The desktop has a coarse process-local single-task / 1 GiB free-RAM admission gate and a local SQLite receipt hash chain. Receipts cover successful local chat, explicitly approved BYOK chat, selected-text summaries, the selected-document/PDF command paths that use the managed-task wrapper, local translation, successful `execute_document_workflow` runs and successful selected-folder imports (`process_selected_folder`) through `run_managed_task`. Model downloads use a separate reserved receipt path and record success, failure, cancellation or timeout; downstream registry host, transferred bytes and final model artifact digest remain unknown because Ollama performs registry traffic inside its own daemon. A complete command-by-command coverage matrix and failure/cancellation audit remain outstanding. The ledger is local, unsigned and not externally anchored.
- Native IPC input bounds, history-key zeroization improvements, optional parser fuzz harnesses and pinned CI dependencies.
- These foundations do not establish a full OS sandbox, hardware attestation, distributed execution or independent verification of model output.

### Enterprise AI

- Signed administrator approval and policy-validation foundations.
- Ed25519 approval claims bound to workload/device/class/resource limits and expiry.
- A local file-backed reservation ledger with OS advisory locking, snapshot checksum validation, backup/rename recovery, opaque lease capabilities, authenticated renewal/release and durable challenge replay claims. It remains single-host and is not a distributed transaction service.
- PostgreSQL migrations now define tenant-scoped base records, task-contract nonce/sequence storage and monotonic privacy-minimised device telemetry admission (migrations 0001–0003), with RLS, replay, rollback and concurrency tests. These are database primitives; there is no production SQLx repository or authenticated HTTP/device-agent adapter.
- The durable device-trust registry supports signed administrative commands, owner consent, authenticated request checks and an optional external monotonic-snapshot-anchor interface. No production KMS/HSM/TPM anchor provider is wired in.
- Native task-contract v1 parsing and verification is present. The verifier is not an execution grant. Production identity hosting, atomic repository transactions and a complete control plane remain future work.

## Critical release gates — not yet implemented or independently verified

### P0 — before describing either product as production-ready

1. **OS-enforced isolation:** implement and test supported Linux, macOS and Windows sandbox adapters. Fortress is currently an application policy boundary, not OS-enforced network/process isolation. Validate filesystem, network-egress, child-process, CPU, memory and file-descriptor restrictions.
2. **PDF worker containment:** OS-specific worker restrictions include Unix resource limits, Linux seccomp denial of network/process-creation/namespace syscalls, a macOS deny-network/file-write profile, and a Windows Job Object with memory/CPU/one-process limits. Validate launch, cancellation, termination and cleanup on all supported OSes; these worker restrictions are not a full application sandbox.
3. **Security verification:** commission independent threat modelling and penetration testing; review dependencies/SBOM, signing, update provenance, secrets, IPC, local services, cloud gateway, storage and installer/release pipeline.
4. **Release engineering:** committed Cargo/npm lockfiles and locked dependency installation are now in `main`. The updater now requires signatures to bind the announced version, and release publication verifies each platform signature; SBOM/provenance generation remains a release gate until a controlled tagged candidate passes. Reproducible binary builds, protected approvals, rollback and tested update/recovery remain outstanding.
5. **Data lifecycle:** user-confirmed export of up to 100 loaded history entries as versioned plaintext JSON and in-app deletion are implemented. Verify history migration, encrypted backup/restore, key rotation/recovery, retention enforcement and concurrent key initialization across supported OS credential stores; exported plaintext copies remain outside UJANI's encryption and deletion controls.

### P1 — required for real distributed Personal-to-Enterprise execution

1. **Interoperable protocol:** v1 schemas, shared consent-binding/model-inventory vectors, strict Python validators and native Rust decoders/verifiers now exist in both repositories. Still required: freeze compatibility/version policy, exercise the same end-to-end fixtures across both implementations, and bind verification to authenticated identities before transport.
2. **Identity and key lifecycle:** authenticate sender and organisation enrollment out of band; bind keys to sender, organisation, permitted audience and validity; support rotation, revocation and expiry.
3. **Confidential transport:** authenticated TLS on every hop or equivalent end-to-end authenticated encryption for payloads and results. Signatures and digests alone do not provide confidentiality.
4. **Durable replay protection:** PostgreSQL tenant-scoped nonce claims and monotonic status/receipt sequence tables now exist with RLS and CI tests. The production service adapter must still bind verifier success to atomic replay claims and sequence updates; no authenticated intake service exists.
5. **Receiver-side authorization:** verify signatures and exact payload digests before parsing/execution; re-evaluate current organisation policy, user consent, device status, scopes, quotas and resource limits immediately before dispatch.
6. **Result provenance:** sign receipts; bind worker and organisation identity; verify task ID and input digest; hash exact returned output bytes and compare to the signed output digest before accepting results.
7. **Authenticated transport/API:** build the Enterprise control-plane API, device agent, mutual authentication, authorization, audit logs, rate limiting, idempotency, failure recovery and tenant isolation.
8. **Durable reservations:** the merged file-backed ledger is a single-host prototype with OS advisory locks and backup/rename recovery. Before production, add a trusted storage boundary, authenticated service principal binding, keyed integrity/encryption at rest, migrations, backup/restore drills, and cross-process crash/power-loss testing on supported filesystems.

### P1 — actual resource-aware execution

1. A tenant-scoped PostgreSQL telemetry sample table and monotonic admission function now exist, including freshness, replay, consent, trust and sequence checks. No authenticated device agent/service adapter supplies signed samples yet; live telemetry is not connected to the scheduler.
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
- The core Public scheduler remains a policy contract: it does not collect live telemetry, dispatch tasks or guarantee capacity. The desktop's single-task / 1 GiB admission gate is process-local and is not a kernel or cross-process reservation.
- Public AI has a local SQLite hash-chain receipt ledger covering successful local chat, explicitly approved BYOK chat, selected-text summaries, selected-document/PDF paths, local translation, successful `execute_document_workflow` runs and successful selected-folder imports (`process_selected_folder`). Model downloads also reserve and persist outcome receipts for success, failure, cancellation or timeout, while downstream registry host, transferred bytes and final model artifact digest remain unknown. A complete command-by-command coverage matrix and failure/cancellation audit remain outstanding. The ledger is unsigned and unanchored, so it is not an independent execution attestation.
- Peer protocol v2 plus local device identity, native-confirmed trust/revocation and persistent nonce claims do not provide encrypted/authenticated network transport, remote dispatch, organisation-level identity, or remote execution.
- Enterprise retains a single-host file-backed reservation ledger, while PostgreSQL migrations now provide tenant-scoped replay/sequence and telemetry storage primitives. The durable device registry supports keyed integrity and an optional external monotonic anchor interface, but no production anchor provider or authenticated SQLx repository is integrated. These components do not form a production control-plane database or distributed transaction service.
- A workload manifest validates requested input only; it does not authenticate a sender, authorize a workload or execute it.
- Parser fuzz targets exist, but no completed fuzz campaign is claimed.
- CI compilation and unit tests do not substitute for native desktop smoke tests, hardware tests or independent security testing.

## Suggested implementation order

1. Complete full application OS isolation and cross-platform PDF-worker verification; the current worker limits are partial and do not provide a filesystem sandbox.
2. Validate release SBOM/provenance generation on a controlled tagged candidate and complete reproducible binary builds, signed updater verification and rollback tests; committed dependency lockfiles are already in `main`.
3. Harden the merged Enterprise file-backed ledger (including capacity handling, trusted storage, key lifecycle, authenticated control-plane APIs and receiver-side authorization) before enabling any external worker.
4. Freeze the shared Personal-to-Enterprise protocol and publish cross-language test vectors before implementing transport.
5. Implement authenticated encrypted peer transport, dispatch and signed result verification only after receiver-side authorization and OS isolation are in place.
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


## Cross-chat product audit and competitive position — 8 November 2026

This addendum reconciles the user's prior UJANI decisions with the repository state and a public-documentation competitor review. It is an engineering planning assessment, not a certification, benchmark result, or claim of feature parity.

### Product principles to preserve

- **Public AI:** local-first, local-capable and energy/resource-aware assistance. Use the customer's existing CPU/GPU/NPU, installed runtimes and operating-system facilities before requesting remote inference. Cloud/BYOK remains optional, task-scoped, budgeted and explicitly approved. Avoid a permanent background model loop and avoid reimplementing OS services or full third-party engines.
- **Enterprise AI:** an open-source-first coordination and customer-experience layer above the customer's existing firewall, endpoint protection, Suricata, Zeek, Malcolm and other selected tools. It is not a replacement antivirus, firewall, packet sensor or OT security engine. Prefer integration, orchestration and clear operational workflows over duplicating mature products.
- Keep the Public and Enterprise trust boundaries separate. Any future bridge needs a versioned interoperable protocol, separate consent, tenant/device identity and receiver-side authorization.
- Keep new code, dependencies, operating cost and endpoint load low. Make optional capabilities independently installable/removable, explain what each profile consumes, and fail closed when identity, authorization or telemetry is missing.
- Do not enable arbitrary remote execution, stealth collection, automatic destructive response or fleet-wide containment.

### Competitive review (public product documentation, checked 8 November 2026)

| Area | Existing strengths to respect | UJANI implication |
|---|---|---|
| Local personal AI — OpenJarvis | Local-first model execution, Ollama support, agents, file research and energy/cost/latency as explicit efficiency concerns. | Do not compete by shipping another model runtime. Differentiate through low-overhead coordination across existing local resources, transparent routing, permissions, energy evidence and useful OS workflows. |
| Desktop AI — AnythingLLM | One-click desktop setup, local/cloud model choice, workspaces, document Q&A, agents/skills and MCP; its overlay can use context from open applications. | Installer simplicity, discoverability and workflow polish are table stakes. UJANI must keep screen/app context explicitly user-selected and consented; do not imply arbitrary OS-wide reading. |
| Self-hosted AI — Open WebUI | Broad provider/model support, RAG, MCP/tools, automation, team controls and scalable deployment options. | Avoid feature-count competition. Keep the core small; integrate through documented provider/MCP contracts and spend effort on safe local execution and resource efficiency. |
| Enterprise security fabric — Fortinet | Integrated network/endpoint/identity policy, central management, unified telemetry and AI-assisted operations. | UJANI Enterprise should coordinate existing controls and present a coherent workflow; it should not claim to reproduce FortiOS/FortiManager enforcement or threat intelligence. |
| OT endpoint/network — TXOne | OT-specific endpoint visibility/protection, lightweight user-space visibility option, offline/air-gap operation and central management; network product line includes OT-aware controls. | Installer profiles, legacy/low-resource compatibility, passive/visibility-only mode, coexistence and explicit operational-safety controls matter. Never promise equivalent OT prevention without independent validation. |
| OT/IoT visibility — Nozomi | Passive network sensors, asset inventory/topology, behavioral baselining, risk prioritization and coordinated response across sites. | Start with read-only, provenance-rich integration and asset/health views. Do not introduce active probing or packet capture by default. |
| Open network monitoring — Security Onion and Malcolm | Mature Suricata/Zeek/Elastic/Arkime/OpenSearch-based network and host visibility pipelines and analyst interfaces. | Reuse/deploy these components as optional profiles rather than rewriting packet analytics, SIEM, case management or protocol parsers. |

Public references reviewed:
- OpenJarvis / Ollama: https://ollama.com/blog/openjarvis
- AnythingLLM desktop assistant and installation: https://docs.anythingllm.com/desktop-assistant/introduction and https://docs.anythingllm.com/installation-desktop/overview
- Open WebUI comparison: https://docs.openwebui.com/alternatives/anythingllm/
- Fortinet FortiOS: https://www.fortinet.com/products/fortigate/fortios
- TXOne portfolio and Stellar: https://www.txone.com/resources/data-sheets/ot-cybersecurity-portfolio/ and https://www.txone.com/products/endpoint-protection/
- Nozomi platform and Guardian: https://www.nozominetworks.com/platform and https://www.nozominetworks.com/platform/guardian
- Security Onion: https://docs.securityonion.net/en/3/dev/introduction/
- CISA Malcolm: https://cisagov.github.io/Malcolm/docs/

### Cross-chat requirements not to lose

1. **Public AI daily-work coordinator:** local reminders, agenda, notifications and later individually consented read-only calendar/email/Zoom/notes adapters. The first release must not scan every installed application, read the whole disk, send messages or mutate external calendars. Existing issues #81 and #83 track the staged planner work; draft PR #82 is specification only.
2. **Public AI Security Centre:** optional, read-only status aggregation for selected installed security providers, with source, timestamp, honest unknown/unavailable states and explicit consent. PR #80 is draft and its Windows adapter is not a full security audit or a substitute for product-specific endpoint telemetry.
3. **Everyday OS utilities and computer workflows:** reuse OS APIs and existing integrations; keep capabilities opt-in and bounded. Do not add a parallel plugin/automation framework or an always-on autonomous loop.
4. **Resource and energy intelligence:** measure CPU/RAM/VRAM, thermals, power/battery, inference latency, failures and energy provenance; schedule only when live measurements are fresh. Label direct measurements separately from OS estimates and modelled estimates.
5. **Enterprise deployment profiles:** the installer/product plan must support customer-selected profiles: firewall-only, lightweight, full-monitoring and Malcolm-integrated, with optional add/remove components, preflight CPU/RAM/storage/network checks, clear third-party licensing and responsibility boundaries, and a unified graphical operator experience. These are packaging/orchestration profiles, not a claim that UJANI replaces the selected tools. No implementation or validated resource sizing is claimed yet.
6. **Enterprise security orchestration:** customer-controlled integration of existing firewalls, antivirus/EDR, Suricata, Zeek and Malcolm; read-only status first, then explicit human-approved and reversible actions only after authenticated control-plane enforcement and audit are proven.
7. **First-release usability:** verify mobile layout/loading, primary navigation and real-time GUI states; generate platform installers and test installation, upgrade, uninstall, permissions, sidecar placement and rollback on real supported devices. CI artifact generation alone is not a human installation test.
8. **Open-source and liability clarity:** document component licences, optional third-party dependencies, data flows, threat boundaries, operational assumptions and non-replacement/non-certification statements. Obtain legal/security review before making compliance or protection guarantees.

### Self-audit findings and blockers

**Public AI**
- The current project status records that safety_policy/policy_gate are not universal gates on every native command. Complete a command-by-command capability/authorization matrix before expanding tools or connectors.
- The earlier audit incorrectly listed `execute_document_workflow`, selected-folder import and model downloads as uncovered. Current code records successful workflow/folder-import results through `run_managed_task`, and model downloads use a separate reserved receipt path that records outcome state. Downstream model-registry host, transferred bytes and final artifact digest remain unknown. Complete the command-by-command coverage matrix, verify failure/cancellation paths for every managed command, and retain the limitation that the local hash-chain ledger is unsigned and unanchored.
- Fortress is application policy, not an OS sandbox. The PDF worker's platform restrictions are not a full application sandbox. Complete and test OS-enforced isolation separately.
- PDF upload/worker protections and CI are necessary reliability work, but do not claim OCR for scanned PDFs, screenshot-to-chat, mobile parity or whole-application isolation unless implemented and validated.
- The Public dependency issue #73 remains a release blocker until the six tracked RustSec warnings are safely remediated or precisely bounded with documented upstream constraints. A green audit job that reports allowed warnings is not remediation.
- Local model onboarding, runtime compatibility, resource telemetry, measured energy benchmarks, mobile loading, signed reproducible releases, SBOM/provenance, update rollback and independent penetration testing remain separate acceptance gates.

**Enterprise AI**
- The durable reservation ledger is a single-host prototype; it is not a distributed transactional service or rollback-resistant production trust store. Production KMS/HSM/TPM key custody and a separately hosted linearizable anti-rollback anchor remain missing.
- Core signed device enrollment/administration, key-generation checks, revocation and owner-consent primitives now exist in the Rust trust layer. Production OIDC/tenant RBAC hosting, protected key-provider/anchor implementations, SQLx-backed atomic lease and audit transactions, and authenticated HTTP/device-agent endpoints remain missing. Do not enable privileged UI actions or dispatch on top of local primitives.
- The opt-in read-only sensor, authenticated tenant event ingestion, durable event store, cross-endpoint correlation, fleet dashboard and installer profiles are not yet a production implementation.
- Keep remote execution and autonomous containment disabled. Any future response action must be narrow, receiver-authorized, expiring, reversible, idempotent, approved and immutably audited.

### Recommended delivery sequence

1. **Release hardening first:** finish Public PR #117 review and manual desktop PDF/cancel/retry checks; resolve dependency advisories; verify signing, SBOM/provenance, update rollback and OS isolation. Keep PR #117 unmerged until required reviews and release gates pass.
2. **Public product reliability:** audit all native commands against one capability/consent policy; close receipt coverage gaps; validate mobile and GUI navigation; publish verified installer artifacts and a truthful support matrix.
3. **Public local-first differentiator:** implement measurable resource/energy telemetry and safe capability-aware routing; then deliver the local reminder/agenda MVP. Add provider adapters only with independent consent and minimal read-only scopes.
4. **Enterprise trust foundation:** integrate the merged signed device-trust registry and PostgreSQL migrations into a customer-hosted authenticated service; implement protected key/anchor providers and atomic acquire/renew/release/cancel lease plus audit transactions; add adversarial tenant/replay/revocation tests.
5. **Enterprise operator experience:** build read-only inventory/health/fleet views and selected integration status; then customer-selectable install profiles with tested sizing and licensing disclosure.
6. **Enterprise detection and response:** read-only sensor and event ingestion, deterministic explainable correlation and analyst alerts; only later consider separately reviewed approval-gated reversible actions.
7. **Benchmark continuously:** compare install friction, idle CPU/RAM, model latency/quality, energy per task, crash rate, permission clarity and operator time-to-triage against the relevant open-source and commercial alternatives. Publish methodology and raw measurements; do not infer superiority from feature lists.

### Audit limitations

This review inspected the two repository backlogs, current pull-request metadata, selected project-status documentation, prior UJANI decisions available in conversation context, and public product documentation. It did not inspect every line of every branch, run a local build, perform penetration testing, validate a physical installer, measure power consumption, or independently verify competitor marketing claims. The label “success” on GitHub Actions means that workflow completed successfully for the inspected commit; it does not close manual, legal, security or hardware gates.


## 2026-11-10 verification delta

- Public PR #176 (Android-only debug-symbol reduction) merged as `e5764c6162d775ed759ceeac3f082d6f8475f988`. Its exact-head CI, CodeQL, dependency audit, native Windows/macOS/Linux tests and Android ARM64 compatibility job passed. The uploaded Actions artifact archive measured 16,718,772 bytes; this is not the raw APK file size. The workflow did not report the uncompressed APK size, so no percentage package-size reduction is verified. PR #178 adds a raw APK size report; this is not a signed release or physical-device acceptance result.
- The Public main-branch CI and CodeQL workflows were still running against the post-merge SHA when this status was recorded. Do not mark the post-merge main validation green until those runs finish successfully.
- Enterprise main SHA `d114556386d21583e28ad2b7d8e77666c7c23fbc` had passing CI, CodeQL and Rust dependency-audit runs. This verifies the policy workspace on hosted runners, not production control-plane or real worker behavior.
- The repositories had no open pull requests at the time of the check. This means there are no PRs awaiting merge; it does not mean the product backlog or release gates are complete.
- Android is the current Public AI mobile target. iOS is outside the current implementation scope.


## 2026-11-12 verified integration delta

**Public AI main baseline audited for this update:** `bf258605870017970044ea90d07cd19e3c855668`. Merged PR #214 adds validated-format, runtime-reported Ollama model digest evidence and a bounded advisory memory estimate; PR #217 adds a fail-closed release-evidence checklist (it validates evidence labels, not report authenticity); PR #218 adds the native task-contract v1 verifier and shared vectors; PR #221 corrects the execution-receipt coverage record. These merges do not establish cryptographic model attestation, a signed release or a complete manual test pass.

**Enterprise AI main:** `c9395194c9afc95866496e4219ce681af7af43a0`. Merged PR #130 adds a read-only inventory metadata contract/view, #134 adds shared contract/schema validators, #136 adds PostgreSQL replay/sequence storage, #140 adds the native task-contract verifier, #141 corrects the operator readiness record, and #142 adds monotonic telemetry persistence/admission tests, runtime tenant-registry privilege restrictions and the durable device-trust registry with an external-anchor interface. Exact-head checks passed for these PRs before merge; no post-merge main workflow run was returned in this audit.

**Still not complete:** Public PR #215 (sensitive-action receipt and device-identity hardening, including cross-process identity mutation locking), #223 (runtime-metadata local-model preflight) and #225 (bounded Ubuntu dependency retries plus the Windows junction regression) remain under exact-head validation. No physical-device credential-vault tests are claimed for #215. Public OS-enforced application sandboxing, complete PDF-worker validation, signed reproducible installers, physical-device installation tests and independent security testing remain release gates. Enterprise still lacks the production OIDC/HTTP service, SQLx repository, authenticated device agent, production external-anchor provider, atomic lease/audit transaction service and worker isolation. No remote execution or autonomous containment is enabled.


## 2026-11-16 current implementation and PR reconciliation

**Current main baselines after the shared-catalogue merges:** Public AI `ccc9c1c707dfec822fbc4833e73b026f4404d028`; Enterprise AI `db9a5ea38c3986ca9367742fb59c3227f704d0de`. Historical snapshots above are retained for audit context; this section is the current planning record. These SHAs are the merge commits for the latest inspected catalogue PRs; re-check the default-branch tip before tagging a release.

### Public AI

- Merged PR #295 (`4ecffa0a02f049ba07d39dfd0bc1b916b5a4e5ec`): selected-folder operations use shared lazy capability/policy preflight before the native picker, with denial/cancellation regression tests and metadata-only cache receipts. Exact-head CI, Rust tests/lints, frontend build, CodeQL, dependency audits and Linux/macOS/Windows native checks and installers passed. Superseded PR #290 is closed.
- Merged PR #297 (`c626e91cb43dfb72089b25d04f5c56c08d079b29`): opt-in, default-off local reminders use AES-256-GCM payloads, an OS-vault key, bounded local CRUD and no cloud/network transfer. Exact-head Rust tests, TypeScript build, CodeQL, dependency audits, Android compile, cross-platform native tests and installer builds passed. This remains a one-time reminder MVP: no OS notifications, background delivery, recurrence, calendar/email/Zoom connectors or device wake.
- Merged PR #299 updates cross-repository project-status and integration-contract documentation.
- **PR #300 — merged to Public main** (`ccc9c1c707dfec822fbc4833e73b026f4404d028`): pins the shared descriptive model catalogue in the contract manifest and verifies the desktop catalogue mirror byte-for-byte. The paired Enterprise PR #225 is also merged. The descriptive catalogue is identical in both repositories; product/tenant-specific approval and distribution policy remains outside the shared catalogue.
- **PR #298 — open, ready for review** (head `b4c83244a47ebec8218c47d573f6aa84f18779a6`): exact approved self-evolution patch-bundle validation is present as a side-effect-free validator only. Exact-head CI, CodeQL, Android compatibility and installer-size checks passed; the unsigned preview-installer workflow was skipped. It performs no filesystem reads/writes, staging, code execution, installation or rollback. Durable approval-nonce consumption exists; staging and application remain blocked.
- **Remaining Public release gates:** full native-command authorization/consent denial coverage; selected-folder and cache tests on Windows/macOS/Linux; encrypted-history key rotation/backup/recovery; exact model artifact and storage preflight; PDF worker isolation; signed reproducible installers and updater rollback; physical-device install/upgrade/uninstall and minimum-OS verification; independent security review. Worker/Partner remain previews, not distributed compute. The local reminder MVP is not a background planner or notification service.

### Enterprise AI

- Merged PR #225 (`db9a5ea38c3986ca9367742fb59c3227f704d0de`) aligns the Enterprise catalogue and operator-console mirror byte-for-byte with Public's canonical descriptive catalogue, pins the catalogue hash in the shared-contract manifest and validates the mirror. Tenant approvals, distribution rules and owner consent remain separate.
- Merged PR #216 documents the developer/operator evaluation boundary; #217 hardens read-only network-log ingestion checkpoints; #218 adds fail-closed, side-effect-free multi-resource admission policy; #220 records the boundary between policy helpers and production telemetry/dispatch; #221 improves read-only operator inventory evidence freshness. These changes do not collect live hardware telemetry, authenticate measurements, reserve capacity or dispatch work.
- **PR #215 and PR #222 are closed as superseded** by replacement PRs #224 and #223 respectively.
- **PR #223 — open, ready for review** (head `a79c385539486cd95873ef0d170e9e7a362eb0ff`): authenticated telemetry intake boundary now has signed durable-registry tests for successful consented requests, request-binding mismatch, missing owner consent, repeated non-consuming verification and trust-store HTTP 503 handling. Exact-head Linux, macOS and PostgreSQL checks passed; Windows policy tests, CodeQL and RustSec were still running at the time of this record. It does not add a production HTTP listener, authenticated device agent or durable SQL intake transaction.
- **PR #224 — open, ready for review** (head `df8993c390fb5ae1007bac0d542937a6c5e73e4a`): PostgreSQL reference lease-admission migration and bounded concurrency/rollback/least-privilege tests. Exact-head Linux, Windows, macOS, PostgreSQL tenant-isolation/concurrency, CodeQL, RustSec and security-watch checks passed. It is not a production authenticated service or final database/topology decision.
- **PR #226 — open, ready for review** (head `c5adae2cebd29229ce21ab232505c2dd8aa0c966`): provider-backed encrypted-ledger interface and snapshot envelope. The test provider is explicitly test-only; no production KMS/HSM/TPM provider is wired in. Read-only snapshot handling and key-rotation behavior received review fixes. Exact-head CI, CodeQL and RustSec were still running at the time of this record.
- **PR #227 — open, ready for review** (head `e9742aad204d50c3f895dcaf504500ad189145d2`): operator-console demo data is synthetic, visibly labelled, read-only and separate from the live authenticated API. Refresh clears demo records before requesting live data; demo rendering itself makes no service call. Exact-head CI, CodeQL and RustSec were still running at the time of this record. This is not a live fleet dashboard.
- **Remaining Enterprise release gates:** production OIDC/HTTP intake and tenant isolation; authenticated device-agent identity/telemetry; binding each multi-resource measurement to the enrolled device and monotonic sequence; protected KMS/HSM/TPM key custody; independent monotonic anti-rollback anchor; atomic task-replay/dual-consent/policy/lease/audit transaction; lease renewal/release/recovery; live operator API and revocation workflows; OS-enforced isolation and worker-side quotas; result verification and signed receipts; independent security review. Remote worker dispatch and automated containment remain disabled.

### Cross-repository boundary

The shared task-contract v1 schema, shared model catalogue and native verifiers are protocol foundations only. Public peer protocol v2 remains distinct. Neither product may silently start the other, share prompts/files by default, or infer trust/consent from model inventory. Before any future remote task, the authenticated service must bind sender, organisation, worker, exact task digest, user/admin/worker-owner consent, replay claims and a durable lease in one fail-closed admission flow; the worker must independently revalidate policy and enforce OS quotas. No remote execution is enabled by this update.
