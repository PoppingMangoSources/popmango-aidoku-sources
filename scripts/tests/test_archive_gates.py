import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ArchiveGateTests(unittest.TestCase):
    def test_archive_omits_only_publication_rules_and_main_retains_them(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            git = lambda *args: subprocess.check_output(["git", *args], cwd=root, text=True, stderr=subprocess.PIPE).strip()
            git("init", "-qb", "main")
            git("config", "user.name", "Gate tests")
            git("config", "user.email", "gate@example.invalid")
            git("config", "commit.gpgsign", "false")
            (root / "scripts/tests").mkdir(parents=True)
            (root / "scripts/tests/test_snapshot.py").write_text('import unittest\nfrom pathlib import Path\nclass Snapshot(unittest.TestCase):\n def test_source_exists(self):\n  self.assertTrue(Path("sources/en.example/Cargo.toml").is_file())\n')
            for name in ["ci-gates.py", "changed-crates.py"]:
                shutil.copy2(ROOT / "scripts" / name, root / "scripts" / name)
            for name in ["check-commits.py", "check-archives.py", "sync-agent-docs.py", "check-workflows.py"]:
                (root / "scripts" / name).write_text("pass\n")
            (root / "scripts/check-sources.py").write_text('import json,sys\nfrom pathlib import Path\nPath("source-args.json").write_text(json.dumps(sys.argv[1:]))\n')
            crate = root / "sources/en.example"
            crate.mkdir(parents=True)
            (crate / "Cargo.toml").write_text("[package]\n")
            (crate / "source.rs").write_text("first\n")
            git("add", ".")
            git("commit", "-qm", "baseline")
            base = git("rev-parse", "HEAD")
            (crate / "source.rs").write_text("changed\n")
            git("add", ".")
            git("commit", "-qm", "change")
            for branch in ["main", "merged-sources", "dead-sources", "feat/en.example"]:
                with self.subTest(branch=branch):
                    result = subprocess.run([sys.executable, "scripts/ci-gates.py", "--base", base], cwd=root,
                                            env={**os.environ, "GITHUB_REF": "refs/heads/" + branch},
                                            capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    arguments = json.loads((root / "source-args.json").read_text())
                    self.assertEqual(arguments, ["--upstream", "en.example"] if branch.endswith("-sources") else ["--base", base])


if __name__ == "__main__":
    unittest.main()
