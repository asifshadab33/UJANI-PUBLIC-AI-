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
    ("lib.rs", "execute_cloud_ai_task"): (
        "if !external_data_approved",
        "native_consent::confirm_native",
        "reserve_cloud_tokens",
    ),
    ("lib.rs", "pull_ollama_model"): (
        'mode == "fortress"',
        "native_consent::confirm_native",
        "reserve_execution_receipt_slot",
    ),
    ("lib.rs", "execute_document_workflow"): ("plan_task_graph(",),
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
}

REQUIRED_MARKERS = {
    ("lib.rs", "run_managed_task_registered"): (
        "execution_outcome(&result",
        "outcome: Some(outcome)",
        "persist_receipt_or_warn",
    ),
    ("lib.rs", "execute_document_workflow"): ("run_managed_task(",),
    ("lib.rs", "execute_cloud_ai_task"): (
        "execution_outcome(&result",
        "persist_receipt_or_warn(\"Cloud inference\"",
        "outcome: Some(outcome)",
    ),
    ("lib.rs", "pull_ollama_model"): (
        "reserve_execution_receipt_slot",
        "outcome: Some(outcome)",
    ),
    ("folder_import.rs", "process_selected_folder"): ("run_managed_task(",),
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
    ),
}


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

    tests = (TAURI / "lib.rs").read_text(encoding="utf-8")
    for test_name in (
        "managed_task_outcomes_are_classified_for_receipts",
        "failed_receipt_digest_does_not_include_error_text",
        "summary_and_translation_fail_closed_without_local_ollama",
        "pdf_pipeline_requires_parser_and_local_model_capabilities",
        "worker_and_partner_are_not_scheduled_by_the_local_policy",
        "public_key_hex_is_fixed_width_and_lowercase",
    ):
        if test_name not in tests:
            raise SystemExit(f"missing receipt regression test: {test_name}")
    print(f"Sensitive receipt and local-policy wiring checks passed for {len(REQUIRED_MARKERS)} receipt paths and {len(REQUIRED_POLICY_MARKERS)} policy paths.")


if __name__ == "__main__":
    main()
