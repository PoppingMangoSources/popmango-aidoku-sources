import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("archives", ROOT / "scripts/check-archives.py")
archives = importlib.util.module_from_spec(spec)
spec.loader.exec_module(archives)


class ArchiveTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-qb", "main")
        self.git("config", "user.name", "Archive tests")
        self.git("config", "user.email", "archive@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        for name, text in {
            "sources/en.example/Cargo.toml": '[dependencies]\nexample = {path="../../templates/example"}\n',
            "sources/en.example/src/lib.rs": "source\n",
            "sources/en.example/src/test.rs": "regression\n",
            "templates/example/Cargo.toml": "[dependencies]\n",
            "templates/example/src/lib.rs": "template\n",
            "templates/example/src/test.rs": "template regression\n",
            "media/sources/example.png": "icon\n",
        }.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        self.commit()
        self.base = self.git("rev-parse", "HEAD")
        self.git("branch", "archive")

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True, stderr=subprocess.PIPE).strip()

    def commit(self):
        self.git("add", "-A")
        self.git("commit", "-qm", "snapshot")

    def remove(self):
        shutil.rmtree(self.root / "sources/en.example")
        (self.root / "media/sources/example.png").unlink()
        self.commit()

    def test_complete_snapshot_and_dependencies_pass(self):
        self.remove()
        self.assertEqual(archives.inspect(self.root, self.base, "HEAD", ["archive"]), (["sources/en.example"], []))

    def test_missing_source_template_tests_or_icon_fail(self):
        for path in ["sources/en.example/src/test.rs", "templates/example/src/test.rs", "media/sources/example.png"]:
            with self.subTest(path=path):
                self.git("checkout", "archive")
                self.git("reset", "--hard", self.base)
                (self.root / path).unlink()
                self.commit()
                self.git("checkout", "main")
                if (self.root / "sources/en.example").exists():
                    self.remove()
                self.assertTrue(archives.inspect(self.root, self.base, "main", ["archive"])[1])

    def test_missing_archive_fails_only_for_removal(self):
        self.assertEqual(archives.inspect(self.root, self.base, "HEAD", []), ([], []))
        self.remove()
        self.assertTrue(archives.inspect(self.root, self.base, "HEAD", [])[1])

    def test_changed_template_cannot_use_an_older_archive(self):
        self.git("checkout", "archive")
        (self.root / "templates/example/src/lib.rs").write_text("older implementation\n")
        self.commit()
        self.git("checkout", "main")
        self.remove()
        self.assertTrue(archives.inspect(self.root, self.base, "HEAD", ["archive"])[1])

    def test_repository_and_push_gates_check_archives(self):
        self.assertIn('"scripts/check-archives.py"', (ROOT / "scripts/ci-gates.py").read_text())
        self.assertIn('scripts/check-archives.py', (ROOT / ".githooks/pre-push").read_text())


if __name__ == "__main__":
    unittest.main()
