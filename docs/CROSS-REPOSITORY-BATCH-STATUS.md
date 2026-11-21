# UJANI shared implementation backlog and evidence register

**Evidence snapshot: 21 November 2026 (post-merge refresh).** Canonical cross-repository priority order for the coordinated 14-day programme. This register consolidates the live open issues, open PRs, current main-branch evidence and the existing Public/Enterprise master plans. It is a planning and evidence ledger, not a security certification or release approval.

## Product boundaries and non-negotiable rules

- **Public AI** is an independently usable, local-first personal desktop runtime. Local inference and OS-backed work are preferred; external processing requires explicit, task-scoped user consent and disclosure.
- **Enterprise AI** is an independently usable, organisation-controlled resource and fleet manager. Device-owner consent and organisation policy are separate authorities.
- The products share versioned contract semantics and test fixtures only where useful. Neither product is a runtime dependency of the other.
- No silent model switch, install, runtime start, background compute, device wake, prompt transfer, arbitrary shell, model-generated executable code, or unrestricted remote execution.
- Prefer existing OS APIs and customer-selected tools over duplicating OS services.
- A schema, test, demo, merged PR or green CI run does not prove a complete user workflow.
- Enterprise worker dispatch and automated containment remain disabled. Public Worker/Partner pathways remain gated until their end-to-end trust and consent controls are proven.

## Evidence vocabulary

- **Implemented** — source exists on the named main branch or PR head; partial implementation is labelled explicitly.
- **Unit-tested** — relevant unit/regression tests exist and have actually run successfully.
- **CI-verified (SHA)** — hosted workflow passed on the exact SHA stated; do not transfer this status to a different commit.
- **Integration-verified** — complete service-to-client or cross-component workflow exercised with real integration data.
- **Device-verified** — manually exercised on named physical devices/OS versions.
- **Blocked** — a known safety, correctness, infrastructure or validation dependency prevents acceptance.
- **Release-ready** — all applicable automated, integration, device, security, packaging and operational gates are satisfied.

Unknown is not Available. In-progress, skipped, stale, cancelled or partial checks are not passes. No item in this snapshot is marked Release-ready.

## Live repository baseline

| Repository | Default branch | Exact main SHA | Exact-SHA evidence |
|---|---|---|---|
| [Public AI](https://github.com/asifshadab33/UJANI-PUBLIC-AI-) | `main` | `baf75d0f57bd0d85e61d9df946a077f64a80feab` | [CI #44892901509](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44892901509) passed; [CodeQL #44892901490](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44892901490) passed; Rust dependency audit job passed but reports **7 allowed advisories**, so this is not a clean dependency posture |
| [Enterprise AI](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI) | `main` | `55407e777eb296241c544ea68def4d796f8c9c6e` | [CI #44861210123](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44861210123) passed; [CodeQL #44887358219](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44887358219) passed; [Rust dependency audit #44835538842](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44835538842) passed |

Recent Public main changes include host-probe offloading (#407), duplicate-key model-inventory validation (#406), batch-register refresh (#405), and merged dependency updates for sysinfo 0.40.4 (#384), Tauri API 2.12.5 (#385) and rusqlite 0.43.0 (#391). The shared evidence register was merged in #409. Recent Enterprise main includes strict duplicate-key model-inventory intake (#313), following tenant-isolation and fail-closed model-match regression work. Enterprise has one open draft PR (#314), listed below.

**Current shared-contract check:** all seven manifest-listed artifacts (catalogue, v1/v2 inventory schemas, task contract and consent/inventory test vectors) have identical Git blob SHAs in both current main branches. No breaking schema or manifest drift was found. Key blobs: v2 inventory schema `6f1d5a6dd3b70a4b5e78203a20a67b71ed35a489`; task contract `566a0d72babd58636a3ef40e55b5c44168776522`; shared manifest `5c41a57411453145f0d80a470f9985375b6e3ca4`.

## Public AI — open PR inventory

| PR | Exact head SHA | Base | Current evidence / decision |
|---|---|---|---|
| [#396 empty-folder preview](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/396) | `6f8d31d1230ba9f2d460e0debf36168df23eaf68` | `main` (base `8015146dd4d719e72ddf258903b523c87f1d088f`; main is 19 commits ahead) | Exact-head CI, CodeQL, RustSec, Windows/macOS/Linux Tauri tests, Android ARM64 compile, installer-size audit and security watch passed; unsigned preview installers were skipped. PR remains draft and currently reports `mergeable=false`. **Do not merge:** selected-root identity is not pinned during metadata-only traversal; tracked in [issue #408](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/408). Rebase and rerun exact-head checks only after handle-relative root pinning is implemented and tested. |

Merged Public PRs #378 and #379 added native confirmation for specific destructive local-state/reminder actions; #395 added recent-file preview; #384/#385/#391 merged the sysinfo, Tauri API and rusqlite updates; and #409 merged this shared evidence register. These are targeted changes, not completion of issue #120 or the shared filesystem boundary. Current main CI passing does not resolve the filesystem root-replacement issue described below.

## Shared ordered backlog — Public AI

### P0 — safety and first-release correctness

1. **Selected-root identity and metadata-only traversal — [Public #408](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/408). Status: Blocked.** The shared `safe_fs::walk_directory` still performs path-based metadata/canonicalization/enumeration. The Windows root helper validates a handle but the current walker does not retain it for enumeration. The merged recent-file finder (#395) uses this shared walker; empty-folder PR #396 has the same class of race. Implement handle-relative enumeration for each supported desktop OS, preserve metadata-only semantics, symlink/reparse rejection, bounds and cancellation, and add deterministic root-replacement tests. Do not merge #396 before this is proven. This is the highest-priority filesystem blocker.
2. **Command-by-command native IPC authorization and receipts — [Public #120](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/120). Status: Implemented (partial); Unit-tested; CI-verified on main SHA `6373073abe8c4f32d9cc5b52c840feb24f9a15b0`; acceptance incomplete.** The command-map parity guard and targeted native confirmations are present. Still review every sensitive invoke path and prove validation, capability/consent authorization, cancellation, resource bounds, receipt creation and denial-before-side-effect. The map is an audit index, not runtime proof. Prioritise filesystem/document/external-transfer, selected-folder import, history/export/cache, model pulls, updater and cloud gateway.
3. **Dependency advisories and reproducible migrations — [Public #73](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/73) and [#173](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/173). Status: Blocked / in progress.** The Rust audit has historically reported seven allowed warnings (glib, proc-macro-error, ttf-parser, zbus_names, zvariant and zvariant_derive); a successful audit workflow with allowed warnings is not a clean audit. #384 now carries a sysinfo 0.40.4 compatibility repair but exact-head CI is pending. #391's Android check failed and its base is stale. Do not suppress advisories or hand-edit lockfiles; coordinate AES-GCM, rand, sysinfo, GTK/Tauri, PDF and keyring changes with reproducible locked builds and cross-platform checks.
4. **PDF, filesystem and stability hardening — [Public #70](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/70). Status: Implemented (partial); Blocked on platform/device evidence.** PDF sidecar/resource/cancellation work and navigation fixes exist, but installed-package PDF/cancellation smoke tests and OS-enforced isolation proof remain incomplete. Screenshot capture-to-chat is not implemented. Continue malformed/oversized/encrypted/interrupted upload tests, filesystem traversal/TOCTOU and recovery tests, and UI hang/sub-link/manual smoke tests.
5. **Ollama, context and model-pull safety. Status: Implemented (partial); full review pending.** Ollama is the real loopback runtime; cancellation and local routing work exists. Verify no redirects, proxy bypass, bounded HTTP and actual request cancellation, explicit `num_ctx`, host/mode/progress bounds for model pulls, and no silent runtime start/download/cloud fallback.
6. **History/cache/export privacy. Status: Implemented (partial); full release evidence pending.** Verify AES-GCM AAD binding, metadata minimisation, key rotation/recovery, DB/file permissions, export lifecycle and cache expiry/clear behavior. Do not treat encryption unit tests as proof of platform ACLs or backup recovery.
7. **Controlled alpha and OS support — [Public #45](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/45), [#67](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/67). Status: Blocked; not Release-ready.** Hosted builds are not physical-device verification. Minimum Windows/macOS/Linux baselines, installer smoke tests, WebView2/Linux dependencies, signing/updater/rollback and manual privacy/recovery testing remain open.

### P1 — complete useful local intelligence without expanding risk

8. **Model Intelligence and real inference. Status: Implemented (partial); Unit-tested; CI-verified on main SHA `6373073abe8c4f32d9cc5b52c840feb24f9a15b0`; not Integration-verified.** Public has model-inventory v2 validation, duplicate-key rejection, host-probe offloading, conservative RAM fit and a real loopback Ollama adapter. The deterministic `model_runtime.rs` adapter remains a debug-only mock and must never be described as inference. Runtime-independent discovery, trustworthy capability/licence/provenance, storage/free-space evidence, explicit recommendations and the model-selection-to-real-inference journey remain incomplete. Unknown values must remain unknown.
9. **Focused fuzzing, dependency policy and recovery. Status: Implemented (partial); parser fuzz smoke CI-verified on the exact main SHA above.** Expand targets only for concrete parsers/contracts, enforce cargo-deny policy without suppressions, and add recovery/cleanup regression coverage.
10. **Security Watch — [Public #179](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/179). Status: Implemented (partial); acceptance not verified.** A security-sensitive change-watch workflow exists; this does not prove all runtime integrity/policy-drift/threat alert requirements. Optional verified email notifications [#183](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/183) remain deferred until the core alert lifecycle is sound.

### P2 — useful but non-blocking scope

11. **Everyday OS utilities — [Public #44](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/44) and [#46](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/46). Status: Implemented (partial); affected filesystem previews blocked by #408.** Local clock, duplicate/large-file previews and recent-file preview exist. Keep previews read-only, selected-folder scoped, bounded and native-app friendly; no silent indexing or destructive actions.
12. **Reminders/planner — [Public #83](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/83). Status: Implemented (partial); Unit-tested; CI-verified on main.** Local reminder storage and targeted destructive confirmation exist. A complete daily agenda and optional OS/app connectors remain separate work; connector writes require per-provider/scope approval.
13. **Android-only support evaluation — [Public #168](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/168). Status: Deferred pending complexity stop gate.** Do not let mobile expansion delay desktop first-release safety.

## Enterprise AI — open PR and issue inventory

- **Open PRs:** one draft — [#314 duplicate authenticated telemetry fields](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/pull/314), exact head `2ffb17cf706e41d1089f7b0c857519fbcffc9ebe`, based on current main `55407e777eb296241c544ea68def4d796f8c9c6e`.
- **Main:** `55407e777eb296241c544ea68def4d796f8c9c6e`; CI, CodeQL and Rust dependency audit passed on the stated main SHA.
- **PR #314 evidence:** all Windows/macOS/Linux policy suites and customer-hosted PostgreSQL tenant-isolation tests passed; CodeQL, RustSec and security-watch passed on exact head `2ffb17cf706e41d1089f7b0c857519fbcffc9ebe`. It adds test-only duplicate-field rejection coverage and remains a draft; it is not merged and does not implement the production HTTP host or cross-store transaction.
- **Detailed existing ledger:** [Enterprise MASTER_BACKLOG.md](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/blob/main/docs/MASTER_BACKLOG.md) and [Master Requirements & Implementation Ledger](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/blob/main/docs/MASTER-REQUIREMENTS-IMPLEMENTATION-LEDGER-2026-11-19.md). Keep those as Enterprise detail; this file is the shared priority index, not a duplicate implementation ledger.

### P0 — before any worker pilot (remote execution stays disabled)

1. **Authenticated tenant-scoped service and device agent — [Enterprise #90](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/90). Status: Blocked.** Database admission, request validation and tests exist, but the production authenticated OIDC/HTTP control plane and enrolled-device agent are not deployed.
2. **Trusted-device enrollment, revocation and authenticated lease lifecycle — [Enterprise #20](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/20). Status: Implemented (partial); Unit-tested; CI-verified on main SHA `55407e777eb296241c544ea68def4d796f8c9c6e`; production integration blocked.** Durable trust/consent/lease and atomic admission foundations exist. PR #314 adds duplicate JSON-field rejection regression coverage (exact head `2ffb17cf706e41d1089f7b0c857519fbcffc9ebe`); its full CI, CodeQL and RustSec checks passed, but it remains a draft test-only change. Prove one authoritative service lifecycle, owner opt-in AND admin policy on every admission, current-state revocation, replay/sequence/freshness checks, transactional reservation/renew/release/expiry/reconciliation and tenant isolation.
3. **Protected key custody, receipts and recovery. Status: Blocked.** Production OS/KMS/HSM/TPM key provisioning/rotation/recovery, independently protected anti-rollback, signed task/input/output receipts, crash/partition recovery, OS process/filesystem/network isolation, backup/restore and independent review remain prerequisites.

### P1 — useful operator/resource intelligence without dispatch

4. **Evidence-qualified model-to-task matching — [Enterprise #257](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/257). Status: Implemented (partial); Unit-tested; CI-verified on main; not Integration-verified.** Model-inventory v2 schema/intake validation and advisory matching tests exist; no live authenticated operator inventory feed or production service projection is proven.
5. **Read-only operator workflows — [Enterprise #23](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/23) and [#21](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/21). Status: Implemented (partial); production integration blocked.** Continue trusted-device, CPU/RAM/GPU/NPU, freshness, thermal/power/participation, reservations, leases and audit visibility. Label all synthetic/demo data; do not present a prototype as a live fleet.
6. **Privacy-minimised resource inventory. Status: Implemented (partial); production service integration blocked.** Keep device capability/resource telemetry limited to enrolled devices and necessary fields. No prompts, file contents, personal paths, credentials, history or employee activity. Show source, confidence, freshness and unknown values.

### P2 — optional customer capabilities

7. Security Centre and provider orchestration [#50](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/50), tenant cache [#26](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/26), planner [#53](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/53), Infrastructure Shield [#65](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/65), and notifications [#95](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/95): defer until trust and service foundations support them.
8. Collective Defence [#35](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/35), dual opt-in [#39](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/39), and event ingestion [#38](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/38): separate opt-in by organisation and participant; no autonomous containment or destructive response.

## Coordinated 14-day sequence and next owner

| Day | Date | Workstream target | Gate / owner |
|---|---|---|---|
| 1 | 21 Nov | Deduplicate audits, PRs, issues and evidence into this shared register | Public stream; this snapshot is the consolidated baseline |
| 2 | 22 Nov | Public native IPC authorization matrix and denial-before-side-effect tests | Public stream; keep #120 open until every sensitive handler is covered |
| 3 | 23 Nov | Public stability, cancellation, filesystem/TOCTOU and PDF safety | Public + integration; #408 is a P0 blocker and must not be bypassed |
| 4 | 24 Nov | Public model catalogue, hardware fit, provenance/licence and explicit choice | Public; do not present the mock adapter as inference |
| 5 | 25 Nov | Enterprise trust, dual consent, withdrawal/revocation and lease lifecycle | Enterprise stream; no production dispatch |
| 6 | 26 Nov | Enterprise resource inventory, capacity admission, reservation concurrency and recovery | Enterprise stream |
| 7 | 27 Nov | Cross-repository contract compatibility and regression | Integration stream |
| 8 | 28 Nov | Public model-selection-to-real-inference workflow | Public; Ollama first, explicit user action |
| 9 | 29 Nov | One complete OS-backed Public everyday workflow | Public; selected scope, OS permissions, privacy and cancellation |
| 10 | 30 Nov | Public onboarding and first-use experience | Public; only after safety gates |
| 11 | 1 Dec | Enterprise enrollment-to-resource-admission using real service data | Enterprise; simulation clearly labelled if service is absent |
| 12 | 2 Dec | Enterprise operator console connected to service-backed data | Enterprise; no fake live data |
| 13 | 3 Dec | Cross-platform regression, recovery and security tests | Integration + both product owners |
| 14 | 4 Dec | Strict release-readiness and user-testing decision | All streams; no release unless evidence supports it |

Dates are targets, not permission to bypass prerequisites. The next Public code target remains #408 handle-relative selected-root traversal and deterministic cross-platform tests; #120 remains the next scheduled authorization-matrix batch. The sysinfo 0.40.4, Tauri API 2.12.5 and rusqlite 0.43.0 dependency updates are merged. The Public Rust audit still reports seven allowed advisories; do not treat the passing workflow as a clean dependency posture.

## Release posture

- **Public AI:** pre-alpha/controlled preview; not Release-ready. Main CI/CodeQL/fuzz evidence is useful but does not replace cross-platform device testing, filesystem root pinning, PDF isolation proof, dependency remediation, native IPC audit, signing/updater/rollback and manual first-use testing.
- **Enterprise AI:** pre-alpha policy/control-plane prototype; not production worker-ready. Authenticated service integration, key custody, trust/consent/lease lifecycle, tenant isolation, OS isolation, signed receipts, backup/recovery and independent security review remain.
- **Integration:** contract/schema compatibility is testable; no end-to-end Public-to-Enterprise production workload path is verified.

**Completion rule:** update this register from live main/PR SHAs and exact workflow runs on each coordinated pass. Do not call an item Integration-verified, Device-verified or Release-ready without the corresponding evidence.