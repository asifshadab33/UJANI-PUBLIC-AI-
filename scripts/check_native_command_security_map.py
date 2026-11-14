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
        CORE_POLICY_GATE,
        ("pub fn evaluate_local_registered_operation(", "external_transfer: false", "max_runtime_seconds: 240"),
        errors,
        "shared local policy gate",
    )

    # Mock/demo commands must remain development-only at both UI and native IPC
    # boundaries; release builds must not silently substitute word-count output.
    if lib_source_count := source.count("debug_demo_commands_enabled()"):
        if lib_source_count < 4:
            errors.append("debug demo commands: expected native release guards for model inventory, mock inference and word-count demo")
    else:
        errors.append("debug demo commands: native build-profile guard is missing")
    app_source = require_tokens(
        APP_SOURCE,
        ("if (!import.meta.env.DEV)", "import.meta.env.DEV && <div className=\"controls\">", "Run word-count demo"),
        errors,
        "development-only demo UI",
    )

    # Keep the model pull fail-closed until registry artifact and daemon-store
    # evidence are available. A future enablement must replace this invariant.
    if not re.search(r"fn model_download_preflight_ready\\(\\)\\s*->\\s*bool\\s*\\{\\s*false\\s*\\}", source):
        errors.append("model download: fail-closed preflight must remain false until artifact/store verification is implemented")
    if "evaluate_registered_external_transfer" not in source or '"cloud.ai.generate"' not in source:
        errors.append("cloud dispatch: registered external-transfer policy must remain on the native dispatch path")

    # Folder selection must be registered/cancellable before opening the native picker.
    folder_source = require_tokens(
        FOLDER_IMPORT,
        ("register_active_task", "blocking_pick_folder", "ensure_task_not_cancelled"),
        errors,
        "folder import cancellation",
    )
    if folder_source and folder_source.find("register_active_task") > folder_source.find("blocking_pick_folder"):
        errors.append("folder import: task registration must precede the native folder picker")

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
