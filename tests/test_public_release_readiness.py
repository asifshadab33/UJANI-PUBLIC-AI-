import unittest
from scripts.public_release_readiness import REQUIRED_EVIDENCE, missing_release_evidence, release_ready

class PublicReleaseReadinessTests(unittest.TestCase):
    def test_incomplete_evidence_fails_closed(self):
        self.assertFalse(release_ready(set()))
        self.assertEqual(set(missing_release_evidence(set())), REQUIRED_EVIDENCE)

    def test_complete_evidence_is_required_for_release(self):
        self.assertTrue(release_ready(set(REQUIRED_EVIDENCE)))

    def test_one_missing_external_report_blocks_release(self):
        evidence = set(REQUIRED_EVIDENCE) - {"independent_external_test_report"}
        self.assertFalse(release_ready(evidence))
        self.assertEqual(missing_release_evidence(evidence), ["independent_external_test_report"])

if __name__ == "__main__":
    unittest.main()
