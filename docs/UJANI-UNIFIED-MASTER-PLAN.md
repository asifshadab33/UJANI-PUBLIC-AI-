# UJANI Unified Master Development Plan

**Baseline:** 20 November 2026. **Status:** consolidated planning baseline, not release certification.

## 1. Purpose and non-negotiable principles
UJANI makes computing resources already available to people and organisations useful for AI, reducing avoidable dependence on centralised AI data centres and unnecessary energy use.
- Local-first/privacy-first: Public AI uses the user's device by default; external processing is per-task, disclosed, minimised and explicitly approved.
- OS-first/lightweight: use safe OS capabilities and existing tools instead of duplicating OS functions or adding unnecessary services.
- Consent/authority: no silent model installation, background compute, device waking, undisclosed data transfer or destructive side effects. Enterprise work requires organisational authorisation and device-owner consent.
- Security before convenience: fail closed when identity, permission, freshness, provenance, resource-limit or isolation evidence is missing.
- Truthful claims: schemas, policy primitives, tests, demos and CI passes are not proof of complete runtime functionality.
- No premature distributed execution: keep dispatch/worker execution disabled until identity, confidential transport, receiver-side authorisation, replay protection, tenant isolation, quotas, OS sandboxing, cancellation and independent review are proven.
- No unsupported energy claims: compare equivalent completed workloads including device, network, retry, failure and cooling overhead. Never wake sleeping devices; prefer central execution on ties or unknowns.
- No feature duplication: extend the existing owner module. Do not add a second planner, model registry, cache, consent system, task router, history store, scheduler or connector. Every requirement needs a unique ID, owner, integration point, acceptance tests and duplicate/conflict check.

## 2. Product boundaries
**Public AI:** free, local-first personal desktop assistant/runtime for ordinary computers. Prioritise small-model-first operation, OS-powered utilities, user-controlled local models, bounded workflows and optional consent-based integrations. The local core must not require a UJANI-hosted inference service or subscription.
**Enterprise AI:** organisation-controlled, on-premises-capable fleet/resource orchestration for authorised existing devices. Prioritise trust, inventory, resource availability, energy-aware scheduling and operator visibility. Security integrations and Collective Defence are optional secondary capabilities.
**Shared protocol:** keep repositories independently governed. Maintain one versioned Personal-to-Enterprise task/capability protocol with compatibility fixtures and version policy. Share semantics and fixtures; do not force a common code crate without demonstrated benefit.

## 3. Complete functional scope
### Public AI
- Setup/onboarding: explain modes, privacy, supported OS, permissions, Ollama prerequisite, optional models, uninstall and recovery.
- Adaptive Capability Architecture (ACA): capability registry, trusted discovery/provenance, compatibility preflight, lifecycle, optional install/update/removal, rollback and recovery.
- Hardware/model intelligence: CPU/RAM/GPU/NPU/storage/runtime detection; small-model-first recommendations; task suitability; verified artifact/quantized size; download/storage needs; RAM/platform requirements; licence and limitations; user choice and optional install.
- Local AI: loopback-only Ollama, model selection, explicit context sizing, bounded inputs/outputs, cancellation, timeouts, progress and safe recovery.
- Everyday OS tools: file/folder/content/PDF/image/video search, duplicate and large-file detection, safe document/image organisation, opening results in native apps. Use native OS APIs; require approval for changes.
- Document/media: text/PDF extraction, OCR/image understanding where available, safe limits, format support, temporary-file cleanup and bounded processing.
- Productivity: local tasks/reminders/planner; optional Outlook, Zoom, calendar, notes and other connectors with scoped access, read/write approval, revocation and visible side effects.
- General assistance: summarisation, rewriting, grammar, translation, basic Q&A and task-specific helpers; prefer deterministic local tools where sufficient.
- Four modes: Fortress, Worker, Smart and Partner with truthful semantics. Fortress is not OS-wide isolation until enforced. Worker/Partner remain gated until safe pathways exist.
- Trust/control: per-task route disclosure, consent, permission gates, progress, cancellation, receipts/provenance, cache/history retention, deletion, export and recovery.
- Usability: accessible setup/settings, actionable errors, progress/recovery, responsiveness, keyboard support, no hangs and clear local-versus-external explanations.

### Enterprise AI
- Real control plane: authenticated HTTPS service, selected OIDC/SAML-compatible identity, RBAC, tenant boundaries, durable repository, append-only audit and live operator-console integration.
- Device lifecycle: invitation/enrollment, proof of possession, tenant binding, device identity, owner consent, heartbeat, pause/opt-out, rotation, revocation, expiry and recovery.
- ACA/model governance: approved capability/model catalogues, compatibility, artifact provenance, distribution policy, install visibility and rollback.
- Inventory/telemetry: authenticated, fresh, privacy-minimised CPU/RAM/GPU/NPU/runtime/storage/network/power/thermal/availability data with provenance and visible stale/unknown states.
- Resource orchestration: workload classes, priorities/deadlines, queue, durable leases, atomic reservations, quotas, concurrency, cancellation, expiry, checkpoint/retry and crash recovery.
- Admission/energy: owner opt-in, admin policy, already-powered-on device, no interactive use, power/thermal constraints, fresh measured capacity, data-classification/egress approval, OS isolation readiness and equivalent-workload energy comparison.
- Operator experience: live device/site/subnet inventory, freshness, health, model policy, workload/lease status, audit search/export, maintenance and rollout visibility. Demo data must be labelled.
- Optional Security Centre: coordinate with customer-selected firewall, antivirus, endpoint and monitoring tools; do not replace them or claim unproven equivalence.
- Optional Collective Defence: tenant-admin and individual opt-in; privacy-minimised signed tenant-isolated events; replay/poisoning controls; no autonomous destructive response or unrestricted personal-file access.
- Older-network integration: optional lightweight adapters for customer-selected firewall/monitoring modes and tools such as Malcolm; modular, low-cost, vendor-neutral.
- Operations: deployment, upgrades, signed agents, SBOM, backups, disaster recovery, observability, incident response, support and customer-specific compliance mapping.

### Shared and cross-cutting
- Versioned task/capability/model-inventory/status/receipt contracts and cross-language fixtures; consistent consent binding/revocation; resource budgets, cancellation, typed errors, output validation and provenance.
- Security policy parity without coupling the apps into one product; licensing/attribution and third-party disclosures; performance, accessibility, localisation readiness and low-resource support.

## 4. Delivery batches — opened as coordinated workstreams
Parallelise independent work across both repositories. Dependencies and security gates remain binding; a later batch cannot enable a privileged operation just because its code is ready.

| Batch | Public AI | Enterprise AI | Exit evidence |
|---|---|---|---|
| 0 Baseline/deduplication | Inventory modules, commands, providers, tests, UX flows and defects | Inventory policy crates, schemas, migrations, console, adapters, tests and defects | One requirements register, duplicate/conflict map, exact main/PR baseline |
| 1 Reliability/contracts | Fix demonstrated hangs, blocking, cancellation and error-path gaps; map IPC authorisation | Verify ledger/trust/policy/operator contracts; map service/runtime gaps | Regression baseline, failure-path tests, protocol compatibility plan |
| 2 ACA foundations | Capability IDs, registry ownership, discovery provenance, hardware/runtime inventory | Capability/model governance contracts and tenant-scoped inventory model | Versioned ACA schema, compatibility tests, no duplicate registry |
| 3 Compatibility/trust | Catalogue verification, small-model recommendations, resource-fit preflight, safe optional lifecycle | Authenticated device identity, enrollment, owner consent, key lifecycle, revocation and protected key-provider boundary | Negative tests for mismatch, stale/revoked identity, provenance and approval |
| 4 Everyday workflows/telemetry | OS file/folder/content/PDF/image/video search, duplicate detection, safe organisation/native open | Authenticated heartbeat/telemetry ingestion, inventory freshness and read-only operator API | Real app-path tests, bounded processing, tenant isolation, stale-data tests |
| 5 Productivity/operator UX | Planner/reminders, local task cache, consent-based connectors, settings/accessibility | Live dashboard, device/site health, audit search/export and admin workflows | User journeys, connector scope/revocation tests, no fake live data |
| 6 Safe lifecycle/orchestration | Typed task graph, route disclosure, approvals, receipts, cancellation, provenance, bounded adapters | Durable lease/queue/reservation, quotas, scheduling, cancel/expiry/recovery; worker admission design | Fault injection, replay/race/tenant/recovery tests; dispatch remains gated |
| 7 Isolation/advanced optional | OS-enforced Fortress and PDF containment; OCR/vision and advanced local helpers | OS-enforced worker sandbox, optional Security Centre, Collective Defence, legacy-network adapters | Platform enforcement tests and independent review; optional features off by default |
| 8 Release/real-world validation | Signed installers, updater/rollback, OS matrix, physical install, accessibility and user acceptance | Deployment/runbooks, signed agent/update, backup/DR, penetration test, lab pilot and measured energy benchmark | Per-platform/workload release gates; no unsupported security/savings/compliance claims |

## 5. Requirement IDs and single ownership
Use unique IDs: PUB-SETUP, PUB-ACA, PUB-MODEL, PUB-OS, PUB-DOC, PUB-PLAN, PUB-EXEC, PUB-PRIV, PUB-UX, PUB-REL; ENT-CTRL, ENT-TRUST, ENT-ACA, ENT-INV, ENT-LEASE, ENT-SCHED, ENT-SEC, ENT-OPS, ENT-REL; SHR-PROTO, SHR-CONSENT, SHR-COMPAT, SHR-TEST.
Each requirement records user outcome, existing owner module, gap, acceptance tests, dependencies, security/privacy risk, platform coverage and PR/commit evidence. If an existing module owns the capability, extend it. Similar UI components may be reused but must retain distinct permission/data boundaries.

## 6. Status vocabulary
- Implemented: code exists and intended code path is present.
- Implemented-unverified: code exists but integration/runtime evidence is missing.
- Partial: only part of the user outcome works.
- Missing: no adequate implementation.
- Intentionally disabled: safety gate deliberately prevents use.
- Blocked: credential, hardware, platform, dependency or review prevents progress.
- Deferred: postponed with reason and revisit condition.
Record design, code, tests, integration, runtime and release evidence separately. A merged PR is not a finished feature; CI is not physical-device validation; policy tests are not production services; mock screens are not live inventory.

## 7. Mandatory quality/security gates
- Threat model/trust boundary for privileged features; fail-closed authorisation before side effects; least privilege; tenant/user boundary tests.
- No arbitrary shell/model-generated code execution; no silent network/cloud transfer/model download/file mutation/background compute.
- Bound input/output, file count/size, memory, CPU, time, concurrency and network; cancellation must terminate underlying work and clean temporary resources.
- Test traversal, symlink, TOCTOU, SSRF, redirects, proxy, DNS rebinding and IPC boundaries where relevant.
- Authenticated encryption and protected keys; no plaintext fallback.
- Replay, expiry, revocation, concurrency, clock-skew and crash recovery for distributed state.
- Accessibility, responsiveness, low-memory and large-file tests; SBOM, dependency audits, signed artifacts, updater provenance and rollback.
- Independent security review before production/remote-execution claims; user-journey and physical installation tests on supported OS/hardware.

## 8. Known non-equivalences and release limitations
Reconfirm repository status against current main before release. Application policy is not OS sandboxing. A catalogue is not artifact provenance or hardware-fit proof. A local receipt/hash chain is not externally anchored audit. Enterprise policy/file ledger is not a distributed authenticated service. SQL migrations are not a live DB adapter. Signed device commands are not production KMS/HSM/TPM custody. A static demo is not live inventory. Dual-consent primitives are not deployed Collective Defence. Capacity policy is not live telemetry/scheduler. Unsigned CI installers are not signed releases. Estimated energy comparison is not measured savings.

## 9. Execution and merge policy
1. Keep independent Public and Enterprise implementation branches in parallel.
2. Split work by batch and module ownership; avoid giant cross-cutting PRs and duplicate code.
3. Use draft PRs for reviewable work; keep branches unmerged while exact-head tests run.
4. Refresh against current main and inspect file overlap before merging.
5. Run format, lint, unit, integration, security, dependency, cross-platform CI and relevant manual/native tests on exact PR head.
6. Fix failures before merge; never rely on stale CI or a different SHA.
7. Merge only after review, exact-head checks and safety gates pass. Do not ask the user to perform merges.
8. Record requirement IDs, branch, PR, commit, checks, gaps and rollback notes.
9. If a feature needs credentials, hardware, an independent reviewer or physical device, mark it blocked/unverified and continue independent work.
10. No unattended/background work is implied; report when actually waiting for CI/builds.

## 10. Immediate execution
Start Batch 0 in both repositories simultaneously. In parallel, begin independent safe Batch 1 and ACA inventory after assigning file ownership. Do not start dispatch, live worker execution, automatic device participation or other privileged remote actions. Establish the single requirement register and duplicate map first, then execute all eligible batches in parallel while respecting dependencies and security gates.