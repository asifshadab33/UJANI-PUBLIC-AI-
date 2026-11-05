# UJANI Public AI — Development Changelog

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

