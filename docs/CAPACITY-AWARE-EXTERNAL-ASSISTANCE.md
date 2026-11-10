# Capacity-aware external assistance — UJANI Public AI

**Status:** Product and implementation contract. This document does not claim that automatic capacity-based cloud routing, live capacity forecasting, or provider failover is fully implemented.

## Objective

UJANI should use the user's available device resources first. When the device is busy, short of memory, lacks a compatible model, or cannot meet the task's quality/time budget, it should explain the limitation and offer an approved external route only when the user's policy permits it.

A fixed local/cloud percentage is not a design requirement. The decision is per task and may vary with task type, available CPU/RAM/GPU/NPU, model compatibility, latency, privacy, quality and user policy.

## Decision sequence

1. **Understand the task.** Build a bounded task description with required capability, quality target, estimated memory/compute, deadline and data classification.
2. **Inspect local options.** Use fresh OS/runtime capability data and model availability. Distinguish total installed memory from currently available memory; reserve OS and interactive-app headroom.
3. **Try the least-cost suitable local path.** Prefer deterministic OS/local tools, then a compatible local model. Do not launch a model pull or large workload without the relevant user action and resource/network checks.
4. **Consider an explicitly trusted local peer only if a future supported feature is enabled.** Never assume that another nearby or internet-connected device is trusted or available.
5. **Offer external assistance if local routes are unsuitable.** Explain why (for example, insufficient available RAM, device busy, missing compatible model, or deadline risk) and show the configured provider, data that would be sent, expected purpose and any known cost/retention implications.
6. **Require a user decision.** The user may approve the specific subtask, choose a different provider, reduce the task, wait, or decline. No automatic upload of the complete task or selected files.
7. **Minimise and verify.** Send only the smallest necessary subtask and context; redact secrets and unrelated personal data where possible. Validate the response and label local versus external work.
8. **Record provenance and allow revocation.** Show which steps ran locally and which used an external provider. Respect cancellation and future changes to provider permission.

## Policy modes

- **Fortress:** external assistance is blocked. If local execution is not possible, explain and defer/decline.
- **Smart:** local-first; ask before sending a task or data to a configured external provider unless the user has explicitly created a narrower standing permission.
- **Worker / Partner:** these names do not grant blanket cloud or peer permission. Apply the task's explicit data, destination and approval policy; do not infer consent from the selected mode.

A standing permission, if introduced, must be scoped by provider, data classification, operation, size/cost budget and expiry, and must be revocable. Sensitive or restricted data should require a fresh confirmation or remain local according to policy.

## User-facing capacity dashboard

Show separate, time-stamped values for:
- registered device resources (where applicable);
- current available CPU and memory, not merely installed totals;
- OS/application reserve and user-interactive load;
- compatible local models and their estimated memory needs;
- the reason a candidate route was rejected;
- estimated task requirement and capacity shortfall;
- external assistance status: not configured, blocked by policy, awaiting approval, approved for this task, declined, unavailable or completed.

Do not describe a device as available using stale telemetry. Unknown, stale and unavailable must remain distinct states.

## Required safeguards

- No silent background compute, model download or network egress.
- No external route in Fortress.
- No upload of entire folders, histories or documents when a smaller extracted subtask is sufficient.
- No provider credentials in prompts, logs or UI diagnostics.
- Enforce data classification and provider allowlists in trusted application code, not model output.
- Bound request size, time, retries and spend where provider pricing is available.
- Cancellation must stop further dispatch and clearly report any request already accepted by the provider.
- Do not claim that external assistance is cheaper, more private or more energy efficient without measured evidence.

## Acceptance tests for implementation

1. Local route succeeds: no external request is made.
2. Local capacity is insufficient: the UI explains the reason and waits for explicit approval.
3. User declines: no external request is made; queue, reduce or decline gracefully.
4. Fortress mode: external route is rejected even if the provider is configured.
5. Restricted data class: provider egress is blocked or requires the defined elevated approval.
6. Stale/unknown capacity: the UI does not present the device as available.
7. Partial task delegation: only the approved subtask and minimum required context are sent.
8. Cancellation, provider timeout and retry limits are tested.
9. Execution provenance clearly identifies local and external stages.

## Current implementation boundary

The Public AI README describes an external assistance gateway and approved cloud requests, and the desktop prototype includes local resource display. This document adds the requirement for **capacity-driven** fallback and explicit explanation of the shortfall. It does not assert that this full decision flow, dynamic capacity estimator or all acceptance tests already exist.