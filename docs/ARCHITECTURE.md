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
