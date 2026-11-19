# Public AI Preservation and Verification Baseline

**Audit date:** 2026-11-19  
**Audited base:** `main` at `ce7d72d55312b230d4585c13f45db471f1e06131`  
**Scope:** Read-only inventory review. This document does not certify runtime security or replace command-by-command testing.

## Governing rule

**Preserve first. Verify second. Extend third.**

Existing authorization, consent, encryption, local-network boundaries, resource controls, audit receipts, cancellation, and OS permission integrations are protected foundations. New capability work must reuse them where appropriate and must not create a parallel authority path.

## Evidence reviewed

- `docs/native-command-security-map.json`, blob `a33fb3c8408157edce1efbdfde9100a26f566036`.
- The manifest contains **51 command records** across default and peer-preview command inventories.
- **17 records** are explicitly marked `source-mapped; full review pending`.
- The manifest separately marks peer-preview-only commands, debug-only mocks, intentionally disabled operations, partial wiring, and command-level denial-matrix work.
- The manifest itself states that command-level review and denial coverage remain pending for multiple entries.

These are inventory facts only. A source mapping is not proof that the runtime boundary enforces authorization, consent, cancellation, resource limits, or receipts in every path.

## Preservation classifications

Every command and security-relevant subsystem must be classified independently:

1. **Implemented and verified** — preserve implementation and regression tests.
2. **Implemented, verification pending** — retain; add focused tests and evidence before changing it.
3. **Partially integrated** — complete the missing connection with the smallest safe patch.
4. **Missing** — design an isolated addition behind existing policy boundaries.
5. **Intentionally disabled** — keep disabled until all explicit release gates pass.
6. **Potential duplicate** — compare semantics and tests before any consolidation; do not delete based on names alone.

## Required next verification

### P0 — native command authorization
- Build a command-by-command matrix covering caller identity, capability, consent, policy generation, data classification, resource bounds, cancellation, receipt semantics, and failure behaviour.
- Prove denial occurs **before** task registration, filesystem/network/model side effects, or persistent mutation where applicable.
- Add negative tests for missing, expired, revoked, mismatched, stale, and replayed authorization evidence.
- Verify read-only operations remain read-only and peer-preview/debug-only commands cannot become production execution paths.
- Trace every command through UI/IPC, Rust command handler, shared policy gate, operation registry, executor, and receipt/audit layer.

### P0 — preserve existing security boundaries
- Ollama remains loopback/local-only by default; redirects and proxy leakage remain blocked; timeouts and cancellation must cover actual HTTP work.
- Encrypted history/cache integrity and key-loss handling must fail closed without plaintext fallback or destructive key regeneration.
- Filesystem traversal, symlink/reparse-point and TOCTOU checks must be tested at the native boundary.
- PDF and document processing must retain bounded input, memory, CPU/time, cancellation, and OS-enforced isolation where claimed.
- Model output and capability discovery are advisory inputs, never authorization.

### P1 — release confidence
- Reconcile dependency advisories without blind version bumps or broad suppressions.
- Verify freeze/navigation/upload/cancellation regressions.
- Run exact-head native tests and installer builds on supported platforms; record physical-device and signing/updater evidence separately.

## Change-control requirements

Before each implementation batch, record:
- exact base and head commit SHAs;
- files and trust boundaries touched;
- why the change is necessary;
- security properties preserved;
- positive and negative regression tests;
- exact CI jobs and their commit SHA;
- unresolved risks and external validation needs.

Do not rewrite a working security module merely to fit the adaptive capability architecture. Do not add arbitrary shell execution, model-generated executable code, silent background compute, automatic external transfer, or remote execution.

## Current conclusion

The manifest is a useful audit index, not a security certificate. The next safe implementation unit is the P0 native-command denial matrix and focused tests, beginning with commands that mutate persistent state, alter security/device identity, or transfer data. Existing security code should remain unchanged unless a demonstrated defect requires a narrowly scoped correction.
