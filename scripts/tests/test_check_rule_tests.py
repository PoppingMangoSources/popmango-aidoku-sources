import importlib.util
import re
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('check_rule_tests', SCRIPTS / 'check-rule-tests.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)

CHECKER = '''failures = []


def fail(message):
    failures.append(message)


def note(message):
    failures.append(message)


def check(text):
    if "tab" in text:
        fail("covered rule")
    if "space" in text:
        note("uncovered rule")
'''

TESTS = '''import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('c', Path(__file__).resolve().parents[1] / 'check-sources.py')
checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks)


class T(unittest.TestCase):
    def test_tab(self):
        checks.check("tab")
        self.assertEqual(checks.failures, ["covered rule"])
'''


class RuleTestGateTests(unittest.TestCase):
    def test_reports_only_the_rule_no_test_catches(self):
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp)
            (work / 'scripts/tests').mkdir(parents=True)
            (work / 'scripts/check-sources.py').write_text(CHECKER)
            (work / 'scripts/tests/test_check_sources.py').write_text(TESTS)
            found = gate.survivors(work)
        self.assertEqual(len(found), 1, found)
        self.assertIn('note("uncovered rule")', found[0])

    def test_push_hook_runs_every_ci_gate(self):
        # The two lists are kept by hand; a gate added to CI alone once let an untested rule reach a push.
        ci = set(re.findall(r'scripts/[\w-]+\.(?:py|sh)', (SCRIPTS / 'ci-gates.py').read_text()))
        hook = (SCRIPTS.parent / '.githooks/pre-push').read_text()
        self.assertIn('scripts/check-rule-tests.py', ci)
        self.assertEqual(sorted(gate for gate in ci if gate not in hook), [])


if __name__ == '__main__':
    unittest.main()
