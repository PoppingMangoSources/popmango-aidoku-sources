import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AgentDocsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)

    def test_sync_detects_docs_instruction_and_command_drift_without_writing(self):
        for directory in ('scripts', 'docs', '.claude/skills'):
            (self.root / directory).mkdir(parents=True)
        shutil.copy2(ROOT / 'scripts/sync-agent-docs.py', self.root / 'scripts')
        shutil.copytree(ROOT / '.claude/skills', self.root / '.claude/skills', dirs_exist_ok=True)
        shutil.copy2(ROOT / 'AGENTS.md', self.root / 'AGENTS.md')  # primary for CLAUDE.md
        shutil.copy2(ROOT / 'docs/AGENTS.md', self.root / 'docs/AGENTS.md')  # primary for docs/CLAUDE.md
        command = ['python3', str(self.root / 'scripts/sync-agent-docs.py')]
        subprocess.run(command, check=True, capture_output=True)
        self.assertEqual(subprocess.run(command + ['--check'], capture_output=True).returncode, 0)
        targets = ['docs/CLAUDE.md', '.claude/commands/docs-sync.md', '.agents/skills/docs-sync/SKILL.md']  # docs/AGENTS.md is primary
        for relative in targets:
            with self.subTest(path=relative):
                path = self.root / relative
                path.write_text('drift\n')
                result = subprocess.run(command + ['--check'], capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(path.read_text(), 'drift\n')
                subprocess.run(command, check=True, capture_output=True)
        (self.root / 'docs/CLAUDE.md').unlink()  # docs/AGENTS.md remains the primary
        self.assertNotEqual(subprocess.run(command + ['--check'], capture_output=True).returncode, 0)
        subprocess.run(command, check=True, capture_output=True)
        self.assertEqual(subprocess.run(command + ['--check'], capture_output=True).returncode, 0)

    def test_docs_skills_are_discoverable_and_use_existing_shared_resources(self):
        for name in ('docs-sync', 'docs-lint', 'docs-query', 'docs-status'):
            skill = ROOT / '.agents/skills' / name / 'SKILL.md'
            content = skill.read_text()
            self.assertTrue(content.startswith(f'---\nname: {name}\ndescription: '))
            self.assertIn('docs/AGENTS.md', content)  # shared with docs/CLAUDE.md
            self.assertNotIn('$ARGUMENTS', content)
            self.assertNotIn('.claude/scripts/docs-lint.py', content)
            for relative in re.findall(r'`((?:scripts|\.agents|\.claude)/[\w./-]+)`', content):
                self.assertTrue((ROOT / relative).is_file(), relative)

    def test_shared_linter_skips_both_instruction_files_but_checks_real_pages(self):
        docs = self.root / 'docs'
        docs.mkdir()
        for name in ('AGENTS.md', 'CLAUDE.md'):
            (docs / name).write_text('Instruction file without page frontmatter.\n')
        (docs / 'index.md').write_text('# Documentation\n')
        command = ['python3', str(ROOT / 'scripts/docs-lint.py'), '--root', str(self.root), '--json']
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['pages'], 0)
        legacy = [*command]
        legacy[1] = str(ROOT / '.claude/scripts/docs-lint.py')
        self.assertEqual(subprocess.check_output(legacy, text=True), result.stdout)
        (docs / 'broken.md').write_text('A real page without required frontmatter.\n')
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertEqual(report['pages'], 1)
        self.assertIn('missing frontmatter: broken', report['problem_list'])
        legacy_result = subprocess.run(legacy, capture_output=True, text=True)
        self.assertEqual(legacy_result.returncode, 1)
        self.assertEqual(legacy_result.stdout, result.stdout)

    def test_codex_configured_commands_accept_real_hook_payloads(self):
        settings = json.loads((ROOT / '.codex/hooks.json').read_text())
        for directory in ('scripts', '.githooks', 'sources/en.example/src'):
            (self.root / directory).mkdir(parents=True)
        for relative in ('scripts/agent-hooks.py', 'scripts/install-hooks.sh', '.githooks/pre-push',
                         '.githooks/commit-msg', 'AGENTS.md'):  # identical rules in CLAUDE.md
            shutil.copy2(ROOT / relative, self.root / relative)
        event = {'session_id': 'config-' + self.root.name, 'cwd': str(self.root), 'tool_name': 'Bash',
                 'tool_input': {'command': 'git commit -m "chore: example"'}}
        hooks = settings['hooks']
        self.assertEqual(set(hooks), {'SessionStart', 'PreToolUse', 'Stop'})
        groups = [g for g in hooks['PreToolUse'] if re.search(g['matcher'], event['tool_name'])]
        self.assertEqual(len(groups), 1)
        for group in groups:
            result = subprocess.run(group['hooks'][0]['command'], shell=True, cwd=self.root,
                                    input=json.dumps(event), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)['hookSpecificOutput']['permissionDecision'], 'deny')
        event.update(tool_name='apply_patch', tool_input={'command':
            '*** Begin Patch\n*** Add File: sources/en.example/src/lib.rs\n+fn x() {}\n*** End Patch\n'})
        groups = [g for g in hooks['PreToolUse'] if re.search(g['matcher'], event['tool_name'])]
        self.assertEqual(len(groups), 1)
        result = subprocess.run(groups[0]['hooks'][0]['command'], shell=True, cwd=self.root,
                                input=json.dumps(event), capture_output=True, text=True)
        self.assertIn('review-source.sh en.example', json.loads(result.stdout)['hookSpecificOutput']['additionalContext'])
        startup = next(g for g in hooks['SessionStart'] if not g.get('matcher'))
        subprocess.run(startup['hooks'][0]['command'], shell=True, cwd=self.root, check=True, capture_output=True)
        path = subprocess.check_output(['git', 'config', 'core.hooksPath'], cwd=self.root, text=True).strip()
        self.assertEqual(path, str(self.root / '.githooks'))
        compact = next(g for g in hooks['SessionStart'] if re.search(g.get('matcher', '^$'), 'compact'))
        result = subprocess.run(compact['hooks'][0]['command'], shell=True, cwd=self.root,
                                input='{}', capture_output=True, text=True)
        self.assertIn('## 0. Hard rules', result.stdout)
        result = subprocess.run(hooks['Stop'][0]['hooks'][0]['command'], shell=True, cwd=self.root,
                                input='{invalid', capture_output=True, text=True)
        self.assertEqual(json.loads(result.stdout)['decision'], 'block')


if __name__ == '__main__':
    unittest.main()
