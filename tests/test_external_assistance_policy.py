import unittest
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from external_assistance_policy import ExternalAssistanceRequest, external_assistance_denials, may_use_external_assistance

class ExternalAssistancePolicyTests(unittest.TestCase):
    def request(self, **changes):
        values = dict(user_enabled_external_assistance=True,
            task_approved_for_external_assistance=True, payload_minimized=True,
            contains_local_file_content=False, file_content_explicitly_selected=False,
            destination_configured=True)
        values.update(changes)
        return ExternalAssistanceRequest(**values)

    def test_allows_only_fully_authorized_minimized_request(self):
        self.assertTrue(may_use_external_assistance(self.request()))

    def test_denies_when_any_required_approval_or_control_is_missing(self):
        denials = external_assistance_denials(self.request(
            user_enabled_external_assistance=False, task_approved_for_external_assistance=False,
            payload_minimized=False, destination_configured=False))
        self.assertEqual(len(denials), 4)

    def test_local_file_content_requires_explicit_selection(self):
        self.assertIn("file_content_not_explicitly_selected",
            external_assistance_denials(self.request(contains_local_file_content=True)))


    def test_rejects_truthy_non_boolean_approval_flags(self):
        denials = external_assistance_denials(self.request(
            user_enabled_external_assistance=1,
            task_approved_for_external_assistance="yes",
            payload_minimized=1,
            destination_configured="configured",
            contains_local_file_content=None,
            file_content_explicitly_selected=1))
        self.assertIn("invalid_user_enabled_external_assistance", denials)
        self.assertIn("invalid_task_approved_for_external_assistance", denials)
        self.assertIn("invalid_payload_minimized", denials)
        self.assertIn("invalid_destination_configured", denials)
        self.assertIn("invalid_contains_local_file_content", denials)
        self.assertIn("invalid_file_content_selection", denials)

if __name__ == "__main__":
    unittest.main()
