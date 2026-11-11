import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from check_native_command_inventory import command_tokens, table_commands


class CommandTokenTests(unittest.TestCase):
    def test_extracts_qualified_command_paths_without_comments(self):
        source = """
            command_one,
            module::command_two,
            // ignored_command,
        """
        self.assertEqual(
            command_tokens(source),
            ["command_one", "module::command_two"],
        )

    def test_does_not_treat_comment_text_as_registered_commands(self):
        self.assertEqual(command_tokens("command_one, // command_two\nmodule::command_three"), [
            "command_one", "module::command_three"
        ])


class InventoryTableTests(unittest.TestCase):
    def test_reads_only_backticked_command_names_in_table_rows(self):
        document = """## Registered command inventory
| Area | Registered command |
| --- | --- |
| Filesystem | `open_file` |
| Search | `search_files` |
### Conditional peer-preview surface
"""
        self.assertEqual(
            table_commands(document, "## Registered command inventory", "### Conditional peer-preview surface"),
            ["open_file", "search_files"],
        )

    def test_missing_section_boundary_fails_closed(self):
        with self.assertRaises(ValueError):
            table_commands("## Registered command inventory", "## Registered command inventory", "### Missing")


if __name__ == "__main__":
    unittest.main()
