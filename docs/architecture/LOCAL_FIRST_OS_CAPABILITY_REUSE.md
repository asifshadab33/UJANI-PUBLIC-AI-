# Local-first resource reuse and operating-system capability policy

**Status:** Proposed engineering policy  
**Applies to:** UJANI Public AI and shared components intended for UJANI Enterprise AI

## Product objective

UJANI should complete useful work with resources already available on the user's device before relying on remote inference. It should observe available CPU, memory, accelerators and installed local runtimes, choose a workload that fits, and request cloud processing only when the task cannot reasonably be completed locally and the user has explicitly approved the transfer.

The aim is not to maximize local resource consumption. It is to make the best use of available resources without making the device unresponsive, draining power unnecessarily, or displacing the user's other applications.

## Core rule: reuse first, implement only what is missing

Before adding a subsystem, the implementation owner must answer:

1. Does the operating system already expose this capability through a supported API?
2. Does an installed local service or existing UJANI module already provide it?
3. Can the existing implementation be extended safely instead of adding a second implementation?
4. What evidence demonstrates that a new implementation is necessary?
5. What is the maintenance, memory, CPU, binary-size and security cost of the proposed code?

If an OS API or installed service is suitable, UJANI should use it rather than reimplementing the same mechanism. Do not add a wrapper that only renames or forwards an API unless it provides a real cross-platform contract, validation, observability or safety boundary.

## Responsibilities: delegate mechanism, retain policy

| Capability | Prefer reusing | UJANI must still own |
|---|---|---|
| CPU and memory discovery | Supported OS APIs and maintained system-information libraries | Conservative admission thresholds, freshness and Unknown state |
| Process creation, cancellation and lifecycle | Native process APIs and OS process supervision | Allowlisted commands, arguments, timeouts, resource ceilings and kill behavior |
| File access and traversal | OS filesystem APIs and one shared hardened walker | User-selected roots, symlink/reparse-point policy, path containment, byte/depth limits and cancellation |
| Credential storage | OS credential vault / keyring | Secret scope, access policy, rotation and never logging secrets |
| Notifications and reminders | Native OS notification and scheduling facilities where available | User consent, privacy-safe message content, duplicate suppression and permission handling |
| Local model execution | Existing installed local runtimes such as the configured Ollama service | Model selection policy, resource admission, local-only routing and truthful capability reporting |
| Sandboxing and resource controls | OS primitives such as process limits, Job Objects and platform sandbox facilities | Fail-closed policy, platform coverage, validation and documented limitations |
| Database and durable storage | Existing database engine and filesystem durability guarantees | Schema, transactions, authorization, retention, integrity verification and anti-rollback design |

Operating-system delegation is not a reason to remove application security. OS APIs provide mechanisms; UJANI remains responsible for deciding whether a task is allowed and what data it may access.

## Local-first execution policy

1. Observe capabilities using bounded, documented APIs. Do not infer that a GPU or NPU is usable merely because hardware is present.
2. Represent missing, stale or unsupported observations as **Unknown**, not Available.
3. Estimate task memory and compute needs before starting work.
4. Prefer local CPU, supported local accelerators and already-installed local runtimes in that order only when they are a safe fit for the task and device.
5. Bound concurrency and allow the user to cancel work. Respect other applications and avoid starving the operating system.
6. Keep cloud routing disabled unless the task requires it, the provider is configured, and the user has approved the exact data transfer.
7. Record execution location and minimal provenance metadata without storing prompts, outputs or credentials in telemetry or security ledgers.

## Duplication and hardening review

During each hardening pass, maintain a capability inventory with:

- capability and user-visible behavior;
- current implementation locations and owners;
- authoritative implementation selected;
- OS API or existing service reused;
- duplicate implementations to consolidate;
- compatibility and security constraints;
- regression tests required before removal.

Consolidation must be behavior-preserving. Remove duplicate code only after tests cover platform-specific behavior, error handling, cancellation, permissions, resource limits and failure recovery. Do not replace a safe implementation with an OS API that is unavailable or weaker on a supported platform.

## Initial review targets

The following are investigation targets, not claims that the current code is necessarily incorrect:

- **Filesystem traversal:** consolidate local search and folder-import traversal behind a single hardened abstraction while preserving their different selection limits and user-visible semantics.
- **Process isolation:** use a dedicated, minimal PDF worker process rather than relaunching the full desktop executable; retain OS-enforced resource and sandbox limits.
- **System inventory:** ensure CPU/memory and accelerator observations have one authoritative provider and do not duplicate expensive polling.
- **Credentials:** continue to use the OS credential vault; avoid creating a parallel plaintext or custom key store.
- **Notifications and scheduling:** prefer native facilities for delivery and timing; keep UJANI's reminder rules and consent logic in the application layer.
- **Task coordination:** keep one bounded task registry/queue rather than independent overlapping concurrency managers.

Each target must be confirmed against current code and tests before changes are made.

## Measures of success

Track these measures over time:

- share of eligible tasks completed locally;
- cloud requests avoided without reducing task success or quality;
- CPU, memory and accelerator use per completed local task;
- effect on foreground responsiveness and battery use;
- number of duplicate implementations removed;
- installer size and idle background resource use;
- correctness of cancellation, privacy and security regression tests.

Do not optimize a single metric at the expense of user privacy, device stability, task quality or security.

## Change acceptance checklist

A change that adds or replaces a capability should include:

- [ ] evidence that no suitable existing OS API, local service or UJANI abstraction already covers the need;
- [ ] one authoritative implementation and documented ownership;
- [ ] explicit resource, privacy and permission boundaries;
- [ ] safe behavior on unsupported platforms and Unknown capability states;
- [ ] regression tests for cancellation, failure and resource exhaustion;
- [ ] measurements showing the change does not unnecessarily increase installer size or background resource use;
- [ ] no remote execution, autonomous firewall change or autonomous containment unless separately reviewed and explicitly authorized.
