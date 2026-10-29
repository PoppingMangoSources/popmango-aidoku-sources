import runpy
import subprocess
import tempfile
from pathlib import Path
import unittest

helpers = runpy.run_path(str(Path(__file__).resolve().parents[1] / 'submission-files.py'))
without_tests = helpers['without_tests']


class SubmissionFilesTests(unittest.TestCase):
    def files(self):
        return {
            'Cargo.toml': b'[package]\nname = "example"\n\n[dependencies]\nserde = "1"\n\n[dev-dependencies]\naidoku-test = "1"\nserde_json = { version = "1", features = ["alloc"] }\n\n[lib]\ncrate-type = ["cdylib"]\n',
            'Cargo.lock': b'[[package]]\nname = "example"\ndependencies = [\n "serde",\n "serde_json",\n]\n\n[[package]]\nname = "other"\ndependencies = [\n "serde_json",\n]\n',
            'src/lib.rs': b'mod models;\n\n#[cfg(test)]\nmod test;\n',
            'src/test.rs': b'let parsed = serde_json::from_str("{}");\n',
            'res/icon.png': b'\x89PNG\r\n',
        }

    def test_test_dependency_and_root_lock_entry_removed(self):
        files = self.files()
        result = without_tests(files)
        self.assertNotIn('src/test.rs', result)
        self.assertEqual(result['src/lib.rs'], b'mod models;\n')
        self.assertNotIn(b'serde_json', result['Cargo.toml'])
        self.assertEqual(result['Cargo.lock'].count(b'"serde_json"'), 1)
        self.assertIn(b'aidoku-test = "1"', result['Cargo.toml'])
        self.assertEqual(result['res/icon.png'], files['res/icon.png'])
        self.assertIn('src/test.rs', files)
        self.assertEqual(without_tests(result), result)

    def test_remaining_json_usage_keeps_dependency(self):
        files = self.files()
        files['src/models.rs'] = b'#[cfg(test)]\nmod tests { use serde_json; }\n'
        result = without_tests(files)
        self.assertEqual(result['Cargo.toml'], files['Cargo.toml'])
        self.assertEqual(result['Cargo.lock'], files['Cargo.lock'])

    def test_production_dependency_keeps_root_lock_entry(self):
        files = self.files()
        files['Cargo.toml'] = files['Cargo.toml'].replace(b'serde = "1"', b'serde = "1"\nserde_json = "1"')
        result = without_tests(files)
        self.assertEqual(result['Cargo.toml'].count(b'serde_json'), 1)
        self.assertEqual(result['Cargo.lock'], files['Cargo.lock'])

    def test_source_without_json_dev_dependency_is_unchanged(self):
        files = {'Cargo.toml': b'[package]\nname = "example"\n', 'src/lib.rs': b'source\n'}
        self.assertEqual(without_tests(files), files)

    def test_snapshot_preserves_binary_content_and_modes(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            source = repo / 'sources/en.example'
            source.mkdir(parents=True)
            script = source / 'script.sh'
            script.write_bytes(b'#!/bin/sh\n')
            script.chmod(0o755)
            (source / 'link.rs').symlink_to('script.sh')
            (source / 'icon.png').write_bytes(b'\x89PNG\r\n')
            def git(*args):
                return subprocess.check_output(['git', '-C', directory, *args], stderr=subprocess.PIPE)
            git('init', '-q')
            git('add', '.')
            git('-c', 'user.name=Tests', '-c', 'user.email=tests@example.invalid', 'commit', '-qm', 'snapshot')
            self.assertEqual(helpers['snapshot_files'](directory, 'HEAD', 'sources/en.example'),
                             {'script.sh': b'#!/bin/sh\n', 'icon.png': b'\x89PNG\r\n', 'link.rs': b'script.sh'})
            self.assertEqual(helpers['snapshot_modes'](directory, 'HEAD', 'sources/en.example'),
                             {'script.sh': '100755', 'icon.png': '100644', 'link.rs': '120000'})
