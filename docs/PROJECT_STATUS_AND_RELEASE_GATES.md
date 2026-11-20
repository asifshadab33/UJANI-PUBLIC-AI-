# UJANI project status and release gates

**Project status: pre-alpha / prototype.** This file consolidates the verified implementation boundary and outstanding work across UJANI Public AI and UJANI Enterprise AI as of 18 November 2026. It is a project planning record, not a security certification.

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
- **PR #229 — merged to Enterprise main** (`10da82ada220af54bc9e374c8612dfb9fd693527`): atomic lease admission now persists the exact approval reference on the durable lease row, so audit evidence can link the admitted lease to its authorizing approval without relying on a transient replay record.
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

The fail-closed release-readiness helper now requires separate evidence for every item below. A passing unit test or successful compile is not a substitute for the underlying evidence.

- [ ] Rust core tests and clippy on locked dependencies.
- [ ] Frontend and Rust dependency audits pass with no unresolved release-blocking advisories.
- [ ] OS-enforced sandbox controls are implemented and tested on every supported desktop OS.
- [ ] PDF worker containment is independently verified on every supported desktop OS.
- [ ] Command-by-command IPC authorization and side-effect ordering audit is complete.
- [ ] Model download transport, host validation, progress bounds, cancellation and artifact verification tests pass.
- [ ] History key rotation, backup/restore and credential-vault recovery tests pass.
- [ ] Signed release artifacts, SBOM/provenance and updater signature/version binding are verified.
- [ ] Update rollback, crash recovery and backup/restore drills pass.
- [ ] Required parser fuzz campaigns have completed with retained evidence.
- [ ] Independent external security review/penetration test is complete.
- [ ] Independent external test report is complete.
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

**Current main baselines after the 2026-11-16 security and consent-test merges:** Public AI `a2d986b90db992e4b1ef0fce1f6794a163a74873`; Enterprise AI `c9d4bbfc7481eaf97fb7d45e3c56477a90124bed`. Historical snapshots above are retained for audit context; this section is the current planning record. These are the latest verified merge commits in this pass; re-check the default-branch tip before tagging a release.

### Public AI

- Merged PR #295 (`4ecffa0a02f049ba07d39dfd0bc1b916b5a4e5ec`): selected-folder operations use shared lazy capability/policy preflight before the native picker, with denial/cancellation regression tests and metadata-only cache receipts. Exact-head CI, Rust tests/lints, frontend build, CodeQL, dependency audits and Linux/macOS/Windows native checks and installers passed. Superseded PR #290 is closed.
- Merged PR #297 (`c626e91cb43dfb72089b25d04f5c56c08d079b29`): opt-in, default-off local reminders use AES-256-GCM payloads, an OS-vault key, bounded local CRUD and no cloud/network transfer. Exact-head Rust tests, TypeScript build, CodeQL, dependency audits, Android compile, cross-platform native tests and installer builds passed. This remains a one-time reminder MVP: no OS notifications, background delivery, recurrence, calendar/email/Zoom connectors or device wake.
- Merged PR #299 updates cross-repository project-status and integration-contract documentation.
- **PR #300 — merged to Public main** (`ccc9c1c707dfec822fbc4833e73b026f4404d028`): pins the shared descriptive model catalogue in the contract manifest and verifies the desktop catalogue mirror byte-for-byte. The paired Enterprise PR #225 is also merged. The descriptive catalogue is identical in both repositories; product/tenant-specific approval and distribution policy remains outside the shared catalogue.
- **PR #306 — merged to Public main** (`969d3b861f22c96d12cc4103cea0e39e8fafc046`): document/PDF native IPC now performs shared operation-policy preflight before capability discovery and before reading/parsing document input. Regression tests guard ordering and late validation. This does not establish PDF worker filesystem/network isolation; the separate PDF isolation gate remains a release blocker.
- **PR #298 — merged** (merge commit `db0fe427c38858114c466851af5c5d20ea948984`): exact approved self-evolution patch-bundle validation is present as a side-effect-free validator only. Exact-head CI, CodeQL, Android compatibility and installer-size checks passed; the unsigned preview-installer workflow was skipped. It performs no filesystem reads/writes, staging, code execution, installation or rollback. Durable approval-nonce consumption exists; staging and application remain blocked.
- **PR #302 — merged** (merge commit d61287b2376c8cd232e4a5d8879e6825180edbdf): records the matching draft task/runtime schemas and shared descriptive model catalogue; final protocol approval and per-artifact verification remain pending.
- **PR #303 — merged** (merge commit d1ac7899b546f11e9f5bf184eb6998d9ce1a2da2): aligns Worker/Partner descriptions with agreed semantics and keeps both preview-only; Fortress still lacks OS-wide network isolation.
- **PR #305 — merged** (merge commit 76eaa195a1a1455943d6783dfe789379f319ed8e): history IPC validates bounds before receipt work, emits content-free metadata receipts, audits list-time migration/retention and refuses to replace a missing OS-vault key when encrypted history may exist. Backup/restore and key recovery remain open release gates.
- **PR #309 — merged** (merge commit 658cf03303bbc720358a5e71f26a9ce35156bfe3): reconciles the shared draft schemas and model-catalogue status without treating schemas as authenticated execution.
- **PR #312 — merged** (`d23adbf39ee7c01acef4418094e55ff8c1d0952e`): reconciles the current cross-repository implementation and release-gate record; its historical baseline has been superseded by the newer merge SHAs recorded above.
- **PR #313 — merged** (`a2d986b90db992e4b1ef0fce1f6794a163a74873`): PDF summarisation and folder PDF search now require worker resource limits, filesystem isolation and network isolation to be verified; Unknown/Unavailable fails closed. This is a release gate, not a claim that the sandbox is already implemented or verified on every platform.
- **PR #314 — merged** (`d2f9c0d1023b0d6cde3919b21cb9f0d3b1c88edb`): adds regression coverage for the loopback-only Ollama client, disabled redirects/proxy discovery and connect/total timeouts. The Ollama daemon's own registry traffic remains a separate trust boundary.
- **Remaining Public release gates:** full native-command authorization/consent denial coverage; selected-folder and cache tests on Windows/macOS/Linux; encrypted-history key rotation/backup/recovery; exact model artifact and storage preflight; PDF worker isolation; signed reproducible installers and updater rollback; physical-device install/upgrade/uninstall and minimum-OS verification; independent security review. Worker remains preview-only for approved cloud plans with local compute; Partner remains preview-only for bounded local/cloud iteration. Fortress OS-wide network isolation is not yet enforced. The local reminder MVP is not a background planner or notification service.

### Enterprise AI

- Merged PR #225 (`db9a5ea38c3986ca9367742fb59c3227f704d0de`) aligns the Enterprise catalogue and operator-console mirror byte-for-byte with Public's canonical descriptive catalogue, pins the catalogue hash in the shared-contract manifest and validates the mirror. Tenant approvals, distribution rules and owner consent remain separate.
- Merged PR #216 documents the developer/operator evaluation boundary; #217 hardens read-only network-log ingestion checkpoints; #218 adds fail-closed, side-effect-free multi-resource admission policy; #220 records the boundary between policy helpers and production telemetry/dispatch; #221 improves read-only operator inventory evidence freshness. These changes do not collect live hardware telemetry, authenticate measurements, reserve capacity or dispatch work.
- **PR #223 — merged** (merge commit 6907d0f47049b4bc41afe78c0eda54d928315b72): strengthens authenticated telemetry replay/persistence behavior and fail-closed handling. This remains an admission primitive, not a production HTTP listener or deployed device agent.
- **PR #224 — merged** (merge commit f27a8109f3dcc792b35ec47631883d4949f05885): rebases and merges PostgreSQL device-lease admission. It is a database transaction primitive, not a live authenticated service.
- **PR #226 — merged** (merge commit 80347ffabde83bcc1f1a0af46e65ac3bb2865f11): adds the provider-backed encrypted-ledger snapshot envelope. The test provider is test-only; production KMS/HSM/TPM/OS-vault adapters and external anti-rollback anchoring remain pending.
- **PR #228 — merged** (merge commit 761701efef6464e49f24f7762d8664811606ff5): composes telemetry acceptance, replay claims, lease reservation and audit in one PostgreSQL transaction with rollback tests.
- **PR #229 — merged** (merge commit 10da82ada220af54bc9e374c8612dfb9fd693527): persists the exact administrator approval reference on admitted lease rows.
- **PR #230 — merged** (merge commit 773d86bd50befebf14d5dbcbfe9b00f8aa64d238): stores immutable request digests and exposes bounded tenant-RLS-preserving lease audit evidence; owner-consent pause/withdrawal revokes reserved/active leases and appends audit events transactionally. This does not close the gap to a production HTTP/IdP control plane.
- **Enterprise PR #231 — merged** (`c9d4bbfc7481eaf97fb7d45e3c56477a90124bed`): adds PostgreSQL regression coverage for consent grant, pause-triggered lease revocation and the invariant that a later grant never reactivates an old lease. Exact-head CI, CodeQL, Rust dependency audit and security-watch checks passed.
- **PR #227 — merged** (merge commit `c3b05afe455af302d91a882ee236c7e24cee00e8`): operator-console demo data is synthetic, visibly labelled, read-only and separate from the live authenticated API. Refresh clears demo records synchronously before awaiting the live API; a Node VM transition test verifies that synthetic device/model/workload/approval rows are gone while the request is still pending. Exact-head CI, CodeQL, RustSec and security-watch checks passed. This is not a live fleet dashboard; authenticated live inventory and control-plane APIs remain pending.
- **Remaining Enterprise release gates:** production OIDC/HTTP service and tenant-scoped SQLx repository; authenticated device-agent identity/telemetry ingestion; production protected key-provider and external monotonic anti-rollback adapters; reconciliation of the durable registry/ledger path with PostgreSQL admission and audit; authenticated lease renewal/release/cancellation/recovery; live operator API and revocation workflows; receiver-side OS isolation and quotas; result verification and signed receipts; operational backup/restore drills and independent security review. The PostgreSQL atomic admission/audit path now exists, but is not yet connected to a production authenticated HTTP service. Remote worker dispatch and automated containment remain disabled.

### Public roadmap reconciliation PRs

PRs #302 and #303 are merged and recorded in the Public AI section above. The remaining protocol and artifact-verification gates are unchanged.

### Cross-repository boundary

The shared task-contract v1 schema, model-runtime-inventory schema, model catalogue, test vectors and hash-pinned manifests are draft protocol foundations only; production semantics and end-to-end interoperability approval remain pending. Public AI's Worker and Partner modes are distinct from Enterprise's organisation-managed worker fleet: Worker means approved cloud plans/delegations while this device performs allow-listed local compute; Partner means iterative, bounded local/cloud collaboration. Both Public modes remain preview-only. Fortress blocks UJANI cloud requests today, but OS-wide network isolation is not yet enforced. Neither product may silently start the other, share prompts/files by default, or infer trust/consent from model inventory. Before any Enterprise remote task, the authenticated service must bind sender, organisation, worker, exact task digest, administrator and worker-owner consent, replay claims and a durable lease in one fail-closed admission flow; the worker must independently revalidate policy and enforce OS quotas. No remote execution is enabled by these changes.


---

# UJANI Master Delivery Plan — consolidated cross-project baseline

**Revision:** 20 November 2026  
**Purpose:** One coordinated requirements, implementation, verification and release plan for UJANI Public AI and UJANI Enterprise AI. This section is the cross-project planning index. The two repositories remain separately governed; their detailed handbooks, roadmaps, threat models, schemas, command inventories and release gates remain the implementation references. Do not create a competing roadmap or mark an item complete without evidence from the relevant repository and runtime.

## 1. Product outcome and non-negotiable principles

UJANI exists to make useful AI work happen on computing resources already available to the user or organisation, reducing avoidable dependence on central AI data centres. It must remain practical, lightweight, transparent and usable—not merely a collection of policy contracts.

1. **Local-first and resource-aware:** prefer deterministic OS tools and suitable local models before external inference. Use other devices only when explicitly authorised and demonstrably appropriate.
2. **Consent before side effects:** no silent model download, runtime start, external transfer, device participation, background work, destructive action or privileged OS change.
3. **OS-first, lightweight design:** reuse native search, file, calendar, notification, credential-vault and security capabilities where appropriate. Do not reimplement a full OS service or bundle heavyweight engines unnecessarily.
4. **Truthful capability reporting:** distinguish detected, runtime-verified, catalogue-claimed, estimated, stale and unknown. Unknown is never silently upgraded to compatible, healthy or schedulable.
5. **Security at enforcement boundaries:** UI state, model output, schema validation and policy previews are not authorization. Enforce permissions in native/service/receiver code before side effects.
6. **Observable and reversible operation:** show execution location, resources, progress, approvals, provenance, errors and cancellation. Provide recovery, revocation and deletion paths.
7. **No premature distributed execution:** Enterprise remote work remains disabled until authenticated identity, owner consent, tenant isolation, encrypted transport, receiver-side policy, OS isolation, quotas, cancellation, audit and independent review are proven.
8. **Evidence-based energy claims:** never wake sleeping devices. Compare equivalent completed work and include device, network, retry, cooling and transfer overhead. Publish measured and estimated inputs separately.

## 2. Product scope by application

### 2.1 UJANI Public AI — personal desktop assistant

**Setup and everyday usability**
- A clean first-run experience that works without Ollama or any model installed; Basic, Advanced, Choose models and Not now paths; accessible settings, understandable errors and clear capability states.
- Small-model-first recommendations for ordinary hardware. Show each model's purpose, suitable tasks, limitations, exact artifact/source/digest, quantisation, download and installed size, RAM/VRAM guidance, runtime/platform support and licence before an optional install.
- Make installed, available-to-download, unsupported and unknown states distinct. Never silently switch models or fall back to cloud.

**Native OS-powered productivity**
- User-scoped file/folder search, metadata search, document-content search, PDF search, duplicate detection, large-file discovery, and safe sorting/organisation of documents and images.
- Locate videos and media using OS metadata/indexing where available. Distinguish locating a video from expensive scene understanding, transcription or summarisation.
- Open results in the user's chosen OS application. Respect selected roots, OS permissions, symlinks/reparse points, exclusions and bounded traversal. No unrestricted whole-disk indexing by default.
- Basic device/system questions and lightweight deterministic helpers should work without a model where possible.

**Local AI and document workflows**
- Local chat, summarisation, translation, rewriting, explanation and document Q&A through typed runtime adapters.
- Bounded PDF/text processing, image metadata and, only after separate resource/privacy review, optional OCR or vision. Scanned-PDF OCR, full image understanding, audio transcription and video scene understanding are not to be claimed until implemented and tested.
- Keep advanced media pipelines optional; do not burden the default installer with large models or duplicate mature OS/media tools.

**Personal intelligence and integrations**
- An optional daily planner for user tasks, commitments, reminders, renewals, birthdays and follow-ups.
- Consent-based connectors to supported calendars, Outlook, Zoom meeting metadata, notes and other documented provider APIs. Read/write scopes are separate; show what is accessed, last sync, provenance and retention; allow revocation.
- Prefer provider-native schedulers and OS notifications when suitable. No hidden mailbox ingestion, screen surveillance, background agent loop or automatic sending/calendar mutation.

**Execution, privacy and reliability**
- Preserve Fortress, Smart, Worker and Partner as distinct, honestly described modes. Fortress must not be called OS-isolated until OS enforcement is verified. Worker/Partner remain unavailable until their separate secure transport, consent, dispatch, quota and receiver-side enforcement gates pass.
- Complete universal native IPC authorization/denial-before-side-effect coverage; typed operations; cancellation, timeouts, bounded output, task receipts and clear execution provenance.
- Protect history, reminders and search caches; opt-in persistence, encrypted sensitive fields, key lifecycle, retention, export/deletion and recovery. Metadata/plaintext boundaries must be disclosed.
- Fix freezes, hangs, stale task state, memory pressure, model/runtime failures and UI responsiveness across supported platforms.

### 2.2 UJANI Enterprise AI — organisation-controlled resource coordination

**Real control plane and trust**
- Deployable authenticated HTTP/API service, organisation identity-provider integration (OIDC/SAML as selected), tenant-derived server identity, RBAC, step-up/dual approval where appropriate, CSRF/origin/rate/body protections and durable fail-closed audit.
- Durable tenant-scoped repositories and migration/backup/restore procedures. Existing Rust policy, PostgreSQL migrations, signed claims and file-backed ledgers are foundations—not a live service by themselves.
- Device lifecycle: invitation/enrolment, proof of possession, tenant binding, owner consent, key custody/rotation/revocation, lost-device handling, monotonic generations, replay/expiry and audit.
- Production KMS/HSM/TPM or protected host key-provider integration and external anti-rollback anchor; reconcile file-backed registry/ledger with the PostgreSQL transaction path before relying on either for live admission.

**Endpoint inventory and resource coordination**
- A least-privilege, visible endpoint agent for explicitly enrolled company-owned or owner-authorised devices; read-only inventory/telemetry first.
- Authenticated fresh CPU/RAM/GPU/NPU/runtime, storage, power, thermal, interactive-use and health evidence. Unknown/stale telemetry remains unknown and cannot admit work.
- Resource-aware placement, quotas, priorities, maintenance windows, fairness, atomic reservations, durable leases, queue/back-pressure, cancellation, expiry and crash recovery.
- Model catalogue parity with Public, but administrator-approved model/runtime policy, controlled distribution, per-device fit evidence and user visibility. Never treat fleet RAM as one machine's model memory.
- Worker-side OS isolation, least privilege, network/filesystem/process restrictions, resource enforcement, signed assignments, independent policy revalidation and signed result receipts before any dispatch.

**Operator and organisation experience**
- Live tenant-scoped operator dashboard: device enrollment/trust/heartbeat/freshness, capacity, alerts, model inventory, approvals, leases/workloads, audit search/export, maintenance and staged updates. Synthetic demo data must always be labelled and never masquerade as live fleet data.
- Optional Enterprise Intelligence Planner coordinating approved Outlook/Microsoft Graph, Google Workspace, Zoom metadata, CalDAV, project/task and compatible MCP/workflow systems. Tenant policy and individual user consent are independent; personal reminders are not automatically manager-visible.
- Enterprise Intelligence Orchestration Layer as a lightweight customer-controlled coordination layer over existing products—not a replacement for them. Reuse the capability registry and connector contracts; do not create parallel plugin, scheduler, consent or event frameworks.
- Optional older-network/security integrations for customer-selected firewall, endpoint protection, Suricata, Zeek and Malcolm deployments. Infrastructure Shield/Security Centre is secondary to the compute-utilisation mission and must not become a proprietary-tool replacement.

**Collective Defence — separately gated optional capability**
- Default off; independent organisation and individual opt-in; clear event categories, purpose, recipients, retention and revocation.
- Read-only bounded endpoint signals first; authenticated tenant ingestion, privacy minimisation, provenance, confidence, replay/poisoning protection, deterministic explainable correlation and analyst workflows.
- No file contents, credentials, private conversations, unrestricted logs, remote shell, autonomous destructive response or fleet-wide containment. Any future reversible response needs separate approval, least privilege, rollback and independent review.

### 2.3 Shared Public/Enterprise foundations

- Versioned task, capability, model-runtime inventory, catalogue, status, receipt and evidence contracts; compatibility policy and end-to-end fixtures tested in both repositories.
- Adaptive Capability Architecture (ACA): capability discovery with provenance/freshness, compatibility preflight, typed adapters, policy/permission metadata, lifecycle (enable/update/remove), rollback and recovery. Discovery is not authorization; a catalogue entry is not proof of runtime capability.
- Common capability vocabulary and execution provenance without forcing a shared code library where independent ownership is safer.
- Contract versioning, schema validation, cross-language vectors, migration policy and explicit deprecation rules.
- Shared benchmark methodology for latency, quality, memory/VRAM, energy, reliability and data movement; measured, estimated and unknown values remain distinct.

## 3. Small-batch delivery programme

Batches are small, reviewable PRs with explicit acceptance tests. Public and Enterprise streams should progress in parallel whenever independent. Start discovery, test authoring, documentation and isolated implementation concurrently; respect dependencies before enabling integrated behaviour. Do not combine unrelated work into a single large PR.

| Batch | Public AI workstream | Enterprise AI workstream | Exit evidence |
|---|---|---|---|
| **0 — reconcile and baseline** | Reconcile roadmap, command inventory, claims ledger, open/closed PRs, CI and manual smoke matrix; review reminder PR #379. | Reconcile roadmap, issues, policy/service contracts, migrations, trust/ledger state and open/closed PRs. | One requirement register; exact main SHAs; duplicate/conflict list; baseline CI and explicit NOT RUN physical tests. |
| **1 — stability and security boundaries** | Finish command-by-command IPC authorization and denial-before-side-effect audit; filesystem/path/TOCTOU review; PDF worker limits/isolation; cancellation, cache/history/reminder protection; freeze/hang regressions. | Threat-model service boundary; reconcile registry/ledger/PostgreSQL admission and audit; harden key lifecycle, tenant isolation, revocation, replay and crash recovery contracts. | Negative tests for unauthorized, malformed, stale, revoked, oversized, concurrent and cancelled operations; no weakened security gates. |
| **2 — ACA, models and identity** | Complete hardware/runtime evidence and small-model-first advisor; model catalogue artifact provenance, licence, exact sizes, fit guidance and optional install lifecycle. | Complete trusted device identity/enrolment/revocation/key-provider integration plan and implement safe production service prerequisites; approved model governance and inventory contract. | Cross-platform compatibility tests; model lifecycle tests; identity/replay/rotation/revocation tests; shared schema vectors pass in both repos. |
| **3 — native tools and authenticated inventory** | Improve OS-first search, folder/file/PDF metadata and content workflows, duplicate/large-file discovery, selected-root permission and open-in-OS flows. | Implement authenticated read-only device inventory/telemetry intake and tenant-scoped persistence; freshness, sequence and consent enforced server-side. | Real application-path tests; tenant isolation, stale telemetry and permission tests; no unauthenticated live data. |
| **4 — documents and operator experience** | Harden document/PDF workflows, supported image metadata/OCR boundaries, progress, cancellation, large-file and recovery tests; improve accessibility and onboarding. | Connect live read-only operator API/dashboard to authenticated repository; device detail, freshness, alerts, model inventory and audit visibility. | Native UI integration tests; synthetic/live separation; accessible workflows; PostgreSQL integration tests on exact migrations. |
| **5 — planners and connectors** | Deliver personal planner, reminders and consent-based calendar/Outlook/Zoom/notes connectors using OS/provider-native services where possible. | Deliver tenant-governed Enterprise planner and read-only connectors; separate tenant policy from individual consent; operator workflows and provider health. | Consent/revocation, scope, timezone/DST, pagination, rate-limit, prompt-injection and data-retention tests; no implicit writes. |
| **6 — safe task lifecycle and orchestration** | Complete typed task graph execution only for allow-listed adapters; mode routing, receipts, verification, cancellation and local resource admission. Worker/Partner stay gated. | Complete durable queue, atomic admission/leases, quotas, fairness, cancellation, recovery and worker-side enforcement. Dispatch remains disabled until all receiver-side and isolation gates pass. | Restart/concurrency/replay/revocation tests; signed provenance; resource limits; failure recovery; adversarial tenant tests. |
| **7 — optional advanced domains** | Evaluate optional OCR/vision/audio/video understanding, extra runtimes/mobile adapters and SDK/plugins only after cost, privacy and resource review. | Implement opt-in read-only Collective Defence, explainable correlation, Security Centre/Infrastructure Shield and older-network integrations in independently gated slices. | Explicit opt-in/revocation; privacy minimisation; lab corpus and false-positive tests; no autonomous destructive response. |
| **8 — performance and release** | Cross-platform native builds, signed installers/updater, rollback, SBOM/provenance, accessibility, manual install/launch, model-absent and low-resource tests, independent security review. | Customer-hosted deployment, protected keys, migration/backup/restore, agent rollout/rollback, operational monitoring, lab fleet tests, energy benchmarks and independent security review. | Release checklist passes with exact artifacts, supported OS baselines, test records and independent review. No release claim based on CI alone. |

## 4. Cross-project dependency and parallelism rules

- Batches 0–2 establish trusted boundaries and contracts. Independent Public UI/search work and Enterprise service/DB work may proceed in parallel, but no live data or dispatch may bypass identity, consent or tenant isolation.
- Public native OS tools do not depend on Enterprise. Enterprise read-only service, identity and inventory can be developed in parallel with Public model/runtime and OS-tool work.
- Planner connectors can be prototyped against read-only fixtures while identity/consent services are built, but live-account integration is gated on least-privilege OAuth, revocation and retention.
- Enterprise dashboard layout and accessibility can progress with clearly labelled synthetic fixtures; live mode remains unavailable until the authenticated service is connected.
- Collective Defence schemas, sensor tests and deterministic correlation can progress in isolation; production collection/ingestion and response remain gated.
- Shared schemas are versioned and tested in both repos. Do not merge repositories or force a shared crate merely to remove duplication.
- Every PR must have one bounded objective, tests for both success and denial/failure paths, docs/claims updates and exact-head CI. Merge only after review, required checks and unchanged expected head SHA.

## 5. Requirement status and evidence standard

Use these distinct states in the master register:
- **Implemented:** code is present on the target branch.
- **Implemented-unverified:** code exists but integration, platform or runtime evidence is missing.
- **Partial:** only part of the user outcome works.
- **Missing:** no adequate implementation.
- **Intentionally disabled:** deliberately blocked by a safety or dependency gate.
- **Deferred:** deliberately postponed with rationale and revisit condition.

Each requirement record must include: stable ID; application/shared scope; user outcome; source conversation/decision; repository paths; issue/PR/commit; dependencies; security/privacy/performance/usability risks; acceptance criteria; test evidence; platform/runtime evidence; release milestone; status and last verification date.

A feature is complete only when it works through the actual application path, has positive and negative regression coverage, exposes understandable progress/errors/cancellation, preserves consent and privacy, and has the relevant platform evidence. CI success alone is not runtime verification. Physical device tests, installer launch, accessibility, independent security review and energy measurement must be separately recorded as PASS, FAIL, BLOCKED or NOT RUN.

## 6. Release gates and non-goals

- Public and Enterprise remain pre-alpha/controlled-preview until their respective release gates are satisfied.
- No silent compute, sleeping-device wake, unapproved data transfer, unrestricted shell, model-generated code execution, stealth monitoring or automatic destructive response.
- No Public peer execution or Enterprise fleet dispatch until authenticated transport, receiver-side authorization, OS isolation, quotas, cancellation, signed receipts, recovery and independent review pass.
- No claims of full Fortress OS isolation, production identity, live fleet management, image/audio/video understanding, GPU/NPU support, measured energy savings or compliance without evidence.
- Do not bundle large models or heavy third-party engines in the default Public installer. Prefer optional adapters and customer-selected Enterprise integrations.
- Preserve open-source licensing, attribution, reproducible dependencies, SBOM, vulnerability response and clear legal/operational disclaimers.

## 7. Operating rule for future development

This document is the single cross-project planning index. Update it when a requirement is added, changed, implemented, deferred or blocked. Keep detailed technical truth in each repository's existing handbook, roadmap, claims ledger, command inventory, threat model and release-gate documents, and link the evidence here. At each development checkpoint: refresh both main branches and open PRs; reconcile requirements; work in small independent batches in parallel; run exact-head CI; review diffs; merge only verified safe changes; then run integration and manual validation. Never claim unattended/background progress between conversations.
