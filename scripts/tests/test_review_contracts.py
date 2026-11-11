import http.server
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest
from unittest.mock import patch
from urllib.error import HTTPError
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("contracts", ROOT / "scripts/review-contracts.py")
contracts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contracts)


class ReviewContractTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)

    def crate(self, name, manifest):
        path = self.repo / name
        path.mkdir(parents=True)
        (path / "Cargo.toml").write_text(manifest)
        return path

    def test_consumers_and_templates_select_one_suite_per_template(self):
        self.crate("templates/keyoapp", "")
        self.crate("sources/en.one", '[dependencies]\ntheme = {path="../../templates/keyoapp"}\n')
        self.crate("sources/en.two", '[dependencies]\nmangathemesia = {path="../../templates/mangathemesia"}\n')
        self.assertEqual(contracts.select_templates(self.repo, ["templates/keyoapp", "sources/en.one", "sources/en.two", "sources/en.deleted"]),
                         ["keyoapp", "mangathemesia"])

    def test_unrelated_crates_do_not_run_template_suites(self):
        self.crate("sources/en.other", '[dependencies]\nother = {path="../../templates/other"}\n')
        self.assertEqual(contracts.select_templates(self.repo, ["sources/en.other"]), [])

    def test_compilation_failure_propagates_without_changing_candidate(self):
        candidate = self.crate("templates/keyoapp", '[dependencies]\naidoku = {git="https://example.invalid/sdk", rev="revision"}\n')
        (candidate / "Cargo.lock").write_text("lock snapshot\n")
        before = {path.relative_to(candidate): path.read_bytes() for path in candidate.rglob("*") if path.is_file()}
        def cargo(command, env):
            harness = Path(command[command.index("--manifest-path") + 1]).parent
            self.assertEqual((harness / "Cargo.lock").read_text(), "lock snapshot\n")
            self.assertIn(str(candidate), (harness / "Cargo.toml").read_text())
            self.assertIn('rev = "revision"', (harness / "Cargo.toml").read_text())
            self.assertNotIn("GIT_DIR", env)
            self.assertIn("--lib", command)
            with urlopen(env["AIDOKU_REVIEW_BASE_URL"] + "/keyoapp/short", timeout=2) as response:
                self.assertIn("Full synopsis", response.read().decode())
            return subprocess.CompletedProcess(command, 9)
        with patch.dict(os.environ, {"GIT_DIR": "wrong-repository"}), \
                patch.object(contracts.subprocess, "check_output", return_value="GIT_DIR\n"), \
                patch.object(contracts.subprocess, "run", side_effect=cargo):
            self.assertEqual(contracts.run_contract(self.repo, "keyoapp"), 9)
        self.assertEqual(before, {path.relative_to(candidate): path.read_bytes() for path in candidate.rglob("*") if path.is_file()})

    def test_sdk_locator_is_required(self):
        self.crate("templates/keyoapp", '[dependencies]\naidoku = "0.3"\n')
        with self.assertRaises(ValueError):
            contracts.run_contract(self.repo, "keyoapp")

    def test_unknown_fixture_fails_and_monthly_fallback_checks_its_parameter(self):
        handler = contracts.runpy.run_path(str(contracts.CONTRACTS / "fixtures.py"))["Responses"]
        with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                base = f"http://127.0.0.1:{server.server_port}"
                with self.assertRaises(HTTPError):
                    urlopen(base + "/unexpected", timeout=2)
                for query, expected in [("range=monthly", "Monthly"), ("range=daily", "Wrong period")]:
                    with urlopen(base + "/keyoapp-home/?" + query, timeout=2) as response:
                        self.assertIn(expected, response.read().decode())
            finally:
                server.shutdown()
                thread.join()

    def test_cli_failure_does_not_skip_remaining_template(self):
        with patch("sys.argv", ["review-contracts.py", "--repo", str(self.repo), "--template", "keyoapp", "--template", "mangathemesia"]), \
                patch.object(contracts, "run_contract", side_effect=[1, 0]) as run:
            self.assertTrue(contracts.main())
        self.assertEqual(run.call_count, 2)

    def test_push_review_and_ci_use_portable_contracts(self):
        hook = (ROOT / ".githooks/pre-push").read_text()
        self.assertIn('scripts/review-contracts.py" --repo "$root" --crate "sources/$source"', hook)
        self.assertIn('scripts/review-contracts.py" --repo "$root" --base "$base"', hook)
        self.assertIn('scripts/review-contracts.py" --repo "$root" --crate "sources/$id"', (ROOT / "scripts/review-source.sh").read_text())
        workflow = (ROOT / ".github/workflows/test.yaml").read_text()
        self.assertIn('scripts/review-contracts.py --crates-file "$RUNNER_TEMP/candidates"', workflow)
        self.assertIn('"scripts/review-contracts/**"', workflow)


if __name__ == "__main__":
    unittest.main()
