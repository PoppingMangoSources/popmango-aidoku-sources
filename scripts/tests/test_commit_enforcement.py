import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
ROOT = SCRIPTS.parent
spec = importlib.util.spec_from_file_location("commit_checks", SCRIPTS / "check-commits.py")
checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks)


class CommitEnforcementTests(unittest.TestCase):
    def test_identity_signature_and_every_attribution_channel(self):
        raw = f"author {checks.NAME} <{checks.EMAIL}> 1 +0000\ncommitter {checks.NAME} <{checks.EMAIL}> 1 +0000\n\nchore: verify metadata\n"
        self.assertEqual(checks.inspect_raw(raw, "test"), [])
        for changed in [raw.replace("author Popmango", "author Other"), raw.replace("committer Popmango", "committer Other"),
                        raw.replace("\n\n", "\ngpgsig signature\n\n")]:
            self.assertTrue(checks.inspect_raw(changed, "test"))
        for trailer in ["Co-Authored-By: Example <a@example.invalid>", "Assisted-by: Example", "Claude-Session: private", "Generated with Codex", "https://example.com/session", "Closes #123"]:
            self.assertTrue(checks.inspect_raw(raw + trailer, "test"), trailer)

    def test_settings_and_publication_gate_cannot_drift(self):
        settings = json.loads((ROOT / ".claude/settings.json").read_text())
        self.assertEqual(settings["attribution"], {"commit": "", "pr": "", "sessionUrl": False})
        for event in ("SessionStart", "PreToolUse", "Stop"):
            self.assertTrue(settings["hooks"][event])
        for filename in ("check.yaml", "build.yaml"):
            workflow = (ROOT / ".github/workflows" / filename).read_text()
            self.assertIn("run: python3 scripts/ci-gates.py --event", workflow)
            self.assertIn("fetch-depth: 0", workflow)
        hook = (ROOT / ".githooks/pre-push").read_text()
        self.assertLess(hook.index("unset $(git rev-parse --local-env-vars)"), hook.index("unittest discover"))
        build = (ROOT / ".github/workflows/build.yaml").read_text()
        self.assertLess(build.index("scripts/ci-gates.py --event"), build.index("- name: Build sources"))
        self.assertIn("bash scripts/lint-crates.sh", build)

    def test_already_pushed_bad_merge_parent_is_still_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            git = lambda *args: subprocess.check_output(["git", *args], cwd=folder, text=True, stderr=subprocess.PIPE).strip()
            git("init", "-q", "-b", "main")
            git("config", "user.name", checks.NAME)
            git("config", "user.email", checks.EMAIL)
            git("config", "commit.gpgsign", "false")
            git("commit", "--allow-empty", "-qm", "chore: baseline")
            before = git("rev-parse", "HEAD")
            git("checkout", "-qb", "side")
            git("commit", "--allow-empty", "-qm", "chore: side", "-m", "Co-Authored-By: Example <a@example.invalid>")
            git("checkout", "main")
            git("commit", "--allow-empty", "-qm", "chore: main")
            git("merge", "--no-ff", "side", "-m", "chore: merge")
            after = git("rev-parse", "HEAD")
            git("update-ref", "refs/remotes/origin/main", after)
            event = Path(folder) / "event.json"
            event.write_text(json.dumps({"before": before, "after": after, "ref": "refs/heads/main"}))
            result = subprocess.run(["python3", str(SCRIPTS / "check-commits.py"), "--event"], cwd=folder,
                                    env={**os.environ, "GITHUB_EVENT_PATH": str(event)}, capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn("attribution", result.stderr)
            scripts = Path(folder) / "scripts"
            scripts.mkdir()
            for filename in ("ci-gates.py", "check-commits.py"):
                shutil.copy2(SCRIPTS / filename, scripts / filename)
            manual = subprocess.run(["python3", str(scripts / "ci-gates.py"), "--base", before], cwd=folder,
                                    capture_output=True, text=True)
            self.assertEqual(manual.returncode, 1)
            self.assertIn("attribution", manual.stderr)

    def test_rebuild_onto_newer_upstream_checks_only_our_commits(self):
        with tempfile.TemporaryDirectory() as folder:
            git = lambda *args, **env: subprocess.check_output(
                ["git", *args], cwd=folder, text=True, stderr=subprocess.PIPE, env={**os.environ, **env}).strip()
            other = {"GIT_AUTHOR_NAME": "Other", "GIT_AUTHOR_EMAIL": "o@example.invalid",
                     "GIT_COMMITTER_NAME": "Other", "GIT_COMMITTER_EMAIL": "o@example.invalid"}
            git("init", "-q", "-b", "main")
            git("config", "commit.gpgsign", "false")
            git("commit", "--allow-empty", "-qm", "chore: upstream base", **other)
            git("config", "user.name", checks.NAME)
            git("config", "user.email", checks.EMAIL)
            git("commit", "--allow-empty", "-qm", "feat: add en.example")
            before = git("rev-parse", "HEAD")
            git("reset", "-q", "--hard", "HEAD~1")
            git("commit", "--allow-empty", "-qm", "chore: newer upstream", **other)
            git("update-ref", "refs/remotes/upstream/main", "HEAD")
            git("commit", "--allow-empty", "-qm", "feat: add en.example")
            after = git("rev-parse", "HEAD")
            with contextlib.chdir(folder):
                self.assertEqual(checks.inspect_push(before, after, "feat/en.example"), [])
                git("commit", "--allow-empty", "-qm", "fix: x", "-m", "Co-Authored-By: Example <a@example.invalid>")
                errors = checks.inspect_push(before, git("rev-parse", "HEAD"), "feat/en.example")
            self.assertTrue(any("attribution" in error for error in errors))

    def test_commit_hook_uses_owner_identity_and_rejects_attribution(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "scripts").mkdir()
            (root / ".githooks").mkdir()
            for file in ("scripts/install-hooks.sh", "scripts/check-commits.py", ".githooks/commit-msg", ".githooks/pre-push"):
                shutil.copy2(ROOT / file, root / file)
            subprocess.check_call(["git", "init", "-q"], cwd=root)
            subprocess.check_call(["bash", "scripts/install-hooks.sh"], cwd=root, stdout=subprocess.DEVNULL)
            message = ["git", "commit", "--allow-empty", "-m", "chore: verify", "-m"]
            rejected = subprocess.run([*message, "Assisted-by: Example"], cwd=root, capture_output=True)
            self.assertNotEqual(rejected.returncode, 0)
            accepted = subprocess.run([*message, "Keep the owner identity."], cwd=root, capture_output=True)
            self.assertEqual(accepted.returncode, 0, accepted.stderr)

    def test_stop_parse_failure_blocks_completion(self):
        spec = importlib.util.spec_from_file_location("hooks", SCRIPTS / "agent-hooks.py")
        hooks = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(hooks)
        output = io.StringIO()
        with mock.patch.object(hooks.sys, "argv", ["agent-hooks.py", "stop"]), mock.patch.object(hooks.sys, "stdin", io.StringIO("{invalid")), mock.patch.object(hooks.sys, "stdout", output), mock.patch.object(hooks.sys, "stderr", io.StringIO()):
            self.assertEqual(hooks.main(), 0)
        self.assertEqual(json.loads(output.getvalue())["decision"], "block")
