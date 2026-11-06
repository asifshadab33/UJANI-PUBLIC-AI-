# UJANI resource-aware execution and safety architecture

## Product objective
Use existing, already-powered, underutilized computing capacity to serve legitimate existing demand, reducing avoidable reliance on centralized AI infrastructure. Do not create demand simply to consume spare capacity.

## Placement policy
- Prefer deterministic local tools where they satisfy the task.
- Prefer an installed local model when quality, privacy and resource limits permit.
- Consider an enterprise worker only if already running, explicitly opted in, within its approved schedule, on AC power, thermally safe, not in interactive use, with fresh telemetry and approved data egress.
- Never wake a sleeping or powered-off computer for a UJANI workload.
- Compare equivalent successful work and include incremental device energy, network energy, retries/failures and attributable overhead.
- Distribute only when the defensible total energy estimate is lower than the central baseline. Otherwise keep work central.
- External cloud services require explicit user/admin approval and data-classification checks.

## Abuse prevention
All execution routes, including local models and cloud providers, must pass a common workload policy. Bulk messaging and unknown intent require review; malicious software development, credential theft, unauthorized access, evasion/persistence and botnet/abuse automation are denied. Authorized security testing requires verified scope and authorization. Apply rate limits, quotas, audit trails, sandboxing and output validation.

The pure intent policy is only one guardrail. It is not a semantic classifier and must not be treated as sufficient to detect deceptive or disguised abuse. Production execution must use layered safety checks.

## Security boundaries
A personal PC is not automatically a trusted cloud server. Enrolment requires authenticated identity, explicit owner consent, revocation, signed task definitions, isolation, least privilege, bounded resources, encrypted transport, data minimization and worker-side revalidation. Do not expose arbitrary shell or unrestricted code execution.

## Measurement
Report direct measurements separately from estimates. Track watt-hours per equivalent successful workload, incremental device energy, network energy, retry energy, central baseline, latency, reliability, interruptions, data transferred and the share of workloads where distribution is lower-energy. Do not claim data-center displacement from device count or theoretical GPU capacity.
