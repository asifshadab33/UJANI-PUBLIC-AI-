import unittest
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from model_fallback_policy import InferenceFailure, next_execution_route

class ModelFallbackPolicyTests(unittest.TestCase):
    def failure(self, **changes):
        values = dict(reason="out_of_memory", retryable=False, local_attempts=1,
            max_local_attempts=2, external_assistance_allowed=False,
            task_approved_for_external=False)
        values.update(changes)
        return InferenceFailure(**values)

    def test_retries_local_only_within_configured_bound(self):
        self.assertEqual(next_execution_route(self.failure(retryable=True)), "retry_local")
        self.assertEqual(next_execution_route(self.failure(retryable=True, local_attempts=2)), "stop_and_explain")

    def test_never_silently_escalates_to_external_service(self):
        self.assertEqual(next_execution_route(self.failure(
            external_assistance_allowed=True, task_approved_for_external=False)), "stop_and_explain")

    def test_external_route_requires_both_permissions(self):
        self.assertEqual(next_execution_route(self.failure(
            external_assistance_allowed=True, task_approved_for_external=True)),
            "request_external_approval")

    def test_invalid_attempt_state_stops(self):
        self.assertEqual(next_execution_route(self.failure(local_attempts=-1)), "stop_invalid_state")


    def test_rejects_truthy_non_boolean_flags_and_unbounded_attempts(self):
        self.assertEqual(next_execution_route(self.failure(retryable=1)), "stop_invalid_state")
        self.assertEqual(next_execution_route(self.failure(external_assistance_allowed=1)), "stop_invalid_state")
        self.assertEqual(next_execution_route(self.failure(task_approved_for_external="yes")), "stop_invalid_state")
        self.assertEqual(next_execution_route(self.failure(local_attempts=True)), "stop_invalid_state")
        self.assertEqual(next_execution_route(self.failure(max_local_attempts=100)), "stop_invalid_state")
        self.assertEqual(next_execution_route(self.failure(local_attempts=3, max_local_attempts=2)), "stop_invalid_state")

if __name__ == "__main__":
    unittest.main()
