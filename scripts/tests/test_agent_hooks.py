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

    def test_codex_patch_covers_multiple_crates_and_home_rules(self):
        self.write_rules()
        patch = ('*** Begin Patch\n*** Update File: sources/en.example/src/lib.rs\n@@\n+fn b() {}\n'
                 '*** Add File: templates/example/src/home.rs\n+HomeComponentValue::Scroller {}\n*** End Patch\n')
        event = {'session_id': 'patch', 'tool_name': 'apply_patch', 'cwd': str(self.root),
                 'tool_input': {'command': patch}}
        context = json.loads(run(hooks.pre_edit, event))['hookSpecificOutput']['additionalContext']
        self.assertIn('sources/en.example', context)
        self.assertIn('templates/example', context)
        self.assertIn('.agents/skills/aidoku-source/SKILL.md', context)
        self.assertEqual(context.count('Rows are cards.'), 1)
        self.assertEqual(run(hooks.pre_edit, event), '')

    def test_codex_patch_paths_follow_cwd_and_include_moves_and_deletes(self):
        self.write_rules()
        event = {'session_id': 'move', 'tool_name': 'apply_patch', 'cwd': str(self.root / 'sources'),
                 'tool_input': {'command': '*** Begin Patch\n*** Update File: en.example/src/lib.rs\n'
                                '*** Move to: en.other/src/home.rs\n@@\n+HomeComponent {}\n'
                                '*** Delete File: en.third/src/lib.rs\n'
                                '*** Update File: ../../outside/src/lib.rs\n@@\n+fn x() {}\n*** End Patch\n'}}
        context = json.loads(run(hooks.pre_edit, event))['hookSpecificOutput']['additionalContext']
        for crate in ('sources/en.example', 'sources/en.other', 'sources/en.third'):
            self.assertIn(crate, context)
        self.assertNotIn('outside', context)
        self.assertIn('Rows are cards.', context)

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

    def write_rules(self):
        (self.root / 'AGENTS.md').write_text(
            '## 0. Hard rules\n\n- Commit with scripts/commit.sh.\n\n---\n\n'
            '## 3. Review feedback\n\n| What was flagged | The rule |\n|---|---|\n'
            '| Search returned tags | Cards carry key, title and cover. |\n\n## 4. Next\n\n'
            '### Home components\n\n- Rows are cards.\n\n### Global mutable state\n\n'
            "## 9. This repo's workflow\n\n- Work on main.\n- Commits are unsigned.\n\n### How a rule\n"
        )

    def bash(self, command):
        return run(hooks.pre_bash, {'session_id': 'b', 'tool_input': {'command': command}})

    def test_direct_commit_is_refused(self):
        self.write_rules()
        for command in ('git commit -m "x"', 'git -c commit.gpgsign=false commit -q -m x', 'cd a && git -C b commit'):
            out = json.loads(self.bash(command))['hookSpecificOutput']
            self.assertEqual(out['permissionDecision'], 'deny', command)
            self.assertIn('scripts/commit.sh', out['permissionDecisionReason'])

    def test_commit_inside_a_script_is_refused(self):
        self.write_rules()
        script = self.root / 'pad/release.sh'
        script.parent.mkdir()
        script.write_text('gc() { git -c commit.gpgsign=false commit --no-gpg-sign "$@"; }\n')
        out = json.loads(self.bash(f'bash {script}'))['hookSpecificOutput']
        self.assertEqual(out['permissionDecision'], 'deny')
        self.assertIn(str(script), out['permissionDecisionReason'])
        script.write_text('scripts/commit.sh -m "fix: x" -m "Body."\n')
        self.assertNotIn('deny', self.bash(f'bash {script}'))

    def test_commit_wrapper_and_other_git_pass(self):
        self.write_rules()
        (self.root / 'scripts').mkdir()
        (self.root / 'scripts/commit.sh').write_text('git -c commit.gpgsign=false commit "$@"\n')
        out = json.loads(self.bash('scripts/commit.sh -m "fix: x" -m "Body."'))['hookSpecificOutput']
        self.assertNotIn('permissionDecision', out)
        self.assertIn('Commits are unsigned.', out['additionalContext'])
        self.assertNotIn('Work on main.', out['additionalContext'])
        for command in ('git log --grep commit', 'git commit-tree HEAD^{tree}', 'git status'):
            self.assertEqual(self.bash(command), '', command)

    def test_source_push_prints_the_compact_checklist(self):
        self.write_rules()
        with mock.patch.object(hooks, 'changed_crates', return_value=['sources/en.example']):
            for command in ('git push -u origin main', 'scripts/pr-branch.sh en.example --push'):
                context = json.loads(self.bash(command))['hookSpecificOutput']['additionalContext']
                self.assertIn('- [ ] Cards carry key, title and cover.', context)
                self.assertNotIn('(Search returned tags)', context)
                self.assertIn('each item the person asked for', context)

    def test_push_without_source_changes_is_quiet(self):
        self.write_rules()
        with mock.patch.object(hooks, 'changed_crates', return_value=[]):
            self.assertEqual(self.bash('git push -u origin main'), '')

    def test_home_edit_prints_home_rules_once(self):
        self.write_rules()
        event = {'session_id': 'h', 'tool_input': {
            'file_path': str(self.root / 'sources/en.example/src/lib.rs'),
            'new_string': 'HomeComponentValue::Scroller {',
        }}
        self.assertIn('Rows are cards.', json.loads(run(hooks.pre_edit, event))['hookSpecificOutput']['additionalContext'])
        self.assertEqual(run(hooks.pre_edit, event), '')

    def test_digest_prints_the_hard_rules(self):
        self.write_rules()
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            hooks.digest()
        self.assertIn('Commit with scripts/commit.sh.', out.getvalue())
        self.assertNotIn('Review feedback', out.getvalue())

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
