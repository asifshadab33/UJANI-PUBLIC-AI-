# UJANI Adaptive Capability Architecture (ACA)

**Status:** Proposed architecture and delivery contract  
**Applies to:** UJANI Public AI  
**Related policy:** [Local-first OS capability reuse](architecture/LOCAL_FIRST_OS_CAPABILITY_REUSE.md), [resource-aware execution](RESOURCE-AWARE-EXECUTION.md)  
**Implementation status:** This document is a design contract. It does not claim that external capability discovery, installation, sandboxing, or resource measurement is implemented.

## 1. Product intent

UJANI should remain a small, useful local-first application whose capabilities can grow without forcing every user to install every tool, runtime, or AI model.

When UJANI cannot complete a requested task with its built-in features, it may explain the missing capability and present a small set of optional, compatible choices. It must first prefer an existing OS API, an already-installed application/runtime, or a capability already shipped with UJANI. External software is a last-mile option, not a reason to duplicate OS functionality.

The user remains in control. Discovery is not installation; recommendation is not execution; installation is not permission to access user data.

## 2. Three independently managed layers

### 2.1 Core

The core contains only the functionality required to launch and manage UJANI:

- user interface, task orchestration and capability routing;
- local-first privacy and consent policy;
- bounded hardware/resource observation and task admission;
- OS integration through supported APIs;
- capability catalogue and manifest validation;
- install/remove lifecycle coordination and clear status;
- audit receipts for user-approved state-changing operations.

The core must not require an AI model, cloud account, external capability pack, or always-running helper service to launch and perform basic OS-assisted tasks.

### 2.2 Optional capability packs

Examples include video format conversion, OCR, image enhancement, audio transcription, PDF utilities and specialised file processing. A pack is installed only when the user chooses it. Prefer invoking an existing supported application or OS facility over bundling another implementation.

Capability packs must be independently versioned, removable and resource-bounded. A pack that is not in use must not keep a background process, model server, watcher or resident memory allocation alive unless the user has separately enabled a clearly described always-on feature.

### 2.3 Optional model runtimes and models

Model runtimes and model weights are separate from both core and utility packs. A user must be able to use UJANI without downloading a model. Before a model download, show its task suitability, model size, expected additional storage, RAM/VRAM needs, supported runtime/platform, licence and known limitations. Unknown hardware or model metadata remains Unknown. Never infer a model's capability from its name.

## 3. Capability discovery and recommendation

A discovery request is triggered by a user task or an explicit visit to the capability catalogue. No silent OS-wide scanning or background internet catalogue polling.

The resolver follows this order:

1. Built-in deterministic UJANI capability.
2. Supported OS API or native facility.
3. Already-installed, user-approved local application or runtime.
4. Optional capability pack from a verified source.
5. Explicitly approved external service only when the task cannot reasonably be completed locally and the user approves the exact data transfer.

A recommendation must explain why the current installation cannot perform the task, what the proposed capability does, whether it is local or networked, what data it receives, its publisher/source, version, licence, platform compatibility, download size, installed size where known, RAM/CPU/storage estimate, permissions, and removal method. Show uncertainty and evidence provenance. Do not rank a provider as trusted merely because it appears in a catalogue.

If no candidate meets policy, compatibility and provenance requirements, explain that no safe supported option is available.

## 4. Manifest and trust contract

Every installable capability must have a versioned, bounded manifest containing at least:

- stable capability and provider identifiers;
- manifest schema version and package version;
- supported OS/architecture and runtime requirements;
- publisher identity, source URL, release provenance and immutable artifact digest;
- cryptographic signature or an explicitly documented, independently verified trust mechanism;
- licence identifier and redistribution terms;
- package/download and installed-size bounds;
- declared operations and read/write/state-changing classification;
- required permissions, filesystem scope, network endpoints and data classification;
- CPU, memory, storage and wall-clock ceilings;
- invocation contract, cancellation and cleanup behavior;
- update, rollback and uninstall metadata.

Reject malformed, oversized, unsupported, stale or unverifiable manifests. A package hash proves byte identity, not publisher identity; signatures and publisher trust must be validated separately. Never execute an install script supplied by an untrusted catalogue response.

## 5. Installation, invocation and removal

- Discovery and preview are read-only.
- Installation requires a separate, explicit user action after the complete impact summary is shown.
- Do not request administrator/root privileges by default. If elevated privileges are genuinely required, explain why and ask again at the OS-controlled elevation boundary.
- Use a platform package manager or a narrowly scoped, reviewed installer adapter where possible. Never invoke a general shell with model-generated or catalogue-supplied command text.
- Pin the selected version and verify the artifact digest and publisher signature before installation.
- Install atomically where supported; preserve rollback or a clear recovery path.
- Invoke only registered executables through a typed adapter with fixed executable identity, validated arguments, bounded input/output, timeout, cancellation and process cleanup.
- No arbitrary plugin code, unrestricted filesystem access, unrestricted network access, shell execution or remote command execution.
- Remove only files owned by the capability manager; preserve user data and disclose residual data/configuration.
- Updates are not silent. Show changed permissions, version, size and licence before applying an update.

A tool that cannot be safely isolated on a supported platform must be marked unsupported there, not silently run with broader privileges.

## 6. Resource manager and low-resource operation

Resource observation must use supported OS APIs and distinguish measured values, estimates, stale values and Unknown. Do not assume an accelerator is usable merely because hardware is present.

Proposed planning targets (not measured or certified requirements):

| Profile | Planning target | Expected use |
|---|---:|---|
| Core compatibility | 4 GB RAM | Launch, settings, basic OS-assisted tools; avoid concurrent heavy tasks |
| Everyday | 8 GB RAM | Recommended general-use configuration |
| Local AI | 16 GB RAM or more | A broader range of small/medium local models, subject to model and VRAM needs |
| Advanced | 32 GB RAM or more | Larger models and heavier professional workflows, still model-dependent |

A 4 GB target does not mean every optional capability or AI model will run on a 4 GB device. Model requirements depend on architecture, quantization, context length, concurrency, runtime and accelerator memory.

Proposed core installed-size planning target: **150–300 MB**, to be validated from actual signed/unsigned installers per platform and packaging format. It is not a current measured size or a release promise. Report core, selected packs, runtime and model storage separately.

The Resource Manager should display:
- core and per-pack installed storage;
- model/runtime storage separately;
- current and peak RAM/CPU use where measurable;
- whether a capability is idle, running or unavailable;
- declared versus observed resource use;
- per-capability limits and user/admin quotas;
- uninstall controls for optional components.

When memory or power headroom is insufficient, defer or reject work with a useful explanation. Do not make the device unresponsive to satisfy a task. Stop optional helper processes after completion and release resources.

## 7. Privacy, consent and receipts

Before a capability can read user data, show the exact selected files/folders or data scope. Network access and external data transfer require a separate task-scoped approval that names the recipient and data categories. A local tool must not acquire network access merely because its installer had network access.

For each installation, update, invocation or removal, record a minimal receipt: operation ID, capability/version, user intent, approved scope, policy decision, timestamps, result/cancellation and artifact digest where relevant. Do not store document contents, prompts, credentials or unnecessary personal data in receipts.

The resolver and UI are not authorization boundaries. Native policy must validate consent, scope, package identity and resource ceilings immediately before side effects. Denials and policy-store failures must happen before those side effects.

## 8. Delivery phases and release gates

1. **Inventory only:** map existing UJANI features, OS APIs and installed runtimes; no external downloads.
2. **Manifest and catalogue contract:** schema, parser, provenance validation, size limits and hostile-input tests.
3. **Recommendation-only UI:** explain missing capability and show verified metadata; no installation or invocation.
4. **First safe local adapter:** one narrowly scoped, low-risk utility with explicit install/preview/run/remove lifecycle and platform-specific tests.
5. **Resource Manager:** measured per-component resource reporting, idle shutdown and user limits.
6. **Enterprise-compatible contract:** versioned shared manifest and policy semantics, without importing Public AI UI or user settings into Enterprise.
7. **Expansion:** add packs only after security review, compatibility evidence and measurable user value.

Do not enable general-purpose third-party plugins or arbitrary downloaded executables as a first release feature.

Acceptance requires: manifest fuzz/negative tests; signature and digest validation; denial-before-side-effect tests; explicit consent tests; path and argument validation; cancellation/timeout/cleanup tests; update/rollback/uninstall tests; platform compatibility matrix; installer-size and idle-RAM measurements; and user-facing licence, provenance and data-flow disclosures.

## 9. Non-goals and invariants

- No forced AI model download.
- No always-on external helper by default.
- No silent installation, update, execution or network transfer.
- No arbitrary shell, model-generated code execution or unrestricted remote execution.
- No OS-wide file/application/message scanning without a separate explicit scope and consent.
- No assumption that all external tools are safe, free, open source or compatible.
- No claim that the proposed RAM or package-size targets are verified until measured.

The governing principle is: **small core, optional capabilities, evidence before trust, consent before side effects, and measured resource use.**
