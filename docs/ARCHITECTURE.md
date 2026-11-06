# UJANI Public AI — architecture

## Design rule

The local device is the default execution environment. Cloud assistance is an optional exception, not the default path. A user can choose local-only operation.

## Logical components

| Component | Responsibility | Trust boundary |
|---|---|---|
| Request intake | Parse user intent and attach policy | User input is untrusted |
| Local analyser | Classify and propose subtasks | Model output is untrusted |
| Capability registry | Report resources, tools, models and permissions | Data is time-stamped and may become stale |
| Planner | Build a typed dependency graph | Plans require validation |
| Policy engine | Apply privacy, consent and resource rules | Trusted application code |
| Local adapters | Invoke approved models and tools | Sandboxed per adapter |
| External gateway | Send approved minimal subtask context | Explicit network/data-transfer boundary |
| Verifier | Validate output and provenance | Outputs remain untrusted until checked |
| UI | Show actions, approvals and resource use | User control surface |

## Request lifecycle

1. Receive request and selected policy (local-only, ask-before-cloud, or user-configured).
2. Inspect only capabilities the application is permitted to inspect.
3. Produce a proposed task graph with typed inputs, outputs and dependencies.
4. Validate the graph against registered tool IDs, schemas and policy.
5. Estimate resource requirements and ask before exceeding user limits.
6. Execute local work first when it meets the task requirements.
7. If a subtask needs external assistance, explain what data would leave the device and obtain required consent.
8. Verify outputs; record execution location, tool/model versions and relevant checks.
9. Present results and allow the user to inspect or delete task history.

## Task protocol requirements

A task should carry a stable ID, operation identifier, schema-versioned inputs, declared outputs, dependencies, resource estimate, data classification, execution constraints and approval requirements. Avoid arbitrary executable strings. Tool IDs must resolve through a local allow-listed registry.

## Capability registry

Each capability should include:
- stable identifier and capability type;
- detected value and source;
- timestamp and expiry/freshness;
- runtime/driver compatibility where relevant;
- permission status and whether detection was user-approved;
- resource budget and current availability.

Do not infer that a GPU/NPU is usable merely because hardware is present. Runtime, driver, memory and OS support must be checked.

## Execution and safety

- Do not pass model-generated shell commands to a shell.
- Validate tool arguments against strict schemas.
- Use least-privilege process identities and per-task temporary directories.
- Restrict filesystem paths, network access, process count, memory, CPU time and wall-clock time.
- Require confirmation for destructive or externally visible actions.
- Treat file contents and retrieved web content as data, not instructions.
- Keep secrets out of prompts, logs and task traces.
- Make cancellation and cleanup reliable.

## Cloud assistance

Cloud requests are subtask-scoped. The gateway should construct a minimal payload from explicitly approved fields, remove unrelated file contents and secrets, disclose the destination/provider, and log the user's approval. A local-only policy must block this gateway at the policy layer.

## Suggested implementation stack (not a dependency commitment)

- Desktop shell: Tauri 2, React and TypeScript.
- Core and policy engine: Rust.
- Local inference: adapter abstraction (e.g. llama.cpp or ONNX Runtime, selected after platform benchmarks).
- Deterministic media/document tools: narrowly scoped adapters around suitable libraries.
- Mobile: native Kotlin/Swift adapters when desktop contracts stabilise.

## Shared protocol

The personal and enterprise projects should share versioned task/capability contracts, not duplicate each other's control planes. Enterprise fleet enrolment, organisation policies and scheduling belong only in UJANI Enterprise AI.

## Initial capability registry and local planner

The desktop prototype now exposes a read-only capability snapshot containing CPU availability, current available memory, local Ollama health, bundled PDF text extraction, bounded image-header inspection, and explicit unknown states for GPU/NPU. Each record includes an identifier, category, state, source and explanatory detail.

The built-in operation catalogue is allow-listed. The local planner checks required capabilities and a conservative minimum available-memory preflight before returning a plan. It does not execute the operation, reserve memory, estimate model-specific VRAM/RAM needs, or enable background compute. Worker and Partner remain unsupported until a separately controlled external gateway exists.

The snapshot is point-in-time information and can become stale. Future work must add timestamps/expiry, explicit resource budgets, model-specific compatibility checks and cancellation before scheduling long-running work.


## Versioned workflow graph (planning only)

The core now accepts schema version 1 graphs containing at most 32 nodes. Each node must use a registered operation identifier and a bounded identifier; dependencies must exist, be unique and be acyclic. The planner produces a deterministic topological order and checks the union of required capabilities plus a conservative sequential peak-memory preflight. The desktop command rejects oversized node/dependency lists before planning. A successful plan is not an execution grant, does not reserve resources, and does not provide a sandbox. The current UI offers a fixed analyse → summarise → translate graph preview; execution still uses the dedicated document pipeline. General graph execution requires typed input/output schemas, per-node permission checks, cancellation, resource accounting and provenance before it should be enabled.


## Resource-aware local scheduling policy

The Rust core exposes a deterministic scheduler contract. A caller supplies a timestamped resource snapshot and a typed task request; the policy rejects stale or unknown telemetry, enforces CPU/RAM/concurrency limits, requires verified GPU runtime and VRAM before GPU placement, and gates background work on explicit owner opt-in, idle state and configured AC-power policy. This is a policy decision only: it does not collect telemetry, reserve resources, dispatch tasks or enforce OS limits. Native adapters must provide trustworthy measurements and atomically enforce reservations before enabling scheduling in production.

## Execution provenance receipt contract

The core defines a versioned receipt with exact input/output SHA-256 fields, operation, adapter/model identity, execution location, timestamps and an external-approval reference. Structural validation and hashing are implemented and tested. The receipt does not prove model correctness or execution by itself; native adapters must hash the exact bytes, bind approval evidence, persist receipts durably and sign them where remote verification is required.
