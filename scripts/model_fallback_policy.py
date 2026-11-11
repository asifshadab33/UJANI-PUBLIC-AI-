"""Bounded fallback decisions; no silent local-to-cloud escalation."""
from dataclasses import dataclass

@dataclass(frozen=True)
class InferenceFailure:
    reason: str
    retryable: bool
    local_attempts: int
    max_local_attempts: int
    external_assistance_allowed: bool
    task_approved_for_external: bool

def next_execution_route(failure: InferenceFailure) -> str:
    if failure.local_attempts < 0 or failure.max_local_attempts < 0:
        return "stop_invalid_state"
    if failure.retryable and failure.local_attempts < failure.max_local_attempts:
        return "retry_local"
    if failure.external_assistance_allowed and failure.task_approved_for_external:
        return "request_external_approval"
    return "stop_and_explain"
