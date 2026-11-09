# Local-first execution and automatic model selection

Product defaults (2026-11-10): local-first execution and automatic model selection.

UJANI Public AI decomposes requests into bounded, registered operations or content segments and evaluates each independently against positively observed device capabilities.

## Execution policy

1. Run deterministic OS-native work locally where possible: file selection, extraction, indexing, metadata, search, checksums and parsing.
2. Run model-assisted steps locally when a compatible installed runtime/model is available and resource admission succeeds.
3. Keep every locally supported segment local even if another segment cannot be handled.
4. Mark only specific unsupported segments as external candidates.
5. Show provider, model, exact selected content scope, purpose and data-handling caveat; require explicit task-specific approval before sending.
6. Return approved results to the local workflow. Keep original files and the local index on-device.
7. Record per-step location, provider/runtime/model, capability evidence, content-scope reference, approval reference, outcome and timing.

A plan is not an execution grant. NeedsExternalApproval never triggers a request by itself. No content is transferred by the core planner.

## Automatic model selection

Automatic selection is the default. Rank only models discovered from trusted, explicitly configured runtimes that are healthy, available, compatible with task modalities/capabilities, within conservative memory/context/resource limits, and permitted by execution policy and data classification. Prefer a suitable local model over any remote provider. Do not assume the largest model is best. If no local candidate qualifies, explain the missing capability and propose only a narrowly scoped external subtask when a configured provider can perform it.

Automatic selection must not download, install, wake, launch or silently switch a runtime/model. Setup and downloads remain explicit user actions. Users may override the automatic choice per task or set a preferred model.

## Provider-neutral adapter contract

A real provider adapter exposes typed provider identity/locality, health/readiness, bounded discovery, model catalogue/metadata, capability/context declarations with evidence quality, inference/streaming, cancellation/timeouts and optional installation as a separate explicit-consent capability.

Ollama remains the first production local adapter, not UJANI's identity. A future OpenAI-compatible local endpoint (for example LM Studio) must be a separate adapter with strict endpoint allow-listing, no proxy leakage, redirect denial, response limits and explicit trust configuration. Never treat arbitrary URLs as trusted or route local endpoints through the cloud gateway. The deterministic mock is for tests/browser preview only and is never an automatic inference candidate.

## Segment planning contract

The new side-effect-free ujani_core local_first planner accepts bounded segment IDs and capability requirements. It returns Local, NeedsExternalApproval or Unsupported for each segment. It handles identifiers and capability metadata only: it does not accept document content, choose a provider, invoke inference or transfer data. Native orchestration must bind a proposed segment to a content-scope reference and exact provider/model, then obtain task-specific approval before dispatch.

## Example: 20 documents

Extract, classify, index and search locally where supported. If one scanned page needs OCR or one passage needs an unavailable language model, isolate that page/passage as a separate candidate. Do not upload all 20 documents to solve one unsupported segment. After approval, send only the selected segment to the named provider, return the result locally and continue local querying.

## Status and boundaries

The pure Rust local-first partition planner and tests are implemented in ujani-core. Existing Ollama chat/document paths remain the only confirmed real local model inference path. The generic model_runtime mock is not a production provider abstraction. Persistent provider/model preference, a real second local adapter, automatic multi-model ranking, selective external segment dispatch and end-to-end per-segment provenance still require implementation and integration tests. Existing PDF/document limits remain; this does not imply OCR, Office extraction or universal document support. Fortress is not full OS-wide application isolation.

## Acceptance criteria

- Local-capable segments remain local when another segment is unsupported.
- External candidates identify segment IDs only; no bytes are sent during planning.
- No external request occurs without explicit task-specific approval.
- Automatic selection uses verified capability/resource evidence and reports its decision.
- No silent model download, runtime launch, provider change or cloud fallback.
- Cancellation, size limits, endpoint policy and execution receipts apply to every provider.
