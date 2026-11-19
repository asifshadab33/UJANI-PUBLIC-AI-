# UJANI Public AI — implementation and release status

**Snapshot:** 2026-11-19 (GitHub evidence checked during this session). This is a point-in-time status, not unattended progress or a security certification.

## Current main

- Main SHA: `d34c47e2e9dc52c544a33626db9bb46caeea2712`.
- PR #347 clock-reliability feature merged.
- Post-merge main CI and CodeQL: **passed**.
- Recent merged security/reliability work includes #336 freshness warning, #338 release-gate checks, #340 history/cache consent hardening, #348 hash-bound scan evidence and #347 timestamp/clock reliability.

## Active change

- PR #351: [verify cancellation of non-streaming Ollama response](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/351)
- Exact head: `ef4b98d5c0a2ac16f09ccf057a689758dc76ff55`.
- Passed at exact head: Rust lint, Rust core tests, Rust dependency audit, frontend dependency audit, React/TypeScript build, workflow lint, JavaScript/TypeScript CodeQL, Rust CodeQL, Android ARM64 compile, security-sensitive change watch, macOS installer.
- Pending at last live check: native tests on Linux/Windows/macOS; Linux and Windows installer builds.
- **Do not merge** until all required exact-head checks pass and mergeability/head SHA are refreshed.

## Release blockers and verification limits

- Full native IPC command authorization/denial matrix and alternate invocation surfaces need security review; the source-mapped command manifest is an inventory guard, not runtime authorization proof.
- Complete application-level OS sandboxing is not implemented/verified across supported platforms. PDF worker confinement has documented gaps: macOS relies on deprecated `sandbox-exec`; Windows Job Objects do not themselves deny filesystem/network access; Linux worker-level restrictions require device validation.
- Managed model downloads remain disabled until artifact provenance, licence, checksum, storage/size and compatibility evidence are verifiable.
- Filesystem reparse-point/symlink and TOCTOU race validation, history key recovery/rotation, PDF upload/cancel/freeze regression tests, fuzzing and recovery cleanup remain in the release backlog.
- Production signing/updater provenance and rollback, physical-device install/launch tests, and independent external security review remain unverified.
- Worker/Partner and peer execution remain disabled. Local-first processing and explicit cloud consent remain the product boundary.

## Honest status

UJANI Public AI remains **pre-alpha / controlled-testing candidate**, not production-ready. Hosted CI results are useful evidence but are not physical-device testing, signing validation, sandbox certification or independent review.

## Next work

1. Complete PR #351 exact-head checks; fix failures rather than suppress them.
2. Continue IPC command authorization and filesystem-boundary review with focused tests.
3. Complete PDF worker/platform containment work and regression evidence.
4. Improve model catalogue/setup explanations and verified hardware/runtime compatibility without enabling downloads prematurely.
5. Perform install/launch, freeze recovery, file/PDF, cancellation, privacy and update/rollback tests on actual supported devices.
