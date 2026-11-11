import unittest
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from model_selection_policy import ModelCandidate, candidate_denials, select_model

class ModelSelectionPolicyTests(unittest.TestCase):
    def candidate(self, model_id, **changes):
        values = dict(local=True, available=True, context_capacity=8192,
            required_context=4096, memory_required_mb=2048, memory_available_mb=4096,
            external_assistance_allowed=False)
        values.update(changes)
        return ModelCandidate(model_id=model_id, **values)

    def test_prefers_eligible_local_candidate(self):
        local = self.candidate("local")
        cloud = self.candidate("cloud", local=False, external_assistance_allowed=True)
        self.assertEqual(select_model([cloud, local]), local)

    def test_external_candidate_requires_explicit_permission(self):
        cloud = self.candidate("cloud", local=False)
        self.assertIn("external_assistance_not_allowed", candidate_denials(cloud))
        self.assertIsNone(select_model([cloud]))

    def test_rejects_unavailable_or_underprovisioned_models(self):
        bad = self.candidate("bad", available=False, context_capacity=1024, memory_available_mb=512)
        self.assertIsNone(select_model([bad]))

if __name__ == "__main__":
    unittest.main()
