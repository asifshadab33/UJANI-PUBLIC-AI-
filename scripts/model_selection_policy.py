"""Deterministic local-first model selection from declared capabilities."""
from dataclasses import dataclass

@dataclass(frozen=True)
class ModelCandidate:
    model_id: str
    local: bool
    available: bool
    context_capacity: int
    required_context: int
    memory_required_mb: int
    memory_available_mb: int
    external_assistance_allowed: bool = False

def candidate_denials(candidate: ModelCandidate) -> list[str]:
    reasons = []
    if not candidate.model_id: reasons.append("model_id_missing")
    if not candidate.available: reasons.append("model_unavailable")
    if candidate.context_capacity < candidate.required_context: reasons.append("context_capacity_insufficient")
    if candidate.memory_required_mb > candidate.memory_available_mb: reasons.append("memory_capacity_insufficient")
    if not candidate.local and not candidate.external_assistance_allowed: reasons.append("external_assistance_not_allowed")
    return reasons

def select_model(candidates: list[ModelCandidate]) -> ModelCandidate | None:
    eligible = [c for c in candidates if not candidate_denials(c)]
    eligible.sort(key=lambda c: (not c.local, c.memory_required_mb, c.model_id))
    return eligible[0] if eligible else None
