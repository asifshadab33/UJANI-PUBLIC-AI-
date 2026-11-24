# UJANI shared implementation backlog and evidence register

**Current evidence snapshot: 24 November 2026.** This top section supersedes older dated snapshots below. Exact commit and PR heads were checked live. This register is evidence, not a certification or release approval.

## Live 24 November status

### Default branches
- **Public main:** `673cbb2805cd10394750fb8887c922443b586e46` — merged PR #424 (shared Ed25519 vector test). Post-merge CI cannot be independently verified through the current connector; do not claim green main CI.
- **Enterprise main:** `8377c2a1e7a3a96ea938eea335a486d55479b77a` — merged PR #331 (fail-closed resource request boundary tests). Post-merge CI likewise not independently verified.
- PR #422 and #326 remain included in the preceding main history. No release was published; remote execution remains disabled.

### Open PRs and exact heads
- **Public #420** reminder agenda boundary tests: rebased onto current main; head `971bae7e068461dab896440a7608825ef64c46f0`. Fresh checks are pending; old-head checks had passed, but are not carried forward.
- **Public #423** this evidence register: refreshed onto current main and this 24-Nov snapshot; fresh checks pending.
- **Public #425** explicit local reminder JSON export: head `5b0b88980a1e4bd03e91649602b633d1bd2e7496`. UJANI CI, CodeQL and security watch passed; installer-size audit remains in progress. Its base predates #424, so it needs a fresh base/checks before merge. Export is explicit, local-only, and not backup/restore.
- **Enterprise #330** shared Ed25519 vector: rebased onto current Enterprise main; head `ab0bf696848b1ebacda501447c2eccff780dd698`. Original-head CI passed, but fresh rebase checks are pending.
- **Merged this pass:** Public #424 → `673cbb2805cd10394750fb8887c922443b586e46`; Enterprise #331 → `8377c2a1e7a3a96ea938eea335a486d55479b77a`. Both were merged only after exact-head CI/security checks passed. Main post-merge workflows are not verified in this run.
- Branches were retained; no stale branch was deleted.

### Integration evidence
- Shared task-contract fixture, manifest, schema artifacts and consent vector remain versioned test-only compatibility material. No transport or remote dispatch is activated.
- Public #424 and Enterprise #330 now share a real test-only Ed25519 vector. Cross-language fixture verification passed before merge/rebase; Enterprise rebase is awaiting fresh CI.
- Public peer TaskEnvelope v2 and Enterprise WorkloadManifest v1 remain incompatible. No relabelling or implicit translation.
- The shared contract does not prove principal/device/tenant key binding, live dual consent, revocation, durable replay admission, atomic leases, cancellation acknowledgement or signed receipt/ledger integration.
- Model/runtime descriptor schemas are a planning boundary only: Public local discovery/selection-to-real-inference and Enterprise enrolled-device inventory remain incomplete.

### Ordered master backlog (deduplicated)
1. **Public P0:** #408 handle-pinned, relative directory traversal with race/symlink/reparse/cancellation/resource-limit tests; #70 PDF worker isolation and installed-package B-01–B-10/D-01–D-07; #120 native IPC capability/consent/denial-before-side-effect matrix.
2. **Enterprise P0:** #20 trusted enrollment, protected key custody, rotation/revocation, owner+admin consent and replay-resistant authenticated leases; #90 real authenticated tenant-scoped control plane and durable fail-closed audit. No remote dispatch.
3. **Integration P1:** #412 complete negative/edge vectors and verified admission/receipt semantics; no production bridge until trust/consent/replay/lease/receipt ledger workflow passes.
4. **Model intelligence:** Public explicit user model choice with verified size/storage/RAM/capabilities/licence/provenance and real inference; Enterprise privacy-minimised inventory from enrolled devices only.
5. **External/manual gates:** physical cross-platform installs, PDF confinement/device probes, Ollama inference, signed installers/updater rollback, production key custody, backup/restore and deployment validation.

Neither product is Release-ready. Manual/device tests, production signing/credentials and deployment tests remain external gates.

---

**Evidence snapshot: 23 November 2026 (cross-repository integration refresh).** Canonical cross-repository priority order for the coordinated 14-day programme. This register consolidates the live open issues, open PRs, current main-branch evidence and the existing Public/Enterprise master plans. It is a planning and evidence ledger, not a security certification or release approval.



## Live refresh — 23 November 2026 (verified integration snapshot)

### Default branches and post-merge checks
- **Public main:** `67f0cb892143266269e714d56d1f190e27f4cf04` (#422 merged). CI [#45223945028](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45223945028) is in progress; CodeQL [#45223945031](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45223945031) passed. Prior main `050b53b...` after #421 had CI [#45213094072](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45213094072) and CodeQL [#45213093801](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45213093801) passed.
- **Enterprise main:** `ad1046ab454badf0a568c2e7e9557dfc9e49ac1f` (#326 zeroize 1.9.2 merged after #329). Post-#326 CI [#45225901856](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/45225901856), CodeQL [#45225901595](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/45225901595) and RustSec [#45225901810](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/45225901810) are queued. The prior main SHA `ffe8b1c...` after #329 had all policy/PostgreSQL CI jobs pass, CodeQL and RustSec pass.
- **Merged today:** Public #418 (URL 2.5.9), #421 (cookie 0.18.3), #422 (forward-only reminder snooze and invalid-record notification guard). Enterprise #323/#324/#328/#329/#326 (fail-closed demo/live transition, bounded request tests, CodeQL action pins, duplicate-key validator, zeroize 1.9.2). #396 closed unmerged; remains blocked by #408.

### Open PR inventory
- **Public #420** — agenda boundary/tie-break/overflow tests only; head `6f8397ae23288d07a9f540502537f0a8ea6005f4`, one ahead/zero behind. CI [#45224170857](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45224170857): Rust core/lint/frontend/workflow/dependency audits and macOS/Windows native pass; Ubuntu native is in progress. CodeQL passed; Android passed; three-platform installer-size audit is in progress; security watch passed. No merge yet.
- **Public #424** — fixed cross-language Ed25519 vector; rebased head `064d792f4a9f9a5ba63d43623ffc817d734c57bc`. Rust core, workflow lint, frontend build/audit and RustSec pass; lint/native/Android/CodeQL are in progress or queued; installer-size queued. The previous pre-rebase head passed all native tests, CodeQL, RustSec, Android and installer-size, but fresh exact-head checks are required. No approval.
- **Public #423** — this evidence refresh; updated head checks are queued/in progress after its latest status update.
- **Enterprise #330** — same vector, head `abe86ac5f5f9bc705526f27382e426b50bc0ecf8`. RustSec, CodeQL/security watch, Ubuntu/Windows policy tests and PostgreSQL tenant-isolation pass; macOS re-run is queued. No approval.
- **Enterprise #326** — merged as `ad1046ab454badf0a568c2e7e9557dfc9e49ac1f`; exact-head CI (Ubuntu/Windows/macOS/PostgreSQL), CodeQL, RustSec and security watch passed before merge.
- No branches deleted. #418/#421/#422 and #323/#324/#328/#329/#326 are merged; #325/#327 are closed as superseded.

### Shared-artifact parity and integration boundary
- Independently fetched from both PR #424/#330 heads: the shared manifest and all seven pinned artifacts are byte-identical. Manifest blob `67292834029efa2ca1b8ba9ef2c98152cece4bce`; catalogue `8f97eeb59d5c6cdf6c07df027eaf587a64184ba6`; inventory-v1 schema `8ff1a1db4700214ba687cd806c449bc1ea7dfdb4`; task-contract-v1 schema `566a0d72babd58636a3ef40e55b5c44168776522`; consent vector `593348db1bfc1d7d03844aa6e2de826f0852c39b`; inventory-v1 vector `543d645d0154c9263ff7784ba4e203f54e498c3c`; inventory-v2 schema `6f1d5a6dd3b70a4b5e78203a20a67b71ed35a489`; inventory-v2 vector `7d3bc7b6990d0572547e2312a97032f5486539a8`.
- The consent vector now carries a real test-only Ed25519 signature. Independent local Python verification passed, and a resource-limit mutation failed verification. The previous Public #424 head passed native tests on Linux/Windows/macOS and both repositories' CI ran the task-contract validator; current rebased heads need fresh checks. This is fixed-vector proof only—not key enrollment, authenticated transport or admission.
- **Legacy compatibility remains intentionally rejected:** Public peer `TaskEnvelope` v2 uses seconds, a 16-byte hex nonce, a bounded in-memory replay guard and a distinct length-prefixed signing profile. Enterprise `WorkloadManifest` v1 has different identity/consent semantics. Neither is wire-compatible with shared task-contract v1; no relabelling.
- Shared v1 defines task/version, sender principal/device and audience org/service, payload digest/length/media type, data/privacy classification, separate scopes/capabilities, CPU/MiB/runtime/output/network bounds, expiry/nonce, signature/key ID, device-trust requirement, user consent and required admin/owner approvals. Status is signed/sequence-numbered with state/policy/consent/reason. Receipt is signed/sequence-numbered and binds input/output digest/length, worker org/device, location, times, outcome, resource usage, consent refs and policy. The schema does not prove approvals or trust: authenticated identity, live consent/revocation, durable replay claim, atomic lease, cancellation acknowledgement and receipt-to-ledger integration remain absent.
- Model/runtime inventory v2 is byte-identical and evidence-qualified (runtime/model identity/version, task capabilities, context/tool/vision/embedding metadata, estimates, platform/accelerator compatibility, source/confidence/freshness and consent/privacy context). Public discovery remains local Ollama metadata; Enterprise has no authenticated live fleet inventory feed. No prompts/private content cross.

### Ordered master backlog (deduplicated)
1. **Public P0:** #408 handle-pinned, relative directory enumeration with root/descendant race, symlink/reparse, cancellation and budget tests; #70 PDF worker isolation plus installed-package B-01..B-10/D-01..D-07 (NOT RUN; Windows/macOS confinement unverified, Linux probe Unknown); #120 native IPC authorization/denial-before-side-effect matrix. #396 stays closed/blocked until #408 is fixed.
2. **Enterprise P0:** #20 trusted enrollment, protected key custody/rotation/revocation, owner+admin consent and authenticated replay-resistant leases; #90 real HTTPS/OIDC host, tenant-scoped live API, durable audit fail-closed. No remote dispatch.
3. **Integration P1 (#412):** finish strict vectors/legacy rejection, then authenticated identity/dual-consent, replay/expiry, atomic lease, cancellation and signed receipt/ledger workflow. Shared schema is not a runtime bridge.
4. **Model intelligence (24 Nov target):** Public explicit model choice and verified size/storage/RAM/capability/licence/provenance to real inference; Enterprise enrolled-device-only metadata for operator planning. No silent installs/model switches or mock inference claims.
5. **External gates:** physical Windows/macOS/Linux install and PDF/UI probes, Linux isolation device probe, Windows/macOS worker confinement, signed installers/updater rollback, real Ollama inference, KMS/HSM/TPM, production database backup/restore and deployment drills.

Neither product is Release-ready. Remote execution/containment remains disabled; signing/credentials and a release publish are not authorised.
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

## Live refresh — 22 November 2026 (Public workstream; supersedes older main/PR rows below)

- **Public main:** 1882201c5e6d1fd6dd8632caf526ae1ecd611d61; main CI [#44972798910](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44972798910) and CodeQL [#44972798931](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44972798931) passed on that exact SHA.
- **Enterprise main:** d39cbdfeead83dc5cba002eb21b9073283f291f2; main CI [#44978412604](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44978412604), CodeQL [#44978412583](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44978412583), and Rust dependency audit [#44978412595](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44978412595) passed on that exact SHA.
- **Open Public PR #396:** head 6f8d31d1230ba9f2d460e0debf36168df23eaf68, base 8015146dd4d719e72ddf258903b523c87f1d088f (stale against current main), draft and mergeable=false. Exact-head CI [#44780970780](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44780970780), CodeQL [#44780970866](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44780970866), Android [#44780970760](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44780970760), installer-size [#44780970845](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44780970845) and security watch [#44780970739](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44780970739) passed; unsigned preview was skipped. **Do not merge:** issue [#408](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/408) still requires selected-root identity pinning and handle-relative traversal with adversarial platform tests.
- **Open Public PR #415:** [cloud-transfer preflight outcome receipts](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/415), initial implementation commit d80db7e6bf10fd45cc696dd5fa75b6748b915b49. Subsequent commits correct the native-command source-order guard and its negative tests, including guard commit 37b4c413b4fdaaccb7fbc982bac755646f3bfc89 and test commit d4e095bfe1a0e21152ffb90e32f8458188995e97. On exact head d4e095b, security watch [#44992558923](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44992558923) passed; CI [#44992558875](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44992558875), CodeQL [#44992558902](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44992558902), Android [#44992558859](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44992558859) and installer-size audit [#44992558905](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44992558905) were in progress; unsigned preview [#44992558894](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44992558894) was skipped. Earlier exact-head CI on 82c94d8 failed in the static native-command security-map guard because it still required receipt reservation after consent and the receipt enum value had become noncanonical. The Rust core test job stopped at that guard before Cargo tests; no Rust unit-test pass is claimed. The guard now permits audit-slot reservation before policy while explicitly rejecting budget/network dispatch before native approval, and the receipt state is canonical again. Revalidate the final PR head after this register-only update.
- **Open Enterprise PR #318:** [single-authority production architecture clarification](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/pull/318), head 11a256e9706e8ddd2039bb5ba69f0d705c0d9524, base d39cbdfeead83dc5cba002eb21b9073283f291f2, non-draft and mergeable. Exact-head CI [#44990164613](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44990164613), CodeQL [#44990164579](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44990164579), Rust dependency audit [#44990164577](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44990164577) and security watch [#44990164582](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI/actions/runs/44990164582) passed. This is documentation-only and does not deploy the production control plane.
- **Day 2 Public IPC work:** issue #120 remains open. The cloud gateway now has a targeted preflight receipt path in PR #415: local receipt capacity is reserved before policy/consent checks; policy/validation denials and native non-approval receive a metadata-only local receipt; an ExternalProvider receipt is emitted only after the adapter's dispatch callback. Unit tests and a source-order guard were added, but those tests have not yet been reported as executed on the final PR head. This is one command slice, not completion of the command matrix.
- **Enterprise next-session blockers:** production authenticated OIDC/HTTP service and enrolled-device agent, atomic owner-consent plus administrator-policy admission/revocation, protected KMS/HSM/TPM key custody and recovery, signed receipts, tenant isolation and OS worker isolation. Remote worker dispatch and automated containment remain disabled.
- **Cross-repository contract evidence:** the latest independently fetched blob-parity check remains the 21-Nov result recorded below. No new parity or service-interoperability run was performed in this session.
- **Release state:** neither product is Release-ready. Main-branch CI is not a substitute for integration/device verification; no signed installer, physical-device validation, production service deployment or release is claimed.

**Current next order:** (1) finish exact-head CI/review for Public #415; (2) continue the remaining #120 command-level denial-before-side-effect matrix; (3) Day 3 filesystem root-identity/TOCTOU work under #408 before #396 can be reconsidered; (4) continue Public PDF/stability and dependency remediation. Enterprise workstream should address #318's documentation review separately from the production service blockers above.

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

- **[Public #412](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/412) — shared task-contract v1 compatibility and receipt integration. Status: schema/blob parity verified; integration blocked.** Day 7 target (27 Nov). No new schema is planned unless tests demonstrate a gap. The task is to cross-test strict parsers and JCS/Ed25519 vectors, exact payload digest/length, legacy Public peer v2 versus Enterprise WorkloadManifest v1 (explicit mapping or rejection), identity/audience binding, user/admin/worker-owner consent and revocation, resource/classification/expiry/replay bounds, monotonic status/cancel acknowledgement, and signed output-bound receipts/ledger ingestion. The shared consent vector now includes a deterministic test-only Ed25519 envelope signature; Public #424 and Enterprise #330 add Python and native Rust verification, with exact-head CI pending. Dependencies: Public #120 and Enterprise #20/#90. Keep transport/remote dispatch disabled.

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

### Dependency audit detail — 23 November 2026

Public RustSec exact-head passes on #421 and #422 confirmed the patched `url 2.5.9` and `cookie 0.18.3` lock entries, but **seven allowed warnings remain**: `chrono` (RUSTSEC-2026-0543), `proc-macro-error 1.0.4` (RUSTSEC-2024-0370), `ttf-parser 0.25.1` (RUSTSEC-2026-0192), `zbus_names 3.0.0` (RUSTSEC-2026-0371), `zvariant 4.2.0` (RUSTSEC-2026-0370), `zvariant_derive 4.2.0` (RUSTSEC-2026-0372), and `glib 0.18.5` (RUSTSEC-2024-0429; patched glib >=0.20). The post-#422 main audit is still part of the in-progress push CI. No advisories are suppressed; Tauri/GTK/keyring/PDF dependency-path upgrades need cross-platform evidence. See [Public #73](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/73).

## Latest integration verification — 23 November 2026

This section is the current run's evidence and supersedes the older same-day snapshot rows above.

- **Public main:** `67f0cb892143266269e714d56d1f190e27f4cf04`; CI [#45223945028](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45223945028) in progress, CodeQL [#45223945031](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45223945031) passed. #421 post-merge main checks were green on `050b53b...`; #422 exact-head checks were green before merge.
- **Enterprise main:** `ffe8b1c50d8d9a9d655f9eeda1e960030c7b8952`; CodeQL/RustSec passed; CI queued with macOS re-run queued and Ubuntu/Windows/PostgreSQL successful.
## Latest integration verification — 23 November 2026

This section is the current run's evidence and supersedes the older same-day snapshot rows above.

- **Public main:** `67f0cb892143266269e714d56d1f190e27f4cf04`; CI [#45223945028](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45223945028) in progress; CodeQL [#45223945031](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/45223945031) passed. #421 post-merge checks were green on `050b53b...`; #422 exact-head checks were green before merge.
- **Enterprise main:** `ad1046ab454badf0a568c2e7e9557dfc9e49ac1f`; post-#326 CI/CodeQL/RustSec queued. Prior #329 main CI, CodeQL and RustSec all passed.
- **Public #420:** head `6f8397ae23288d07a9f540502537f0a8ea6005f4`; Ubuntu native and installer-size in progress; CodeQL/Android pass; macOS/Windows native pass.
- **Public #424:** head `064d792f4a9f9a5ba63d43623ffc817d734c57bc`; Rust core, frontend, workflow lint and RustSec pass; native/lint/Android/CodeQL/installer checks in progress or queued.
- **Public #423:** this current evidence update; fresh CI/CodeQL/installer checks queued/in progress.
- **Enterprise #330:** head `abe86ac5f5f9bc705526f27382e426b50bc0ecf8`; Linux/Windows/PostgreSQL, RustSec, CodeQL/security-watch pass; macOS queued.
- **Enterprise #326:** merged at `ad1046ab454badf0a568c2e7e9557dfc9e49ac1f`; post-merge checks queued.
- Reviews: comment reviews on Public #420, Public #424 and Enterprise #330; no approvals. Branches retained.

### Integration boundary findings retained

- Public peer `TaskEnvelope` v2 and Enterprise `WorkloadManifest` v1 remain incompatible; do not reinterpret either as shared v1. Shared v1 is not active transport/dispatch.
- The shared envelope's required approvals are role labels, not verified admin/owner signatures. Production must authenticate the key-to-principal/device/tenant mapping and validate live consent, trust and policy before an atomic replay/lease admission.
- Public local receipt v3 is hash-chained metadata, not a worker-signed execution receipt. Enterprise's durable ledger tracks leases/telemetry. The shared signed receipt schema is not wired to live task admission or ledger ingestion.
- Model/runtime v2 parity is a descriptor compatibility result, not live inventory. Public's current discovery is Ollama-local; selected model-to-real inference and artifact provenance/size/licence verification remain pending. Enterprise needs authenticated enrolled-device inventory and privacy-minimised operator projection. No prompt/private content crosses.
- Day 3 (23 Nov) work: merged reminder snooze safety and rebased boundary tests. Filesystem identity race #408 and PDF/device-validation #70 remain blocked on platform-native implementation/probes; no speculative Rust rewrite or manual device PASS is claimed.

