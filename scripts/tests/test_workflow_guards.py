import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('scope', SCRIPTS / 'check-upstream-scope.py')
scope = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scope)


class ScopeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.git('init', '-q', '-b', 'main')
        self.git('config', 'user.name', 'Guard tests')
        self.git('config', 'user.email', 'guard@example.invalid')
        (self.repo / 'README.md').write_text('upstream\n')
        self.commit('upstream')
        self.base = self.git('rev-parse', 'HEAD')
        self.source = self.repo / 'sources/en.example'
        (self.source / 'res').mkdir(parents=True)
        self.metadata(1)
        (self.source / 'src').mkdir()
        (self.source / 'src/lib.rs').write_text('source\n')
        self.commit('source')

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.repo), *args], stderr=subprocess.PIPE, text=True).strip()

    def commit(self, message):
        self.git('add', '-A')
        self.git('commit', '-qm', message)

    def metadata(self, version):
        (self.source / 'res/source.json').write_text(json.dumps({'info': {'id': 'en.example', 'version': version}}) + '\n')

    def inspect(self):
        return scope.inspect(self.repo, self.base, 'HEAD', 'en.example')['errors']

    def test_clean_submission(self):
        self.assertEqual(self.inspect(), [])

    def test_fork_base_is_rejected(self):
        (self.repo / 'README.md').write_text('fork branding\n')
        self.commit('fork')
        self.assertTrue(any('README.md' in e for e in self.inspect()))

    def test_reverted_foreign_change_still_fails(self):
        original = (self.repo / 'README.md').read_text()
        (self.repo / 'README.md').write_text('foreign\n')
        self.commit('foreign')
        (self.repo / 'README.md').write_text(original)
        self.commit('revert')
        self.assertTrue(any('README.md' in e for e in self.inspect()))

    def test_rename_cannot_hide_foreign_deletion(self):
        (self.repo / 'README.md').rename(self.source / 'src/copied.rs')
        self.commit('rename')
        self.assertTrue(any('README.md' in e for e in self.inspect()))

    def test_new_source_version(self):
        self.metadata(3)
        self.commit('fork version')
        self.assertIn('a new upstream source must start at version 1', self.inspect())

    def test_update_and_review_version(self):
        self.base = self.git('rev-parse', 'HEAD')
        self.metadata(2)
        self.commit('update')
        (self.source / 'src/lib.rs').write_text('review fix\n')
        self.commit('review')
        self.assertEqual(self.inspect(), [])
        self.metadata(3)
        self.commit('unnecessary bump')
        self.assertTrue(any('version 2' in e for e in self.inspect()))

    def test_squash_merged_branch_is_not_a_pending_update(self):
        head = self.git('rev-parse', 'HEAD')
        self.git('checkout', '-qb', 'upstream-squash', self.base)
        self.git('checkout', head, '--', 'sources/en.example')
        self.commit('squashed source')
        self.base = self.git('rev-parse', 'HEAD')
        self.git('checkout', '--detach', head)
        self.assertEqual(self.inspect(), [])

    def test_bad_metadata_retains_commit_for_failure_status(self):
        (self.source / 'res/source.json').write_text('broken json')
        self.commit('broken metadata')
        result = subprocess.run(['python3', str(SCRIPTS / 'check-upstream-scope.py'), '--repo', str(self.repo), '--base', self.base, '--source', 'en.example', '--json'], text=True, capture_output=True)
        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)[0]
        self.assertEqual(report['sha'], self.git('rev-parse', 'HEAD'))
        self.assertTrue(report['errors'])

    def test_exporter_refuses_publishing_and_other_source_destinations(self):
        for branch in ['main', 'gh-pages', 'dead-sources', 'feat/en.other']:
            result = subprocess.run(['bash', str(SCRIPTS / 'pr-branch.sh'), 'en.example', '--branch', branch, '--rebuild', '--push'], cwd=self.repo, text=True, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Destination must be a recognized submission branch', result.stderr)

    def test_generated_package(self):
        (self.source / 'package.aix').write_text('generated')
        self.commit('artifact')
        self.assertTrue(any('generated' in e for e in self.inspect()))

    def test_missing_history_fails(self):
        with self.assertRaises(subprocess.CalledProcessError):
            scope.inspect(self.repo, 'missing-upstream', 'HEAD', 'en.example')

    def test_branch_routing(self):
        self.assertEqual(scope.source_for_branch('feat/en.example'), 'en.example')
        self.assertEqual(scope.source_for_branch('fix/kagane-chapter-fetch'), 'multi.kagane')
        self.assertIsNone(scope.source_for_branch('chore/en.example-before-rebuild-12345678'))

    def test_other_source(self):
        other = self.repo / 'sources/en.other'
        other.mkdir()
        (other / 'Cargo.toml').write_text('foreign')
        self.commit('another source')
        self.assertTrue(any('en.other' in e for e in self.inspect()))

    def template_consumer(self, consumer_version):
        uses = 'theme = { path = "../../templates/theme" }\n'
        (self.repo / 'templates/theme/src').mkdir(parents=True)
        (self.repo / 'templates/theme/src/lib.rs').write_text('template\n')
        other = self.repo / 'sources/en.other'
        (other / 'res').mkdir(parents=True)
        (other / 'Cargo.toml').write_text(uses)
        (other / 'res/source.json').write_text(json.dumps({'info': {'id': 'en.other', 'version': 2}}) + '\n')
        self.commit('upstream template and consumer')
        self.base = self.git('rev-parse', 'HEAD')
        (self.source / 'Cargo.toml').write_text(uses)
        (self.repo / 'templates/theme/src/lib.rs').write_text('shared feature\n')
        (other / 'res/filters.json').write_text('[]\n')
        (other / 'res/source.json').write_text(json.dumps({'info': {'id': 'en.other', 'version': consumer_version}}) + '\n')
        self.metadata(2)
        self.commit('template change')

    def test_template_change_with_bumped_consumer(self):
        self.template_consumer(3)
        self.assertEqual(self.inspect(), [])

    def test_template_consumer_must_be_bumped(self):
        self.template_consumer(2)
        self.assertTrue(any('en.other' in e and 'version 3' in e for e in self.inspect()))

    def test_unrelated_template(self):
        (self.repo / 'templates/other/src').mkdir(parents=True)
        (self.repo / 'templates/other/src/lib.rs').write_text('unrelated\n')
        self.commit('unrelated template')
        self.assertTrue(any('templates/other' in e for e in self.inspect()))


class VerificationTests(unittest.TestCase):
    def test_clippy_exit_status_blocks_packaging_even_without_diagnostics(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / 'source'
            source.mkdir()
            (source / 'Cargo.toml').write_text('test')
            bin_dir = root / 'bin'
            bin_dir.mkdir()
            cargo = bin_dir / 'cargo'
            cargo.write_text('#!/bin/sh\ncase "$1" in clippy) exit 42 ;; esac\n')
            cargo.chmod(0o755)
            aidoku = bin_dir / 'aidoku'
            aidoku.write_text('#!/bin/sh\ntouch "' + str(root / 'package-was-called') + '"\n')
            aidoku.chmod(0o755)
            result = subprocess.run(['bash', str(SCRIPTS / 'verify-source.sh'), str(source)], env={**os.environ, 'PATH': str(bin_dir) + ':' + os.environ['PATH']}, capture_output=True)
            self.assertEqual(result.returncode, 42)
            self.assertFalse((root / 'package-was-called').exists())

    def test_clippy_annotations_and_summary_count_messages_once(self):
        if not shutil.which('jq'):
            self.skipTest('jq unavailable')
        message = {'reason': 'compiler-message', 'message': {'level': 'warning', 'message': 'value, title: percent%', 'rendered': 'line one\nline two', 'spans': [{'is_primary': True, 'file_name': 'src/lib.rs', 'line_start': 2, 'line_end': 2, 'column_start': 1, 'column_end': 2}, {'is_primary': False, 'file_name': 'src/lib.rs'}]}}
        body = json.dumps(message) + '\n'
        annotations = subprocess.check_output(['jq', '--arg', 'WORKING_DIRECTORY', 'sources/en.example', '-rsf', str(SCRIPTS.parent / '.github/workflows/supporting/annotations.jq')], input=body, text=True)
        self.assertEqual(annotations.count('::warning'), 1)
        self.assertIn('value%2C title%3A percent%25', annotations)
        self.assertIn('line one%0Aline two', annotations)
        summary = subprocess.check_output(['jq', '--arg', 'RUSTC_VERSION', 'rustc test', '--arg', 'CARGO_VERSION', 'cargo test', '--arg', 'CLIPPY_VERSION', 'clippy test', '-rsf', str(SCRIPTS.parent / '.github/workflows/supporting/summary.jq')], input=body, text=True)
        self.assertIn('| warning | 1 |', summary)
        self.assertIn('| error | 0 |', summary)

    def test_mirror_drift_and_extra_files_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'scripts').mkdir()
            shutil.copy(SCRIPTS / 'sync-agent-docs.py', root / 'scripts')
            (root / '.claude/skills/aidoku-source').mkdir(parents=True)
            (root / '.claude/skills/aidoku-source/SKILL.md').write_text('skill\n')
            (root / 'AGENTS.md').write_text('.claude/skills/aidoku-source/SKILL.md\n.agents/skills/aidoku-source/SKILL.md\n')
            command = ['python3', str(root / 'scripts/sync-agent-docs.py')]
            subprocess.run(command, check=True, capture_output=True)
            self.assertEqual(subprocess.run(command + ['--check'], capture_output=True).returncode, 0)
            (root / '.agents/skills/extra').write_text('drift')
            self.assertNotEqual(subprocess.run(command + ['--check'], capture_output=True).returncode, 0)
            subprocess.run(command, check=True, capture_output=True)
            (root / 'CLAUDE.md').write_text('drift')
            self.assertNotEqual(subprocess.run(command + ['--check'], capture_output=True).returncode, 0)


if __name__ == '__main__':
    unittest.main()
