# UJANI Public AI — product strategy and engineering principles

**Status:** strategy baseline for the pre-alpha project. This document describes direction and release gates, not a claim that planned features are implemented.

## Mission

Help people and small organisations get useful work done with the computing resources they already own. Prefer local, efficient and deterministic execution when it meets the task's quality and reliability requirements. Use external compute only when it provides a meaningful benefit and the user or organisation has authorised it.

UJANI is not intended to maximise AI usage, model size, GPU consumption or revenue. It should help avoid unnecessary centralised compute by making appropriate use of existing devices, while measuring energy, reliability and total cost rather than assuming that distributed execution is automatically greener or cheaper.

## Product promise

- A practical toolbox and assistant for general users.
- Local-first processing, with optional and clearly disclosed cloud assistance.
- Honest capability and resource estimates; unknown means unknown.
- User control over data, permissions, network access and resource consumption.
- An extensible tool ecosystem with security and compatibility gates.

## Principles

1. **Local when appropriate, not local at any cost.** Compare quality, latency, energy, privacy, hardware load and reliability. A small local model that gives poor results is not automatically preferable to a narrowly scoped remote task.
2. **Deterministic tools before AI.** Use normal code for conversion, parsing, compression, hashing and other exact transformations. Reserve AI for tasks that need inference.
3. **No silent resource use.** No background compute, GPU use, telemetry beyond disclosed essentials, or sharing of a device with another party without clear opt-in and visible controls.
4. **Data minimisation.** Keep processing on-device where possible. For approved remote work, transmit only the necessary subtask data, disclose provider/destination and obtain consent.
5. **Capability claims require evidence.** Hardware presence is not proof of a usable accelerator. Report source, timestamp, freshness and confidence; use an explicit unknown state.
6. **Safe extension model.** Tools use stable IDs, versioned input/output schemas, explicit permissions, resource budgets, bounded execution and tests. Model output is data, never authority.
7. **Progressive complexity.** The app must remain responsive on modest hardware. Load optional capabilities on demand, bound queues and payloads, avoid always-on heavy models and keep background work disabled by default.
8. **Interruption and recovery.** Every long task needs cancellation, timeouts, cleanup, clear partial-result semantics and recoverable checkpoints where useful.
9. **Open and interoperable.** Prefer documented protocols and reusable open standards; review dependency licences, security posture and maintenance before adoption.
10. **No monetisation objective.** The primary objective is useful, accessible and resource-efficient computing. Operating costs may be covered transparently through donations, grants, sponsorship or optional hosting/support only if needed; no advertising, data sale or hidden paywall is part of the product vision.

## Resource-aware execution policy

The planner should choose among deterministic local tools, local AI, an authorised enterprise worker pool and optional external cloud services. It must not assume distributed execution is always superior.

A candidate execution should be evaluated against:
- task quality and validation requirements;
- privacy/data classification and location constraints;
- CPU, RAM, GPU/NPU runtime compatibility and current headroom;
- power, thermal state, battery and interactive-user activity;
- network transfer volume, latency and connectivity;
- expected energy and total cost, including idle power, cooling and network overhead;
- reliability, deadline, checkpointing and retry cost.

Prefer the smallest sufficient model and shortest valid workflow. Do not download or start a model automatically without user approval and a clear storage/resource estimate. Never use another person's device without explicit opt-in and revocable consent.

## Tool Engine direction

Build a small, stable engine before growing the library. Each tool manifest should define:
- unique tool ID, version, category and description;
- supported platforms and offline/online requirements;
- input/output schema and maximum payload sizes;
- file, network, clipboard and system permissions;
- CPU, memory, GPU and wall-clock budgets;
- execution adapter and cancellation/cleanup contract;
- deterministic test fixtures and expected outputs;
- data handling statement and provenance fields.

Start with a curated set of high-frequency deterministic tools. Expand toward a large library only when usage evidence, maintenance ownership and tests justify it. A catalogue of 100–150 is a long-term target, not a launch gate.

## Competitor and ecosystem learning

Study existing projects as engineering references, not templates to copy wholesale:
- **Open WebUI / LibreChat:** provider abstraction, model selection, conversation UX and tool integration.
- **AnythingLLM:** approachable document workspaces and local knowledge workflows.
- **Dify / Flowise / n8n:** explicit workflow composition, reusable nodes, run history and debugging.
- **SearXNG:** self-hosted metasearch and privacy-aware research discovery.
- **BOINC / HTCondor:** distributed job scheduling, heterogeneous workers, queues, checkpoints and resource policies.
- **Salad / Vast.ai / Golem:** capacity discovery, worker reliability, pricing/usage metering and provider onboarding.

Before adopting a component, verify current maintenance, security advisories, licence compatibility, platform support, dependency footprint and whether it solves a real UJANI user problem. Do not bundle entire competing applications or duplicate their control planes. Build adapters around well-defined protocols where possible.

## Phased implementation order

### P0 — baseline and architecture
- Audit current code, CI, dependencies, release signing and security boundaries.
- Record verified features and gaps in implementation status.
- Agree versioned task, capability, provenance and policy contracts with Enterprise.
- Establish performance budgets and test conventions.

### P1 — lightweight Tool Engine
- Implement a typed registry and manifest validation in the Rust core.
- Add deterministic, bounded tool adapters and a minimal UI catalogue.
- Require per-tool permissions, limits, progress, cancellation and errors.
- Add contract, fuzz/property and regression tests.
- Do not add a plugin marketplace or arbitrary code execution.

### P2 — useful curated tool collection
- Prioritise document, PDF, text, image metadata and developer utilities.
- Keep expensive image decoding/OCR optional and bounded.
- Measure real use and remove low-value duplicates.

### P3 — capability and resource advisor
- Add timestamped hardware/runtime capability evidence and expiry.
- Add model-specific compatibility and conservative resource estimates.
- Add interactive-use, thermal and power-aware constraints where supported.
- Keep the advisor read-only until scheduling and reservation semantics are proven.

### P4 — local/cloud gateway hardening
- Complete data minimisation, explicit consent, cost limits, cancellation and provenance.
- Verify local-only operation with network disabled.
- Support providers through isolated adapters and clear user-supplied credentials handling.

### P5 — browser and research
- Isolate web content from privileged desktop APIs.
- Treat pages and search results as untrusted data.
- Add search, extraction, source capture and citation checks as distinct stages.
- Self-host SearXNG only if operational ownership, rate limits and source quality are acceptable.

### P6 — optional enterprise integration
- Use versioned contracts; keep fleet identity, admin policy and scheduling in Enterprise.
- Do not turn personal Public AI into a hidden fleet agent.

## Performance and reliability budgets

Define and test budgets before enabling features. Initial engineering targets (to be measured and adjusted, not guarantees):
- No continuous model residency by default.
- No background workload without user initiation or explicit opt-in schedule.
- Bound every file, image, prompt, result, queue and history operation.
- Keep UI work asynchronous; never block the main thread on model inference or large file processing.
- Expose cancellation for every long-running operation.
- Use bounded retries with backoff; never retry non-idempotent work blindly.
- Report estimated and actual resource use where trustworthy measurements exist.
- Degrade gracefully when GPU, model runtime, network or storage is unavailable.

## Success measures

Measure useful outcomes, not feature count:
- task completion and repeat usage;
- tool error rate, crash rate and cancellation latency;
- CPU/RAM/GPU use, energy where measurable and idle overhead;
- local versus remote execution share, with task quality held constant;
- remote bytes sent and user consent rate;
- successful results per watt-hour or per unit of total compute cost where measurement is defensible;
- accessibility and performance on modest hardware.

Do not claim energy or data-centre reductions from device counts alone. Compare equivalent completed workloads and include device idle/active energy, network, cooling, failed jobs and hardware lifecycle assumptions.

## Explicit non-goals for the current phase

- Public distributed-compute marketplace.
- Anonymous peer-to-peer execution.
- Arbitrary third-party code on user devices.
- Silent GPU mining or hidden idle-time jobs.
- Automatic model downloads or cloud fallback.
- 100+ tools before the registry, permissions and test harness are mature.

## Release gates

A capability is not complete because it appears in the UI or roadmap. It must have implementation evidence, tests on supported platforms, security review appropriate to its risk, documented limitations and an honest user-facing status. Production claims require independent review.
