# UJANI Public AI — implementation and release status

**Snapshot:** 2026-11-19 evening. This is a live GitHub evidence snapshot, not unattended progress or a security certification.

## Current main

- Main SHA: `6b4ab2ec5ccae5528b5de3e4b1ee16e63100d32f`.
- This main includes today's merged PRs #365 (native-command manifest mapping regressions; merge `9b74ec8dc4f3c2ae2e024b4939f1f114f7289c37`), #366 (cloud-gateway DNS answer-set filtering tests; merge `ebd909a04a16659c314927ee3c192efbcbec8034`) and #369 (catalogue-aligned model RAM recommendations; merge `6b4ab2ec5ccae5528b5de3e4b1ee16e63100d32f`).
- Current-head CI [44582163967](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44582163967) is in progress; CodeQL [44582164172](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44582164172) has passed. Do not describe this post-merge SHA as fully green until native Linux/Windows/macOS jobs finish.
- The immediately preceding main SHA `ebd909a04a16659c314927ee3c192efbcbec8034` had CI [44579400789](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44579400789) and CodeQL [44579408265](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44579408265) cancelled when PR #369 advanced main. The earlier baseline `95cd331ca870a3ca4de534997ee07f508a00669a` passed CI [44549577584](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44549577584) and CodeQL [44549577477](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44549577477), including Linux/Windows/macOS native tests.
- Today's earlier merged work also includes #351 (non-streaming Ollama cancellation regression), #354 (evidence-qualified model inventory v2 UI), and #363 (history deletion input validation before receipt). Earlier same-day changes include clock reliability, history/cache consent ordering and hash-bound file-scan evidence.

## Current open PRs

| PR | Exact head | Latest evidence at this snapshot |
|---|---|---|
| [#364](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/364) | `b6ddd05f89f8a265eb0403b39757e7537e9f1dff` | Removed a redundant nonce binding after Clippy failed on the prior head; exact-head CI rerun is in progress/queued. Do not merge until every required check passes. |
| [#367](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/367) | `c5b3ac14d9e769ace576676201fa270643681d3b` | Rust tests/lint/audits, TypeScript build, CodeQL, Linux/Windows native tests and Linux/Windows installers passed; macOS native and installer jobs remain in progress. |
| [#368](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/368) | `095807597b5d69e569b38f02181196ef49bd4470` | All functional, native and security checks passed; macOS installer was cancelled and needs a successful rerun. |
| [#370](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/370) | `152f32ccb28602ca8570a1ab65e0753602932cf6` | Cross-platform filename validation now skips and reports unsafe folder entries without discarding valid files. Rust/frontend tests, audits, CodeQL and Linux/Windows installers passed; Linux native and Android ARM64 reruns are in progress, with macOS native/installer pending. |
| [#371](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/371) | `cb988d329a4999b09258d4d76dd59f6b89e04c99` | Updated audit wording and named CodeQL in the merge prerequisite. Exact-head CI is in progress; CodeQL analysis is still running. |
Open PR heads and base branches must be rechecked before merging; none of the above is counted as merged.

## Model/runtime intelligence

- Public's local model/runtime v2 producer and evidence-qualified UI are merged. It uses existing bounded OS and loopback Ollama observations, keeps collection `personal_local_only`, distinguishes verified/reported/estimated/unknown evidence, and preserves explicit user choice.
- No prompt, file content, path, credential or conversation data is sent to Enterprise. No model is automatically selected, downloaded, installed or switched. Managed downloads remain disabled pending provenance, licence, checksum, size/storage and fresh compatibility validation.
- Shared v2 schema/vector parity does not create an Enterprise transport or authorization path.

## Release blockers and verification limits

- Native IPC inventory and manifest checks are audit aids, **not runtime authorization proof**. Issue #120 remains open for command-by-command policy, consent, receipt, denial-before-side-effect and alternate invocation coverage.
- The latest completed main Rust dependency audit reported **7 non-failing advisory warnings** (the audit job does not fail on these findings; this is not an explicit advisory allowlist): unmaintained `chrono` (RUSTSEC-2026-0543), `proc-macro-error` (RUSTSEC-2024-0370), `ttf-parser` (RUSTSEC-2026-0192), `zbus_names` (RUSTSEC-2026-0371), `zvariant` (RUSTSEC-2026-0370), `zvariant_derive` (RUSTSEC-2026-0372), and unsound `glib::VariantStrIter` (RUSTSEC-2024-0429). A successful audit job with these warnings is not a clean dependency set.
- Complete application-level OS sandboxing and PDF-worker confinement are not independently verified across platforms. Windows Job Objects alone do not deny filesystem/network access; macOS `sandbox-exec` is deprecated; Linux restrictions still require device validation.
- History key rotation/recovery, filesystem symlink/reparse-point and TOCTOU tests, PDF upload/cancel/freeze regressions, fuzz campaigns, crash recovery, production signing, SBOM/provenance, updater rollback and physical install/upgrade/uninstall tests remain separate gates.
- Worker/Partner, peer execution and remote task dispatch remain disabled. No production release is authorised by this status update.

## Next work

1. Complete current-head CI and CodeQL for #364, #367–#371; resolve failures, stale bases and review blockers before merging.
2. Finish native IPC authorization/negative tests and filesystem/PDF boundary review.
3. Remediate or precisely bound the seven Rust advisories with cross-platform evidence.
4. Complete OS-enforced PDF/application isolation, key recovery and crash/cancel regressions.
5. Run physical-device installation, upgrade, uninstall, minimum-OS, signing and rollback validation; commission independent security review.
