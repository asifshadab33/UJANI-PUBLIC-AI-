"""Fail-closed declarations for local OS helper capabilities."""
from dataclasses import dataclass

@dataclass(frozen=True)
class OSHelperCapability:
    name: str
    operation: str
    scope: str
    read_only: bool
    network_access: bool
    requires_approval: bool
    max_results: int
    timeout_ms: int

def validate_capability(item: OSHelperCapability) -> list[str]:
    errors = []
    if not item.name or len(item.name) > 80:
        errors.append("invalid name")
    if item.operation not in {"search_files", "search_folders", "inspect_metadata", "find_duplicates"}:
        errors.append("operation not allowlisted")
    if not item.scope or len(item.scope) > 512:
        errors.append("scope must be a bounded user-selected path")
    if not item.read_only:
        errors.append("helper must be read-only")
    if item.network_access:
        errors.append("helper must not access the network")
    if not item.requires_approval:
        errors.append("explicit invocation approval is required")
    if not 1 <= item.max_results <= 5000:
        errors.append("max_results outside 1..5000")
    if not 100 <= item.timeout_ms <= 30000:
        errors.append("timeout_ms outside 100..30000")
    return errors
