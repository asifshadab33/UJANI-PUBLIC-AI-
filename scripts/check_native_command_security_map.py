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
CACHE_SOURCE = ROOT / "apps/desktop/src-tauri/src/search_cache.rs"
SECURITY_STORE_SOURCE = ROOT / "apps/desktop/src-tauri/src/security_store.rs"
SECURITY_PROVIDERS_SOURCE = ROOT / "apps/desktop/src-tauri/src/security_providers.rs"
CORE_FILE_PROCESSOR = ROOT / "crates/ujani-core/src/file_processor.rs"
CORE_LIB = ROOT / "crates/ujani-core/src/lib.rs"
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


def missing_defender_status_guards(source: str, adapter_source: str) -> list[str]:
    """Require one-time native consent and bounded read-only Defender collection."""
    missing = []
    try:
        body = rust_function_body(source, "get_defender_status")
    except ValueError:
        return ["get_defender_status"]
    ordered = (
        "native_consent::confirm_native",
        "Duration::from_secs(120)",
        "security_providers::read_defender_status().await",
    )
    positions = [body.find(token) for token in ordered]
    if any(position < 0 for position in positions) or positions != sorted(positions):
        missing.append("get_defender_status consent/order")
    for token in (
        "MAX_OUTPUT_BYTES: usize = 16 * 1024",
        "QUERY_TIMEOUT: Duration = Duration::from_secs(8)",
        ".kill_on_drop(true)",
        "GetSystemDirectoryW",
        "Get-MpComputerStatus -ErrorAction Stop",
        "GetSystemDirectoryW",
    ):
        if token not in adapter_source:
            missing.append("Defender adapter " + token)
    if 'std::env::var_os("SystemRoot")' in adapter_source:
        missing.append("Defender adapter must not trust SystemRoot environment override")
    return missing


def missing_chat_ipc_preflight_guards(source: str) -> list[str]:
    """Reject invalid chat input before task ownership, runtime probes or reservations."""
    try:
        body = rust_function_body(source, "execute_local_ai_task")
    except ValueError:
        return ["execute_local_ai_task"]

    ordered = (
        "ollama::validate_chat_input",
        "register_active_task",
        "get_capability_snapshot_for_task",
        "ollama::list_models",
        "MemoryReservation::acquire_queued",
        "reserve_execution_receipt_slot",
    )
    executable_body = strip_rust_noncode(body)
    positions = [executable_body.find(token) for token in ordered]
    if any(position < 0 for position in positions) or positions != sorted(positions):
        return ["execute_local_ai_task"]
    return []



def missing_async_os_probe_offload_guards(native_source: str, task_engine_source: str) -> list[str]:
    """Keep synchronous OS resource probes off async command executors."""
    missing: list[str] = []
    try:
        resource_probe = rust_function_body(task_engine_source, "read_local_resource_snapshot")
        model_admission = rust_function_body(task_engine_source, "ensure_local_model_ready")
        capability_probe = rust_function_body(native_source, "get_capability_snapshot")
        device_info_command = rust_function_body(native_source, "get_device_info")
        device_info_collector = rust_function_body(native_source, "collect_device_info")
    except ValueError as error:
        return [str(error)]

    def has_suffix(tokens: list[str], suffix: str) -> bool:
        return any(token == suffix or token.endswith("." + suffix) or token.endswith("::" + suffix) for token in tokens)

    resource_tokens = command_tokens(resource_probe)
    if not all(has_suffix(resource_tokens, token) for token in ("spawn_blocking", "System::new", "refresh_memory", "available_memory")):
        missing.append("read_local_resource_snapshot must offload sysinfo memory probes")

    model_tokens = command_tokens(model_admission)
    if model_tokens.count("read_local_resource_snapshot") != 1:
        missing.append("ensure_local_model_ready must reuse exactly one resource snapshot")
    if any(has_suffix(model_tokens, token) for token in ("System::new", "refresh_memory", "available_memory")):
        missing.append("ensure_local_model_ready must not perform synchronous OS probes")

    capability_tokens = command_tokens(capability_probe)
    if not all(has_suffix(capability_tokens, token) for token in ("spawn_blocking", "System::new", "refresh_memory", "available_memory")):
        missing.append("get_capability_snapshot must offload sysinfo probes")

    device_command_tokens = command_tokens(device_info_command)
    collector_tokens = command_tokens(device_info_collector)
    if not has_suffix(device_command_tokens, "spawn_blocking") or not has_suffix(device_command_tokens, "collect_device_info"):
        missing.append("get_device_info must call its collector through spawn_blocking")
    if not all(has_suffix(collector_tokens, token) for token in ("System::new", "refresh_cpu", "refresh_memory")):
        missing.append("collect_device_info must keep OS reads in the blocking collector")
    return missing



def token_index(tokens: list[str], suffix: str) -> int:
    """Find a Rust identifier by its final path component."""
    return next(
        (index for index, token in enumerate(tokens)
         if token == suffix or token.endswith("." + suffix) or token.endswith("::" + suffix)),
        -1,
    )

def missing_model_recommendation_probe_offload_guards(source: str) -> list[str]:
    """Keep synchronous host memory discovery off the async model-recommendation command."""
    try:
        body = rust_function_body(source, "get_model_recommendation")
    except ValueError:
        return ["get_model_recommendation"]

    tokens = command_tokens(body)
    positions = [
        token_index(tokens, "spawn_blocking"),
        token_index(tokens, "System::new"),
        token_index(tokens, "refresh_memory"),
        token_index(tokens, "total_memory"),
        token_index(tokens, "available_memory"),
        token_index(tokens, "await"),
    ]
    if any(position < 0 for position in positions):
        return ["get_model_recommendation must offload bounded memory probes"]
    if positions != sorted(positions):
        return ["get_model_recommendation must run all sysinfo memory probes inside spawn_blocking"]
    return []


def missing_ollama_status_probe_offload_guards(source: str) -> list[str]:
    """Keep host CPU, memory and OS-version discovery off the async Ollama status command."""
    try:
        body = rust_function_body(source, "get_ollama_status")
    except ValueError:
        return ["get_ollama_status"]

    tokens = command_tokens(body)
    blocking_at = token_index(tokens, "spawn_blocking")
    if blocking_at < 0:
        return ["get_ollama_status must offload all host inventory probes before awaiting the worker"]

    # The command legitimately awaits Ollama network calls before collecting host
    # inventory. Validate only the blocking closure and its own join await.
    worker_tokens = tokens[blocking_at + 1:]
    required = ("System::new", "refresh_cpu", "refresh_memory",
                "available_parallelism", "total_memory", "available_memory",
                "System::name", "System::os_version", "await")
    positions = [token_index(worker_tokens, token) for token in required]
    if any(position < 0 for position in positions) or positions != sorted(positions):
        return ["get_ollama_status must offload all host inventory probes before awaiting the worker"]
    return []


def missing_cloud_transfer_preflight_guards(source: str) -> list[str]:
    """Require classification, transfer policy and native approval before cloud side effects."""
    try:
        body = rust_function_body(source, "execute_cloud_ai_task")
    except ValueError:
        return ["execute_cloud_ai_task"]

    ordered = (
        "policy_gate::parse_data_class",
        "evaluate_registered_external_transfer",
        "cloud_gateway::validate_request",
        "native_consent::confirm_native",
        "reserve_execution_receipt_slot",
        "reserve_cloud_tokens",
        "cloud_gateway::execute",
    )
    positions = [body.find(token) for token in ordered]
    if any(position < 0 for position in positions) or positions != sorted(positions):
        return ["execute_cloud_ai_task"]
    return []


def missing_folder_import_preflight_guards(source: str) -> list[str]:
    """Require task/capability checks before the native picker and bounded file traversal."""
    try:
        body = rust_function_body(source, "process_selected_folder")
    except ValueError:
        return ["process_selected_folder"]

    ordered = (
        "register_active_task",
        "get_capability_snapshot_for_task",
        "require_local_operation",
        "blocking_pick_folder",
        "collect_selected_folder",
    )
    positions = [body.find(token) for token in ordered]
    if any(position < 0 for position in positions) or positions != sorted(positions):
        return ["process_selected_folder"]
    if "PathBuf" in body or "selected_path" in body:
        return ["process_selected_folder"]
    return []


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


def missing_history_delete_input_guards(source: str) -> list[str]:
    """Require invalid entry IDs to be rejected before receipt reservation or deletion."""
    try:
        command = rust_function_body(source, "delete_task_history")
    except ValueError:
        return ["delete_task_history"]
    validation_at = command.find("if entry_id <= 0")
    rejection_at = command.find("return Err", validation_at)
    receipt_at = command.find("with_local_state_receipt")
    if not (0 <= validation_at < rejection_at < receipt_at):
        return ["delete_task_history"]
    if "Invalid history entry identifier" not in command[rejection_at:receipt_at]:
        return ["delete_task_history"]
    return []


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
    marker_after_create_at = source.find("mark_history_key_initialized(db)?", create_at)
    if (
        policy_at < 0
        or create_at < 0
        or policy_at > create_at
        or marker_after_create_at < create_at
    ):
        return ["history_key"]
    if source.count("history_key(&db)?") != 3:
        return ["history_key"]
    return []



def missing_local_state_mutation_receipt_guards(cache_source: str, history_source: str, security_source: str) -> list[str]:
    """Require provenance reservation before cache/history/budget state mutations."""
    missing: list[str] = []

    cache_expectations = {
        "set_local_search_cache_settings": ("with_local_state_receipt", "transaction_with_behavior", "UPDATE cache_settings"),
        "clear_local_search_cache": ("confirm_native", "if !approved", "with_local_state_receipt", "transaction_with_behavior", "DELETE FROM search_cache"),
    }
    for name, ordered in cache_expectations.items():
        try:
            body = rust_function_body(cache_source, name)
        except ValueError:
            missing.append(name)
            continue
        positions = [body.find(token) for token in ordered]
        if any(position < 0 for position in positions) or positions != sorted(positions):
            missing.append(name)
        if name == "set_task_history_enabled":
            enable_guard = body.find("if enabled_value")
            if enable_guard < 0 or enable_guard > positions[0]:
                missing.append(name)
        if name == "set_local_search_cache_settings":
            for table in ("search_cache", "recent_searches", "approved_scopes"):
                if f"DELETE FROM {table}" not in body:
                    missing.append(name)
                    break
        if name == "clear_local_search_cache":
            for table in ("search_cache", "recent_searches", "approved_scopes"):
                if f"DELETE FROM {table}" not in body:
                    missing.append(name)
                    break

    history_expectations = {
        "set_task_history_enabled": ("confirm_native", "if !approved", "with_local_state_receipt", "migrate_plaintext_rows", "UPDATE history_settings"),
        "delete_task_history": ("confirm_native", "if !approved", "with_local_state_receipt", "DELETE FROM task_history"),
        "clear_task_history": ("confirm_native", "if !approved", "with_local_state_receipt", "DELETE FROM task_history"),
    }
    for name, ordered in history_expectations.items():
        try:
            body = rust_function_body(history_source, name)
        except ValueError:
            missing.append(name)
            continue
        positions = [body.find(token) for token in ordered]
        if any(position < 0 for position in positions) or positions != sorted(positions):
            missing.append(name)

    try:
        budget = rust_function_body(security_source, "set_cloud_budget_limit")
    except ValueError:
        missing.append("set_cloud_budget_limit")
    else:
        ordered = ("confirm_native", "with_local_state_receipt", "set_cloud_budget_limit_db")
        positions = [budget.find(token) for token in ordered]
        declined = budget.find("if !approved")
        receipt = positions[1]
        if any(position < 0 for position in positions) or positions != sorted(positions) or declined < 0 or declined > receipt:
            missing.append("set_cloud_budget_limit")

    return sorted(set(missing))

def missing_reminder_destructive_confirmation_guards(source: str) -> list[str]:
    """Require native confirmation for reminder deletion before DB work starts."""
    missing: list[str] = []
    try:
        helper = strip_rust_noncode(rust_function_body(source, "destructive_confirmation"))
        handler = strip_rust_noncode(rust_function_body(source, "manage_local_reminders"))
    except ValueError:
        return ["delete", "clear_all"]

    if "ReminderRequest::Delete" not in helper or "valid_id(id)?" not in helper:
        missing.append("delete")
    if "ReminderRequest::ClearAll" not in helper:
        missing.append("clear_all")

    ordered = (
        "destructive_confirmation(&request)?",
        "native_consent::confirm_native",
        "CancellationToken::new()",
        "Duration::from_secs(120)",
        "if !approved",
        "return Err",
        "spawn_blocking",
    )
    positions = [handler.find(token) for token in ordered]
    if any(position < 0 for position in positions) or positions != sorted(positions):
        missing.extend(("delete", "clear_all"))
    return sorted(set(missing))


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



def manifest_entry_errors(group_name: str, name: str, entry: dict) -> list[str]:
    """Reject unknown classifications/receipt states and unsafe cross-group labels."""
    errors: list[str] = []
    classifications = {
        "read_only_local", "read_only_os_info", "read_only_persistent_state",
        "read_only_security_state", "local_model_execution", "local_tool_execution",
        "local_document_execution", "local_image_execution", "selected_folder_metadata_read",
        "selected_folder_read", "selected_folder_read_and_local_execution",
        "external_data_transfer", "disabled_external_mutation", "task_control",
        "encrypted_local_state_mutation", "persistent_local_maintenance",
        "persistent_local_mutation", "persistent_security_mutation",
        "device_identity_mutation", "peer_authenticated_state_mutation",
        "peer_cryptographic_action", "peer_trust_mutation",
    }
    receipts = {
        "none-read-only", "none-debug-only", "none-control-event",
        "none-or-managed-receipt", "required-local-state",
        "metadata-outcome-receipt", "required-but-disabled",
        "receipt-coverage-review-required",
    }
    classification = entry.get("classification")
    receipt = entry.get("receipt")
    status = entry.get("status", "")
    if classification not in classifications:
        errors.append(f"{group_name}:{name}: unknown classification {classification!r}")
    if receipt not in receipts:
        errors.append(f"{group_name}:{name}: unknown receipt state {receipt!r}")
    if group_name == "default_commands" and "peer-preview only" in status:
        errors.append(f"{group_name}:{name}: default command cannot be labelled peer-preview only")
    if group_name == "peer_preview_commands" and "peer-preview only" not in status:
        errors.append(f"{group_name}:{name}: peer command must retain peer-preview-only status")
    if classification in {
        "encrypted_local_state_mutation", "persistent_local_maintenance",
        "persistent_local_mutation", "persistent_security_mutation",
        "device_identity_mutation", "peer_authenticated_state_mutation",
        "peer_cryptographic_action", "peer_trust_mutation",
        "external_data_transfer", "task_control",
        "selected_folder_read_and_local_execution",
    } and receipt == "none-read-only":
        errors.append(f"{group_name}:{name}: mutating command cannot be classified as read-only/no-receipt (including external-transfer and execution commands)")
    return errors


def manifest_mapping_errors(groups: dict, required: tuple[str, ...]) -> list[str]:
    """Validate registered IPC commands against their source-reviewed manifest entries."""
    errors: list[str] = []
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
            errors.extend(manifest_entry_errors(group_name, name, entry))
            if entry.get("receipt") == "required-but-disabled" and "disabled" not in entry.get("status", ""):
                errors.append(f"{group_name}:{name}: disabled mutation must say so in status")
    return errors


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
    errors.extend(manifest_mapping_errors(groups, required))
    task_engine_source = TASK_ENGINE.read_text(encoding="utf-8")
    missing_os_probe_guards = missing_async_os_probe_offload_guards(source, task_engine_source)
    if missing_os_probe_guards:
        errors.extend("async OS probe: " + item for item in missing_os_probe_guards)
    missing_model_probe_guards = missing_model_recommendation_probe_offload_guards(source)
    if missing_model_probe_guards:
        errors.extend("model recommendation OS probe: " + item for item in missing_model_probe_guards)
    missing_ollama_probe_guards = missing_ollama_status_probe_offload_guards(source)
    if missing_ollama_probe_guards:
        errors.extend("Ollama status OS probe: " + item for item in missing_ollama_probe_guards)
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
            "pub fn valid_display_file_name(",
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
        CORE_LIB,
        ("pub use file_processor::{analyze_text, valid_display_file_name, FileAnalysis};",),
        errors,
        "cross-crate display-name validator export",
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

    defender_adapter_source = SECURITY_PROVIDERS_SOURCE.read_text(encoding="utf-8")
    missing_defender_guards = missing_defender_status_guards(source, defender_adapter_source)
    if missing_defender_guards:
        errors.append(
            "Defender status: native confirmation and bounded read-only Defender cmdlet required in "
            + ", ".join(missing_defender_guards)
        )

    missing_cloud_guards = missing_cloud_transfer_preflight_guards(source)
    if missing_cloud_guards:
        errors.append(
            "cloud dispatch: classification, registered transfer policy and native approval "
            "must precede receipt/budget reservation and network execution in "
            + ", ".join(missing_cloud_guards)
        )

    folder_source = require_tokens(
        FOLDER_IMPORT,
        ("pub async fn process_selected_folder(", "register_active_task", "blocking_pick_folder"),
        errors,
        "selected-folder native command",
    )
    missing_folder_guards = missing_folder_import_preflight_guards(folder_source)
    if missing_folder_guards:
        errors.append(
            "selected-folder import: task and live capability preflight must precede the native picker "
            "and bounded traversal in " + ", ".join(missing_folder_guards)
        )

    missing_chat_preflight = missing_chat_ipc_preflight_guards(source)
    if missing_chat_preflight:
        errors.append(
            "local chat IPC: input validation must precede task registration, runtime discovery "
            "and resource/receipt reservations in " + ", ".join(missing_chat_preflight)
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

    missing_history_delete_guards = missing_history_delete_input_guards(history_source)
    if missing_history_delete_guards:
        errors.append(
            "history deletion: invalid entry IDs must fail before receipt reservation in "
            + ", ".join(missing_history_delete_guards)
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

    cache_source = require_tokens(
        CACHE_SOURCE,
        ("fn set_local_search_cache_settings(", "fn clear_local_search_cache("),
        errors,
        "local search-cache state mutations",
    )
    security_store_source = require_tokens(
        SECURITY_STORE_SOURCE,
        ("fn set_cloud_budget_limit(", "fn set_cloud_budget_limit_db("),
        errors,
        "cloud-budget state mutation",
    )
    missing_state_receipts = missing_local_state_mutation_receipt_guards(
        cache_source, history_source, security_store_source
    )
    if missing_state_receipts:
        errors.append(
            "local state mutation provenance: receipt/consent must precede cache, history and budget writes in "
            + ", ".join(missing_state_receipts)
        )

    # Reminder storage must stay encrypted, explicitly disabled by default and bounded.
    reminder_source = require_tokens(
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
    missing_reminder_guards = missing_reminder_destructive_confirmation_guards(reminder_source)
    if missing_reminder_guards:
        errors.append(
            "reminder deletion: native confirmation must cover Delete and ClearAll before the blocking storage worker in "
            + ", ".join(missing_reminder_guards)
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
