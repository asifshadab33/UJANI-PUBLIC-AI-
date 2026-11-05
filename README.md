# UJANI Public AI

**Local intelligence first. Local execution by default. External assistance only when needed — with user control.**

UJANI Public AI is an open-source, device-first AI runtime concept. It aims to help people use the compute already available on their own computers and devices, while keeping data and decisions under their control.

> **Project status: early prototype / pre-alpha.** This repository contains an initial architecture and a small, illustrative Rust core. It is not yet a production-ready AI agent, secure sandbox, model manager, or cross-platform hardware detector.

## Goals

- Discover available CPU, memory, GPU/NPU, storage, installed runtimes and model compatibility.
- Analyse a request and split it into independently verifiable tasks.
- Prefer local models and deterministic local tools when suitable.
- Request external/cloud help only for a specific subtask that cannot reasonably be completed locally.
- Share the minimum necessary information with an external service, only under the user's configured policy and consent.
- Keep permissions, resource budgets and destructive actions under explicit user control.
- Verify outputs and report what ran locally versus externally.

## Non-goals

- No public marketplace or anonymous network of other people's devices.
- No silent mining or background use of device resources.
- No unrestricted shell access for an AI model.
- No automatic upload of complete user tasks or files to a cloud service.

## Architecture at a glance

1. **Request intake** — receives a user request and its data-handling policy.
2. **Local analyser** — rules and/or a small local model classify the task.
3. **Capability registry** — records available resources, tools, models and permissions.
4. **Planner** — creates a typed task graph with explicit dependencies.
5. **Policy gate** — checks privacy, network, resource and approval requirements.
6. **Execution adapters** — invoke registered local tools or model runtimes.
7. **External assistance gateway** — optional, per-subtask, minimised and policy-gated.
8. **Verifier** — checks outputs and records provenance.
9. **User interface** — shows progress, resource use, approvals and execution location.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [ROADMAP.md](ROADMAP.md).

## Desktop prototype

The Tauri 2 desktop prototype is in [`apps/desktop`](apps/desktop/README.md). It includes a four-mode policy preview, a loopback-only Ollama adapter for real local inference in Fortress and Smart modes, bounded in-memory text-file analysis, selected-PDF text extraction, local Ollama summarisation and translation, a read-only computer resource/OS-locale panel, basic image-header inspection, a versioned local workflow-graph planning preview, and opt-in SQLite task history with bounded retention, AES-256-GCM encrypted prompt/output fields and deletion controls, plus cancellable local chat and approved cloud requests with stage events and bounded timeout recovery. The graph planner validates allow-listed steps and dependencies but does not yet execute arbitrary graphs. Browser translation is a manual external option only; no text is sent automatically. Secure distributed Worker/Partner peer execution, encrypted local history, OS-level Fortress isolation, browser automation, scanned-PDF OCR/vision, verified GPU/NPU scheduling, and production sandboxing are not implemented. The cloud provider gateway is a single remote API path, not a peer network. See the app README for Ollama setup and limitations.

## Prototype core

The Rust crate in `crates/ujani-core` defines basic task, capability and execution-policy types plus a conservative routing decision. It deliberately does **not** execute commands, access files, discover hardware, or contact a cloud provider.

Run the tests:

```sh
cargo test --workspace
```

## Security principles

- Treat prompts, files, web pages and model output as untrusted input.
- Models may propose structured tasks; trusted application code validates them.
- Tools must be registered, typed and narrowly scoped.
- Enforce resource/network/filesystem limits outside the model.
- Require confirmation for destructive, publishing, messaging, purchasing or external data-transfer actions.
- Make local-only mode genuinely network-independent.
- Keep audit records understandable and allow users to revoke permissions.

Read [SECURITY.md](SECURITY.md) before reporting a vulnerability.

## Licensing

This repository currently includes the GNU General Public License v3.0. Contributions are expected to comply with that license; see [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Early contributions that are especially useful: platform capability detection, secure tool adapters, local model integration, reproducible benchmarks, threat modelling and usability testing.

## Relationship to UJANI Enterprise AI

UJANI Public AI is the personal-device project. [UJANI Enterprise AI](https://github.com/asifshadab33/UJANI-ENTERPRISE-AI) is a separate organisation-controlled project for enrolled company-owned or explicitly authorised devices. Enterprise scheduling must never be silently enabled in the personal product.
