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

## Implementation status

This is the v1 contract and product boundary. Runtime discovery, model capability verification, hardware profiling, task-aware recommendations and user-selection UI must be delivered as separately tested implementation steps. A schema file does not mean those features are already implemented.
