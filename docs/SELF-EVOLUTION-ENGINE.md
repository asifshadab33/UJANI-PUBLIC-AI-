# UJANI Public AI — Self-Evolution Engine

**Status: approved architectural direction; implementation not yet enabled.**  
**Scope:** Public AI application only. This document defines a safe path for model-assisted application improvement without granting a model authority over the protected core.

## 1. Product intent

Allow a user-selected, compatible AI model to inspect UJANI's upgradeable application surfaces, explain improvement opportunities, prepare code/module changes in an isolated workspace, and submit a reviewable update proposal. The user remains in control of production installation.

The engine is a development and maintenance capability, not a self-modifying unrestricted agent. It must not execute model-generated shell commands, grant itself permissions, bypass review, or rewrite its own enforcement boundary.

## 2. Trust zones

### Protected core (not autonomously writable)
- Authentication, device identity, trust enrollment and revocation.
- Permission and consent enforcement, execution-mode policy, network-egress policy.
- Update signature verification, signing-key custody, rollback/recovery trust anchors.
- IPC authorization, secret/key handling, security-critical filesystem boundaries.
- The policy that constrains the Self-Evolution Engine itself.

Changes to these areas require a separate human-led security engineering process, independent review, explicit release approval and the normal signed release path. A model cannot approve or deploy such a change.

### Upgradeable platform (candidate for controlled change)
- User interface and accessibility.
- Non-security-critical workflows and presentation.
- Registered tool adapters, model adapters and document/search modules.
- Capability descriptions, non-privileged configuration and help content.
- Independently versioned modules whose permissions are declared and enforced by trusted host code.

### Extension environment (isolated)
- Experimental plugins, connectors and generated modules.
- No ambient filesystem, shell, credential-store, network or process privileges.
- Explicit capability manifests, scoped data access, resource budgets and revocation.

## 3. Proposed lifecycle

1. **Request:** user explicitly starts an analysis or improvement task and selects the repository/version and permitted scope.
2. **Inventory:** trusted application code provides a bounded, read-only source manifest. Secrets, local credentials, user files and unrelated workspace data are excluded.
3. **Analyse:** the selected model returns structured findings with file/line references, rationale, risk classification and proposed acceptance criteria.
4. **Plan:** trusted code validates the proposal against a versioned allow-list of writable paths, protected-path deny-list, dependency policy and change-size/resource limits.
5. **Generate:** code is created only in a disposable, isolated workspace or branch. No generated code runs with host privileges.
6. **Validate:** run formatting, type checks, unit tests, static analysis, dependency/license checks, secret scanning, policy tests and build checks in a disposable environment. Validation results are evidence, not proof of safety.
7. **Review:** show a complete diff, affected paths, dependency changes, permissions requested, test results, risks and rollback target.
8. **Approve:** require an explicit user action for each production update. High-risk or security-sensitive changes require a separate elevated review.
9. **Package:** trusted release tooling builds a versioned package and verifies provenance/signatures. The model never receives signing keys.
10. **Install and recover:** install through the existing signed update mechanism with atomic activation where supported; retain the last known-good version and support rollback.
11. **Audit:** record model/provider/version, task and proposal IDs, source revision, changed-file hashes, validation evidence, approver, package digest and outcome. Do not store source prompts or private code in a remote service without separate consent.

## 4. Model selection and privacy

- Reuse the existing model catalogue/onboarding rather than creating a second model manager.
- Display model identity, version, licence, download size, storage, RAM/CPU/GPU requirements, supported platforms, coding capability, limitations and local/external processing status.
- Prefer a compatible local model. External coding assistance is a separate per-task data-transfer decision and must disclose the exact code/context leaving the device.
- Never upload secrets, credentials, user documents, private history or the full repository by default.
- Do not treat model confidence or self-reported safety as authorization.
- Model installation remains optional and must not silently change the user's selected model.

## 5. Mandatory enforcement

All enforcement is performed by trusted host code, not prompts:
- Immutable protected-path policy and signed policy bundle.
- Allow-listed operations; no arbitrary shell or model-directed process execution.
- Isolated build/test environment with CPU, memory, disk, time and network limits.
- Network disabled by default; allow only explicit, auditable dependency retrieval when approved.
- Read-only source access and narrowly scoped write access to the generated workspace.
- Dependency pinning, lockfile review, license policy and vulnerability scanning.
- Secret scanning and output redaction before logs or remote calls.
- Human approval before production installation.
- Signed update verification, atomic activation and rollback.
- Fail closed when policy, validation, provenance or recovery state is missing.

## 6. Initial implementation slices

- **SE-0:** threat model, protected-path inventory, trust-zone mapping and threat-driven acceptance tests.
- **SE-1:** read-only repository/module inventory and structured finding contract.
- **SE-2:** proposal schema, writable-path allow-list, protected-path deny-list and diff viewer.
- **SE-3:** disposable generation workspace with bounded resources and no host credentials.
- **SE-4:** validation pipeline and machine-readable evidence bundle.
- **SE-5:** explicit approval, signed package handoff, atomic update and rollback integration.
- **SE-6:** append-only local evolution ledger and privacy controls.
- **SE-7:** opt-in user-facing workflow and documentation.

Each slice must be separately reviewable. Do not enable automatic production updates as part of the MVP.

## 7. Explicit non-goals and release gates

The MVP does not:
- rewrite authentication, permission, trust, policy or update-signing code;
- execute arbitrary generated code on the host;
- self-approve, self-sign or silently install changes;
- run background development or consume compute without user visibility;
- transmit private source externally without explicit scope-specific consent;
- claim safe self-modification merely because tests pass.

Before enabling the feature, require threat-model review, protected-path tests, sandbox escape tests, malicious prompt/code fixtures, dependency and secret scanning, update-signature tests, rollback/recovery tests, and independent security review. Physical-device and real-user validation remain a separate release activity.
