"""Fail-closed evidence checklist for Public AI release candidates."""

REQUIRED_EVIDENCE = frozenset({
    "native_builds_all_supported_os",
    "installer_smoke_tests",
    "ollama_transport_security_tests",
    "filesystem_boundary_tests",
    "privacy_consent_transport_tests",
    "document_parser_limits_and_fuzzing",
    "crash_freeze_regression_tests",
    "frontend_and_rust_dependency_audits",
    "os_enforced_sandbox_all_supported_os",
    "pdf_worker_containment_all_supported_os",
    "ipc_authorization_coverage_audit",
    "model_download_security_tests",
    "history_key_rotation_and_recovery_tests",
    "signed_release_and_update_provenance",
    "update_rollback_and_recovery_tests",
    "fuzz_campaign_completion",
    "independent_external_security_review",
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
