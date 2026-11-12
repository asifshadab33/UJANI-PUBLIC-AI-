from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "apps/desktop/src/App.tsx"
STATUS = ROOT / "docs/IMPLEMENTATION_STATUS.md"


class MediaCapabilityBoundaryTests(unittest.TestCase):
    def test_ui_states_current_local_capabilities_and_limitations(self):
        source = APP.read_text(encoding="utf-8")
        for phrase in (
            "What this computer can do",
            "Documents:",
            "Images:",
            "Audio and video:",
            "does not currently perform OCR",
            "does not currently transcribe audio",
            "does not upload these files automatically",
            "text-only provider gateway cannot transmit media attachments",
        ):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, source)

    def test_external_upload_is_user_directed_and_disclosed(self):
        source = APP.read_text(encoding="utf-8")
        self.assertIn("upload the file there yourself", source)
        self.assertIn("privacy, retention and billing terms", source)
        self.assertIn("confidential or personal files", source)
        for url in (
            "https://chatgpt.com/",
            "https://gemini.google.com/",
            "https://claude.ai/",
        ):
            with self.subTest(url=url):
                match = re.search(
                    r'<a href="' + re.escape(url) +
                    r'" target="_blank" rel="noopener noreferrer">',
                    source,
                )
                self.assertIsNotNone(match)

    def test_implementation_status_matches_the_lightweight_boundary(self):
        source = STATUS.read_text(encoding="utf-8")
        self.assertIn("## Lightweight media capability boundary", source)
        self.assertIn("OCR, image-content understanding", source)
        self.assertIn("no local recording/transcription", source)
        self.assertIn("no local scene-understanding", source)
        self.assertIn("does not upload media automatically", source)


if __name__ == "__main__":
    unittest.main()
