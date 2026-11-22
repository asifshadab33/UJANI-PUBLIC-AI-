# UJANI shared implementation backlog and evidence register

**Evidence snapshot: 22 November 2026 (integration refresh).** Canonical cross-repository priority order for the coordinated 14-day programme. This register consolidates the live open issues, open PRs, current main-branch evidence and the existing Public/Enterprise master plans. It is a planning and evidence ledger, not a security certification or release approval.

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

## Live refresh — 22 November 2026 (supersedes stale PR/main rows below)

- **Public main:** `d4567ee6843799dae118a64e2b417d59bac1b4f2` (PR #413 merge). **Enterprise main:** `5b99a290468938661f69bc6444010550e6b147c5` (PR #315 merge).
- Public PR #413, head `173d18a3490c02111c9b78d013eb0a82488abccd`, is merged. Exact-head CI, CodeQL, Android compatibility, security-sensitive change watch and installer-size audit all passed; preview installers were skipped. The merged read-only cache settings/history fix is present on main. No post-merge workflow runs or combined statuses were returned for merge SHA `d4567ee...`; therefore post-merge CI is **not verified** and no physical-device test is claimed.
- PR #411 (documentation evidence correction) is merged at `0696251e73764c8966be4e8828fb4b35852873af`; Enterprise PR #315 (status pointer) is merged at `5b99a290468938661f69bc6444010550e6b147c5`. Both are documentation-only.
- **Open Public PRs:** code PR #396 is draft, stale base (current main is ahead), `mergeable=false`, and blocked by selected-folder root/descendant TOCTOU #408; status-refresh PR #414 is this documentation-only update pending checks. Do not merge/delete #396 before the root-handle fix and exact-head revalidation. Enterprise has no open code PR; status-refresh PR #316 is this documentation-only pointer update pending checks.
- The shared-contract blob parity result remains the latest independent verification from 21 Nov. The intervening PRs #411, #315 and #413 changed status documentation or the Public cache file only; no contract/schema artifacts were changed. This is not a new 22-Nov parity run and does not prove service interoperability.
- Day 2 Public IPC inventory audit: existing `docs/native-command-security-map.json` reconciles 45 default commands and 7 feature-gated peer-preview commands against the invoke-handler registration. No unmatched commands were found. This map/parity guard is an inventory check only; runtime denial-before-side-effect coverage remains incomplete in #120. Latest audit prioritized selected-folder reads (#408), external cloud transfer denial/receipt paths, document/PDF paths, persistent-state/history/cache/reminder mutations, and the disabled model-download path.

**Release state:** neither product is Release-ready. Enterprise remote worker execution remains disabled. Public remains blocked by outstanding P0 security/privacy/reliability, cross-platform packaging/device checks, filesystem root identity and full native IPC audit.


## Live repository baseline

| Repository | Default branch | Exact main SHA | Exact-SHA evidence |
|---|---|---|---|
| [Public AI](https://github.com/asifshadab33/UJANI-PUBLIC-AI-) | `main` | `6ec7f950b4e41c4061d356a762e0858829f1d379` | Current main is the documentation-only #410 merge atop functional code SHA `baf75d0f57bd0d85e61d9df946a077f64a80feab`. CI #44892901509 and CodeQL #44892901490 passed on `baf75`; Rust audit passed with **7 allowed advisories**. PR #410 exact head checks passed (CI #44907636852, CodeQL #44907636875, installer audit #44907636853, security watch #44909297413); preview installers skipped. No post-merge workflow runs were returned for `6ec7f950...`.
| [Enterprise AI](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI) | `main` | `864b0a029637eba320ef99649685f2ea812e8989` | PR #314 exact-head CI #44896423107, CodeQL #44896423072, RustSec #44896423039 and security-watch #44896423064 passed before merge. No post-merge workflow runs were returned for the merge SHA.

Recent Public main changes include host-probe offloading (#407), duplicate-key model-inventory validation (#406), batch-register refresh (#405), and merged dependency updates for sysinfo 0.40.4 (#384), Tauri API 2.12.5 (#385) and rusqlite 0.43.0 (#391). The shared evidence register was merged in #409. Recent Enterprise main includes strict duplicate-key model-inventory intake (#313) and merged duplicate authenticated-telemetry-field regression coverage (#314), following tenant-isolation and fail-closed model-match regression work. Enterprise has one open documentation-only status refresh PR #315; its exact-head checks are queued/in progress. Public has feature PR #396 blocked on #408 and this documentation-only evidence correction PR #411, whose exact-head security-watch is in progress.

**Current shared-contract check (independently fetched 21 Nov):** the manifest and all seven manifest-listed artifacts have identical Git blob SHAs on both current main branches. Verified files: manifest `5c41a57411453145f0d80a470f9985375b6e3ca4`; catalogue `8f97eeb59d5c6cdf6c07df027eaf587a64184ba6`; v1 inventory schema `8ff1a1db4700214ba687cd806c449bc1ea7dfdb4`; v2 inventory schema `6f1d5a6dd3b70a4b5e78203a20a67b71ed35a489`; task contract `566a0d72babd58636a3ef40e55b5c44168776522`; consent vector `dcd7021560212504810e8438e14efd591e5e657d`; v1 inventory vector `543d645d0154c9263ff7784ba4e203f54e498c3c`; v2 inventory vector `7d3bc7b6990d0572547e2312a97032f5486539a8`. Schema/blob parity is verified; protocol-service interoperability is not.

## Public AI — open PR inventory

| PR | Exact head SHA | Base | Current evidence / decision |
|---|---|---|---|
| [#396 empty-folder preview](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/396) | `6f8d31d1230ba9f2d460e0debf36168df23eaf68` | `main` (base `8015146dd4d719e72ddf258903b523c87f1d088f`; current main is **20 commits ahead**) | Exact-head CI #44780970780, CodeQL #44780970866, installer-size audit #44780970845, security watch #44780970739 and Android #44780970760 passed; unsigned preview installer skipped (#44780971046). Draft, `mergeable=false`. **Do not merge:** root/descendant path replacement race; issue #408 requires handle-relative traversal and adversarial cross-platform tests. Rebase and rerun after fix. |

Recent Public main changes include host-probe offloading (#407), duplicate-key inventory validation (#406), batch-register refresh (#405), #395 recent-file preview, and dependency updates #384/#385/#391. #409 consolidated the backlog; #410 refreshed evidence and merged documentation-only at `6ec7f950...`. The functional code checks cited above are on `baf75d0f...`, not the current documentation merge SHA. #411 is an open documentation-only correction PR; its live head/checks are tracked on the PR to avoid a self-referential SHA in this register. These changes do not complete #120 or #408.

## Shared ordered backlog — Public AI

### P0 — safety and first-release correctness

1. **Selected-root identity and metadata-only traversal — [Public #408](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/408). Status: Blocked.** The shared `safe_fs::walk_directory` still performs path-based metadata/canonicalization/enumeration. The Windows root helper validates a handle but the current walker does not retain it for enumeration. The merged recent-file finder (#395) uses this shared walker; empty-folder PR #396 has the same class of race. Implement handle-relative enumeration for each supported desktop OS, preserve metadata-only semantics, symlink/reparse rejection, bounds and cancellation, and add deterministic root-replacement tests. Do not merge #396 before this is proven. This is the highest-priority filesystem blocker.
2. **Command-by-command native IPC authorization and receipts — [Public #120](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/120). Status: Implemented (partial); Unit-tested; CI-verified on functional main SHA `baf75d0f57bd0d85e61d9df946a077f64a80feab` (the current main SHA is the later docs-only `6ec7f950...`, whose post-merge runs are pending); acceptance incomplete.** The command-map parity guard and targeted native confirmations are present. Still review every sensitive invoke path and prove validation, capability/consent authorization, cancellation, resource bounds, receipt creation and denial-before-side-effect. The map is an audit index, not runtime proof. Prioritise filesystem/document/external-transfer, selected-folder import, history/export/cache, model pulls, updater and cloud gateway.
3. **Dependency advisories and reproducible migrations — [Public #73](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/73) and [#173](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/173). Status: Blocked / in progress.** The Rust audit job on functional SHA `baf75d0f57bd0d85e61d9df946a077f64a80feab` passed but still reports seven allowed advisories: chrono, glib, proc-macro-error, ttf-parser, zbus_names, zvariant and zvariant_derive. This is not a clean dependency posture. PRs #384 (sysinfo 0.40.4), #385 (Tauri API 2.12.5) and #391 (rusqlite 0.43.0) are now merged. The final PR #391 head's Android ARM64 compile passed (run #44713454500); an earlier failure note is stale. Android has not been rerun on current main `6ec7f950...` after the documentation-only merge. Do not suppress advisories or hand-edit lockfiles; coordinate AES-GCM, rand, sysinfo, GTK/Tauri, PDF and keyring changes with reproducible locked builds and cross-platform checks.
4. **PDF, filesystem and stability hardening — [Public #70](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/70). Status: Implemented (partial); Blocked on platform/device evidence.** PDF sidecar/resource/cancellation work and navigation fixes exist, but installed-package PDF/cancellation smoke tests and OS-enforced isolation proof remain incomplete. Screenshot capture-to-chat is not implemented. Continue malformed/oversized/encrypted/interrupted upload tests, filesystem traversal/TOCTOU and recovery tests, and UI hang/sub-link/manual smoke tests.
5. **Ollama, context and model-pull safety. Status: Implemented (partial); full review pending.** Ollama is the real loopback runtime; cancellation and local routing work exists. Verify no redirects, proxy bypass, bounded HTTP and actual request cancellation, explicit `num_ctx`, host/mode/progress bounds for model pulls, and no silent runtime start/download/cloud fallback.
6. **History/cache/export privacy. Status: Implemented (partial); full release evidence pending.** Verify AES-GCM AAD binding, metadata minimisation, key rotation/recovery, DB/file permissions, export lifecycle and cache expiry/clear behavior. Do not treat encryption unit tests as proof of platform ACLs or backup recovery.
7. **Controlled alpha and OS support — [Public #45](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/45), [#67](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/67). Status: Blocked; not Release-ready.** Hosted builds are not physical-device verification. Minimum Windows/macOS/Linux baselines, installer smoke tests, WebView2/Linux dependencies, signing/updater/rollback and manual privacy/recovery testing remain open.

### P1 — complete useful local intelligence without expanding risk

8. **Model Intelligence and real inference. Status: Implemented (partial); Unit-tested; CI-verified on functional SHA `baf75d0f57bd0d85e61d9df946a077f64a80feab`; not Integration-verified (current main is docs-only `6ec7f950...` with post-merge checks pending).** Public has model-inventory v2 validation, duplicate-key rejection, host-probe offloading, conservative RAM fit and a real loopback Ollama adapter. The deterministic `model_runtime.rs` adapter remains a debug-only mock and must never be described as inference. Runtime-independent discovery, trustworthy capability/licence/provenance, storage/free-space evidence, explicit recommendations and the model-selection-to-real-inference journey remain incomplete. Unknown values must remain unknown.
9. **Focused fuzzing, dependency policy and recovery. Status: Implemented (partial); parser fuzz smoke was CI-verified on prior main SHA `6373073abe8c4f32d9cc5b52c840feb24f9a15b0`; it has not been rerun on functional main `baf75d0f57bd0d85e61d9df946a077f64a80feab`; current main `6ec7f950...` is a documentation-only merge.** Expand targets only for concrete parsers/contracts, enforce cargo-deny policy without suppressions, and add recovery/cleanup regression coverage.
10. **Security Watch — [Public #179](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/179). Status: Implemented (partial); acceptance not verified.** A security-sensitive change-watch workflow exists; this does not prove all runtime integrity/policy-drift/threat alert requirements. Optional verified email notifications [#183](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/183) remain deferred until the core alert lifecycle is sound.

### P2 — useful but non-blocking scope

11. **Everyday OS utilities — [Public #44](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/44) and [#46](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/46). Status: Implemented (partial); affected filesystem previews blocked by #408.** Local clock, duplicate/large-file previews and recent-file preview exist. Keep previews read-only, selected-folder scoped, bounded and native-app friendly; no silent indexing or destructive actions.
12. **Reminders/planner — [Public #83](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/83). Status: Implemented (partial); Unit-tested; CI-verified on main.** Local reminder storage and targeted destructive confirmation exist. A complete daily agenda and optional OS/app connectors remain separate work; connector writes require per-provider/scope approval.
13. **Android-only support evaluation — [Public #168](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/168). Status: Deferred pending complexity stop gate.** Do not let mobile expansion delay desktop first-release safety.

## Enterprise AI — open PR and issue inventory

- **Open PRs:** none (verified 21 November 2026).
- **Main:** `864b0a029637eba320ef99649685f2ea812e8989`; CI, CodeQL and Rust dependency audit passed on the stated main SHA.
- **Recently merged PR #314:** merge commit `864b0a029637eba320ef99649685f2ea812e8989`; its exact-head Windows/macOS/Linux policy suites, PostgreSQL tenant-isolation, CodeQL, RustSec and security-watch checks passed before merge. It adds test-only duplicate-field rejection coverage; it does not implement the production HTTP host or cross-store transaction. The merge commit itself has no workflow runs returned yet, so do not infer post-merge CI from the PR-head checks.
- **Detailed existing ledger:** [Enterprise MASTER_BACKLOG.md](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/blob/main/docs/MASTER_BACKLOG.md) and [Master Requirements & Implementation Ledger](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/blob/main/docs/MASTER-REQUIREMENTS-IMPLEMENTATION-LEDGER-2026-11-19.md). Keep those as Enterprise detail; this file is the shared priority index, not a duplicate implementation ledger.

### P0 — before any worker pilot (remote execution stays disabled)

1. **Authenticated tenant-scoped service and device agent — [Enterprise #90](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/90). Status: Blocked.** Database admission, request validation and tests exist, but the production authenticated OIDC/HTTP control plane and enrolled-device agent are not deployed.
2. **Trusted-device enrollment, revocation and authenticated lease lifecycle — [Enterprise #20](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/20). Status: Implemented (partial); Unit-tested; merged at `864b0a029637eba320ef99649685f2ea812e8989`; PR-head CI passed before merge, but post-merge workflow evidence is pending; production integration blocked.** Durable trust/consent/lease and atomic admission foundations exist. PR #314 is merged at `864b0a029637eba320ef99649685f2ea812e8989`; its exact-head CI, CodeQL and RustSec checks passed before merge. It is test-only coverage, not production HTTP service integration. Prove one authoritative service lifecycle, owner opt-in AND admin policy on every admission, current-state revocation, replay/sequence/freshness checks, transactional reservation/renew/release/expiry/reconciliation and tenant isolation.
3. **Protected key custody, receipts and recovery. Status: Blocked.** Production OS/KMS/HSM/TPM key provisioning/rotation/recovery, independently protected anti-rollback, signed task/input/output receipts, crash/partition recovery, OS process/filesystem/network isolation, backup/restore and independent review remain prerequisites.

### P1 — useful operator/resource intelligence without dispatch

4. **Evidence-qualified model-to-task matching — [Enterprise #257](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/257). Status: Implemented (partial); Unit-tested; CI-verified on main; not Integration-verified.** Model-inventory v2 schema/intake validation and advisory matching tests exist; no live authenticated operator inventory feed or production service projection is proven.
5. **Read-only operator workflows — [Enterprise #23](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/23) and [#21](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/21). Status: Implemented (partial); production integration blocked.** Continue trusted-device, CPU/RAM/GPU/NPU, freshness, thermal/power/participation, reservations, leases and audit visibility. Label all synthetic/demo data; do not present a prototype as a live fleet.
6. **Privacy-minimised resource inventory. Status: Implemented (partial); production service integration blocked.** Keep device capability/resource telemetry limited to enrolled devices and necessary fields. No prompts, file contents, personal paths, credentials, history or employee activity. Show source, confidence, freshness and unknown values.

### P2 — optional customer capabilities

7. Security Centre and provider orchestration [#50](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/50), tenant cache [#26](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/26), planner [#53](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/53), Infrastructure Shield [#65](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/65), and notifications [#95](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/95): defer until trust and service foundations support them.
8. Collective Defence [#35](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/35), dual opt-in [#39](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/39), and event ingestion [#38](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/38): separate opt-in by organisation and participant; no autonomous containment or destructive response.

## Cross-repository integration backlog (P1)

- **[Public #412](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/412) — shared task-contract v1 compatibility and receipt integration. Status: schema/blob parity verified; integration blocked.** Day 7 target (27 Nov). No new schema is planned unless tests demonstrate a gap. The task is to cross-test strict parsers and JCS/Ed25519 vectors, exact payload digest/length, legacy Public peer v2 versus Enterprise WorkloadManifest v1 (explicit mapping or rejection), identity/audience binding, user/admin/worker-owner consent and revocation, resource/classification/expiry/replay bounds, monotonic status/cancel acknowledgement, and signed output-bound receipts/ledger ingestion. The consent vector is a placeholder and does not test Ed25519. Dependencies: Public #120 and Enterprise #20/#90. Keep transport/remote dispatch disabled.

## Coordinated 14-day sequence and next owner

| Day | Date | Workstream target | Gate / owner |
|---|---|---|---|
| 1 | 21 Nov | Deduplicate audits, PRs, issues and evidence into this shared register | Public stream completed; integration independently verified live heads, PRs, checks and shared artifact parity; this correction PR refreshes stale SHA details |
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

Dates are targets, not permission to bypass prerequisites. The next Public code target remains #408 handle-relative selected-root traversal and deterministic cross-platform tests; #120 remains the next scheduled authorization-matrix batch. Cross-repository contract work is tracked in #412 for Day 7. The sysinfo 0.40.4, Tauri API 2.12.5 and rusqlite 0.43.0 dependency updates are merged. The Public Rust audit still reports seven allowed advisories; do not treat the passing workflow as a clean dependency posture.

## Release posture

- **Public AI:** pre-alpha/controlled preview; not Release-ready. Main CI/CodeQL/fuzz evidence is useful but does not replace cross-platform device testing, filesystem root pinning, PDF isolation proof, dependency remediation, native IPC audit, signing/updater/rollback and manual first-use testing.
- **Enterprise AI:** pre-alpha policy/control-plane prototype; not production worker-ready. Authenticated service integration, key custody, trust/consent/lease lifecycle, tenant isolation, OS isolation, signed receipts, backup/recovery and independent security review remain.
- **Integration:** contract/schema compatibility is testable; no end-to-end Public-to-Enterprise production workload path is verified.

**Completion rule:** update this register from live main/PR SHAs and exact workflow runs on each coordinated pass. Do not call an item Integration-verified, Device-verified or Release-ready without the corresponding evidence.