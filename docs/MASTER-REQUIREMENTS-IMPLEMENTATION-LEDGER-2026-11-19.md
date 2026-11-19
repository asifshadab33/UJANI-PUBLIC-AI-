# UJANI Public AI — Master Requirements & Implementation Ledger (2026-11-19)

Reconciles retrieved project discussions and repository evidence. Historical chat coverage is not claimed exhaustive. Status describes current main; open PRs are not counted as merged.

## Product invariants
- Local-first personal AI using the user's device and OS capabilities before external infrastructure.
- Public and Enterprise remain separate products, authorities, permissions and data stores.
- No silent cloud transfer, model install/switch, background compute, device wake, destructive action, arbitrary shell or model-authored executable code.
- Fortress blocks UJANI cloud requests at application policy; this is not OS isolation.
- Worker/Partner unavailable until authenticated transport, receiver authorization, quotas, isolation, audit and review pass.
- Reuse OS functions and existing tools rather than rebuilding them.

## Requirements and status (current main)
| ID | Requirement | Status | Remaining/evidence |
|---|---|---|---|
| PUB-001 | Desktop shell/local chat | Prototype implemented | Device validation |
| PUB-002 | Loopback Ollama | Foundation implemented | Compatibility/privacy/reliability |
| PUB-003 | Cloud/BYOK exact-request consent | Bounded implementation | Provider tests and controls |
| PUB-004 | Fortress/Smart | Policy foundation | OS isolation |
| PUB-005 | Worker/Partner | Unavailable | Peer transport/dispatch |
| PUB-006 | Model catalogue/choice | Partial | Verified licences, checksums, quantisation, exact size, onboarding |
| PUB-007 | Hardware/runtime inventory | Partial | GPU/NPU/driver compatibility and freshness |
| PUB-008 | Capability/tool registry and task planning | Partial | Canonical operation manifest/adapters |
| PUB-009 | Task graph | Planning only | General typed execution |
| PUB-010 | Document/PDF summary/translation | Bounded partial | Format coverage/isolation/testing |
| PUB-011 | Folder import | Bounded implemented | UX/platform tests |
| PUB-012 | File/content/PDF search | Partial | Coverage/reliability |
| PUB-013 | Duplicate and large-file finder | Read-only implemented | Cross-platform regression |
| PUB-014 | Image inspection | Limited | Vision/OCR/conversion optional |
| PUB-015 | Video discovery/understanding | Not implemented | Optional, OS/tool reuse first |
| PUB-016 | Search cache | Foundation implemented | Invalidation/privacy validation |
| PUB-017 | Encrypted opt-in history/export | Foundation implemented | Key recovery/rotation/deletion tests |
| PUB-018 | Cancellation/progress/recovery | Partial | Command-by-command verification |
| PUB-019 | Planner/calendar/email/Zoom/reminders | Not integrated | Consent-based adapters |
| PUB-020 | Capability discovery/optional tools | Partial architecture | Catalogue and safe lifecycle |
| PUB-021 | Security Centre/provider coordination | Partial | Live provider/freshness warnings |
| PUB-022 | Self-evolution | Restricted foundation | Protected-core governance |
| PUB-023 | Updates/installers | Pipeline foundation | Signing and physical testing |
| PUB-024 | Accessibility/stability | Not fully audited | Accessibility, freeze/navigation/upload regressions |
| PUB-025 | Release readiness | Not achieved | Security, cross-platform, signing and review |

## Deduplication findings
1. Catalogue copies are identical. Public app mirror and catalogue must stay byte-identical; Enterprise mirror likewise. Cross-repo parity is a separate gate.
2. Inventory v1/v2 should share normalized internal evidence and separate version serializers.
3. Operation/tool/capability registries, router, task graph, policy gate and native command map need one canonical operation definition. Policy and adapter boundaries remain separate.
4. Filesystem features reuse `safe_fs`, traversal, cancellation, resource limits and metadata types.
5. Cache, history and receipts are separate privacy purposes; do not collapse stores.

## Implementation order
- P0: command inventory and authorization-before-side-effect audit; policy bypass checks; filesystem/PDF/cancel/privacy regressions; release/security gates.
- P1: canonical operation manifest; catalogue parity; inventory normalization; model onboarding/compatibility; security-provider freshness; useful OS integrations.
- P2: planner/connectors, OCR/vision/audio/video optional capabilities and graph execution after typed adapters/verification.

## Completion rule
Complete only when code is on main, exact merged SHA CI is green and evidence scope is stated. Static/schema tests do not prove runtime security or physical-device behaviour.
