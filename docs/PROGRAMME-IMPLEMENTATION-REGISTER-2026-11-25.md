# Public AI implementation register — 25 November 2026

This register is an execution aid, not a release claim. It records the next independently reviewable work slices and preserves incomplete work for later foundation integration. Update statuses only with linked evidence and exact commit SHA.

## Evidence labels

- **Implemented**: code exists; no test claim implied.
- **Unit-tested**: relevant focused tests pass locally or in a named workflow.
- **CI-verified**: exact current commit SHA has passing required hosted checks.
- **Integration-verified**: complete cross-component workflow has passed.
- **Device-verified**: installed application has passed on a physical target device.
- **Blocked**: dependency, platform capability, credentials, or independent review is missing.
- **Release-ready**: all applicable gates have evidence; not currently asserted for this product.

## Current baseline

- Default branch: `main`
- Baseline SHA at register creation: `72d6e2d09b41098afa7f5c5344a97ec4b71cf651`
- Product status: pre-alpha / controlled preview.
- Public and Enterprise remain independently buildable and usable. This repository must not require the Enterprise service.
- Worker and Partner remain preview-only; no remote execution is enabled.

## Ordered implementation batches

| Order | Work slice | Existing tracking | Current evidence/status | Acceptance evidence still required |
|---|---|---|---|---|
| P0-1 | Native IPC command-to-capability authorization matrix; deny before side effect | #120 | Partial policy gates and command inventory; universal enforcement not verified | Every registered native command mapped to capability/data scope/side effects; negative tests for filesystem, document, transfer, history, model pull and updater |
| P0-2 | Filesystem selected-root race resistance | #408 | Handle-relative/no-follow design documented; separate TOCTOU audit remains open | Deterministic replacement/symlink/reparse-point tests on Linux, macOS and Windows; prove no outside-root read |
| P0-3 | PDF worker fail-closed behavior and freeze regression | #70 | Recent main commit `af60e5015a760e2fc1c2503acb8c5f41c471364b` deliberately disables PDF operations until isolation is verified | Platform worker containment, cancellation/timeout/cleanup, corrupt/oversized PDF tests, installed-app regression |
| P0-4 | Dependency advisory migration | #73 | Advisory inventory remains; do not suppress advisories without documented reachability/risk | Minimal compatible upgrades, lockfile diff, full Linux/Windows/macOS CI and audit |
| P0-5 | Ollama adapter and model-pull hardening | Existing runtime/security backlog | Loopback-only inference exists; host validation, redirects/proxy behavior, bounded progress, cancellation and timeout need exact tests | Malicious redirect/proxy/local-host tests, pull size/progress caps, cancellation and real-runtime smoke tests |
| P0-6 | History/receipt integrity and privacy lifecycle | Existing history/receipt backlog | AES-GCM history fields and local hash-chain receipt foundations exist | Key rotation/recovery, migration, concurrent init, deletion/export boundaries, cancellation/failure receipt coverage |
| P1-1 | Model Intelligence: discovery, provenance and explicit model choice | Model catalogue work | Publisher/licence provenance recently added; discovery and model selection are not yet a complete real inference workflow | Installed runtime discovery, observed/runtime/catalogue/estimated evidence labels, hardware fit, no-model state, explicit choice and selected-model inference |
| P1-2 | OS-backed local file assistant | #44, #46 | Selected-file/folder primitives exist; full user workflow not verified | OS folder picker, bounded search, preview, cancellation, model-assisted explanation, user approval before modifications |
| P1-3 | Reminders and Personal Intelligence | #83 | Local planner/reminder foundations and agenda tests exist | Persistence/restart, timezone/DST, notifications, consent-based calendar/mail connector and deletion tests |
| P1-4 | Installer and minimum OS matrix | #67 | Unsigned preview artifacts exist in CI; not production signed/device certified | Install/upgrade/uninstall and launch tests on supported OS versions; signing and rollback evidence |
| P2-1 | Screenshot capture and richer media tools | #419 and media roadmap | Not a first-release dependency; current image support is metadata/header inspection | Explicit OS permission, bounded image dimensions/memory, no silent capture |
| P2-2 | Security Watch and optional email alerts | #179, #183 | Not a complete user-facing feature | Local integrity/policy signals, opt-in notification configuration, verified event source and privacy review |
| P2-3 | Android feasibility | Android roadmap | CI compatibility check is not an Android product workflow | Separate scope/UX/security feasibility decision before implementation |

## Execution rules

1. Work in small PRs with one acceptance objective; avoid parallel branches changing the same security boundary.
2. Do not claim PDF processing works while the platform worker isolation check is unavailable; fail closed.
3. Do not treat hardware presence as proof of GPU/NPU runtime compatibility. Unknown stays unknown.
4. Model downloads remain optional and require explicit user choice; display provenance/licence and resource requirements before download.
5. Local model selection must not silently switch to cloud. Any external transfer remains separately consented and data-classification gated.
6. Do not accept model-generated paths, shell commands or arbitrary code as authority.
7. Keep test status separate from exact-head CI, integration, physical-device verification and release readiness.

## Next implementation owner/order

1. Finish P0-1 native authorization and P0-2 filesystem boundary before expanding native file operations.
2. Reproduce and close P0-3 PDF/freeze regressions on supported platforms.
3. Migrate P0-4 dependencies without suppressing unresolved advisories.
4. In parallel where files do not overlap, complete P1-1 real model discovery/selection and P1-3 reminder persistence.
5. Keep the local file-assistant P1-2 workflow scoped to read/preview until write approval is separately enforced.

## Current release decision

**Not release-ready.** No release publication, signing claim, production sandbox claim or Worker/Partner remote execution is authorized by this register.
