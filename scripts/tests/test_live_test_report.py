import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("live_tests", ROOT / "scripts/test-sources.py")
live_tests = importlib.util.module_from_spec(spec)
spec.loader.exec_module(live_tests)


class LiveTestReportTests(unittest.TestCase):
    def test_http_diagnostics_are_evidence_not_a_skip(self):
        challenge = 'URL: https://example.com/; HTTP 403; Content-Type: Some("text/html"); body prefix: Some("<title>Just a moment...</title>")'
        self.assertIn("Cloudflare 403", live_tests.http_evidence(challenge))
        self.assertIn("522", live_tests.http_evidence("URL: https://example.com/; HTTP 522; body prefix: Some(\"error code: 522\")"))
        self.assertEqual(live_tests.http_evidence("assertion failed: 403 != 522"), "See log")
        self.assertEqual(live_tests.http_evidence("URL: https://example.com/; HTTP 403; body prefix: Some(\"Forbidden\")"), "See log")

    def test_failures_keep_logs_fail_the_job_and_allow_remaining_suites(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "bin").mkdir()
            cargo = root / "bin/cargo"
            cargo.write_text('#!/bin/sh\nprintf "%s\\n" "$*"\ncase "$PWD" in\n*/en.blocked) echo \'URL: https://example.com/; HTTP 403; body prefix: Some("Just a moment...")\'; exit 1;;\n*/en.bug) echo "assertion failed"; exit 1;;\nesac\necho "test result: ok"\n')
            cargo.chmod(0o755)
            crates = ["sources/en.blocked", "sources/en.bug", "sources/en.good"]
            for crate in crates:
                path = root / crate
                path.mkdir(parents=True)
                (path / "Cargo.toml").write_text("[package]\n")
            (root / "tested").write_text("\n".join(crates) + "\n")
            result = subprocess.run([sys.executable, str(ROOT / "scripts/test-sources.py"), "tested", "--output", "logs"],
                                    cwd=root, env={**os.environ, "PATH": str(root / "bin") + os.pathsep + os.environ["PATH"], "GITHUB_STEP_SUMMARY": str(root / "summary")},
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            summary = (root / "summary").read_text()
            self.assertIn("Cloudflare 403", summary)
            self.assertIn("`sources/en.bug` | Failed | See log", summary)
            self.assertIn("`sources/en.good` | Passed", summary)
            self.assertIn("test -- --skip review_", (root / "logs/sources_en.good.log").read_text())
            self.assertEqual((root / "logs/summary.md").read_text(), summary)

    def test_workflow_runs_offline_contracts_first_and_uploads_failed_logs(self):
        workflow = (ROOT / ".github/workflows/test.yaml").read_text()
        self.assertLess(workflow.index("cargo test review_"), workflow.index("run: python3 scripts/test-sources.py"))
        self.assertIn("- name: Upload source test logs\n        if: always()", workflow)
        self.assertNotIn("continue-on-error", workflow)


if __name__ == "__main__":
    unittest.main()
