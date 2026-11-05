# Roadmap

This roadmap describes intended work, not completed features. Items are not production-ready until implemented, tested and documented.

## Phase 0 — foundations (current)
- [x] Define local-first product principles and explicit non-goals.
- [x] Add an initial typed Rust task/capability model and routing tests.
- [x] Document privacy, security and contribution expectations.
- [ ] Establish CI for formatting, linting, tests and dependency review.
- [ ] Agree a versioned task protocol shared with UJANI Enterprise AI.

## Phase 1 — desktop proof of concept
- [ ] Windows desktop shell (Tauri 2 + React/TypeScript).
- [ ] Read-only hardware inventory: CPU, RAM, storage and available accelerators.
- [ ] Capability registry with provenance, freshness and permission metadata.
- [ ] Local task history and visible progress.
- [x] First integrated local text-document pipeline: bounded analysis followed by Ollama summarisation.
- [ ] Registered deterministic tools: PDF text extraction, image conversion and media metadata.
- [ ] Sandboxed execution adapter with enforceable filesystem and resource limits.

## Phase 2 — local models and planning
- [ ] Model catalogue with licence, checksum, source and compatibility metadata.
- [x] Integrate one local inference runtime behind a stable adapter (Ollama loopback adapter; prototype scope).
- [ ] Structured task graph with dependency and output schemas (future work; current document pipeline is a fixed two-stage workflow).
- [ ] Verification and reproducible benchmark harness.

## Phase 3 — optional external assistance
- [ ] Per-subtask cloud assistance interface.
- [ ] Data minimisation and explicit approval policies.
- [ ] Local-only mode tested with network disabled.
- [ ] Clear provenance for local, remote and deterministic-tool work.

## Phase 4 — hardening and community
- [ ] Threat model and independent security review.
- [ ] Signed releases and software bill of materials.
- [ ] Accessibility, internationalisation and installer testing.
- [ ] Public benchmark methodology and issue triage process.

## Later
- [ ] Android and iOS capability adapters, subject to OS restrictions.
- [ ] Plugin/SDK interface with capability and permission boundaries.
- [ ] Optional integrations with UJANI Enterprise AI through a documented, versioned protocol.

## Release gates

No release should be described as secure, private-by-design in implementation, or production-ready solely because it follows this roadmap. Those claims require implementation evidence, tests and review.
