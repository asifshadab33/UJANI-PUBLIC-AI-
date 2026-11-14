#!/usr/bin/env python3
"""Require an explicit security-audit disposition for every registered native IPC command.

This checks inventory completeness and required metadata only. It does not prove
that a policy gate is enforced, a receipt is durable, or a denial precedes side effects.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "apps/desktop/src-tauri/src/lib.rs"
MANIFEST = ROOT / "docs/native-command-security-map.json"


def command_tokens(text: str) -> list[str]:
    text = re.sub(r"//[^\n]*", "", text)
    return re.findall(r"(?<![\w$])(?:[A-Za-z_]\w*::)*[A-Za-z_]\w*", text)


def registered_commands(source: str) -> tuple[list[str], list[str]]:
    marker = "tauri::generate_handler!["
    start = source.find(marker)
    if start < 0:
        raise ValueError("could not locate tauri::generate_handler! in lib.rs")
    body_start = start + len(marker)
    body_end = source.find("]", body_start)
    if body_end < 0:
        raise ValueError("could not locate end of default Tauri handler list")
    default = [item for item in command_tokens(source[body_start:body_end]) if item != "peer_command"]

    peer = re.search(
        r'#\[cfg\(feature = "peer-preview"\)\]\s*'
        r'let builder = builder\.invoke_handler\(ujani_invoke_handler!\((.*?)\)\);',
        source,
        re.DOTALL,
    )
    if not peer:
        raise ValueError("could not locate peer-preview handler expansion")
    return default, command_tokens(peer.group(1))


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
    if errors:
        print("Native command security-map check FAILED:")
        for error in errors:
            print(" - " + error)
        return 1
    print(
        "Native command security-map check passed: "
        f"{len(default)} default commands and {len(peer)} peer-preview commands have explicit audit mappings."
    )
    print("This is mapping completeness only; it is not a security sign-off or proof of runtime enforcement.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
