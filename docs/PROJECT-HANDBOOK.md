# UJANI Public AI — Project Handbook

> A navigable, maintained handbook for users, developers, testers and maintainers. It is an index into the source of truth, not a substitute for code, tests, platform documentation or security review.

**Project state:** pre-alpha / controlled preview. Read [Implementation Status](IMPLEMENTATION_STATUS.md) and [Release Gates](PROJECT_STATUS_AND_RELEASE_GATES.md) before treating any feature as production-ready.

## Contents

1. [Start here](#1-start-here)
2. [Build from zero](#2-build-from-zero)
3. [How a request travels through UJANI](#3-how-a-request-travels-through-ujani)
4. [Repository map](#4-repository-map)
5. [Native command and permission index](#5-native-command-and-permission-index)
6. [Models and local runtime](#6-models-and-local-runtime)
7. [Tests, checks and CI](#7-tests-checks-and-ci)
8. [Troubleshooting](#8-troubleshooting)
9. [Security and privacy rules](#9-security-and-privacy-rules)
10. [Change, release and maintenance procedure](#10-change-release-and-maintenance-procedure)
11. [Glossary and further reading](#11-glossary-and-further-reading)

## 1. Start here

- Ordinary users: [Desktop setup](../apps/desktop/README.md), [model selection onboarding](MODEL-SELECTION-ONBOARDING.md), [execution modes](EXECUTION-MODES.md), [platform support](PLATFORM_SUPPORT.md).
- Developers: [Architecture](ARCHITECTURE.md), [Contributing](../CONTRIBUTING.md), [Security](../SECURITY.md).
- Testers: [manual smoke-test matrix](PUBLIC_AI_MANUAL_SMOKE_TEST_MATRIX.md), [external validation checklist](EXTERNAL-VALIDATION-CHECKLIST.md).
- Maintainers/reviewers: [implementation status](IMPLEMENTATION_STATUS.md), [native command audit inventory](NATIVE-COMMAND-AUDIT-INVENTORY.md), [release gates](PROJECT_STATUS_AND_RELEASE_GATES.md), [claims ledger](IMPLEMENTATION_CLAIMS_LEDGER.md).

This handbook describes the repository layout and known workflows as inspected on 18 November 2026. Re-check linked source files and CI before relying on a command count, path, supported platform or release claim.

## 2. Build from zero

### 2.1 Prerequisites

- Git.
- Node.js 20 or newer and npm.
- Rust stable toolchain with Cargo.
- The operating-system prerequisites for Tauri 2: <https://v2.tauri.app/start/prerequisites/>.
- Windows: Microsoft C++ Build Tools and WebView2 Runtime.
- For real local inference: Ollama installed separately and bound to loopback. It is optional; UJANI does not bundle Ollama or a model.

### 2.2 Obtain source

```sh
git clone https://github.com/asifshadab33/UJANI-PUBLIC-AI-.git
cd UJANI-PUBLIC-AI-
```

### 2.3 Check the Rust workspace

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
```

### 2.4 Start the desktop browser preview

```sh
cd apps/desktop
npm ci
npm run dev
```

The browser preview is a UI/demo path. It cannot exercise native Tauri commands or prove access to device hardware.

### 2.5 Run the native desktop application

From `apps/desktop`:

```sh
npm ci
npm run tauri dev
```

Build a local package (not a signed production release):

```sh
npm run tauri build
```

Native builds require the platform-specific Tauri prerequisites. A successful compile is not proof that an installer runs correctly on a physical device.

### 2.6 Optional Ollama setup

1. Install Ollama from its official distribution.
2. Keep its API bound to `127.0.0.1:11434`; do not expose it to an untrusted network.
3. Review a model's exact artifact, quantisation, licence, download/storage size and hardware needs before installing through your trusted Ollama interface.
4. Refresh UJANI's inventory and inspect reported capabilities and unknown fields.
5. Select the model explicitly. No silent model switch, runtime start, model pull or cloud fallback is permitted.

UJANI-managed model pulls are currently fail-closed/disabled until artifact provenance and daemon-store disk preflight can be verified. See [model selection onboarding](MODEL-SELECTION-ONBOARDING.md).

## 3. How a request travels through UJANI

1. The UI collects a request and the user's selected execution/privacy mode.
2. A typed Tauri IPC command receives untrusted arguments.
3. Native code validates sizes, formats, identifiers, permissions and task state.
4. The capability/policy layer determines whether the requested registered operation is allowed.
5. A least-privilege adapter performs the permitted local operation (for example, a bounded parser or loopback-only Ollama request).
6. Cancellation, timeout, progress and failure are surfaced to the UI.
7. Any external transfer uses a separate explicit, task-scoped consent and approval path.
8. Results and provenance are returned; persistence is optional and subject to separate user settings.

This is an architectural explanation, not a claim that every command has passed its full security audit. See [architecture](ARCHITECTURE.md), [execution modes](EXECUTION-MODES.md) and [native command audit](NATIVE-COMMAND-AUDIT-INVENTORY.md).

## 4. Repository map

The following is a guide to the principal top-level areas; it is not an exhaustive file listing.

| Path | Purpose |
|---|---|
| `apps/desktop/` | Tauri 2 + React/Vite desktop application |
| `apps/desktop/src/` | Frontend UI and TypeScript application code |
| `apps/desktop/src-tauri/` | Rust native host, IPC commands, adapters and Tauri configuration |
| `crates/ujani-core/` | Shared task, capability, policy and model-advisor types |
| `catalogs/` | Curated local model catalogue data |
| `schemas/` | Versioned JSON contracts and test vectors |
| `fuzz/fuzz_targets/` | Parser fuzz targets |
| `tests/` | Repository-level regression and validation tests |
| `scripts/` | Inventory, schema, release-readiness and contract checks |
| `docs/` | Architecture, security, product, validation and release documentation |
| `.github/workflows/` | CI, CodeQL, dependency, platform and release workflows |

To inspect the live tree from a checkout:

```sh
git ls-files
find apps crates schemas scripts tests docs .github/workflows -maxdepth 3 -type f | sort
```

Do not assume that a path in an old handbook still exists: check `git ls-files` on the exact branch/commit being built.

## 5. Native command and permission index

The authoritative registered command list, conditional feature-gated commands, current command count, command areas and known security gaps are maintained in [Native IPC Command Inventory](NATIVE-COMMAND-AUDIT-INVENTORY.md). The machine-readable security map is [native-command-security-map.json](native-command-security-map.json).

### Open-source completeness and version warning

For the final public release, the Native IPC Command Inventory and this handbook are intended to be the complete public reference for the software we actually ship. The command index must account for every registered command (including platform- or feature-gated commands) and, for each one, explain its purpose, inputs and validation limits, permissions/consent requirements, side effects, outputs and errors, cancellation/timeout behaviour, platform availability, implementation location and relevant tests. It must also link to the supporting operations, scripts, configuration, folder map and troubleshooting guidance needed to understand and maintain the complete application. Anything not shipped must be clearly labelled as experimental, unavailable or planned—not presented as a working command.

UJANI Public AI is open source. Subject to the repository's open-source licence and applicable law, members of the public may inspect, modify, extend, fork and redistribute it. This handbook is documentation, not a restriction on those freedoms.

> **Version warning:** This handbook and command index describe the software version/commit identified by the documentation. If later development changes commands, folders, configuration, permissions, interfaces or behaviour, the technical handbook and command index may also need to change. Always use the documentation corresponding to the exact version being built; do not assume an older handbook describes newer code.

Useful source entry points:

- Tauri command registration: `apps/desktop/src-tauri/src/lib.rs`.
- Native command implementations: `apps/desktop/src-tauri/src/` and its submodules.
- Shared task and capability contracts: `crates/ujani-core/src/`.
- Frontend invocation sites: `apps/desktop/src/`.

### Command review rule

Before adding, changing or removing an IPC command:

1. Find its registration and every caller.
2. Trace the full call path into the native implementation and adapter.
3. Treat every webview argument as untrusted; never trust a hidden button or caller-supplied boolean as authorization.
4. Validate identifiers, paths, sizes, enum values, counts, timeouts and output limits at the native boundary.
5. Identify every side effect (filesystem, database, runtime, process, network, model download, OS setting).
6. Check authorization/consent and denial-before-side-effect ordering in native code.
7. Add negative tests for malformed, unauthorized, stale, cancelled and resource-exhausting requests.
8. Update the inventory and security map, then run their repository validation scripts.

The inventory is not a blanket security approval. Issue #120 tracks the broader command-by-command audit.

## 6. Models and local runtime

- [Model intelligence and runtime discovery](MODEL-INTELLIGENCE-AND-RUNTIME-DISCOVERY.md)
- [Model runtime inventory v2](MODEL-RUNTIME-INVENTORY-V2.md)
- [Model selection onboarding](MODEL-SELECTION-ONBOARDING.md)
- [Hardware-aware defaults](HARDWARE-AWARE-MODEL-DEFAULTS.md)

Important distinctions:
- `model_runtime.rs` deterministic mock behaviour is not real inference.
- The Ollama adapter is the real local inference integration and is loopback-only.
- Reported capabilities are not proof of quality or performance.
- Unknown CPU/GPU/VRAM/storage/capability values must remain unknown.
- RAM fit is an estimate, not a guarantee.
- Model download, installation, runtime startup and cloud transmission require separate explicit user actions/consent.

## 7. Tests, checks and CI

Common Rust checks:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

The Clippy command may expose feature/platform-specific issues; report the exact result and do not weaken lints or disable tests to obtain green CI.

Other maintained checks include:

- `python scripts/test_native_command_inventory.py`
- `python scripts/test_native_command_security_map.py`
- `python scripts/validate_model_runtime_inventory_schema.py`
- `python scripts/public_release_readiness.py`
- `npm test` / other scripts defined in `apps/desktop/package.json`.

Inspect the package scripts before assuming a command exists:

```sh
cd apps/desktop
npm run
```

CI workflows are under `.github/workflows/`. Verify each result against the exact PR head SHA; a green result on an older commit does not validate a newer commit. Read failed job logs and fix the underlying regression.

## 8. Troubleshooting

| Symptom | Checks and safe recovery |
|---|---|
| Browser preview loads but native features fail | Expected: browser preview has no Tauri native bridge. Run `npm run tauri dev` with Tauri prerequisites installed. |
| Tauri native build cannot find platform libraries | Follow the official Tauri 2 prerequisites for the host OS; install missing system packages/build tools, then retry. |
| Ollama is unavailable | Confirm Ollama is installed and running locally; refresh status. Check that it is bound to loopback and listening on port 11434. Do not expose the API publicly to make UJANI connect. |
| No model appears | Install a compatible model through a trusted Ollama interface, then refresh inventory. UJANI does not silently download models. |
| Model is not recommended | Review model size, quantisation, context, reported capabilities and RAM estimate. Unknown evidence is not treated as compatible. Choose another model or use deterministic local tools. |
| Local inference times out | Reduce input/context size, check device resources and Ollama logs locally, retry once. Do not assume cancellation instantly stops every downstream daemon operation. |
| PDF is rejected | Check the documented file-size/page/text limits and whether it is scanned. OCR is not implemented unless a future release explicitly documents it. |
| Folder/file search returns fewer results | Check selected root, depth/entry/time budgets, permissions and skipped symlinks/reparse points. UJANI intentionally does not scan the entire device. |
| History/cache is unavailable | Check the user's explicit enablement and OS credential-vault availability. Do not bypass encryption or copy database files to another location as a workaround. |
| A task appears stuck | Use the visible cancel control, wait for bounded cleanup, then inspect app logs and the matching native task/CI evidence. Capture OS, app version, task type and reproduction steps; never include prompts, secrets or private files in a public issue. |
| CI fails only on one OS | Open that exact job log, identify target-specific imports/APIs/permissions, add a platform-gated fix and regression test. Do not mark unsupported platforms as passed. |
| Dependency audit fails | Identify the direct/transitive dependency and advisory, check upstream patched versions and lockfile reproducibility. Do not suppress the advisory or force an unreviewed override. |
| Update/install fails | Verify the release artifact source, signature and checksum. Do not bypass signature checks or install an untrusted build. |

### What to include in a bug report

- UJANI version/commit and operating system/version.
- Whether this is browser preview or native app.
- Exact action sequence and expected/actual behaviour.
- Relevant non-sensitive error text and timestamp.
- Whether it reproduces after restart and on another supported device.
- CI workflow/job URL and exact head SHA, if applicable.

Never post API keys, private signing keys, passwords, prompts, personal documents or unredacted logs.

## 9. Security and privacy rules

- No silent background compute or model/runtime startup.
- No unrestricted shell or model-generated code execution.
- No cloud transfer without fresh, task-scoped user consent.
- Fortress must not use cloud AI.
- Use OS APIs rather than reimplementing OS functionality where practical.
- Keep file access rooted in the user's selected scope and handle symlinks/reparse points safely.
- Keep model/runtime endpoints loopback-only by default.
- Keep history/cache opt-in and encrypted where specified; metadata may have separate limitations.
- Report unknown evidence as unknown; never convert missing evidence into a green status.

Report vulnerabilities privately as described in [SECURITY.md](../SECURITY.md).

## 10. Change, release and maintenance procedure

1. Inspect current `main`, open PRs, exact SHAs and workflow results.
2. Find an existing issue/PR before creating new tracking.
3. Create a focused feature branch from the current base SHA.
4. Inspect source and tests before editing.
5. Make small changes and add regression/negative tests.
6. Run relevant formatting, lint, unit/integration and schema/inventory checks.
7. Push and verify the exact commit and changed files on GitHub.
8. Inspect all required workflows and failed logs for the exact head.
9. Keep the PR open until review/security blockers and all required checks are resolved.
10. Do not publish a release without explicit release gates, signing verification and approval.

See [Public release evidence gate](PUBLIC-RELEASE-EVIDENCE-GATE.md), [project release gates](PROJECT_STATUS_AND_RELEASE_GATES.md) and [updater feed](UPDATER-FEED.md). Never place release secrets in source control or logs.

## 11. Glossary and further reading

- **IPC:** communication between the frontend webview and the native Tauri host.
- **Adapter:** a typed, bounded integration with a local runtime or OS capability.
- **Evidence/provenance:** where a reported capability or status came from and how fresh it is.
- **Fail closed:** deny an operation when a required security condition cannot be verified.
- **Loopback:** a service reachable only from the same device, typically `127.0.0.1`.
- **Task-scoped consent:** approval bound to a specific operation and data/endpoint context, not blanket permission.

Start with [Architecture](ARCHITECTURE.md), [Threat and security review](SECURITY_REVIEW_2026-11.md), [Platform support](PLATFORM_SUPPORT.md), [PDF worker security](PDF-WORKER-PLATFORM-SECURITY.md) and [release gates](PROJECT_STATUS_AND_RELEASE_GATES.md).
