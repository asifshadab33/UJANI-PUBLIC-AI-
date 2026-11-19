# UJANI Public AI — implementation and release status

**Snapshot:** 2026-11-19. Evidence was checked against GitHub during this run; this is a point-in-time status, not unattended progress or a security certification.

## Current main

- Main SHA: `ce7d72d55312b230d4585c13f45db471f1e06131` (PR #351 merged).
- Pre-merge main `d34c47e2e9dc52c544a33626db9bb46caeea2712` CI and CodeQL passed.
- Post-merge CI/CodeQL for `ce7d72d55312b230d4585c13f45db471f1e06131` are pending at this snapshot.
- Recent merged work includes #336 freshness warning, #338 release gates, #340 history/cache consent, #348 hash-bound scan evidence, #347 clock reliability, and #351 non-streaming Ollama cancellation regression.

## Merged cancellation regression (#351)

- Exact head: `ef4b98d5c0a2ac16f09ccf057a689758dc76ff55`.
- Passed exact-head: Rust lint, Rust core tests, Rust dependency audit, frontend dependency audit, React/TypeScript build, workflow lint, native tests on Linux/Windows/macOS, installer size audit on Linux/Windows/macOS, JavaScript/TypeScript and Rust CodeQL, Android ARM64 compatibility, security-sensitive change watch.
- Unsigned preview installers were skipped (no signing/release).
- Merged as `ce7d72d55312b230d4585c13f45db471f1e06131`.
- The test uses a local TCP server with an incomplete chunked response and verifies cancellation closes the response; it does not enable any new remote route.

## Active PRs

- #353 [native command audit-manifest contract tests](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/353), exact head `a035f0a398cbbc907705a5e18e19fda1e8d6b423`: CI, CodeQL and installer audit are still in progress/queued.
- #354 [evidence-qualified model inventory v2 UI](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/354), exact head `03fbec05766052be3edbe73672c71ca223d555ac`: local Node helper tests passed 6/6; exact-head CI, CodeQL and installer audit are pending/in progress. This presents existing bounded local metadata only; unknowns stay unknown and it does not select/download/run models.
- This status-document PR #352 is open; update it before merging if the snapshot changes.

## Release blockers and verification limits

- Full native IPC command authorization/denial matrix and alternate invocation surfaces still need security review. The source-mapped manifest is not proof of runtime authorization.
- Rust desktop dependency audit still reports **7 allowed warnings** (glib, proc-macro-error, ttf-parser, zbus_names, zvariant, zvariant_derive, chrono); see issue #73. The audit job's green status is not a clean dependency audit.
- Complete application-level OS sandboxing is not implemented/verified across platforms. PDF worker confinement has documented gaps: macOS uses deprecated `sandbox-exec`; Windows Job Objects do not deny filesystem/network access by themselves; Linux worker restrictions require device validation.
- Managed model downloads remain disabled pending provenance, licence, checksum, size/storage and compatibility evidence.
- Filesystem symlink/reparse-point and TOCTOU validation, history key recovery/rotation, PDF upload/cancel/freeze regression tests, fuzzing, recovery cleanup and platform-specific containment remain.
- Production signing/updater provenance/rollback, physical-device install/launch tests and independent external security review remain unverified.
- Worker/Partner and peer execution remain disabled. Local-first processing, visible user control and explicit cloud consent remain the product boundary.

## Honest status

UJANI Public AI remains **pre-alpha / controlled-testing candidate**, not production-ready. Hosted CI is evidence for those exact commits, not physical-device testing, signing validation, sandbox certification or independent review.

## Next work

1. Complete exact-head CI for #354 and #353; fix failures rather than suppress them.
2. Continue command-by-command IPC review and filesystem-boundary tests.
3. Resolve the seven Rust dependency warnings through reproducible, cross-platform-tested migrations.
4. Complete PDF worker/platform containment and regressions.
5. Perform physical install/launch, freeze recovery, file/PDF, cancellation, privacy and update/rollback testing.
