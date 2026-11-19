# UJANI Consolidated Master Backlog — Public AI

**Snapshot:** 2026-11-19, live repository check. Update only during a development run; no unattended progress is claimed.

## Boundaries
- Public remains standalone, local-first, owner-consented; installed model inventory and private content never go to Enterprise.
- Enterprise remains standalone. Shared task/model schemas are specifications, not runtime dependencies.
- No public release without explicit release instruction/signing. Enterprise remote execution and automated containment remain disabled.

## Ordered work

### P0 — Public release blockers
1. **Native consent and persistence:** complete PR #340 exact-head tests for history/cache opt-in; review all persistence/IPC routes, denial/cancel/timeout, migration, receipt ordering, recovery/rotation.
2. **Model identity/inventory:** PR #332 validate digest integrity before capability/context joins; complete Windows/macOS/Linux native tests, Rust lint/audit, Android, CodeQL, installer size/build checks. Then refresh base and revalidate.
3. **Filesystem/PDF:** no-follow path handling, file limits, PDF parser memory/time limits, cancellation and malformed/oversized tests; threat-scan evidence must fail closed and never imply full OS sandboxing.
4. **Ollama:** local-only network policy, no redirects/proxy leakage, response/body limits, explicit context, timeouts/cancellation, model errors and UI hang regressions.
5. **IPC and policy:** registered command audit, direct IPC denial, policy preflight before side effects, consent decline/timeout, signed/metadata receipts and history integrity.
6. **Dependency and release supply chain:** resolve known audit advisories without suppressions/unreviewed downgrades; signed update metadata, verified download, rollback, reproducible build and installers.
7. **Cross-platform packaging:** current-head native builds/tests for Linux/Windows/macOS, Android compile where applicable, unsigned preview artifacts distinct from signed production packages.

### P0 — Enterprise execution safety (disabled)
1. Authenticated control plane, tenant isolation, enrollment, identity binding, key rotation/revocation.
2. Exact-task dual consent (owner + organisation), policy-generation binding and revocation recheck.
3. Atomic replay/nonce claim + admission + lease/reservation; concurrency and crash recovery.
4. Protected key custody, external anti-rollback and KMS/HSM/TPM validation.
5. OS-enforced process/filesystem/network isolation and CPU/RAM/time/output quotas.
6. Signed/verifiable receipts tied to input/task digest and exact output bytes; state/lease recovery.
7. Production OIDC/HTTP, backup/restore, network partitions, independent penetration testing.

### Cross-repository / Model Intelligence
1. Reconcile task envelope, sender/tenant/device identity, consent, classification, capabilities, units, expiry, replay, state and receipts; implement a narrow authenticated adapter only after semantics match.
2. Keep model-runtime inventory v2 metadata-only; evidence types (observed/reported/catalogue/estimate/unknown), freshness and platform/accelerator fit remain distinct.
3. Public: detect installed models, show evidence/limitations/resources, require explicit model selection; no silent switching/install/cloud.
4. Enterprise: inventory enrolled/authorised devices only; privacy-minimised operator planning, owner/admin conflict and revocation handling; matching remains advisory.
5. Schema compatibility, unknown fields, stale evidence, unsupported runtimes, empty inventories, resource mismatch, consent revocation and owner selection tests.

## Live PR status
- Main: `010b24a927a75411e8f7c6d2ec46cb4c3ca6e9fb` (PR #336 security-update freshness merged).
- #340 `22f23e9168ee62866f6889875ce51038100499f4`: history/cache consent consolidation; CI native tests, Rust tests, lint, TS, audits, Android, CodeQL and installer jobs running; workflow lint and JS CodeQL passed; unsigned preview skipped.
- #332 `1866b8f413efda4ca6f6fd9cc2990bd3a35fb1d0`: digest integrity/model inventory v2; Rust core, Linux/macOS native, workflow lint, TS and frontend/Rust audits passed; Windows native, installer audit in progress; Android and CodeQL passed. Keep draft; branch stale relative to main.
- #334 `e194d3e1f63a0ebfd095b31eee0645a47acb401f`: fail-closed scan evidence; exact-head CI cancelled/Windows pending; stale base, draft.
- #341 `d8a75b1c4723cf73a7d4407b1a9fe7c49a214e50`: file-threat gates + handbook consolidation; checks pending.
- #324/#328: stale clock/Ollama cancellation work; update/revalidate before merge.
- #333/#337 closed as superseded; #339 backlog PR is stale and superseded by this current-main PR. Branches are retained.

## Evidence rules
Exact head SHA required. No green claim from partial/stale checks. Main-branch post-merge CI, physical devices, production signing, updater rollback and independent security tests are not verified in this run.
