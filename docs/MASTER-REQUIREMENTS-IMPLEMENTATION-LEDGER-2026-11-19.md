# UJANI Public AI — Master Requirements & Implementation Ledger

**Evidence snapshot:** 2026-11-19 evening. Public `main` at `6b4ab2ec5ccae5528b5de3e4b1ee16e63100d32f`. CI is in progress and CodeQL has passed on this exact SHA; this is not a release certification. Recheck live refs before acting.

## Product invariants

- Public AI is a local-first personal runtime. Prefer device and OS capabilities before external infrastructure.
- Public and Enterprise remain separate products, authorities, permissions, data stores and independently usable applications.
- No silent cloud transfer, model install/switch, background compute, device wake, destructive action, arbitrary shell or model-authored executable code.
- Fortress blocks UJANI cloud requests at application policy; it is not a substitute for OS isolation.
- Fresh installs start with no selected model. The user chooses an installed model or explicitly opts into Automatic; UJANI does not silently switch models.
- UJANI-managed model downloads remain disabled until artifact identity, licence, digest, size, model-store path and free storage can be verified.
- Worker/Partner, peer dispatch and remote execution remain unavailable until authenticated transport, receiver authorization, dual consent, quotas, isolation, replay resistance, receipts and recovery pass.

## Requirements and implementation status

| Area | Status | Remaining evidence |
|---|---|---|
| Desktop shell and local chat | Prototype implemented | Physical-device stability and supported-OS validation |
| Loopback Ollama | Local adapter implemented | Redirect/proxy isolation, timeout/cancel and runtime compatibility regressions |
| Model discovery and choice | Local Ollama discovery, evidence UI and explicit manual/opt-in automatic choice implemented | Additional runtime adapters; verified licence, digest, quantisation and exact-size catalogue data |
| Hardware/runtime inventory v2 | Local producer and evidence-qualified UI implemented | GPU/NPU/driver coverage and broader runtime evidence |
| Catalogue and RAM guidance | Canonical catalogue, mirror parity guard and RAM-aligned recommendation merged | Cross-repository generated-mirror automation and stronger publisher/artifact provenance |
| Fortress/Smart and cloud consent | Policy foundation implemented | OS-level isolation and provider-specific negative-path coverage |
| Worker/Partner | Disabled | Authenticated peer protocol, receiver policy, consent, durable replay, lease, isolation and signed receipts |
| Native command registry | Audit maps and regression tooling exist | Command-by-command runtime authorization and denial-before-side-effect proof |
| Documents, folders and PDF | Bounded partial workflows | PDF worker confinement, format coverage, filesystem TOCTOU/symlink/reparse-point tests and cancellation/freeze regressions |
| Search, duplicate and large-file tools | Read-only foundations | Cross-platform coverage, reliability and performance bounds |
| History, cache and receipts | Privacy/consent foundations | Key rotation/recovery, deletion, AAD and crash-recovery evidence |
| Planner and OS connectors | Not integrated | Consent-based adapters for calendar, email, meetings, notes and reminders |
| Security Centre | Partial | Read-only provider adapters, freshness and uncertainty warnings |
| Self-evolution | Restricted foundation | Protected-core governance and independently verified approval/replay |
| Installers and updates | CI pipeline foundation | Production signing, SBOM/provenance, updater rollback and physical install/upgrade/uninstall |
| Release readiness | Not achieved | All P0 security, privacy, reliability, PDF/Ollama, platform and external validation gates |

## Ordered master backlog

### P0 — release and security blockers

1. Finish native IPC command-by-command authorization, alternate invocation and denial-before-side-effect/receipt regressions.
2. Complete Ollama no-redirect/no-proxy, timeout, cancellation, model-pull host validation and failure-recovery tests; keep managed pulls disabled until exact artifact and storage checks exist.
3. Complete PDF upload, parsing, cancellation and freeze regressions; prove OS-enforced worker isolation rather than relying on application flags alone.
4. Complete filesystem symlink/reparse-point, path traversal, TOCTOU, size/depth/time and cancellation coverage across platforms.
5. Complete history/cache consent ordering, AES-GCM AAD, key rotation/recovery, deletion and crash-recovery tests.
6. Remediate or precisely bound Rust dependency advisories. A successful audit with non-failing warnings is not a clean dependency set.
7. Require exact-head CI, CodeQL, dependency audits, Linux/Windows/macOS native tests and installer/package checks before release-candidate consideration.

### P1 — useful first-release work without weakening safeguards

1. Improve local model discovery, model-selection explanations and evidence freshness; preserve explicit owner choice and no automatic download.
2. Finish the canonical operation manifest and keep native command inventory, policy, consent, adapters and receipt requirements aligned.
3. Expand OS-powered file/content search, folder sorting, duplicate detection and cache invalidation without duplicating OS facilities.
4. Improve accessible feature/permission explanations, progress, cancellation, navigation and error recovery.
5. Add optional consent-based planner/connectors only through narrow, revocable adapters.

### P2 — after foundations

1. Optional OCR, vision, audio and video capabilities behind verified runtime/tool adapters.
2. Typed task graphs and advanced planning after authorization, resource bounds and receipts are complete.

## Deduplication and compatibility rules

1. `catalogs/local-model-catalog-v1.json` is the canonical Public catalogue. Regenerate the single Public UI mirror with `scripts/sync_model_catalogue.py`; CI checks byte parity.
2. Shared Public/Enterprise catalogue and schema artifacts are pinned by the shared-contract manifest. Do not add another catalogue source or runtime dependency.
3. Inventory v1 and v2 remain separately versioned; do not infer v2 evidence from v1 fields.
4. Operation/tool/capability registries, router, task graph, policy gate and native command map must converge on one canonical operation definition; policy and adapter responsibilities remain distinct.
5. Filesystem features reuse `safe_fs`, traversal, cancellation and resource-limit primitives. Cache, history and receipts remain separate privacy purposes.

## Repository evidence at snapshot time

- Merged today: #365 native-command manifest mapping regressions; #366 cloud-gateway DNS answer-set filtering tests; #369 catalogue-aligned model RAM recommendations.
- Earlier same-day merged work includes #351 Ollama non-streaming cancellation, #354 evidence-qualified model inventory v2 UI and #363 history deletion input validation.
- Current main CI [44582163967](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44582163967) is in progress; CodeQL [44582164172](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44582164172) passed. Linux, Windows and macOS native jobs must finish before this SHA is considered green.
- The preceding main SHA `ebd909a04a16659c314927ee3c192efbcbec8034` had CI/CodeQL cancelled when #369 advanced main; the earlier `95cd331ca870a3ca4de534997ee07f508a00669a` baseline passed its CI and CodeQL workflows.

## Open PR snapshot

- **#364** — catalogue mirror parity guard and this ledger; exact-head checks are rerunning after removal of a redundant Clippy binding.
- **#367** — readable model-evidence timestamps; macOS native and installer jobs remain in progress.
- **#368** — Feature & Permissions Centre accessibility; macOS installer job was cancelled and needs a successful rerun.
- **#370** — cross-platform file-name validation and safe folder-import skip/report behavior; most checks passed, with Linux native and Android ARM64 reruns still in progress and macOS checks pending.
- **#371** — cross-repository contract/status documentation; exact-head CI is running after snapshot and audit wording corrections.

Do not merge from this snapshot alone. Merge only an unchanged exact head that is mergeable, has all required CI/security/platform checks passed and has no unresolved review blockers. Do not delete unmerged branches.

## Completion and release rule

A requirement is complete only when its implementation is on `main`, exact post-merge CI/security evidence is green and the tested scope is stated. Schema fixtures, audit maps, static UI and synthetic data do not prove runtime authorization, production services or physical-device behavior. Production signing, KMS, updater rollback, deployment and physical install tests require external devices, credentials and infrastructure. No release is authorised by this ledger.
