# UJANI Public AI — implementation and release status

**Snapshot:** 2026-11-19 evening. This is a live GitHub evidence snapshot, not unattended progress or a security certification.

## Current main

- Main SHA: `ebd909a04a16659c314927ee3c192efbcbec8034`.
- This main includes today's merged PRs #365 (native-command manifest mapping regression tests; merge `9b74ec8dc4f3c2ae2e024b4939f1f114f7289c37`) and #366 (cloud-gateway DNS answer-set filtering tests; merge `ebd909a04a16659c314927ee3c192efbcbec8034`).
- Current-head CI [44579400789](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44579400789) and CodeQL [44579408265](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44579408265) are queued. Do not describe this post-merge SHA as green yet.
- The immediately preceding main SHA `95cd331ca870a3ca4de534997ee07f508a00669a` passed CI [44549577584](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44549577584) and CodeQL [44549577477](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/44549577477), including Rust lint/core tests, frontend and Rust dependency audits, TypeScript build, workflow lint, and native tests on Linux, Windows and macOS.
- Today's earlier merged work also includes #351 (non-streaming Ollama cancellation regression), #354 (evidence-qualified model inventory v2 UI), and #363 (history deletion input validation before receipt). Earlier same-day changes include clock reliability, history/cache consent ordering and hash-bound file-scan evidence.

## Current open PRs

| PR | Exact head | Latest evidence at this snapshot |
|---|---|---|
| [#364](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/364) | `e88d8691745f31a4d35fb385efc0cc38ed3c44d3` | Removed a Clippy clone-on-copy warning. CI queued, installer-size audit running; Android, CodeQL and security-watch checks passed. |
| [#367](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/367) | `c5b3ac14d9e769ace576676201fa270643681d3b` | Timestamp test now asserts the complete formatted value. CI and installer audit queued; CodeQL/security watch passed. |
| [#368](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/368) | `095807597b5d69e569b38f02181196ef49bd4470` | Added a named capability region and assertions for both ARIA label targets; both review threads resolved. CI/installer audit queued; CodeQL/security watch passed. |
| [#369](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/369) | `b9274dff110e4df89cc7b82e01e2a03575e54fe3` | Installer-size audit, CodeQL and security watch passed. A prior macOS native job was cancelled; its rerun was requested and the CI run remains queued. |
| [#370](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/370) | `152f32ccb28602ca8570a1ab65e0753602932cf6` | Fixed the static security-map check to require the shared display-name validator's public cross-crate export. The prior failure was a stale `pub(crate)` expectation; new CI, CodeQL, Android and installer checks are pending. |
| [#371](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/371) | `4f17f612347996a3a0287269ec8403bf3da4e415` | Cross-repository contract/status documentation; exact-head CI and CodeQL queued. |

Open PR heads and base branches must be rechecked before merging; none of the above is counted as merged.

## Model/runtime intelligence

- Public's local model/runtime v2 producer and evidence-qualified UI are merged. It uses existing bounded OS and loopback Ollama observations, keeps collection `personal_local_only`, distinguishes verified/reported/estimated/unknown evidence, and preserves explicit user choice.
- No prompt, file content, path, credential or conversation data is sent to Enterprise. No model is automatically selected, downloaded, installed or switched. Managed downloads remain disabled pending provenance, licence, checksum, size/storage and fresh compatibility validation.
- Shared v2 schema/vector parity does not create an Enterprise transport or authorization path.

## Release blockers and verification limits

- Native IPC inventory and manifest checks are audit aids, **not runtime authorization proof**. Issue #120 remains open for command-by-command policy, consent, receipt, denial-before-side-effect and alternate invocation coverage.
- The latest completed main Rust dependency audit reported **7 allowed warnings**: unmaintained `chrono` (RUSTSEC-2026-0543), `proc-macro-error` (RUSTSEC-2024-0370), `ttf-parser` (RUSTSEC-2026-0192), `zbus_names` (RUSTSEC-2026-0371), `zvariant` (RUSTSEC-2026-0370), `zvariant_derive` (RUSTSEC-2026-0372), and unsound `glib::VariantStrIter` (RUSTSEC-2024-0429). A green allowlisted audit is not a clean dependency set.
- Complete application-level OS sandboxing and PDF-worker confinement are not independently verified across platforms. Windows Job Objects alone do not deny filesystem/network access; macOS `sandbox-exec` is deprecated; Linux restrictions still require device validation.
- History key rotation/recovery, filesystem symlink/reparse-point and TOCTOU tests, PDF upload/cancel/freeze regressions, fuzz campaigns, crash recovery, production signing, SBOM/provenance, updater rollback and physical install/upgrade/uninstall tests remain separate gates.
- Worker/Partner, peer execution and remote task dispatch remain disabled. No production release is authorised by this status update.

## Next work

1. Complete current-head CI for #364, #367–#371; resolve failures, stale bases and review blockers before merging.
2. Finish native IPC authorization/negative tests and filesystem/PDF boundary review.
3. Remediate or precisely bound the seven Rust advisories with cross-platform evidence.
4. Complete OS-enforced PDF/application isolation, key recovery and crash/cancel regressions.
5. Run physical-device installation, upgrade, uninstall, minimum-OS, signing and rollback validation; commission independent security review.
