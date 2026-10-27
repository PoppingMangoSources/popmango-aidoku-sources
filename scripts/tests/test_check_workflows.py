import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('check_workflows', SCRIPTS / 'check-workflows.py')
workflows = importlib.util.module_from_spec(spec)
spec.loader.exec_module(workflows)

NODE20 = "name: x\nruns:\n  using: 'node20'\n  main: index.js\n"
NODE24 = "name: x\nruns:\n  using: node24\n  main: index.js\n"
COMPOSITE = "name: x\nruns:\n  using: composite\n  steps: []\n"


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / 'ci.yaml'

    def write(self, text):
        self.path.write_text(text)
        return [self.path]

    def test_deprecated_runtime_fails(self):
        files = self.write('steps:\n  - uses: actions/cache@v4\n  - uses: actions/checkout@v6\n')
        actions = {'actions/cache@v4': NODE20, 'actions/checkout@v6': NODE24}
        problems = workflows.check_runtimes(files, fetch=actions.get)
        self.assertEqual(len(problems), 1)
        self.assertIn('actions/cache@v4 runs on node20', problems[0])

    def test_local_docker_and_composite_pass(self):
        files = self.write(
            'steps:\n  - uses: ./.github/actions/x\n  - uses: docker://rhysd/actionlint:1.7.7\n'
            '  - uses: dtolnay/rust-toolchain@stable\n'
        )
        self.assertEqual(workflows.check_runtimes(files, fetch={'dtolnay/rust-toolchain@stable': COMPOSITE}.get), [])

    def test_missing_action_fails(self):
        files = self.write('steps:\n  - uses: actions/cache@v99\n')
        self.assertTrue(workflows.check_runtimes(files, fetch=lambda ref: None))

    def test_test_runner_without_font_headers_fails(self):
        files = self.write('run: cargo install --locked --git https://github.com/Aidoku/aidoku-rs aidoku-test-runner\n')
        self.assertTrue(any('libfontconfig-dev' in p for p in workflows.check_test_runner(files)))

    def test_test_runner_after_font_headers_passes(self):
        files = self.write(
            'run: |\n  sudo apt-get install -y libfontconfig-dev libfreetype-dev\n'
            '  cargo install --git https://github.com/Aidoku/aidoku-rs aidoku-test-runner\n'
        )
        self.assertEqual(workflows.check_test_runner(files), [])


if __name__ == '__main__':
    unittest.main()
