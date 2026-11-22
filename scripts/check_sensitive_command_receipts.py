#!/usr/bin/env python3
"""Fail CI when known persistent side-effect commands lose receipt wiring.

This is a source-wiring guard, not proof that a receipt is cryptographically
anchored or that every command's authorization policy is correct.
"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
TAURI = ROOT / "apps/desktop/src-tauri/src"


def function_body(relative_path: str, name: str) -> str:
    source = (TAURI / relative_path).read_text(encoding="utf-8")
    match = re.search(r"\bfn\s+" + re.escape(name) + r"(?:\s*<[^>]*>)?\s*\(", source)
    if not match:
        raise SystemExit(f"missing audited command/function: {relative_path}:{name}")
    opening = source.find("{", match.end())
    if opening < 0:
        raise SystemExit(f"missing function body: {relative_path}:{name}")
    closing = re.search(r"\n}\s*(?:\n|$)", source[opening:])
    if not closing:
        raise SystemExit(f"could not bound function body: {relative_path}:{name}")
    return source[opening : opening + closing.start() + 2]


FORBIDDEN_MARKERS = {
    ("security_store.rs", "verify_and_claim_peer_envelope"): ("get_or_create_device_identity",),
    ("device_identity.rs", "rotate_device_identity"): ("get_or_create_device_identity",),
}

REQUIRED_POLICY_MARKERS = {
    ("lib.rs", "execute_local_ai_task"): ("plan_local_task(", "local_plan.accepted"),
    ("lib.rs", "get_security_provider_status"): (
        "validate_security_provider_ids",
        "native_consent::confirm_native",
    ),
    ("lib.rs", "execute_document_workflow"): (
        "validate_document_workflow_content(&content)?;",
        "register_active_task(&control, &task_id)?;",
        "ensure_task_not_cancelled(&token)?;",
        "run_managed_task_registered(",
        "plan_task_graph(&graph, &mode, snapshot.available_memory_mb, &snapshot.capabilities)",
    ),
    ("lib.rs", "execute_cloud_ai_task"): (
        "security_store::reserve_execution_receipt_slot",
        "ujani_core::policy_gate::parse_data_class",
        "ujani_core::policy_gate::evaluate_registered_external_transfer",
        "cloud_gateway::validate_request",
        "native_consent::confirm_native",
        "reserve_cloud_tokens",
        "record_cloud_preflight_outcome",
    ),
    ("lib.rs", "pull_ollama_model"): (
        'mode == "fortress"',
        "native_consent::confirm_native",
        "reserve_execution_receipt_slot",
    ),
    ("lib.rs", "summarize_selected_text"): ('require_local_operation(&snapshot, "document.summarize"',),
    ("lib.rs", "process_selected_document"): ('require_local_operation(&snapshot, "document.summarize"',),
    ("lib.rs", "process_selected_pdf"): ('require_local_operation(&snapshot, "pdf.extract_and_summarize"',),
    ("lib.rs", "translate_text_local"): ('require_local_operation(&snapshot, "translate.text"',),
    ("security_store.rs", "set_cloud_budget_limit"): (
        "confirm_native(",
        "set_cloud_budget_limit_db",
    ),
    ("security_store.rs", "enroll_trusted_peer"): ("confirm_native(",),
    ("security_store.rs", "revoke_trusted_peer"): ("confirm_native(",),
    ("security_store.rs", "verify_and_claim_peer_envelope"): (
        "signed.verify(",
        "claim_nonce_tx(",
    ),
    ("device_identity.rs", "load_or_create_signing_key"): ("with_local_state_lock",),
    ("device_identity.rs", "get_or_create_device_identity"): ("load_or_create_signing_key(&app)",),
    ("device_identity.rs", "sign_peer_task_envelope"): (
        "confirm_native(",
        "load_or_create_signing_key(&app)",
    ),
    ("folder_import.rs", "process_selected_folder"): (
        "require_local_operation(",
        '"file.analyze_text"',
        '"document.summarize"',
    ),
    ("local_search.rs", "find_local_files"): (
        "register_active_task",
        "get_selected_folder_capability_snapshot_for_task",
        "run_selected_folder_preflight",
        '"file.search.local"',
        '"file.search.pdf_text"',
    ),
    ("everyday_tools.rs", "find_duplicate_files"): (
        "register_active_task",
        "get_selected_folder_capability_snapshot_for_task",
        "run_selected_folder_preflight",
        '"file.duplicates.preview"',
    ),
    ("file_insights.rs", "find_large_files"): (
        "register_active_task",
        "get_selected_folder_capability_snapshot_for_task",
        "run_selected_folder_preflight",
        '"file.large_files.preview"',
    ),
    ("lib.rs", "get_selected_folder_capability_snapshot_for_task"): (
        "ensure_task_not_cancelled",
        "spawn_blocking",
        "await_task_with_limits",
    ),
    ("folder_import.rs", "collect_selected_folder"): (
        "collect_selected_folder_with_budget(folder, token, MAX_FOLDER_SCAN_DURATION)",
    ),
    ("folder_import.rs", "collect_selected_folder_with_budget"): (
        "crate::ensure_task_not_cancelled(token)?;",
        "crate::safe_fs::walk_directory(",
        "max_duration: Some(max_duration)",
        "if walked.truncated",
        "crate::ensure_task_not_cancelled(token)?;",
        "crate::safe_fs::open_regular_file_under_root",
    ),
}

REQUIRED_ORDERED_MARKERS = {
    ("lib.rs", "execute_cloud_ai_task"): (
        "security_store::reserve_execution_receipt_slot",
        "ujani_core::policy_gate::parse_data_class",
        "record_cloud_preflight_outcome",
        "ujani_core::policy_gate::evaluate_registered_external_transfer",
        "record_cloud_preflight_outcome",
        "cloud_gateway::validate_request",
        "record_cloud_preflight_outcome",
        "cloud_gateway::endpoint_host",
        "record_cloud_preflight_outcome",
        "native_consent::confirm_native",
        "record_cloud_preflight_outcome",
        "reserve_cloud_tokens",
        "record_cloud_preflight_outcome",
        "cloud_gateway::execute(",
        "if dispatch_started",
        "record_cloud_preflight_outcome",
    ),
    ("lib.rs", "execute_local_ai_task"): (
        "let registration = register_active_task(&control, &task_id)?;",
        "let token = registration.token.clone();",
        "get_capability_snapshot_for_task(&token).await?",
        "let local_plan = plan_local_task(",
        "if !local_plan.accepted",
        "await_task_with_limits(",
        "ollama::list_models()",
        "ensure_task_not_cancelled(&token)?;",
    ),
    ("lib.rs", "execute_document_workflow"): (
        "validate_document_workflow_content(&content)?;",
        "register_active_task(&control, &task_id)?;",
        "ensure_task_not_cancelled(&token)?;",
        "local_receipt_context(",
        "run_managed_task_registered(",
        "get_capability_snapshot_for_task(&task_token).await?",
        "plan_task_graph(&graph, &mode, snapshot.available_memory_mb, &snapshot.capabilities)",
    ),
    ("lib.rs", "summarize_selected_text"): (
        "let registration = register_active_task(&control, &task_id)?;",
        "let token = registration.token.clone();",
        "get_capability_snapshot_for_task(&token).await?",
        "ensure_task_not_cancelled(&token)?;",
        'require_local_operation(&snapshot, "document.summarize"',
        "run_managed_task_registered(",
    ),
    ("lib.rs", "process_selected_document"): (
        "let registration = register_active_task(&control, &task_id)?;",
        "let token = registration.token.clone();",
        "get_capability_snapshot_for_task(&token).await?",
        "ensure_task_not_cancelled(&token)?;",
        'require_local_operation(&snapshot, "document.summarize"',
        "run_managed_task_registered(",
    ),
    ("lib.rs", "process_selected_pdf"): (
        "let registration = register_active_task(&control, &task_id)?;",
        "let token = registration.token.clone();",
        "get_capability_snapshot_for_task(&token).await?",
        "ensure_task_not_cancelled(&token)?;",
        'require_local_operation(&snapshot, "pdf.extract_and_summarize"',
        "decode_pdf_upload_off_thread(",
        "run_managed_task_registered(",
    ),
    ("lib.rs", "translate_text_local"): (
        "let registration = register_active_task(&control, &task_id)?;",
        "let token = registration.token.clone();",
        "get_capability_snapshot_for_task(&token).await?",
        "ensure_task_not_cancelled(&token)?;",
        'require_local_operation(&snapshot, "translate.text"',
        "run_managed_task_registered(",
    ),
    ("folder_import.rs", "process_selected_folder"): (
        "crate::register_active_task(&control, &task_id)?;",
        "crate::get_capability_snapshot_for_task(&task_token).await?;",
        "crate::ensure_task_not_cancelled(&task_token)?;",
        "crate::await_task_with_limits(",
        "crate::ollama::is_available().await",
        "crate::ensure_task_not_cancelled(&task_token)?;",
        "let picker_token = task_token.clone();",
        "blocking_pick_folder()",
        "collect_selected_folder(&path, &picker_token)",
        "crate::ensure_task_not_cancelled(&task_token)?;",
        "crate::run_managed_task_registered(",
    ),
    ("local_search.rs", "find_local_files"): (
        "register_active_task",
        "get_selected_folder_capability_snapshot_for_task",
        "required_operations",
        '"file.search.local"',
        '"file.search.pdf_text"',
        "run_selected_folder_preflight",
        "blocking_pick_folder()",
    ),
    ("everyday_tools.rs", "find_duplicate_files"): (
        "register_active_task",
        "get_selected_folder_capability_snapshot_for_task",
        "run_selected_folder_preflight",
        '"file.duplicates.preview"',
        "blocking_pick_folder()",
        "require_selected_folder",
    ),
    ("file_insights.rs", "find_large_files"): (
        "register_active_task",
        "get_selected_folder_capability_snapshot_for_task",
        "run_selected_folder_preflight",
        '"file.large_files.preview"',
        "blocking_pick_folder()",
        "require_selected_folder",
    ),
    ("search_cache.rs", "store"): (
        "settings_from_db(&db)?.enabled",
        "with_local_state_receipt",
        "transaction_with_behavior(TransactionBehavior::Immediate)",
        "settings_from_db(&tx)",
        "scope_for_path_in_transaction",
        "tx.execute(",
        "trim_cache(&tx",
        "tx.commit()",
    ),
}

REQUIRED_MARKERS = {
    ("lib.rs", "execute_local_ai_task"): (
        "execution_outcome(&result",
        "outcome: Some(outcome)",
        'persist_receipt_or_warn("Local inference"',
    ),
    ("lib.rs", "run_managed_task_registered"): (
        "execution_outcome(&result",
        "outcome: Some(outcome)",
        "persist_receipt_or_warn",
    ),
    ("lib.rs", "execute_document_workflow"): ("run_managed_task_registered(",),
    ("lib.rs", "execute_cloud_ai_task"): (
        "execution_outcome(&result",
        "persist_receipt_or_warn(\"Cloud inference\"",
        "outcome: Some(outcome)",
    ),
    ("lib.rs", "pull_ollama_model"): (
        "reserve_execution_receipt_slot",
        "outcome: Some(outcome)",
    ),
    ("folder_import.rs", "process_selected_folder"): ("run_managed_task_registered(",),
    ("search_cache.rs", "store"): ("with_local_state_receipt", '"search_cache.store"'),
    ("search_cache.rs", "set_local_search_cache_settings"): ("with_local_state_receipt",),
    ("search_cache.rs", "clear_local_search_cache"): ("with_local_state_receipt",),
    ("history.rs", "set_task_history_enabled"): ("with_local_state_receipt",),
    ("history.rs", "save_task_history"): ("with_local_state_receipt",),
    ("history.rs", "delete_task_history"): ("with_local_state_receipt",),
    ("history.rs", "clear_task_history"): ("with_local_state_receipt",),
    ("security_store.rs", "set_cloud_budget_limit"): ("with_local_state_receipt",),
    ("security_store.rs", "enroll_trusted_peer"): ("with_local_state_receipt",),
    ("security_store.rs", "revoke_trusted_peer"): ("with_local_state_receipt",),
    ("security_store.rs", "verify_and_claim_peer_envelope"): ("with_local_state_receipt",),
    ("device_identity.rs", "load_or_create_signing_key"): ("with_local_state_receipt",),
    ("device_identity.rs", "rotate_device_identity"): (
        "load_existing_device_identity",
        "confirm_native(",
        "with_local_state_receipt",
        "with_local_state_lock",
        "latest.device_id != current.device_id",
    ),
}


def require_ordered_markers(
    path: str, name: str, body: str, markers: tuple[str, ...]
) -> None:
    cursor = 0
    for marker in markers:
        position = body.find(marker, cursor)
        if position < 0:
            raise SystemExit(
                f"{path}:{name} has missing or out-of-order safety marker: {marker}"
            )
        cursor = position + len(marker)


def main() -> None:
    for (path, name), markers in REQUIRED_MARKERS.items():
        body = function_body(path, name)
        missing = [marker for marker in markers if marker not in body]
        if missing:
            raise SystemExit(
                f"{path}:{name} is missing receipt wiring: " + ", ".join(missing)
            )
    for (path, name), markers in FORBIDDEN_MARKERS.items():
        body = function_body(path, name)
        present = [marker for marker in markers if marker in body]
        if present:
            raise SystemExit(
                f"{path}:{name} performs a forbidden pre-consent mutation: " + ", ".join(present)
            )
    for (path, name), markers in REQUIRED_POLICY_MARKERS.items():
        body = function_body(path, name)
        missing = [marker for marker in markers if marker not in body]
        if missing:
            raise SystemExit(
                f"{path}:{name} is missing local policy preflight: " + ", ".join(missing)
            )

    for (path, name), markers in REQUIRED_ORDERED_MARKERS.items():
        require_ordered_markers(
            path, name, function_body(path, name), markers
        )

    required_tests = {
        "search_cache.rs": (
            "cache_store_receipt_input_uses_only_non_sensitive_flags",
        ),
        "lib.rs": (
            "managed_task_outcomes_are_classified_for_receipts",
            "failed_receipt_digest_does_not_include_error_text",
            "cloud_preflight_receipt_omits_payload_and_provider_details",
            "schema_three_failure_and_cancellation_receipts_validate",
            "summary_and_translation_fail_closed_without_local_ollama",
            "pdf_pipeline_requires_parser_and_local_model_capabilities",
            "worker_and_partner_are_not_scheduled_by_the_local_policy",
            "cancellation_drops_the_in_flight_preflight_future",
            "selected_folder_tools_do_not_require_ollama",
            "missing_capability_denies_before_folder_picker",
            "unknown_or_stale_capability_denies_before_folder_picker",
            "optional_pdf_search_requires_its_own_capability_before_picker",
            "capability_discovery_failure_denies_before_folder_picker",
            "cancellation_before_picker_denies_without_side_effect",
            "registered_local_preflight_invokes_picker_only_after_all_checks",
            "missing_or_malformed_native_folder_scope_fails_before_traversal",
        ),
        "device_identity.rs": ("public_key_hex_is_fixed_width_and_lowercase",),
        "security_store.rs": ("immediate_local_state_transaction_commits_and_rolls_back",),
    }
    for path, test_names in required_tests.items():
        tests = (TAURI / path).read_text(encoding="utf-8")
        for test_name in test_names:
            if test_name not in tests:
                raise SystemExit(f"missing receipt/policy regression test in {path}: {test_name}")
    core_capability_tests = (ROOT / "crates/ujani-core/src/capability.rs").read_text(encoding="utf-8")
    if "selected_folder_operations_are_registered_and_resource_bounded" not in core_capability_tests:
        raise SystemExit("missing selected-folder operation catalogue/resource regression test")

    policy_gate_tests = (ROOT / "crates/ujani-core/src/policy_gate.rs").read_text(encoding="utf-8")
    for test_name in (
        "data_classification_parser_rejects_unknown_and_unclassified_values",
        "external_transfer_requires_a_registered_operation",
        "external_transfer_requires_approval_and_blocks_sensitive_classes",
    ):
        if test_name not in policy_gate_tests:
            raise SystemExit(f"missing cloud data-classification policy regression test: {test_name}")

    print(f"Sensitive receipt and local-policy wiring checks passed for {len(REQUIRED_MARKERS)} receipt paths and {len(REQUIRED_POLICY_MARKERS)} policy paths.")


if __name__ == "__main__":
    main()
