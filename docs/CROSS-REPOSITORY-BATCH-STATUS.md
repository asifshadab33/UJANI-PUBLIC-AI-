# UJANI cross-repository batch status

**Evidence snapshot: 20 November 2026.** This is an ordered execution register, not a release certification. Update it only from current repository heads, PRs and check evidence. The two products remain independently installable and usable.

## Current main

- Public AI: `main` last verified at `e22a31fc7912d7d4db72327e7e95a964b6e8381f` (recent-file preview PR #395 merged after #397/#401).
- Enterprise AI: `main` last verified at `1f70b5d1ce5b08c4a2bed3ebcc16fc7f845db224` (strict operator-projection JSON PR #312 merged).
- Shared task contract, consent vector, model-runtime inventory v2 schema and validator are mirrored; parity must be checked by exact blob SHA or equivalent schema/fixture comparison each integration pass.

## Batch execution order

Batches run concurrently when independent; dependencies and release gates remain enforced.

1. **Stability and safety (P0):** resolve Public filesystem root/descendant TOCTOU and Windows reparse/drive restrictions; cancellation/timeout, UI hang, PDF/Ollama, history/cache/privacy and IPC regressions; add regression tests and run current-head native workflows. Do not merge filesystem preview features until path/handle boundaries are reviewed and current-base checks pass.
2. **Model Intelligence v2 (P1):** merge mirrored vocabulary/unknown-status/provider-selection tests only after exact-head CI and current-base review; preserve explicit user selection, no silent fallback, no auto-install. Expand runtime adapters only with supported, testable implementations.
3. **Contract compatibility (P1):** strict duplicate-key/unknown-field decoding, cross-language signing vectors, schema parity, consent binding, task-state/expiry/replay/receipt tests. Draft schema is not a transport or execution protocol.
4. **Public local-first utilities (P1/P2):** complete safe preview-only recent-file and empty-folder tools; then large-file discovery, image metadata/preview and organisation plans. Use OS primitives rather than reimplementing OS features. No content indexing/background scans without consent and bounded budgets.
5. **Planner and connectors (P2):** typed task graph adapters, deterministic local tools, user-approved calendar/notes/reminder connectors; no connector writes without specific user approval.
6. **Enterprise trust and control plane (P0/P1):** authenticated enrollment/intake, tenant RBAC, device owner opt-in + admin approval, protected key custody/revocation, durable replay and transactional leases, live inventory/telemetry and read-only operator UI.
7. **Enterprise resource scheduling (P1/P2):** resource/thermal/interactive-aware inventory and planning, quotas, cancellation/expiry/recovery, benchmarks and transparent energy/cost evidence. Continue useful read-only/operator features while execution safety is developed.
8. **Isolation and receipts (P0 before execution):** OS-enforced sandbox and egress, verifiable signed receipt/output binding, ledger integration, rollback/recovery, penetration/threat review. Remote execution stays disabled until all prerequisites pass.
9. **Release evidence (P0):** exact main SHA CI, CodeQL, dependency audits, Linux/Windows/macOS native tests and package/installer checks; manual physical-device, signing, KMS/TPM, deployment/rollback and independent tests listed separately.

## PRs currently requiring follow-up

- Public #397 JPEG header dimensions: merged as `a102ed0552decd46801173c46b8538b0972319b9`; exact-head CI, CodeQL, Android check and installer audit passed before merge; main-branch workflow evidence has not yet been returned by the connector.
- Public #395 recent-file preview: merged as `e22a31fc7912d7d4db72327e7e95a964b6e8381f`; exact-head CI, CodeQL, Android compatibility and installer audit passed before merge.
- Public #396 empty-folder preview: draft; hosted CI passed, but a verified selected-root TOCTOU redirection risk remains (symlink swap between root check and canonicalization); cancellation checks also need tightening. Not mergeable from a safety perspective.
- Public #400 async host-probe offloading: draft; exact-head checks passed, but current PR metadata reports mergeable=false (conflict/outdated base); do not merge until refreshed/reconciled.
- Public #401 model-runtime v2 capability boundary: merged as `669a98b79eff3340e77f19efdb450ace88e5a83f`; exact-head CI, CodeQL and Android checks passed before merge. Post-merge main workflow evidence remains pending.
- Public dependency PRs #384, #391, #388–#390, #385–#387: assess independently; do not batch blindly. Failed platform/native checks are blockers.
- Public #404 strict duplicate-key schema validator: draft; Python/Rust core tests, TypeScript, workflow lint, dependency audit and Rust lint have passed; native Linux/Windows/macOS checks still in progress.
- Enterprise #312 strict duplicate-key projection validator: merged as `1f70b5d1ce5b08c4a2bed3ebcc16fc7f845db224`; CI, CodeQL, dependency audit and security watch passed before merge.
- Enterprise #313 strict duplicate-key model-inventory intake parser: first CI run caught a missing trait qualification and then formatting; both were fixed on the branch. New exact-head CI is running; do not merge until green.

## Release posture

- **Public AI:** pre-alpha/controlled preview, not release-candidate. Filesystem, PDF/Ollama, privacy/reliability and latest-main cross-platform release gates remain.
- **Enterprise AI:** policy/control-plane prototype, not production remote-worker ready. Remote execution/dispatch remains disabled.

## Evidence discipline

Record PR head SHA, base SHA, exact check conclusions and main post-merge workflow results. A PR head green does not mean main green after merge. Stale, partial, skipped or in-progress checks are not green. Do not publish releases, sign artifacts, enable remote execution, or delete branches without the required evidence/authorization.
