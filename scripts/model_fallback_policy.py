"""Bounded fallback decisions; no silent local-to-cloud escalation."""
from dataclasses import dataclass

MAX_LOCAL_ATTEMPTS = 3

@dataclass(frozen=True)
class InferenceFailure:
    reason: str
    retryable: bool
    local_attempts: int
    max_local_attempts: int
    external_assistance_allowed: bool
    task_approved_for_external: bool

def next_execution_route(failure: InferenceFailure) -> str:
    if not isinstance(failure, InferenceFailure):
        return "stop_invalid_state"
    if (
        not isinstance(failure.reason, str)
        or not failure.reason
        or len(failure.reason) > 128
        or type(failure.retryable) is not bool
        or type(failure.local_attempts) is not int
        or type(failure.max_local_attempts) is not int
        or failure.local_attempts < 0
        or failure.max_local_attempts < 0
        or failure.max_local_attempts > MAX_LOCAL_ATTEMPTS
        or failure.local_attempts > failure.max_local_attempts
        or type(failure.external_assistance_allowed) is not bool
        or type(failure.task_approved_for_external) is not bool
    ):
        return "stop_invalid_state"
    if failure.retryable is True and failure.local_attempts < failure.max_local_attempts:
        return "retry_local"
    if (
        failure.external_assistance_allowed is True
        and failure.task_approved_for_external is True
    ):
        return "request_external_approval"
    return "stop_and_explain"
