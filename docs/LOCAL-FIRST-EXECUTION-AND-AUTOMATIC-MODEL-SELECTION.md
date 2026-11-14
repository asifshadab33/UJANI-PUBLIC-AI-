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

## No fixed local/external allocation ratio

UJANI does not target a fixed percentage split such as 50:50. Placement is decided independently for each bounded operation or content segment. The amount handled locally versus proposed for external assistance can therefore vary from task to task and from one segment to another.

The planner and orchestrator should prefer local execution when the device has the required capability and safe resource headroom. For each segment, selection considers (in order): data classification and user policy; capability fit and output quality; local model/runtime readiness; available memory, compute and thermal/power headroom; latency and reliability; and only then the value of a configured external provider. A remote provider is not selected merely because it is larger or faster.

Keep deterministic OS-native work on-device. If only one subtask lacks a local capability, isolate that subtask rather than exporting the full request or source document. External assistance is a proposal, never an automatic fallback: show the exact provider, model, purpose and content scope, obtain task-specific approval, and record the decision and result provenance. If approval is absent, continue locally where possible and clearly report unsupported work.

These are placement principles, not a promise that all tasks can currently be partitioned or dispatched. The core planner currently plans capability-based segments; production orchestration, selective dispatch, consent UI and end-to-end receipts remain separate integration requirements.

## Automatic model selection

Automatic selection is an explicit user choice, not the fresh-install default. Once selected, rank only models discovered from trusted, explicitly configured runtimes that are healthy, available, compatible with task modalities/capabilities, within conservative memory/context/resource limits, and permitted by execution policy and data classification. Prefer a suitable local model over any remote provider. Do not assume the largest model is best. If no local candidate qualifies, explain the missing capability and propose only a narrowly scoped external subtask when a configured provider can perform it.

Automatic selection must not download, install, wake, launch or silently switch a runtime/model. Setup and downloads remain explicit user actions. Users may switch between Manual and Automatic and may set a preferred installed model. The current desktop UI persists the local model-selection preference on that device; cloud-provider preference persistence and a provider-neutral multi-runtime abstraction are not yet complete.

## Provider-neutral adapter contract

A real provider adapter exposes typed provider identity/locality, health/readiness, bounded discovery, model catalogue/metadata, capability/context declarations with evidence quality, inference/streaming, cancellation/timeouts and optional installation as a separate explicit-consent capability.

Ollama remains the first production local adapter, not UJANI's identity. A future OpenAI-compatible local endpoint (for example LM Studio) must be a separate adapter with strict endpoint allow-listing, no proxy leakage, redirect denial, response limits and explicit trust configuration. Never treat arbitrary URLs as trusted or route local endpoints through the cloud gateway. The deterministic mock is for tests/browser preview only and is never an automatic inference candidate.

## Segment planning contract

The new side-effect-free ujani_core local_first planner accepts bounded segment IDs and capability requirements. It returns Local, NeedsExternalApproval or Unsupported for each segment. It handles identifiers and capability metadata only: it does not accept document content, choose a provider, invoke inference or transfer data. Native orchestration must bind a proposed segment to a content-scope reference and exact provider/model, then obtain task-specific approval before dispatch.

## Example: 20 documents

Extract, classify, index and search locally where supported. If one scanned page needs OCR or one passage needs an unavailable language model, isolate that page/passage as a separate candidate. Do not upload all 20 documents to solve one unsupported segment. After approval, send only the selected segment to the named provider, return the result locally and continue local querying.

## Status and boundaries

The pure Rust local-first partition planner and tests are implemented in ujani-core. The shared `policy_gate` and `safety_policy` modules are also present, exported and unit-tested, but the Tauri native dispatch functions do not currently call `evaluate_execution`; the live local and cloud paths retain separate route-specific checks. Do not treat the shared gate as enforced at runtime. Integrate it only after trusted data-classification, authorization, owner-consent and operation-registry values are bound at the native boundary. Existing Ollama chat/document paths remain the only confirmed real local model inference path. The generic model_runtime mock is not a production provider abstraction. The local model-selection preference is persisted on-device, but persistent cloud-provider preference, a real second local adapter, automatic multi-model ranking, selective external segment dispatch and end-to-end per-segment provenance still require implementation and integration tests. Existing PDF/document limits remain; this does not imply OCR, Office extraction or universal document support. Fortress is not full OS-wide application isolation.

## Acceptance criteria

- Local-capable segments remain local when another segment is unsupported.
- External candidates identify segment IDs only; no bytes are sent during planning.
- No external request occurs without explicit task-specific approval.
- Automatic selection uses verified capability/resource evidence and reports its decision.
- No silent model download, runtime launch, provider change or cloud fallback.
- Cancellation, size limits, endpoint policy and execution receipts apply to every provider.
