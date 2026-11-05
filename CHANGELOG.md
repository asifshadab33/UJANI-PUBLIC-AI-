## 2026-11-06 — Fixed local workflow execution and model guidance

### Added and improved
- A native command validates the registered schema-v1 document workflow before invoking the one fixed local analyse → summarise → optional translate pipeline.
- Installed Ollama model-fit guidance based on reported model disk size and current OS-available RAM. The estimate is advisory; the app never silently switches models.
- A persistent model-advice panel with visible loading/error states and a retry action, including when the first request fails.
- Additional core tests for the exact advisory budget boundary, deterministic tie ordering and an empty model catalogue.

### Verification
- The current main commit's GitHub Actions run passed the Rust core tests, Tauri native compile check and React/TypeScript build.
- The changes in this section require a new CI run after they are pushed; no test result is claimed for this follow-up before that run completes.

### Limitations retained intentionally
- Arbitrary task graphs are not executable; only the fixed document workflow is.
- Model-fit estimates are not guarantees or hard execution gates.
- Worker/Partner cloud execution, SQLite history, verified GPU/NPU scheduling, full image decoding/OCR and filesystem sandboxing remain unimplemented.

# UJANI Public AI — Development Changelog

## 2026-11-05 — Bounded workflow graph planning

### Added
- A versioned schema-v1 task graph with a maximum of 32 nodes and 16 dependencies per node.
- Validation for allow-listed operation IDs, valid and unique node IDs, missing/duplicate/self dependencies, and dependency cycles.
- Deterministic topological ordering and conservative sequential peak-memory preflight based on registered operation metadata.
- A Tauri command and desktop UI preview for an analyse → summarise → translate workflow.
- Core tests for valid ordering, unsupported operations, invalid dependencies, cycles, schema/mode/node limits, missing capabilities and memory constraints.

### Explicit limitations
- Workflow graph planning is not general graph execution. It does not execute nodes, reserve CPU/RAM, open files, grant permissions, or create a sandbox.
- The existing document summarisation pipeline remains a separate fixed workflow.
- Worker/Partner remain unavailable for execution because no secure external gateway exists.

This log records shipped repository changes. GitHub's commit metadata is the authoritative UTC timestamp for each push; links below open the exact commits.

## 2026-11-05 — Local execution and data tools

**Time:** See UTC timestamps on the linked GitHub commits (individual commits were pushed during this development session).  
**Type:** Local runtime integration and safety hardening.

### Added
- A loopback-only Ollama adapter in the desktop Tauri application. It checks `127.0.0.1:11434`, lists locally installed models, and runs non-streaming generation with connection/request timeouts and prompt/response size limits.
- A native local AI command for Fortress and Smart modes. It evaluates the existing four-mode policy first and invokes Ollama only when the local route is allowed.
- A hard no-cloud behavior for Worker and Partner execution until a secure external gateway is implemented.
- In-memory text analysis for user-selected text files, limited to 1 MiB, with word/line counts, preview and frequent keywords.
- Optional local Ollama summarisation for selected text files up to 24 KiB.
- In-memory image-header inspection for user-selected PNG, JPEG, GIF, WebP and BMP files, with a 20 MiB original-file limit and only the first 64 KiB sent across Tauri IPC.
- A trusted-operation allow-list in the core router.
- Desktop UI for Ollama status/model selection, local AI results, and file/image selection.
- Documentation of Ollama setup and current capability boundaries.

### Security and design decisions
- Ollama requests are directed to a fixed loopback URL; there is no configurable remote host or cloud fallback.
- No API key or cloud credential is accepted or stored by this integration.
- File tools accept the bytes/content selected by the user; they do not accept arbitrary filesystem paths.
- Image inspection reads metadata only. It is not OCR or visual image understanding; only 64 KiB of header data crosses the UI/native boundary.
- Fortress currently means UJANI does not send task data to cloud AI. The app has not implemented OS-level firewall/network isolation; Ollama is contacted over loopback HTTP.
- GPU/NPU detection, model downloads, task decomposition, sandboxing, and the Worker/Partner gateway remain future work.

### Main files affected
- `crates/ujani-core/src/router.rs`
- `crates/ujani-core/src/lib.rs`
- `crates/ujani-core/src/file_processor.rs`
- `crates/ujani-core/src/image_processor.rs`
- `apps/desktop/src-tauri/Cargo.toml`
- `apps/desktop/src-tauri/src/ollama.rs`
- `apps/desktop/src-tauri/src/task_engine.rs`
- `apps/desktop/src-tauri/src/lib.rs`
- `apps/desktop/src/App.tsx`
- `apps/desktop/src/styles.css`
- `apps/desktop/README.md`
- `README.md`

### Key commits
- [Ollama adapter](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/3a354ff7a7079e02e96767129fc8100370c0673c)
- [Safe in-memory file and image processors](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/40415e5393879fc4dd81258bf9a7dbc94d17baae)
- [Router operation allow-list](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/4317368dfaf7366bdcc5a24eacaf4d1a9077e929)
- [Native Ollama and file/image commands](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/e830d4ce6821ba2a19311b8c4854031fd265d540)
- [Desktop UI integration](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/19923afff2db3cceff2b1a047a4c5e53e66d7c83)
- [Desktop and project documentation](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/c4eab0a1d5ade632bfb037035f02290fd21f7547)
- [Bound Ollama response memory](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/d49534c6b3be05c37f27cd7407c1180537bb7961)
- [Bound image-header IPC data](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/81ed6f563ad21d4971249d60ec7bea3b552b84f8)
- [Add local text summarisation](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/1130426a723e7cd4f789579e806fa2edcbd787ab)
- [Create native local task engine](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/4a12f231cc7e06331a160362bea4a4ff5acd6218)
- [Add CI for core, native and frontend checks](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commit/18a8ba37e8457dddd80f3d7fbb2c94440cebdf47)

## Earlier development

See the repository commit history for the earlier four-mode router, Tauri scaffold, model adapter prototype and security foundations:
https://github.com/asifshadab33/UJANI-PUBLIC-AI-/commits/main

## Future entry template

```markdown
## YYYY-MM-DD — Short change title

**Time:** HH:MM UTC (use the GitHub commit timestamp)  
**Type:** Feature / Fix / Security / Documentation

### Added or changed
- What changed and why.

### Files affected
- `path/to/file`

### Commit
- [Commit title](GitHub commit URL)
```

## 2026-11-06 — Integrated local document pipeline

**Type:** Feature integration and reliability fix.

### Added
- A single native `process_selected_document` operation now validates a user-selected text document, runs bounded text analysis, and then requests a summary from the selected local Ollama model.
- The desktop UI now calls the integrated pipeline and displays the analysis and summary returned by the same native operation.
- Added native validation tests for allowed modes, file names, empty content, and the 24 KiB AI-processing limit.

### Reliability and safety
- Repaired Rust source syntax in the Ollama retry helper/tests from the previous patch.
- Rejected traversal-like local model identifiers and bounded the Ollama availability check.
- The pipeline accepts file content only, not a filesystem path. It supports Fortress and Smart only, uses the loopback Ollama service, and has no cloud fallback.
- This is a first integrated text-document workflow, not yet a general multi-step planner, PDF extractor, OCR engine, or task-graph executor.

## 2026-11-06 — PDF workflow, local translation and computer capabilities

**Type:** Local-first feature integration.

### Added
- User-selected PDF text extraction using a bounded in-memory payload (10 MiB file limit), with CPU-heavy parsing dispatched through Tokio's blocking worker pool.
- Extracted PDF text is analysed and summarised by the selected local Ollama model; the current AI text limit is 24 KiB.
- Explicit local translation command for Fortress and Smart modes, bounded to 16 KiB and routed only to the loopback Ollama adapter.
- Read-only operating-system locale detection for the computer capability panel.
- A resource centre showing logical CPU count, available RAM, OS locale, local Ollama availability and honest boundaries around GPU/NPU and browser capabilities.
- Manual browser translation option that opens Google Translate without transferring or pre-filling user text.

### Limits and safety
- Image-only/scanned PDFs return a clear message; OCR is not implemented.
- Browser translation is a user-controlled copy/paste workflow. The current app does not read browser tabs, automate a browser session or silently submit text to a translation provider.
- The OS locale is a language preference, not a built-in translation service. OS speech/OCR/translation packs and browser translation APIs need separate platform-specific adapters.
- Local model compute is performed by Ollama; its CPU/GPU usage depends on the installed runtime, model and drivers. UJANI does not yet reserve CPU, inspect GPU/NPU readiness or enforce resource quotas.
- PDF extraction and local translation are prototype features and require native build and runtime testing before release.

## 2026-11-06 — Capability registry and local task planner

**Type:** Local-first orchestration foundation.

### Added
- Read-only capability snapshot for CPU, available memory, Ollama loopback health, PDF text extraction and image-header inspection.
- GPU and NPU are represented as unknown until actual hardware/runtime compatibility checks exist; they are never assumed available.
- Trusted built-in operation catalogue for local AI, document summarisation, PDF extraction/summarisation, translation, text analysis and image-header inspection.
- Conservative local task-plan preview that checks required capabilities and a coarse minimum available-memory threshold.
- Desktop UI to refresh capability status, select an allow-listed operation and inspect its proposed local plan.
- Core unit tests for unknown operations, missing local model runtime, memory preflight, PDF requirements, local-only mode and non-execution guarantees.

### Safety boundaries
- A successful plan is not an execution grant: no task is started and no CPU/RAM is reserved.
- Capability snapshots are point-in-time and do not yet include expiry/freshness metadata or resource budgets.
- Worker/Partner and external scheduling remain unavailable until a separately controlled gateway is implemented.
