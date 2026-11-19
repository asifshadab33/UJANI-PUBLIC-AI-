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
4. **Clock integrity:** wall/monotonic divergence warnings; reject pre-epoch/out-of-range timestamps; use monotonic elapsed-time and TTL logic; ensure wall-clock rollback cannot extend cache/history retention.
5. **Ollama:** local-only network policy, no redirects/proxy leakage, response/body limits, explicit context, timeouts/cancellation, model errors and UI hang regressions.
6. **IPC and policy:** registered command audit, direct IPC denial, policy preflight before side effects, consent decline/timeout, signed/metadata receipts and history integrity.
7. **Dependency and release supply chain:** resolve known audit advisories without suppressions/unreviewed downgrades; signed update metadata, verified download, rollback, reproducible build and installers.
8. **Cross-platform packaging:** current-head native builds/tests for Linux/Windows/macOS, Android compile where applicable, unsigned preview artifacts distinct from signed production packages.

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
- Main: `69f7794bd045dc50e9421b214e1dedd20f6b4d11` (PR #336 security-update freshness and PR #332 local model-inventory v2 producer merged). Post-merge CI #44453580274 pending; CodeQL #44453580233 queued.
- #340 `22f23e9168ee62866f6889875ce51038100499f4`: history/cache consent consolidation; workflow lint, Rust core/lint, TypeScript, frontend/Rust audits, Linux/macOS/Windows native tests, Android, CodeQL and security-sensitive check passed. Windows/Linux installers remain in progress; macOS installer passed. Branch is 1 commit behind main; no changed-file overlap with the inventory-v2 merge was observed, but verify exact base and installer completion before merge.
- #332 `1866b8f413efda4ca6f6fd9cc2990bd3a35fb1d0`: merged as `69f7794bd045dc50e9421b214e1dedd20f6b4d11`. Exact-head Rust core, Linux/Windows/macOS native, Rust lint/audit, frontend/TypeScript, Android, CodeQL, installer-size and security-watch checks passed; unsigned preview skipped. Post-merge CI is pending.
- #334 closed as superseded by current-main PR #343; branch retained.
- #343 closed as superseded by current-main PR #348; branch retained.
- #341 closed as superseded by current-main PR #348; branch retained.
- #324: stale clock-reliability branch, behind main and overlaps #340 history/cache consent; consolidate/rebase before merge.
- #328 closed as superseded by current-main PR #344; original branch retained.
- #333/#337/#334/#341/#343 closed as superseded by consolidated current-main PR #348; #339/#342 closed as superseded by #345. Original branches retained.
- #344 `60e309ff49cacbdb2a3ea1c94ab4d9c2919ad295`: current-main non-streaming Ollama cancellation regression; CI, Android, CodeQL, installer and security watch queued; unsigned preview skipped.
- #347 `ed7f1104d6dbe8ebd6d789c0c16f81dc9bcc9d46`: current-main clock-jump diagnostic and native timestamp range checks. First Android compile failed due a misplaced test; corrected on this head, all CI/security/platform reruns queued; unsigned preview skipped.
- #348 `c5a236658435128684683fed215fbb77e13ae3b1`: hash-bound file-scan evidence plus handbook and delivery-gate documentation, current main; CI/CodeQL/Android/installer/security-watch pending; unsigned preview skipped.
- #345: this current-main backlog PR; exact-head CI, CodeQL, installer and security-watch checks queued; unsigned preview skipped.
- #342 closed as superseded by #345; branch retained.

## Evidence rules
Exact head SHA required. No green claim from partial/stale checks. Public post-merge CI is pending; physical devices, production signing, updater rollback and independent security tests are not verified.

**Verified shared-schema evidence (2026-11-19):** Both main branches have byte-identical `schemas/shared-contract-manifest-v1.json` (Git blob SHA `5c41a57411453145f0d80a470f9985375b6e3ca4`). The manifest pins the same model/runtime inventory v1 and v2 schema, v1 task-contract schema, consent-binding vector, and v1/v2 inventory vectors. This verifies artifact parity—not production transport compatibility, authenticated inventory admission, or remote execution.