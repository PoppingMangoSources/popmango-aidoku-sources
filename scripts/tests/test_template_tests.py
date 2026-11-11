import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class TemplateTestWorkflowTests(unittest.TestCase):
    def test_contract_edits_select_existing_templates_without_fork_tests(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            git = lambda *args: subprocess.check_output(['git', *args], cwd=root, text=True).strip()
            git('init', '-q', '-b', 'main')
            git('config', 'user.name', 'Workflow tests')
            git('config', 'user.email', 'workflow@example.invalid')
            git('config', 'commit.gpgsign', 'false')
            (root / 'templates/mangathemesia/src').mkdir(parents=True)
            (root / 'templates/mangathemesia/Cargo.toml').write_text('')
            (root / 'templates/mangathemesia/src/lib.rs').write_text('')
            (root / 'scripts').mkdir()
            shutil.copy2(ROOT / 'scripts/changed-crates.py', root / 'scripts')
            (root / 'scripts/review-contracts.py').write_text('before\n')
            git('add', '.')
            git('commit', '-qm', 'baseline')
            base = git('rev-parse', 'HEAD')
            (root / 'scripts/review-contracts.py').write_text('after\n')
            git('commit', '-qam', 'contract change')
            workflow = (ROOT / '.github/workflows/test.yaml').read_text()
            step = workflow.split('      - name: Pick sources\n', 1)[1].split('      - name:', 1)[0]
            script = '\n'.join(line[10:] for line in step.split('        run: |\n', 1)[1].splitlines())
            temporary = root / 'temporary'
            temporary.mkdir()
            env = {**os.environ, 'GITHUB_EVENT_NAME': 'push', 'BASE_SHA': base, 'REQUESTED': '',
                   'RUNNER_TEMP': str(temporary), 'GITHUB_STEP_SUMMARY': str(root / 'summary')}
            subprocess.run(['bash', '-euo', 'pipefail', '-c', script], cwd=root, env=env,
                           check=True, capture_output=True)
            self.assertEqual((temporary / 'candidates').read_text().splitlines(), ['templates/mangathemesia'])
            self.assertEqual((temporary / 'tested').read_text(), '')
            install = workflow.split('      - name: Install the test runner\n', 1)[1].split('      - name:', 1)[0]
            self.assertIn('[[ -s "$RUNNER_TEMP/candidates" ]] || exit 0', install)

    def test_removed_consumer_is_not_selected_after_archiving(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            git = lambda *args: subprocess.check_output(['git', *args], cwd=root, text=True).strip()
            git('init', '-q', '-b', 'main')
            git('config', 'user.name', 'Workflow tests')
            git('config', 'user.email', 'workflow@example.invalid')
            git('config', 'commit.gpgsign', 'false')
            for crate in ('templates/example', 'sources/en.removed'):
                (root / crate / 'src').mkdir(parents=True)
                (root / crate / 'Cargo.toml').write_text('path = "../../templates/example"\n')
                (root / crate / 'src/test.rs').write_text('use aidoku_test::aidoku_test;\n')
            (root / 'scripts').mkdir()
            shutil.copy2(ROOT / 'scripts/changed-crates.py', root / 'scripts')
            git('add', '.')
            git('commit', '-qm', 'baseline')
            base = git('rev-parse', 'HEAD')
            git('rm', '-qr', 'sources/en.removed')
            (root / 'templates/example/src/lib.rs').write_text('changed\n')
            git('add', '.')
            git('commit', '-qm', 'archive consumer')
            workflow = (ROOT / '.github/workflows/test.yaml').read_text()
            step = workflow.split('      - name: Pick sources\n', 1)[1].split('      - name:', 1)[0]
            script = '\n'.join(line[10:] for line in step.split('        run: |\n', 1)[1].splitlines())
            temporary = root / 'temporary'
            temporary.mkdir()
            env = {**os.environ, 'GITHUB_EVENT_NAME': 'push', 'BASE_SHA': base, 'REQUESTED': '',
                   'RUNNER_TEMP': str(temporary), 'GITHUB_STEP_SUMMARY': str(root / 'summary')}
            subprocess.run(['bash', '-euo', 'pipefail', '-c', script], cwd=root, env=env,
                           check=True, capture_output=True)
            self.assertEqual((temporary / 'tested').read_text().splitlines(), ['templates/example'])

    def test_template_changes_select_the_template_and_its_consumers(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            git = lambda *args: subprocess.check_output(['git', *args], cwd=root, text=True).strip()
            git('init', '-q', '-b', 'main')
            git('config', 'user.name', 'Workflow tests')
            git('config', 'user.email', 'workflow@example.invalid')
            git('config', 'commit.gpgsign', 'false')
            for crate in ('templates/example', 'sources/en.example'):
                (root / crate / 'src').mkdir(parents=True)
                (root / crate / 'Cargo.toml').write_text('path = "../../templates/example"\n')
                (root / crate / 'src/test.rs').write_text('use aidoku_test::aidoku_test;\n')
            (root / 'scripts').mkdir()
            shutil.copy2(ROOT / 'scripts/changed-crates.py', root / 'scripts')
            git('add', '.')
            git('commit', '-qm', 'baseline')
            base = git('rev-parse', 'HEAD')
            (root / 'templates/example/src/lib.rs').write_text('changed\n')
            git('add', '.')
            git('commit', '-qm', 'template change')
            workflow = (ROOT / '.github/workflows/test.yaml').read_text()
            step = workflow.split('      - name: Pick sources\n', 1)[1].split('      - name:', 1)[0]
            script = '\n'.join(line[10:] for line in step.split('        run: |\n', 1)[1].splitlines())
            temporary = root / 'temporary'
            temporary.mkdir()
            for event, requested in [('push', ''), ('workflow_dispatch', 'templates/example en.example'),
                                     ('workflow_dispatch', '')]:
                with self.subTest(event=event, requested=requested):
                    env = {**os.environ, 'GITHUB_EVENT_NAME': event, 'BASE_SHA': base, 'REQUESTED': requested,
                           'RUNNER_TEMP': str(temporary), 'GITHUB_STEP_SUMMARY': str(root / 'summary')}
                    subprocess.run(['bash', '-euo', 'pipefail', '-c', script], cwd=root, env=env,
                                   check=True, capture_output=True)
                    self.assertEqual(sorted((temporary / 'tested').read_text().splitlines()),
                                     ['sources/en.example', 'templates/example'])


if __name__ == '__main__':
    unittest.main()
