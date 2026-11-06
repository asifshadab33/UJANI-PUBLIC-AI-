# UJANI Public AI — competitor and ecosystem learning register

This is an engineering research register. It records patterns worth evaluating, not endorsements, complete product audits or permission to copy proprietary code, branding or design.

## Product references and lessons

| Project | What to study | UJANI adaptation | Caution |
|---|---|---|---|
| Open WebUI | Local model connections, tools, knowledge workflows and model UX | A simple model/runtime adapter layer and transparent model status | Avoid copying its full interface or assuming every feature is needed |
| LibreChat | Multi-provider architecture, agent/tool integration and auth boundaries | Provider adapters with consistent policy, usage and provenance | Provider APIs, pricing and terms change |
| AnythingLLM | Document-centric workspaces and approachable local knowledge UX | Optional local knowledge vault with explicit indexing and deletion | Document indexing can consume storage/RAM and create sensitive derived data |
| Dify / Flowise | Visual workflows, nodes, debugging and run inspection | Typed workflow nodes with schemas, test fixtures and traceable runs | Visual workflow engines can become complex; begin with fixed safe graphs |
| n8n | Integration catalog, triggers, credentials and execution history | Reusable integrations and clear credential scopes | Third-party integrations enlarge the attack surface |
| SearXNG | Self-hosted metasearch, configurable engines and JSON search interface | Optional search adapter under operator-controlled configuration | It aggregates search engines; it is not a universal crawler or index |
| BOINC | Heterogeneous volunteer devices, intermittent availability and project-scoped tasks | Later opt-in worker model, task boundaries and interruption handling | Scientific batch jobs differ from private commercial AI workloads |
| HTCondor | Queueing, matchmaking, resource requirements and checkpoint/retry patterns | Enterprise scheduling policies and capability matching | Requires careful policy, identity and operations; not a desktop plug-in |
| Salad | Distributed GPU capacity, worker onboarding and workload operations | Study worker health, reliability and capacity verification | Commercial provider model and economics may not match UJANI's non-profit mission |
| Vast.ai | Provider marketplace, hardware listings, pricing and job lifecycle | Study transparent capacity descriptions and metering | Marketplace reliability, security and provider incentives are hard problems |
| Golem | Open distributed compute protocol and provider/requestor concepts | Study task lifecycle, provider identity and verification approaches | Token/economic mechanisms are not required for UJANI |

## Cross-cutting patterns worth adopting

1. **Adapters, not hard dependencies.** Integrate external runtimes and search services behind stable interfaces. Keep the core usable if an optional provider is unavailable.
2. **Workflow traceability.** Show each step, its inputs/outputs, execution location, duration, resource usage and failure reason.
3. **Capability-based matching.** Match jobs to verified runtime/driver/memory capabilities, not just CPU/GPU labels.
4. **Queue and cancellation semantics.** Treat devices as intermittently available; use leases, heartbeats, bounded retries and checkpointing for appropriate tasks.
5. **Credential isolation.** Keep provider credentials in OS-backed secret storage and scope them to the integration that needs them.
6. **User-visible data boundaries.** Show what files/data are indexed, transferred or retained, and provide deletion/revocation.
7. **Reproducibility.** Version tool manifests, model/runtime identifiers and task inputs; record enough provenance to reproduce or diagnose a result.
8. **Operational health.** Measure availability, task success, failure causes, latency and resource use; do not equate registered nodes with usable capacity.

## Evaluation checklist before integrating a project

- Is the project actively maintained? Check recent releases, issue response and security advisories.
- Is its licence compatible with the UJANI repository and distribution model?
- Does it have a small, stable API or protocol we can wrap?
- Can it run without adding a heavy always-on service?
- Does it support our target operating systems and hardware?
- What data leaves the device, and can we disable that path?
- What are its CPU, RAM, storage and network costs?
- Can we test and update it independently?
- What happens when the upstream project disappears or changes its API?

## Non-profit and resource-efficiency lens

UJANI's mission is to improve access to useful compute and reduce unnecessary demand for large centralised resources where technically and environmentally justified. It is not to maximise the number of distributed nodes.

A distributed job is beneficial only when it is safe, useful, reliable and more resource-efficient for the same quality target. Include data transfer, device energy, cooling, idle overhead, retries and hardware wear in comparisons. Keep the local owner in control and do not compensate resource providers in ways that encourage unsafe overuse.

## Research status

This register is a starting point for structured product comparison. Before implementing an integration, create a dated assessment with official documentation links, licence review, feature-level comparison, security review and a small benchmark. Avoid treating marketing pages as proof of performance or environmental benefit.
