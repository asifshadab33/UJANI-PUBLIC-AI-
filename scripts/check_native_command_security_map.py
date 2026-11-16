#!/usr/bin/env python3
"""Check native IPC inventory completeness and selected security-wiring invariants.

These static checks catch regressions in known command paths. They do not prove
runtime enforcement, durable receipt integrity, sandbox effectiveness or absence
of alternate call paths; those still require code review and platform testing.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "apps/desktop/src-tauri/src/lib.rs"
MANIFEST = ROOT / "docs/native-command-security-map.json"
CORE_CAPABILITY = ROOT / "crates/ujani-core/src/capability.rs"
CORE_POLICY_GATE = ROOT / "crates/ujani-core/src/policy_gate.rs"
CORE_TASK_GRAPH = ROOT / "crates/ujani-core/src/task_graph.rs"
APP_SOURCE = ROOT / "apps/desktop/src/App.tsx"
FOLDER_IMPORT = ROOT / "apps/desktop/src-tauri/src/folder_import.rs"
REMINDER_STORE = ROOT / "apps/desktop/src-tauri/src/reminder_store.rs"
HISTORY_SOURCE = ROOT / "apps/desktop/src-tauri/src/history.rs"
CORE_FILE_PROCESSOR = ROOT / "crates/ujani-core/src/file_processor.rs"
CORE_IMAGE_PROCESSOR = ROOT / "crates/ujani-core/src/image_processor.rs"
TASK_ENGINE = ROOT / "apps/desktop/src-tauri/src/task_engine.rs"


def find_matching_delimiter(source: str, opening_index: int, opening: str, closing: str) -> int:
    """Find a matching delimiter while ignoring Rust comments and string literals."""
    if opening_index >= len(source) or source[opening_index] != opening:
        raise ValueError(f"expected {opening!r} at delimiter start")
    depth = 0
    index = opening_index
    while index < len(source):
        if source.startswith("//", index):
            newline = source.find("\n", index + 2)
            index = len(source) if newline < 0 else newline + 1
            continue
        if source.startswith("/*", index):
            index += 2
            comment_depth = 1
            while index < len(source) and comment_depth:
                if source.startswith("/*", index):
                    comment_depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    comment_depth -= 1
                    index += 2
                else:
                    index += 1
            if comment_depth:
                raise ValueError("unterminated Rust block comment")
            continue

        # Rust raw strings: r"...", r#"..."#, r##"..."##, etc.
        raw = re.match(r'r(#+)?"', source[index:])
        if raw:
            hashes = raw.group(1) or ""
            terminator = '"' + hashes
            index += raw.end()
            end = source.find(terminator, index)
            if end < 0:
                raise ValueError("unterminated Rust raw string")
            index = end + len(terminator)
            continue

        char = source[index]
        if char == '"':
            index += 1
            while index < len(source):
                if source[index] == "\\":
                    index += 2
                elif source[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError("unterminated Rust string literal")
            continue

        if char == opening:
            depth += 1
        elif char == closing:
            depth -= 1
            if depth == 0:
                return index
        index += 1
    raise ValueError(f"could not find matching {closing!r} delimiter")


def strip_rust_noncode(text: str) -> str:
    """Blank Rust comments and string literals without exposing their tokens."""
    output: list[str] = []
    index = 0

    def blank(segment: str) -> str:
        return "".join("\n" if char == "\n" else " " for char in segment)

    while index < len(text):
        if text.startswith("//", index):
            newline = text.find("\n", index + 2)
            end = len(text) if newline < 0 else newline
            output.append(blank(text[index:end]))
            index = end
            continue

        if text.startswith("/*", index):
            start = index
            index += 2
            depth = 1
            while index < len(text) and depth:
                if text.startswith("/*", index):
                    depth += 1
                    index += 2
                elif text.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            if depth:
                raise ValueError("unterminated Rust block comment")
            output.append(blank(text[start:index]))
            continue

        raw = re.match(r'r(#+)?"', text[index:])
        if raw:
            hashes = raw.group(1) or ""
            terminator = '"' + hashes
            start = index
            index += raw.end()
            end = text.find(terminator, index)
            if end < 0:
                raise ValueError("unterminated Rust raw string")
            index = end + len(terminator)
            output.append(blank(text[start:index]))
            continue

        # Rust character literals must be masked before string handling: a
        # literal such as '"' must not open a string that consumes later code.
        # The closing quote is required, so lifetimes like &'a str do not match.
        char_literal = re.match(
            r"'(?:\\(?:u\{[0-9A-Fa-f_]+\}|x[0-9A-Fa-f]{2}|.)|[^\\'\n])'",
            text[index:],
        )
        if char_literal:
            output.append(blank(char_literal.group(0)))
            index += char_literal.end()
            continue

        if text[index] == '"':
            start = index
            index += 1
            while index < len(text):
                if text[index] == "\\":
                    index += 2
                elif text[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError("unterminated Rust string literal")
            output.append(blank(text[start:index]))
            continue

        output.append(text[index])
        index += 1

    return "".join(output)


def command_tokens(text: str) -> list[str]:
    cleaned = strip_rust_noncode(text)
    return re.findall(r"(?<![\w$])(?:[A-Za-z_]\w*::)*[A-Za-z_]\w*", cleaned)


def rust_function_body(source: str, function_name: str) -> str:
    """Return the actual Rust function body, locating syntax only in masked code."""
    # Preserve offsets/newlines while blanking comments and strings first. This
    # prevents a commented-out fake fn declaration from shadowing the real item.
    masked = strip_rust_noncode(source)
    match = re.search(rf"\bfn\s+{re.escape(function_name)}\s*\(", masked)
    if not match:
        raise ValueError(f"could not locate Rust function {function_name}")
    opening_paren = masked.find("(", match.start())
    closing_paren = find_matching_delimiter(masked, opening_paren, "(", ")")
    opening_brace = masked.find("{", closing_paren + 1)
    if opening_brace < 0:
        raise ValueError(f"could not locate body for Rust function {function_name}")
    closing_brace = find_matching_delimiter(masked, opening_brace, "{", "}")
    return source[opening_brace + 1:closing_brace]


def missing_demo_command_guards(source: str) -> list[str]:
    """Identify demo commands whose own bodies lack the build-profile guard."""
    expected = ("list_local_models", "run_local_model_task", "run_local_task")
    missing = []
    for name in expected:
        try:
            body = rust_function_body(source, name)
        except ValueError:
            missing.append(name)
            continue
        if "debug_demo_commands_enabled" not in command_tokens(body):
            missing.append(name)
    return missing


def missing_file_metadata_guards(source: str) -> list[str]:
    """Ensure selected-file metadata IPC commands delegate to bounded core parsers."""
    expected = {
        "analyze_text_file": "LocalTaskEngine::analyze_text",
        "inspect_image_file": "LocalTaskEngine::inspect_image",
    }
    missing = []
    for command, validator in expected.items():
        try:
            body = rust_function_body(source, command)
        except ValueError:
            missing.append(command)
            continue
        if validator not in command_tokens(body):
            missing.append(command)
    return missing


def missing_document_ipc_preflight_guards(source: str) -> list[str]:
    """Require caller-controlled document inputs to be rejected before task/runtime work."""
    expected = {
        "execute_document_workflow": (
            "task_engine::validate_document_request",
            "register_active_task",
        ),
        "process_selected_document": (
            "task_engine::validate_document_request",
            "register_active_task",
            "get_capability_snapshot_for_task",
        ),
        "process_selected_pdf": (
            "task_engine::validate_pdf_metadata",
            "validate_bounded_pdf_encoded_length",
            "register_active_task",
            "get_capability_snapshot_for_task",
            "decode_pdf_upload_off_thread",
        ),
    }
    missing = []
    for command, ordered_tokens in expected.items():
        try:
            body = rust_function_body(source, command)
        except ValueError:
            missing.append(command)
            continue
        positions = [body.find(token) for token in ordered_tokens]
        if any(position < 0 for position in positions) or positions != sorted(positions):
            missing.append(command)
    return missing


def missing_history_receipt_guards(source: str) -> list[str]:
    """Require bounded history IPC validation before receipt reservation and metadata-only receipts."""
    missing = []
    try:
        command = rust_function_body(source, "save_task_history")
        metadata = rust_function_body(source, "history_receipt_metadata")
    except ValueError:
        return ["save_task_history"]

    validation_at = command.find("validate_history_values")
    metadata_at = command.find("history_receipt_metadata")
    receipt_at = command.find("with_local_state_receipt")
    if not (0 <= validation_at < metadata_at < receipt_at):
        missing.append("save_task_history")
    if "sha256_hex" in command_tokens(command):
        missing.append("save_task_history")

    metadata_tokens = command_tokens(metadata)
    required = ("mode", "model", "kind", "prompt_bytes", "output_bytes")
    if any(token not in metadata_tokens for token in required):
        missing.append("history_receipt_metadata")
    if "prompt" in metadata_tokens or "output" in metadata_tokens:
        missing.append("history_receipt_metadata")
    return sorted(set(missing))


def missing_history_listing_receipt_guards(source: str) -> list[str]:
    """Require a metadata-only receipt around history listing's migration/retention writes."""
    try:
        command = rust_function_body(source, "list_task_history")
    except ValueError:
        return ["list_task_history"]

    receipt_at = command.find("with_local_state_receipt")
    migration_at = command.find("migrate_plaintext_rows")
    cleanup_at = command.find("DELETE FROM task_history")
    read_at = command.find("list_secure")
    if not (0 <= receipt_at < migration_at < read_at and receipt_at < cleanup_at < read_at):
        return ["list_task_history"]
    if '"history.entry.list"' not in command or "history.entry.list-maintenance.v1" not in command:
        return ["list_task_history"]
    if "serde_json::to_vec(&entries.len())" not in command:
        return ["list_task_history"]
    return []




def missing_history_key_recovery_guards(source: str) -> list[str]:
    """Prevent silently replacing a lost OS-vault key when encrypted history may exist."""
    required = (
        "fn history_has_rows(",
        "fn history_key_was_initialized(",
        "fn mark_history_key_initialized(",
        "fn history_has_encrypted_rows(",
        "fn missing_history_key_error(",
        "let has_rows = history_has_rows(db)?;",
        "let key_was_initialized = history_key_was_initialized(db)?;",
        "let has_encrypted_rows = history_has_encrypted_rows(db)?;",
        "history_key(&db)?",
        "mark_history_key_initialized(db)?",
    )
    if any(token not in source for token in required):
        return ["history_key"]
    policy_at = source.find(
        "if let Some(message) = missing_history_key_error(has_rows, key_was_initialized, has_encrypted_rows)"
    )
    create_at = source.find("entry.set_password(encoded.as_str())")
    if policy_at < 0 or create_at < 0 or policy_at > create_at:
        return ["history_key"]
    if source.count("history_key(&db)?") != 3:
        return ["history_key"]
    return []


def registered_commands(source: str) -> tuple[list[str], list[str]]:
    marker = "tauri::generate_handler!["
    start = source.find(marker)
    if start < 0:
        raise ValueError("could not locate tauri::generate_handler! in lib.rs")
    opening = start + len(marker) - 1
    body_end = find_matching_delimiter(source, opening, "[", "]")
    default = [item for item in command_tokens(source[opening + 1:body_end]) if item != "peer_command"]

    peer = re.search(
        r'#\[cfg\(feature = "peer-preview"\)\]\s*'
        r'let builder = builder\.invoke_handler\(ujani_invoke_handler!\(',
        source,
    )
    if not peer:
        raise ValueError("could not locate peer-preview handler expansion")
    peer_opening = peer.end() - 1
    peer_end = find_matching_delimiter(source, peer_opening, "(", ")")
    return default, command_tokens(source[peer_opening + 1:peer_end])


def require_tokens(path: Path, tokens: tuple[str, ...], errors: list[str], label: str) -> str:
    try:
        source = path.read_text(encoding="utf-8")
    except OSError as error:
        errors.append(f"{label}: could not read {path}: {error}")
        return ""
    for token in tokens:
        if token not in source:
            errors.append(f"{label}: missing source invariant {token!r}")
    return source


def main() -> int:
    source = SOURCE.read_text(encoding="utf-8")
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1:
        raise ValueError("unsupported native command security map schema_version")

    default, peer = registered_commands(source)
    groups = {
        "default_commands": (default, manifest.get("default_commands", {})),
        "peer_preview_commands": (peer, manifest.get("peer_preview_commands", {})),
    }
    errors: list[str] = []
    required = ("classification", "capability", "scope", "effects", "boundary", "receipt", "tests", "status", "caller", "data_classification", "consent", "cancellation", "resource_limits", "failure_mode")
    for group_name, (registered, mapped) in groups.items():
        if len(registered) != len(set(registered)):
            errors.append(f"{group_name}: duplicate command in native handler")
        if not isinstance(mapped, dict):
            errors.append(f"{group_name}: manifest entries must be an object")
            continue
        missing = sorted(set(registered) - set(mapped))
        stale = sorted(set(mapped) - set(registered))
        if missing:
            errors.append(f"{group_name}: commands missing audit mapping: {', '.join(missing)}")
        if stale:
            errors.append(f"{group_name}: mappings no longer registered: {', '.join(stale)}")
        for name, entry in mapped.items():
            if not isinstance(entry, dict):
                errors.append(f"{group_name}:{name}: mapping must be an object")
                continue
            for field in required:
                value = entry.get(field)
                if not isinstance(value, str) or not value.strip():
                    errors.append(f"{group_name}:{name}: missing non-empty {field}")
            if entry.get("receipt") == "required-but-disabled" and "disabled" not in entry.get("status", ""):
                errors.append(f"{group_name}:{name}: disabled mutation must say so in status")
    # The shared local policy evaluator must remain on both ordinary local
    # operation planning and every node in a local task graph.
    require_tokens(
        CORE_CAPABILITY,
        ("evaluate_local_registered_operation(", "policy.decision != crate::policy_gate::GateDecision::Allow"),
        errors,
        "local capability planner",
    )
    require_tokens(
        CORE_TASK_GRAPH,
        ("evaluate_local_registered_operation(", "Shared local execution policy blocked node"),
        errors,
        "local task-graph planner",
    )
    require_tokens(
        CORE_CAPABILITY,
        (
            "pub max_runtime_seconds: u32",
            '"file.search.local"',
            '"file.search.pdf_text"',
            '"file.duplicates.preview"',
            '"file.large_files.preview"',
        ),
        errors,
        "registered local operation budgets",
    )
    require_tokens(
        CORE_POLICY_GATE,
        (
            "pub fn evaluate_local_registered_operation(",
            "let max_runtime_seconds = operation",
            ".map(|value| value.max_runtime_seconds)",
            "requested_runtime_seconds: max_runtime_seconds",
            "external_transfer: false",
        ),
        errors,
        "shared local policy gate",
    )

    # Mock/demo commands must remain development-only at both UI and native IPC
    # boundaries; release builds must not silently substitute word-count output.
    missing_demo_guards = missing_demo_command_guards(source)
    if missing_demo_guards:
        errors.append(
            "debug demo commands: missing build-profile guard in "
            + ", ".join(missing_demo_guards)
        )

    missing_file_metadata = missing_file_metadata_guards(source)
    if missing_file_metadata:
        errors.append(
            "selected-file metadata commands: missing bounded core parser delegation in "
            + ", ".join(missing_file_metadata)
        )
    require_tokens(
        CORE_FILE_PROCESSOR,
        (
            "pub(crate) fn valid_display_file_name(",
            "if !valid_display_file_name(file_name)",
            "character.is_control()",
            "character == '/'",
            "character == char::from(92)",
            "character == ':'",
        ),
        errors,
        "shared pathless display-name validation",
    )
    require_tokens(
        CORE_IMAGE_PROCESSOR,
        (
            "use crate::file_processor::valid_display_file_name;",
            "if !valid_display_file_name(file_name)",
            "MAX_IMAGE_BYTES",
            "MAX_ORIGINAL_IMAGE_BYTES",
        ),
        errors,
        "bounded image-header IPC input",
    )
    require_tokens(
        APP_SOURCE,
        ("if (!import.meta.env.DEV)", "import.meta.env.DEV && <div className=\"controls\">", "Run word-count demo"),
        errors,
        "development-only demo UI",
    )

    # Keep the model pull fail-closed until registry artifact and daemon-store
    # evidence are available. A future enablement must replace this invariant.
    if not re.search(r"fn model_download_preflight_ready\(\)\s*->\s*bool\s*\{\s*false\s*\}", source):
        errors.append("model download: fail-closed preflight must remain false until artifact/store verification is implemented")
    if "evaluate_registered_external_transfer" not in source or '"cloud.ai.generate"' not in source:
        errors.append("cloud dispatch: registered external-transfer policy must remain on the native dispatch path")

    # Document IPC validates caller-controlled metadata and size before task/runtime work.
    missing_document_preflight = missing_document_ipc_preflight_guards(source)
    if missing_document_preflight:
        errors.append(
            "document IPC: input preflight must precede registration, capability discovery and decode in "
            + ", ".join(missing_document_preflight)
        )
    require_tokens(
        TASK_ENGINE,
        (
            "pub(crate) fn validate_document_request(",
            "pub(crate) fn validate_pdf_metadata(",
            "validate_pdf_metadata(file_name, mode)?",
        ),
        errors,
        "shared document IPC preflight validators",
    )

    # History IPC must reject oversized caller content before reserving a receipt.
    history_source = require_tokens(
        HISTORY_SOURCE,
        (
            "fn validate_history_values(",
            "prompt.len() > MAX_PROMPT",
            "output.len() > MAX_OUTPUT",
            "fn history_receipt_metadata(",
        ),
        errors,
        "history IPC input validation",
    )
    missing_history_guards = missing_history_receipt_guards(history_source)
    if missing_history_guards:
        errors.append(
            "history persistence: missing early validation or content-free receipt metadata in "
            + ", ".join(missing_history_guards)
        )

    missing_history_list_guards = missing_history_listing_receipt_guards(history_source)
    if missing_history_list_guards:
        errors.append(
            "history listing: maintenance writes must be receipt-wrapped with count-only output metadata in "
            + ", ".join(missing_history_list_guards)
        )

    missing_history_key_guards = missing_history_key_recovery_guards(history_source)
    if missing_history_key_guards:
        errors.append(
            "history key recovery: do not create a replacement key when encrypted rows may exist in "
            + ", ".join(missing_history_key_guards)
        )

    # Folder selection must be registered/cancellable before opening the native picker.
    folder_source = require_tokens(
        FOLDER_IMPORT,
        ("register_active_task", "blocking_pick_folder", "ensure_task_not_cancelled"),
        errors,
        "folder import cancellation",
    )
    if folder_source and folder_source.find("register_active_task") > folder_source.find("blocking_pick_folder"):
        errors.append("folder import: task registration must precede the native folder picker")

    # Reminder storage must stay encrypted, explicitly disabled by default and bounded.
    require_tokens(
        REMINDER_STORE,
        (
            "with_local_state_receipt",
            "transaction_with_behavior(TransactionBehavior::Immediate)",
            "require_enabled",
            "encrypt_record",
            "decrypt_record",
            "MAX_REMINDERS",
            "MAX_SNOOZE_MS",
            "notifications_enabled: false",
            "upcoming_reminders",
            "ReminderRequest::ClearAll",
            "DELETE FROM local_reminders",
        ),
        errors,
        "local reminder storage",
    )

    if errors:
        print("Native command security-map check FAILED:")
        for error in errors:
            print(" - " + error)
        return 1
    print(
        "Native command security-map check passed: "
        f"{len(default)} default commands and {len(peer)} peer-preview commands have explicit audit mappings."
    )
    print("Selected source-wiring checks passed; this is not a security sign-off or proof of runtime enforcement.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
