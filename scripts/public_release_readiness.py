"""Fail-closed evidence checklist for Public AI release candidates."""

REQUIRED_EVIDENCE = frozenset({
    "native_builds_all_supported_os",
    "installer_smoke_tests",
    "ollama_transport_security_tests",
    "filesystem_boundary_tests",
    "privacy_consent_transport_tests",
    "document_parser_limits_and_fuzzing",
    "crash_freeze_regression_tests",
    "independent_external_test_report",
})


def missing_release_evidence(evidence: set[str]) -> list[str]:
    if not isinstance(evidence, (set, frozenset)):
        return sorted(REQUIRED_EVIDENCE)
    if any(not isinstance(item, str) or item not in REQUIRED_EVIDENCE for item in evidence):
        return sorted(REQUIRED_EVIDENCE)
    return sorted(REQUIRED_EVIDENCE - evidence)


def release_ready(evidence: set[str]) -> bool:
    return not missing_release_evidence(evidence)
