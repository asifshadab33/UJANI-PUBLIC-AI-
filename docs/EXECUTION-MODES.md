# UJANI Public AI execution modes

This document records the intended mode semantics and the current implementation boundary. A selected mode is **not** permission to transfer data or execute a remote task.

## Fortress

**Intended behaviour:** 100% local processing with network access strictly disabled for the UJANI workload.

**Current build:** UJANI policy blocks its cloud-gateway requests in Fortress. This is not yet OS-wide network isolation: the application process and loopback Ollama traffic are not fully isolated. Do not claim that the operating system's network is disabled or that Fortress provides a complete sandbox until platform enforcement is implemented and physically verified.

## Smart

**Intended behaviour:** local-first execution. Use local models and OS capabilities when suitable; use external assistance only when the user explicitly approves the exact transfer and applicable policy permits it.

**Current build:** Smart is the default mode. It supports the BYOK cloud gateway with fresh per-request native approval, data classification, a configurable installation-local daily token ceiling, request limits, timeouts and cancellation. A denial or timeout must not send the request.

## Worker

**Intended behaviour:** an external service may provide bounded plans or delegate structured tasks, while the user's own device performs only approved, allow-listed local compute. The external service must not receive free device control, shell access or unrestricted file access.

**Required gates:** signed structured task contracts, authenticated provider/gateway identity, exact-scope user approval before any outbound transfer, local capability and policy checks, resource and provider budgets, bounded payloads/results, timeout/cancellation, local result validation and durable signed receipts.

**Current build:** policy preview and draft task-contract foundations only. Production cloud planning/delegation and execution are not implemented; Worker remains unavailable.

## Partner

**Intended behaviour:** iterative, bounded collaboration between local AI and an approved cloud/partner service.

**Required gates:** explicit approval before each outbound transfer, hard caps on collaboration rounds and data volume, cancellation at each stage, local validation and verification of returned results, and no automatic sharing of complete files or conversation history.

**Current build:** policy preview and draft task-contract foundations only. Production Partner iteration is not implemented; Partner remains unavailable.

## Cross-mode invariants

- Mode and permission are separate. Selecting Worker or Partner never grants external consent.
- Fortress never falls back to external processing.
- Smart cloud requests require explicit, fresh approval; denial, timeout or cancellation fails closed.
- Worker tasks must be structured and allow-listed; remote instructions cannot become arbitrary shell commands or unrestricted code execution.
- Partner transfers are bounded per round; no silent expansion to full files or conversation history.
- No Worker or Partner execution until authentication, consent, policy, isolation, quotas, cancellation, verification and audit gates pass end-to-end and receive independent security review.
- Public AI remains independently usable; it does not require UJANI Enterprise AI to run local tasks.
