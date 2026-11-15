#!/usr/bin/env python3
"""Fail CI when the registered Tauri IPC command surface drifts from its audit inventory.

This is an inventory-parity guard only. It does not prove that a command has a
correct authorization policy, receipt, or denial-before-side-effect test.
Those per-command audit obligations remain tracked by Public AI issue #120.
"""

from pathlib import Path
import re
import sys

from check_native_command_security_map import registered_commands, strip_rust_noncode

ROOT = Path(__file__).resolve().parents[1]
SOURCE_PATH = ROOT / "apps/desktop/src-tauri/src/lib.rs"
INVENTORY_PATH = ROOT / "docs/NATIVE-COMMAND-AUDIT-INVENTORY.md"


def command_tokens(text: str) -> list[str]:
    """Extract Rust command paths from a handler argument list."""
    text = strip_rust_noncode(text)
    tokens = re.findall(r"(?<![\w$])(?:[A-Za-z_]\w*::)*[A-Za-z_]\w*", text)
    return [token for token in tokens if token != "peer_command"]


def table_commands(document: str, start_heading: str, end_heading: str) -> list[str]:
    start = document.find(start_heading)
    if start < 0:
        raise ValueError(f"missing inventory heading: {start_heading}")
    end = document.find(end_heading, start + len(start_heading))
    if end < 0:
        raise ValueError(f"missing inventory boundary: {end_heading}")
    section = document[start:end]
    commands: list[str] = []
    for line in section.splitlines():
        if line.lstrip().startswith("|"):
            cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
            if len(cells) < 2:
                continue
            # The command is the final column; category/notes cells may also
            # contain backticked identifiers that are not registered commands.
            commands.extend(
                re.findall(r"`([A-Za-z_]\w*(?:::[A-Za-z_]\w*)*)`", cells[-1])
            )
    return commands


def main() -> int:
    source = SOURCE_PATH.read_text(encoding="utf-8")
    inventory = INVENTORY_PATH.read_text(encoding="utf-8")

    # Reuse the security-map parser so inventory parity and security mapping
    # interpret Rust comments, strings, raw strings, character literals and
    # nested delimiters consistently.
    registered, peer_commands = registered_commands(source)

    documented = table_commands(
        inventory,
        "## Registered command inventory",
        "### Conditional peer-preview surface",
    )
    documented_peer = table_commands(
        inventory,
        "### Conditional peer-preview surface",
        "## High-priority findings to close",
    )

    errors: list[str] = []
    if len(registered) != len(set(registered)):
        errors.append("duplicate command in the default Tauri handler list")
    if len(peer_commands) != len(set(peer_commands)):
        errors.append("duplicate command in the peer-preview expansion")
    if len(documented) != len(set(documented)):
        errors.append("duplicate command in the default inventory table")
    if len(documented_peer) != len(set(documented_peer)):
        errors.append("duplicate command in the peer-preview inventory table")

    missing = sorted(set(registered) - set(documented))
    stale = sorted(set(documented) - set(registered))
    missing_peer = sorted(set(peer_commands) - set(documented_peer))
    stale_peer = sorted(set(documented_peer) - set(peer_commands))

    if missing:
        errors.append("default commands missing from inventory: " + ", ".join(missing))
    if stale:
        errors.append("default inventory entries not registered: " + ", ".join(stale))
    if missing_peer:
        errors.append("peer-preview commands missing from inventory: " + ", ".join(missing_peer))
    if stale_peer:
        errors.append("peer-preview inventory entries not registered: " + ", ".join(stale_peer))

    if errors:
        print("Native IPC inventory check FAILED:")
        for error in errors:
            print(f" - {error}")
        return 1

    print(
        "Native IPC inventory check passed: "
        f"{len(registered)} default commands and "
        f"{len(peer_commands)} peer-preview-only commands match the audit inventory."
    )
    print(
        "Reminder: inventory parity is not a security sign-off; "
        "issue #120 still requires policy, receipt and negative-test coverage."
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as error:
        print(f"Native IPC inventory check FAILED: {error}", file=sys.stderr)
        raise SystemExit(1)
