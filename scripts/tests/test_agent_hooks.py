import contextlib
import importlib.util
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('agent_hooks', SCRIPTS / 'agent-hooks.py')
hooks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hooks)


def run(function, event):
    out = io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
        function(event)
    return out.getvalue()


class AgentHookTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'sources/en.example/src').mkdir(parents=True)
        (self.root / 'sources/en.example/Cargo.toml').write_text('[package]\n')
        (self.root / 'sources/en.example/src/lib.rs').write_text('fn a() {}\n')
        patches = [
            mock.patch.object(hooks, 'ROOT', self.root),
            mock.patch.object(hooks, 'stamp_path', lambda crate: self.root / 'stamps' / crate.replace('/', '__')),
            mock.patch.object(hooks.tempfile, 'gettempdir', lambda: str(self.root / 'tmp')),
        ]
        for patch in patches:
            patch.start()
            self.addCleanup(patch.stop)

    def test_pre_edit_reminds_once_per_source(self):
        event = {'session_id': 's1', 'tool_input': {'file_path': str(self.root / 'sources/en.example/src/lib.rs')}}
        first = json.loads(run(hooks.pre_edit, event))
        self.assertIn('scripts/review-source.sh en.example', first['hookSpecificOutput']['additionalContext'])
        self.assertEqual(first['hookSpecificOutput']['hookEventName'], 'PreToolUse')
        self.assertEqual(run(hooks.pre_edit, event), '')

    def test_pre_edit_ignores_other_files(self):
        for path in (self.root / 'README.md', Path('/etc/hosts')):
            self.assertEqual(run(hooks.pre_edit, {'session_id': 's2', 'tool_input': {'file_path': str(path)}}), '')

    def checked(self, returncode=0):
        result = subprocess.CompletedProcess([], returncode, stdout='FAIL rule\n' if returncode else '', stderr='')
        return mock.patch.object(hooks.subprocess, 'run', return_value=result)

    def test_stop_is_silent_without_changed_sources(self):
        with mock.patch.object(hooks, 'changed_crates', return_value=[]):
            self.assertEqual(run(hooks.stop, {}), '')

    def test_stop_blocks_until_reviewed(self):
        with mock.patch.object(hooks, 'changed_crates', return_value=['sources/en.example']), self.checked():
            blocked = json.loads(run(hooks.stop, {}))
            self.assertEqual(blocked['decision'], 'block')
            self.assertIn('review-source.sh en.example', blocked['reason'])
            # A second stop in the same turn reports instead of looping.
            self.assertEqual(run(hooks.stop, {'stop_hook_active': True}), '')
            hooks.stamp('en.example', 'pass')
            self.assertEqual(run(hooks.stop, {}), '')

    def test_stop_blocks_on_failed_rules_or_review(self):
        hooks.stamp('en.example', 'fail')
        with mock.patch.object(hooks, 'changed_crates', return_value=['sources/en.example']), self.checked(1):
            reason = json.loads(run(hooks.stop, {}))['reason']
        self.assertIn('check-sources.py fails', reason)
        self.assertIn('last failed', reason)

    def test_edit_after_review_invalidates_the_stamp(self):
        hooks.stamp('en.example', 'pass')
        (self.root / 'sources/en.example/src/lib.rs').write_text('fn b() {}\n')
        with mock.patch.object(hooks, 'changed_crates', return_value=['sources/en.example']), self.checked():
            self.assertIn('changed since its last review', json.loads(run(hooks.stop, {}))['reason'])

    def test_checklist_reads_section_three(self):
        (self.root / 'AGENTS.md').write_text(
            '## 3. Review feedback\n\n| What was flagged | The rule |\n|---|---|\n'
            '| Search returned tags | Cards carry key, title and cover. |\n\n- **Tag order:** sort.\n\n## 4. Next\n'
        )
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            hooks.checklist()
        self.assertIn('- [ ] Cards carry key, title and cover.  (Search returned tags)', out.getvalue())
        self.assertIn('Tag order:', out.getvalue())


if __name__ == '__main__':
    unittest.main()
