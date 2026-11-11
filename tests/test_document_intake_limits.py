import unittest
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from document_intake_limits import DocumentMetrics, intake_denials

class DocumentIntakeLimitTests(unittest.TestCase):
    def test_accepts_bounded_document(self):
        self.assertEqual(intake_denials(DocumentMetrics(1024, 5, 20000, 3, False)), [])

    def test_rejects_oversized_and_complex_documents(self):
        errors = intake_denials(DocumentMetrics(60_000_000, 900, 6_000_000, 2500, False))
        self.assertTrue({"file_size_limit", "page_count_limit", "text_limit",
            "embedded_object_limit"}.issubset(errors))

    def test_encrypted_document_is_not_silently_processed(self):
        self.assertIn("encrypted_document_requires_user_action",
            intake_denials(DocumentMetrics(100, 1, 100, 0, True)))

    def test_rejects_negative_metrics(self):
        errors = intake_denials(DocumentMetrics(-1, -1, -1, -1, False))
        self.assertEqual(len(errors), 4)

if __name__ == "__main__":
    unittest.main()
