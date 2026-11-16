from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
APP = (ROOT / "apps/desktop/src/App.tsx").read_text(encoding="utf-8")
ROADMAP = (ROOT / "ROADMAP.md").read_text(encoding="utf-8")
MODES = (ROOT / "docs/EXECUTION-MODES.md").read_text(encoding="utf-8")
README = (ROOT / "README.md").read_text(encoding="utf-8")
IMPLEMENTATION_STATUS = (ROOT / "docs/IMPLEMENTATION_STATUS.md").read_text(encoding="utf-8")


class ExecutionModeContractTests(unittest.TestCase):
    def test_worker_means_approved_cloud_plan_with_local_compute(self):
        self.assertIn(
            "Use approved cloud plans while this device performs the authorised compute locally.",
            APP,
        )
        self.assertIn(
            "Worker mode</b> is intended to receive bounded, signed cloud plans or delegations",
            APP,
        )
        self.assertIn(
            "Worker — approved cloud plan + local compute (preview)",
            APP,
        )
        self.assertIn(
            "Worker tasks must be structured and allow-listed",
            MODES,
        )
        self.assertIn(
            "schema validity is not identity authentication",
            MODES,
        )

    def test_partner_is_bounded_local_cloud_collaboration(self):
        self.assertIn(
            "Iterate between local and approved external AI within explicit data and round limits.",
            APP,
        )
        self.assertIn(
            "explicit approval; rounds and data volume must be capped",
            APP,
        )
        self.assertIn(
            "Full files and conversation history must never be shared automatically.",
            APP,
        )
        self.assertIn(
            "Partner transfers are bounded per round",
            MODES,
        )

    def test_worker_and_partner_remain_unavailable(self):
        disabled_cards = (
            'className="launcher-card launcher-unavailable" disabled '
            'aria-describedby="advanced-mode-status"'
        )
        self.assertEqual(APP.count(disabled_cards), 2)
        self.assertIn("Worker and Partner remain preview-only", APP)
        self.assertIn(
            "Production cloud planning/delegation and execution are not implemented",
            MODES,
        )
        self.assertIn(
            "Production Partner iteration is not implemented",
            MODES,
        )

    def test_smart_cloud_gateway_remains_the_only_external_ai_path(self):
        self.assertIn('if (executionMode !== "smart")', APP)
        self.assertIn(
            "The BYOK cloud gateway is available in Smart mode only.",
            APP,
        )
        self.assertIn(
            "fresh per-request native approval",
            MODES,
        )
        self.assertIn(
            "A denial or timeout before dispatch must not send the request; once the POST has been dispatched",
            MODES,
        )

    def test_fortress_is_not_misrepresented_as_os_isolated(self):
        self.assertIn(
            "Fortress policy preview does not disable OS networking.",
            APP,
        )
        self.assertIn(
            "Fortress — local-only policy (OS isolation pending)",
            APP,
        )
        self.assertIn(
            "OS-wide network isolation is not yet enforced.",
            APP,
        )
        self.assertIn(
            "This is not yet OS-wide network isolation",
            MODES,
        )

    def test_mode_selection_never_grants_permission(self):
        self.assertIn(
            "Selecting a mode never grants consent by itself.",
            APP,
        )
        self.assertIn(
            "Mode and permission are separate.",
            MODES,
        )
        self.assertIn(
            "No free device control, shell access or automatic full-file/conversation-history sharing.",
            ROADMAP,
        )

    def test_public_ai_remains_independent_of_enterprise(self):
        self.assertIn(
            "Public AI remains independently usable",
            MODES,
        )
        self.assertIn(
            "Any experimental peer identity/nonce primitives are separate transport foundations",
            MODES,
        )

    def test_public_docs_do_not_redefine_worker_partner_as_peer_execution(self):
        self.assertNotIn("Secure distributed Worker/Partner peer execution", README)
        self.assertIn("approved cloud-plan/local-compute gateway", README)
        self.assertIn("bounded local/cloud iteration gateway", README)
        self.assertIn("Experimental peer protocol primitives", IMPLEMENTATION_STATUS)
        self.assertIn(
            "they do not define or enable Public AI's Worker or Partner modes",
            IMPLEMENTATION_STATUS,
        )


if __name__ == "__main__":
    unittest.main()
