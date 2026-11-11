import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from document_intake_limits import DocumentMetrics, intake_denials, may_process_document


class DocumentIntakeLimitTests(unittest.TestCase):
    def test_accepts_bounded_document(self):
        self.assertTrue(may_process_document(DocumentMetrics(1024, 5, 20000, 3, False)))

    def test_rejects_oversized_and_complex_documents(self):
        errors = intake_denials(DocumentMetrics(60_000_000, 900, 6_000_000, 2500, False))
        self.assertTrue({"file_size_limit", "page_count_limit", "text_limit",
                         "embedded_object_limit"}.issubset(errors))

    def test_encrypted_document_is_not_silently_processed(self):
        self.assertIn("encrypted_document_requires_user_action",
                      intake_denials(DocumentMetrics(100, 1, 100, 0, True)))

    def test_rejects_negative_metrics(self):
        errors = intake_denials(DocumentMetrics(-1, -1, -1, -1, False))
        self.assertIn("invalid_metric_value", errors)

    def test_rejects_boolean_or_non_integer_metrics(self):
        errors = intake_denials(DocumentMetrics(True, "1", 1.5, 0, False))
        self.assertIn("invalid_metric_value", errors)

    def test_rejects_invalid_limits_and_unknown_encryption_state(self):
        errors = intake_denials(
            DocumentMetrics(10, 1, 100, 0, None), max_bytes=True, max_pages=-1
        )
        self.assertIn("invalid_limit_configuration", errors)
        self.assertIn("invalid_encryption_state", errors)

    def test_rejects_unknown_metrics_object(self):
        self.assertEqual(intake_denials(None), ["invalid_metrics"])


if __name__ == "__main__":
    unittest.main()
