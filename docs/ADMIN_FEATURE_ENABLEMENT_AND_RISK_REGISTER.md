# Public AI — administrator feature enablement and risk register

**Status:** implementation inventory and enablement contract; not a security certification.

This document distinguishes features that are already usable from features that are present only as contracts, previews or plans. A visible setting must never be mistaken for an authorization boundary. Native code must enforce every permission before a side effect.

## Who controls features

UJANI Public AI is a personal-device application. The local device owner controls device-wide settings. A UI preference or browser-local value is not proof of administrator authority; privileged settings must be checked by the native runtime and, where applicable, the operating system.

## Required feature-card help

Every feature presented in the future Feature & Permissions Centre must show:

- **Purpose:** what it does and a practical example.
- **Status:** Available, Requires confirmation, Administrator configurable, Blocked by safety gate, or Not implemented.
- **Risk:** Low, Medium, High or Critical, with a plain-language reason.
- **Scope:** selected files/folders, device metadata, network/provider, or other exact resource boundary.
- **Data movement:** local-only, external destination, or unknown/unverified.
- **Resource budget:** CPU/RAM/storage/network/time limits and whether the feature runs only after an explicit action.
- **Permissions and consent:** device-owner approval, per-task confirmation, and revocation path.
- **Limitations:** what the feature does not do.
- **Audit:** whether an operation receipt is produced and what remains uncovered.

No feature may be enabled solely by hiding/showing a UI control. The native policy gate must re-check authority, scope, consent, capability freshness and cancellation before the side effect.

## Current implementation inventory

| Feature | Current state | Risk / enablement rule |
|---|---|---|
| Device-local clock | Available; browser/device API only | Low. No network or persistent data access. |
| Duplicate-file finder | Available; selected-folder, bounded, read-only hash scan | Medium. User selects folder; limits and cancellation remain visible. Never deletes or modifies files. |
| Large-file finder | Available; selected-folder metadata scan | Low–Medium. User selects scope; no file contents opened. |
| Writing shortcuts | Available; prompt templates only | Low. Selecting a template does not submit a task or enable cloud access. |
| Local Finder / file workflows | Available only within existing user-selected and bounded native operations | Medium. Preserve selected-scope, size, cancellation and receipt gates. |
| Local Ollama inference | Optional, user-initiated local runtime | Medium–High resource impact. Must re-check available memory and user-selected model. No silent model install or inference. |
| UJANI-managed model downloads | Disabled / fail-closed | High supply-chain and storage risk. Do not enable until exact publisher, immutable artifact digest, licence, size, canonical model-store path, free-space preflight, cancellation and receipt coverage are verified. |
| BYOK cloud requests | Available only in Smart mode with fresh per-task approval | High privacy risk. Show provider, endpoint, model and exact task; changed inputs invalidate approval. Fortress remains local-only. |
| Security Centre provider status | Read-only preview; currently Windows aggregate antivirus/firewall categories only | Medium. One-time native confirmation; no rule changes, file inspection, threat-event collection or remediation. |
| External integration directory | Links only; no account connection or data exchange | Low for opening links; external provider privacy/account terms apply. |
| Worker / Partner peer execution | Disabled | Critical. Do not add an administrator override until authenticated enrollment, encrypted transport, receiver-side authorization, OS isolation, bounded task contracts, cancellation, revocation and adversarial tests pass. |
| Screenshot capture / screenshot-to-chat | Not implemented | Do not expose an enable switch until capture, disclosure, attachment, retention and permission paths exist and are tested. |
| OS application connectors, message sending or calendar mutation | Not implemented as active connectors | High. Require per-provider, per-scope consent and action preview; read-only access first. |
| Arbitrary shell, unrestricted process launch or unrestricted remote execution | Not an approved feature | Critical. No global enable switch or administrator bypass. |

## Enablement state machine

Use these states in the UI and native policy model:

1. `available_by_default` — low-risk, local, read-only capability.
2. `available_after_confirmation` — user-initiated operation with exact scope and one-time approval.
3. `administrator_configurable` — native enforcement exists, and the device owner/admin can configure a bounded allowlist or policy.
4. `blocked_pending_safety_gate` — implementation exists but one or more required security, consent, platform or test gates are missing.
5. `not_implemented` — no functional implementation exists; do not present a working toggle.

A blocked feature must explain the missing prerequisites. Administrator configuration cannot override a missing native enforcement boundary, operating-system permission, user consent, or failed safety test.

## Minimum tests before a feature becomes administrator-configurable

- Deny by default when policy is missing, corrupt, stale or unavailable.
- Reject a non-admin caller and a UI-only forged enablement flag.
- Verify the exact resource scope before any filesystem, process or network side effect.
- Require fresh user consent where the data or action belongs to the device owner.
- Verify cancellation, timeout, resource limits and safe recovery.
- Emit an operation receipt for persistent or external side effects without logging prompts, secrets or file contents.
- Prove revocation prevents the next operation and invalidates cached authorization.
- Test the packaged application on every supported operating system.

## Release boundary

The Public AI native command audit in issue #120 remains a prerequisite for broadening sensitive features. The current project status explicitly records that `safety_policy` and `policy_gate` are not yet universal gates on every native command. Until that audit is complete, do not make a blanket administrator switch that enables all native commands.
