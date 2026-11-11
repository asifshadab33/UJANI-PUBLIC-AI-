# Model Intelligence and Runtime Discovery (v1)

UJANI Public AI should first make useful, privacy-preserving use of models and compute already present on the user's device. This contract is the stable metadata boundary for that work; it is not itself a runtime adapter or proof that a model can perform a task.

## First implementation target

1. Discover the local Ollama service through the existing loopback-only adapter and enumerate installed models using its bounded model-list API.
2. Report the host OS, architecture, logical CPU count, total/available RAM and only accelerator facts confirmed by supported OS/runtime APIs. An unknown GPU/NPU state stays unknown.
3. Add future runtime adapters (for example llama.cpp-based local servers) behind explicit, bounded provider adapters. Do not probe arbitrary network hosts, scan arbitrary disk locations, or follow redirects.
4. Distinguish runtime-observed facts, runtime metadata, signed/published catalogue claims and estimates. Never present estimates as benchmark results.
5. Recommend models by the requested task and the device's verified capabilities/resource budget. Explain trade-offs (fit, speed/quality expectations, context and capability evidence); let the user choose, override or keep the current model.

## User-control and privacy invariants

- Discovery is read-only. No model is downloaded, installed, started, switched or removed automatically.
- Inference stays on the selected local runtime unless the user explicitly authorises another execution mode.
- Do not send prompts, file contents, local paths, credentials or conversation history as inventory.
- No arbitrary endpoint discovery. Ollama's current adapter remains loopback-only; future adapters must declare and enforce their network scope.
- Do not claim vision, reasoning, tool use, context length, GPU/NPU acceleration or language quality unless supported by evidence. Label catalogue claims and estimates.
- A missing runtime or empty model list is a normal, explicit state, not an error that blocks other local tools.

## Contract

schemas/model-runtime-inventory-v1.schema.json is shared with UJANI Enterprise AI. Public AI may use a null device_ref; Enterprise may populate it with an enrolled, pseudonymous device reference after consent and policy checks. Keep the schema additive only through a new version; v1 rejects unknown fields to avoid silently accepting unreviewed semantics.

## Current implementation status (2026-11-10)

The first Public AI implementation slice now serializes this v1 contract from the existing read-only Ollama adapter and the OS-reported device profile:

- `get_ollama_status` continues to discover only the fixed loopback Ollama service. It reports installed models from `/api/tags`, runtime version from `/api/version`, active-model GPU-use evidence from `/api/ps`, and model metadata from `/api/show`.
- Metadata inspection is bounded to 32 installed models, at most four concurrent requests, an eight-second aggregate deadline, three seconds per metadata request, and the existing 2 MiB response ceiling. The installed-model list is capped at 512. Unsupported, failed, uninspected or absent metadata stays unknown.
- CPU logical cores and total/available RAM come from OS APIs. GPU is reported only when Ollama reports positive active-model VRAM allocation; this is not total physical VRAM. If the OS reports available RAM above a known positive total, Public AI records that available-memory value as unknown so Enterprise's stricter inventory validator receives a consistent snapshot. NPU and free storage remain unknown because no supported, unambiguous cross-platform evidence is currently collected.
- Ollama-reported capabilities are allow-listed: completion maps to chat, vision to vision, tools to tool use, and embedding to embeddings. Context length is accepted only as a bounded positive integer in model metadata. Reasoning, coding and language quality are not inferred from model names.
- The inventory now includes `estimated_min_memory_mb` using the same local heuristic as model-fit advice (rounded model disk size × 2 + 512 MiB). Estimates above 16,777,216 MiB are withheld as unknown. This is an estimate, not runtime-reported evidence, a benchmark or a fit guarantee; Enterprise must not treat it as verified admission evidence.
- Automatic chat recommendation now requires runtime-reported chat capability as well as the existing advisory RAM-fit heuristic. The UI offers a recommended/smaller-fit choice, a median balanced-fit choice and a largest-fit quality-oriented proxy when distinct fitting candidates exist. These are size/RAM heuristics, not speed or quality benchmarks. New installations start with no model selected; users must explicitly choose a model or select Automatic. An existing saved Automatic preference remains respected and makes its initial choice once; later inventory refreshes never replace a selected model silently. A removed model is cleared and the user must confirm a replacement. A user may also choose manually when metadata is unknown.
- The inventory contains no prompts, file contents, local paths, credentials or conversation history. Discovery does not start runtimes, download models, change the user's selection or expose an endpoint.

This is an initial Ollama adapter, not a runtime-independent multi-provider catalogue. llama.cpp/other runtimes, trustworthy reasoning/coding/language claims, total GPU/VRAM and storage inventory, per-task matching beyond verified chat capability, performance benchmarking and hardware-specific fit remain unfinished. A schema or passing unit test is not proof of cross-platform hardware accuracy; Windows, Linux and macOS device verification is still required.
