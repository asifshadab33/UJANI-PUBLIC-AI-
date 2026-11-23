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



## Live validation update — 23 November 2026 (supersedes the 22-Nov status above)

- **Public main SHA:** `bc31ddbf5c874dd651ff4303a595424de398c1c5`. Scheduled CodeQL and parser-fuzz smoke passed. The latest main CI run [#45043277205](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45043277205) reports Rust core tests, React build, frontend audit, Ubuntu/Windows/macOS native tests, Rust lint and workflow lint passed; **Rust dependency security audit failed** on desktop `url 2.5.8` (RUSTSEC-2026-0592; patched in `url >=2.5.9`). PR [#418](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/418) updates only Cargo.lock to `url 2.5.9` and `form_urlencoded 1.2.3`; exact-head CI, RustSec (zero vulnerabilities; seven allowed warnings), CodeQL, Android compatibility, native tests, installers-size checks and security watch all passed. It remains open, unmerged and awaiting review.
- **Enterprise main SHA:** `6d9305f6e671ab15c21acfa3ced10829adf625c2`. Post-merge CI [#45095411466](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/45095411466) and CodeQL [#45109771902](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/45109771902) passed on that exact SHA.
- **#408 remains an unresolved P0:** the shared walker still enumerates via path-based operations after root canonicalization. Fix requires pinned root/child directory handles and handle-relative enumeration; keep #395/#396 dependent callers from treating canonical paths as security identity. No fix or native race test is claimed yet.
- **#396 review coverage:** pre-cancel, symlink rejection and depth-limit reporting exist; root/descendant replacement races and active-cancellation/budget regressions still need dedicated validation. Keep draft and blocked by #408. A review note questioning the cancellation guard was corrected and withdrawn—the guard itself was not a defect.
- **Enterprise PR inventory:** [#321](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/pull/321) is the open draft evidence-refresh PR, head `e7f90800c2d3031b80835c70373033b75e8c0969`. Policy tests (Ubuntu/Windows/macOS), PostgreSQL tenant-isolation, RustSec and sensitive-change inspection passed; CodeQL was queued at the last check. It is docs-only and remains unmerged pending final validation.
- **Release state:** both products remain not release-ready. These hosted CI results do not prove device testing, signed installers, production deployments or end-to-end interoperability.

## Evidence vocabulary

- **Implemented** — source exists on the named main branch or PR head; partial implementation is labelled explicitly.
- **Unit-tested** — relevant unit/regression tests exist and have actually run successfully.
- **CI-verified (SHA)** — hosted workflow passed on the exact SHA stated; do not transfer this status to a different commit.
- **Integration-verified** — complete service-to-client or cross-component workflow exercised with real integration data.
- **Device-verified** — manually exercised on named physical devices/OS versions.
- **Blocked** — a known safety, correctness, infrastructure or validation dependency prevents acceptance.
- **Release-ready** — all applicable automated, integration, device, security, packaging and operational gates are satisfied.

Unknown is not Available. In-progress, skipped, stale, cancelled or partial checks are not passes. No item in this snapshot is marked Release-ready.

## Live refresh — 22 November 2026 (integration pass; supersedes stale evidence below)

- **Public main:** `bc31ddbf5c874dd651ff4303a595424de398c1c5` (PR #415 merge). Main CI [#45043277205](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45043277205) and CodeQL [#45043277182](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45043277182) are in progress; no post-merge pass is claimed.
- **Enterprise main:** `6d9305f6e671ab15c21acfa3ced10829adf625c2` (PR #320 merge). Main CI [#45043397417](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/45043397417), CodeQL [#45043397397](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/45043397397) and Rust dependency audit [#45043397407](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/45043397407) are in progress.
- **Merged Public PR #416:** recent-file IPC regression guards, merge `ce5274db9d7b488bc56b58c2e2f88733ee282532`, exact head `19098e751b9051f3a2c15b0ff7021439293d0d10`. Rust core tests, lint, JS/Rust analysis, React build, dependency audits, CodeQL, sensitive-change inspection, Ubuntu/Windows/macOS native tests and installers, Android arm64 compile and installer-size audit passed; unsigned preview installer was skipped. This adds tests only, not a new permission.
- **Merged Public PR #415:** metadata-only cloud-transfer preflight receipts, merge `bc31ddbf5c874dd651ff4303a595424de398c1c5`, exact head `65fb868d1d5b4089c74f82b253407f1c3ee9c5dc`. Exact-head CI, CodeQL, native tests/installers on Ubuntu/Windows/macOS, Android arm64, Rust/frontend audits, workflow lint, sensitive-change watch and installer-size audit passed; unsigned preview installer skipped. Receipt capacity is reserved before policy/consent; no-dispatch outcomes are metadata-only; provider receipts follow the adapter dispatch callback. This is a partial #120 slice, not complete authorization coverage.
- **Open Public PR #396:** bounded empty-folder preview, exact head `6f8d31d1230ba9f2d460e0debf36168df23eaf68`, stale base `8015146dd4d719e72ddf258903b523c87f1d088f`, draft and `mergeable=false`. CI, CodeQL, native tests/installers, Android, audits and security watch passed; unsigned preview skipped. **Blocked by #408:** path-based selected-root/descendant enumeration still has a replacement race. Do not merge or rebase-and-merge until the shared walker is pinned-handle/handle-relative and adversarial tests pass on supported platforms.
- **Merged Enterprise PR #318:** production-authority documentation, merge/main `81c406121021db99da0ab33b6f31aaab744048b5`; PostgreSQL is the single production authority, Rust ledger is not a production lease authority.
- **Merged Enterprise PR #319:** read-only device inventory search/filter/pagination, merge `682c0ce83fbe0ebca9a168429f736bfaee6c67f`, exact head `c22903dd301c3ca4537ae8a038b3b5db503e8759`. Ubuntu/Windows/macOS policy tests, PostgreSQL tenant-isolation job, Rust analysis, CodeQL, RustSec and sensitive-change inspection passed. Post-merge Enterprise CI [#45043096606](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/45043096606) passed on that merge SHA; its CodeQL/RustSec were still running when main advanced.
- **Merged Enterprise PR #320:** cross-platform device-agent contract documentation only, merge `6d9305f6e671ab15c21acfa3ced10829adf625c2`, exact head `4f2bde2bea0366dde94cbac860861e58061be583`. CI, CodeQL, RustSec, PostgreSQL tenant-isolation and sensitive-change checks passed. The duplicate production-authority ADR was removed; the document references existing `ARCHITECTURE.md`. No endpoint agent, service host, isolation backend or physical-platform support was implemented.
- **Open PR inventory at the 23-Nov refresh:** Public #418 (lockfile security fix, ready for review), #417 (draft docs/evidence refresh), and #396 (draft empty-folder feature blocked by #408). Enterprise #321 is a draft evidence-refresh PR. All PRs remain unmerged unless explicitly stated; no branches were deleted.
- **Day 2 Public IPC progress:** #416 and #415 are merged, but #120 remains open. Alternate invocation surfaces, full command-to-capability matrix, denial-before-side-effect tests for other high-risk commands and receipts for remaining persistent/external mutations are incomplete.
- **P0/P1 next:** Day 3 #408 shared filesystem root-identity/TOCTOU hardening; keep #396 blocked. Continue #120 as a deduplicated matrix. Enterprise #20/#90 remain production blockers: authenticated host, enrollment/trust, dual consent/revocation, transactional replay/lease/admission, protected key custody/recovery, isolation, receipts and operational recovery.
- **Fresh shared artifact parity (22 Nov):** task contract v1 schema `566a0d72babd58636a3ef40e55b5c44168776522`; inventory v2 schema `6f1d5a6dd3b70a4b5e78203a20a67b71ed35a489`; consent vector `dcd7021560212504810e8438e14efd591e5e657d`; inventory v2 vector `7d3bc7b6990d0572547e2312a97032f5486539a8`; catalogue `8f97eeb59d5c6cdf6c07df027eaf587a64184ba6`. Each blob SHA matches across current mains. The consent vector remains a placeholder, so this is artifact parity—not JCS/Ed25519 or live interoperability.
- **Release state:** neither product is Release-ready. No signing, production deployment, physical-device verification or release is claimed.

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

- **Open PRs:** none (verified 22 Nov 2026).
- **Main:** `6d9305f6e671ab15c21acfa3ced10829adf625c2`; post-merge CI, CodeQL and RustSec are in progress. PR #318 authority decision, #319 read-only inventory filtering/pagination, and #320 cross-platform device-agent contract documentation are merged. #319 post-merge CI passed on `682c0ce...`; #320 exact-head CI, CodeQL, RustSec, PostgreSQL tenant-isolation and sensitive-change checks passed before merge.
- **Current backlog:** #20 device trust/enrollment/key lifecycle/dual consent/revocation/transactional lease; #90 production authenticated HTTP/OIDC service, tenant-scoped repositories and live operator data. No production agent/service or remote dispatch exists. Keep PostgreSQL as the sole production authority.
- Detailed Enterprise status is in [MASTER_BACKLOG.md](MASTER_BACKLOG.md) and the [requirements ledger](MASTER-REQUIREMENTS-IMPLEMENTATION-LEDGER-2026-11-19.md); this register is the shared priority index, not a duplicate.

### P0 — before any worker pilot (remote execution stays disabled)

1. **Authenticated tenant-scoped service and device agent — [Enterprise #90](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/90). Status: Blocked.** Production HTTPS/TLS, OIDC/mTLS, tenant RBAC, live repositories, durable audit and enrolled endpoint agent are absent.
2. **Trusted-device enrollment, revocation and authenticated lease lifecycle — [Enterprise #20](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/20). Status: Implemented (partial); CI-tested foundations; production integration blocked.** Reuse migrations and admission primitives. Prove owner opt-in AND admin policy, current-state revocation, replay/freshness, transactional admission/reservation/lease/audit and recovery.
3. **Protected keys, isolation, receipts and recovery. Status: Blocked.** Production OS/KMS/HSM/TPM key provisioning/rotation/recovery, independent anti-rollback, signed task/input/output receipts, OS-enforced isolation/egress, crash/partition recovery, backup/restore and independent review remain prerequisites.

### P1 — useful operator/resource intelligence without dispatch

4. **Model-to-task matching — [Enterprise #257](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/257). Status: Implemented (partial); CI-tested; not Integration-verified.** Inventory v2 and advisory matcher exist; no live authenticated inventory feed or service projection.
5. **Read-only operator workflows — [Enterprise #23](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/23) and [#21](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/21). Status: Implemented (partial); service integration blocked.** Continue trust/resource/freshness/participation/reservation/lease visibility; demo data must stay labelled.
6. **Privacy-minimised resource inventory. Status: Implemented (partial); service integration blocked.** Only enrolled devices and necessary metadata; no prompts, personal files/paths, credentials, history or employee activity.

### P2 — optional customer capabilities

7. Security Centre/provider orchestration [#50](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/50), tenant cache [#26](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/26), planner [#53](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/53), Infrastructure Shield [#65](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/65), and notifications [#95](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/95): defer until trust/service foundations support them.
8. Collective Defence [#35](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/35), dual opt-in [#39](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/39), and event ingestion [#38](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/issues/38): separate organisation and participant opt-in; no autonomous containment or destructive response.

## Cross-repository integration backlog (P1)

- **[Public #412](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/412) — shared task-contract v1 compatibility and receipt integration. Status: schema/blob parity verified; integration blocked.** Day 7 target (27 Nov). No new schema is planned unless tests demonstrate a gap. The task is to cross-test strict parsers and JCS/Ed25519 vectors, exact payload digest/length, legacy Public peer v2 versus Enterprise WorkloadManifest v1 (explicit mapping or rejection), identity/audience binding, user/admin/worker-owner consent and revocation, resource/classification/expiry/replay bounds, monotonic status/cancel acknowledgement, and signed output-bound receipts/ledger ingestion. The consent vector is a placeholder and does not test Ed25519. Dependencies: Public #120 and Enterprise #20/#90. Keep transport/remote dispatch disabled.

## Coordinated 14-day sequence and next owner

| Day | Date | Workstream target | Gate / owner |
|---|---|---|---|
| 1 | 21 Nov | Deduplicate audits, PRs, issues and evidence into this shared register | Completed; current live main/PR evidence refreshed in this register |
| 2 | 22 Nov | Public native IPC authorization matrix and denial-before-side-effect tests | Targeted recent-file guards (#416) and cloud-transfer preflight receipts (#415) merged; #120 remains open for the full matrix |
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