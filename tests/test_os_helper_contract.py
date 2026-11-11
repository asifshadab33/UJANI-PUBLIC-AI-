import unittest
from scripts.os_helper_contract import OSHelperCapability, validate_capability

class OSHelperContractTests(unittest.TestCase):
    def capability(self, **changes):
        values = dict(name="Search selected folder", operation="search_files",
            scope="/selected/docs", read_only=True, network_access=False,
            requires_approval=True, max_results=250, timeout_ms=5000)
        values.update(changes)
        return OSHelperCapability(**values)

    def test_accepts_bounded_read_only_local_helper(self):
        self.assertEqual(validate_capability(self.capability()), [])

    def test_rejects_write_network_and_missing_approval(self):
        errors = validate_capability(self.capability(
            read_only=False, network_access=True, requires_approval=False))
        self.assertTrue(any("read-only" in e for e in errors))
        self.assertTrue(any("network" in e for e in errors))
        self.assertTrue(any("approval" in e for e in errors))

    def test_rejects_unknown_operation_and_unbounded_limits(self):
        errors = validate_capability(self.capability(
            operation="run_shell", max_results=100000, timeout_ms=0))
        self.assertGreaterEqual(len(errors), 3)

if __name__ == "__main__":
    unittest.main()
