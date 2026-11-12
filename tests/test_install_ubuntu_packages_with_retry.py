from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
INSTALLER = ROOT / "scripts" / "install_ubuntu_packages_with_retry.sh"


class UbuntuPackageRetryTests(unittest.TestCase):
    def run_installer(self, *, fail_updates: int = 0, fail_installs: int = 0, packages: tuple[str, ...] = ("curl",)):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            log_path = root / "sudo.log"

            sudo = bin_dir / "sudo"
            sudo.write_text(
                """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "$FAKE_SUDO_LOG"
if [[ "$1" != "apt-get" ]]; then exit 97; fi
case "$2" in
  update)
    count="$(grep -c '^apt-get update' "$FAKE_SUDO_LOG" || true)"
    if (( count <= FAKE_FAIL_UPDATES )); then exit 42; fi
    ;;
  install)
    count="$(grep -c '^apt-get install' "$FAKE_SUDO_LOG" || true)"
    if (( count <= FAKE_FAIL_INSTALLS )); then exit 43; fi
    ;;
  *) exit 98 ;;
esac
exit 0
""",
                encoding="utf-8",
            )
            sudo.chmod(0o755)
            fake_sleep = bin_dir / "sleep"
            fake_sleep.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
            fake_sleep.chmod(0o755)

            env = os.environ.copy()
            env.update(
                PATH=f"{bin_dir}{os.pathsep}{env.get('PATH', '')}",
                FAKE_SUDO_LOG=str(log_path),
                FAKE_FAIL_UPDATES=str(fail_updates),
                FAKE_FAIL_INSTALLS=str(fail_installs),
            )
            result = subprocess.run(
                ["bash", str(INSTALLER), *packages],
                cwd=ROOT,
                env=env,
                text=True,
                capture_output=True,
                check=False,
            )
            log = log_path.read_text(encoding="utf-8") if log_path.exists() else ""
            return result, log

    def test_retries_transient_update_failure_and_installs_requested_packages(self):
        result, log = self.run_installer(fail_updates=2, packages=("curl", "libssl-dev"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(log.count("apt-get update"), 3)
        self.assertEqual(log.count("apt-get install"), 1)
        self.assertIn("curl libssl-dev", log)
        self.assertIn("-o Acquire::Retries=2", log)
        self.assertIn("-o Acquire::http::Timeout=30", log)
        self.assertIn("-o Acquire::https::Timeout=30", log)
        self.assertIn("--no-install-recommends", log)

    def test_retries_transient_install_failure(self):
        result, log = self.run_installer(fail_installs=1)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(log.count("apt-get update"), 2)
        self.assertEqual(log.count("apt-get install"), 2)

    def test_stops_after_four_failed_update_attempts(self):
        result, log = self.run_installer(fail_updates=99)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(log.count("apt-get update"), 4)
        self.assertEqual(log.count("apt-get install"), 0)
        self.assertIn("failed after 4 attempts", result.stderr)

    def test_stops_after_four_failed_install_attempts(self):
        result, log = self.run_installer(fail_installs=99)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(log.count("apt-get update"), 4)
        self.assertEqual(log.count("apt-get install"), 4)
        self.assertIn("failed after 4 attempts", result.stderr)

    def test_requires_at_least_one_package(self):
        result, _ = self.run_installer(packages=())
        self.assertEqual(result.returncode, 2)


if __name__ == "__main__":
    unittest.main()
