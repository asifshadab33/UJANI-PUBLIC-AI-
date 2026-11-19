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
- Main: `010b24a927a75411e8f7c6d2ec46cb4c3ca6e9fb` (PR #336 security-update freshness merged). Post-merge CI #44451529292 is still running (Linux/Windows native pending); CodeQL #44451529244 passed.
- #340 `22f23e9168ee62866f6889875ce51038100499f4`: history/cache consent consolidation; workflow lint and security-sensitive check passed; Linux/macOS/Windows native tests in progress, Android/installer/CodeQL in progress, Rust core/lint/TypeScript/audits queued; unsigned preview skipped.
- #332 `1866b8f413efda4ca6f6fd9cc2990bd3a35fb1d0`: digest integrity/model inventory v2; Rust core, Linux native, workflow lint, TypeScript, frontend/Rust audits and security watch passed; macOS/Windows native and installer audit in progress; Android and CodeQL passed. Keep draft; branch is 3 commits behind main.
- #334 closed as superseded by current-main PR #343; branch retained.
- #343 `3995edf70eb9e1ca6ae4a9db0c7d0bbf2d8b7a3b`: fail-closed file-scan evidence plus control-character provider-version regression; exact-head CI, CodeQL, Android and installer audit queued; unsigned preview skipped.
- #341 `d8a75b1c4723cf73a7d4407b1a9fe7c49a214e50`: handbook/file-threat delivery-gate documentation; CI, CodeQL and installer audit queued; security-sensitive watch passed.
- #324/#328: stale clock/Ollama cancellation work; update/revalidate before merge.
- #333/#337 closed as superseded by #341; #334 closed as superseded by #343; #339 closed as superseded by #342. Original branches retained.
- #342: current-main master-backlog documentation PR; CI, CodeQL, installer and security-sensitive checks queued; unsigned preview skipped. Draft.

## Evidence rules
Exact head SHA required. No green claim from partial/stale checks. Public main CI is still running; physical devices, production signing, updater rollback and independent security tests are not verified.

**Verified shared-schema evidence (2026-11-19):** Both main branches have byte-identical `schemas/shared-contract-manifest-v1.json` (Git blob SHA `5c41a57411453145f0d80a470f9985375b6e3ca4`). The manifest pins the same model/runtime inventory v1 and v2 schema, v1 task-contract schema, consent-binding vector, and v1/v2 inventory vectors. This verifies artifact parity—not production transport compatibility, authenticated inventory admission, or remote execution.