"""Fail-closed evidence checklist for Public AI release candidates."""
REQUIRED_EVIDENCE = {
    "native_builds_all_supported_os",
    "installer_smoke_tests",
    "ollama_transport_security_tests",
    "filesystem_boundary_tests",
    "privacy_consent_transport_tests",
    "document_parser_limits_and_fuzzing",
    "crash_freeze_regression_tests",
    "independent_external_test_report",
}

def missing_release_evidence(evidence: set[str]) -> list[str]:
    return sorted(REQUIRED_EVIDENCE - evidence)

def release_ready(evidence: set[str]) -> bool:
    return not missing_release_evidence(evidence)
