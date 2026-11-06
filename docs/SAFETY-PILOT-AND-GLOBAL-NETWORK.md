# Safety integration, energy dashboard, pilot and network gates

## Shared dispatch gate
policy_gate::evaluate_execution is a deterministic pre-dispatch gate intended for every local tool, local model, enterprise worker and cloud-provider adapter immediately before dispatch. It checks registered operations, identifiers, policy version, time/memory budgets, workload intent, worker consent and data egress. Unknown and bulk-messaging intent requires review. Confidential/restricted data is denied by the generic external-transfer path.

This is not a semantic classifier, OS sandbox, network firewall or secret manager. Production must layer abuse detection, authorization verification, OS-enforced isolation, egress allow-lists, rate limits, signed jobs, output validation, audit and cancellation. If an adapter does not call the gate, it is not protected. Fail closed on errors.

## Capability engine
Represent every capability as available, unavailable or unknown, with source, observation time, confidence and platform/runtime version. Unknown is not available. Hardware discovery is read-only and low-frequency; do not benchmark without consent. GPU/NPU presence is not proof of model/runtime compatibility. Refresh memory, thermal and interactive-use telemetry immediately before dispatch.

## Energy dashboard
energy_telemetry keeps direct-meter readings separate from OS and modelled estimates. Show provenance, time window, successful equivalent workload count, output quality, latency, retries, network energy and uncertainty. Never combine measured and estimated values into one unlabeled savings number. The module aggregates typed observations; it does not read a physical meter.

## Internal pilot stages
1. Offline tests with synthetic fixtures only.
2. Two or three explicitly enrolled lab devices; no employee endpoints and no public peers.
3. Deterministic non-sensitive tasks first, then low-risk local inference.
4. Devices must already be powered on. Never wake or power-cycle for scheduling.
5. Owner opt-in, admin schedule, interactive-use pause, thermal/power checks, signed short-lived jobs, default-deny egress, quotas and kill switch.
6. Match output quality and task volume across central/local/distributed baselines; meter worker and network energy; publish failures and uncertainty.
7. Stop on consent failure, isolation failure, data leak, unexplained resource spike or measurement-integrity failure.

## Global network
Do not enable anonymous public nodes yet. Prerequisites: independently reviewed worker isolation, device identity/revocation, signed workload manifests, privacy/data classification, abuse response, output verification, anti-Sybil/reputation design, cross-region legal review, incident response and independently reproducible energy advantage. The first global deliverable should be a protocol and threat model, not an internet-reachable execution service.
