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
}


def main() -> None:
    for (path, name), markers in REQUIRED_MARKERS.items():
        body = function_body(path, name)
        missing = [marker for marker in markers if marker not in body]
        if missing:
            raise SystemExit(
                f"{path}:{name} is missing receipt wiring: " + ", ".join(missing)
            )
    tests = (TAURI / "lib.rs").read_text(encoding="utf-8")
    for test_name in (
        "managed_task_outcomes_are_classified_for_receipts",
        "failed_receipt_digest_does_not_include_error_text",
    ):
        if test_name not in tests:
            raise SystemExit(f"missing receipt regression test: {test_name}")
    print(f"Sensitive receipt wiring check passed for {len(REQUIRED_MARKERS)} functions.")


if __name__ == "__main__":
    main()
