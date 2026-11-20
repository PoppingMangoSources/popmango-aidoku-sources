#!/usr/bin/env python3
"""Fail when a check-sources.py rule has no test that notices it switched off.

Each fail() or note() call is disabled in turn in a temporary copy of scripts/, and
scripts/tests/test_check_sources.py runs against it; a run that still passes means that rule's
tests cannot fail. Rules for single reviewed sources kept off main are listed in ARCHIVED.

Usage: scripts/check-rule-tests.py
"""

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CALL = re.compile(r"^(\s*)(fail|note)\(", re.M)
# Reviewed-consumer rules for sources archived on merged-sources; they still run on the
# upstream submission worktrees, where those sources exist.
ARCHIVED = (
    "Vine sites have no adult visibility control",
    "Keyoapp synopsis depends only",
    "Keyoapp credits split only",
    "uses child_nodes; every consumer",
    "Vine dynamic genre/tag taps",
    "enables exclusions on a reviewed include-only site",
    "reviewed genre filter must retain isGenre",
    "author search has no verified author metadata",
    "must stay inside this site's reviewed",
    "reviewed Manga labels require",
    "restores the rejected white-background icon",
    "differs from the reviewed icon",
)


def survivors(work: Path) -> list[str]:
    target = work / "scripts/check-sources.py"
    original = target.read_text()
    found = []
    for match in CALL.finditer(original):
        target.write_text(original[:match.start(2)] + "(lambda *_: None)" + original[match.end(2):])
        # Same-size mutants written within a second would otherwise reuse a stale cached .pyc.
        shutil.rmtree(work / "scripts/__pycache__", ignore_errors=True)
        result = subprocess.run([sys.executable, "-B", "-m", "unittest", "scripts/tests/test_check_sources.py"],
                                cwd=work, capture_output=True, text=True)
        if result.returncode == 0:
            line = original.count("\n", 0, match.start()) + 1
            found.append(f"scripts/check-sources.py:{line}: {original[match.start():].split(chr(10), 1)[0].strip()}")
    return found


def main() -> int:
    with tempfile.TemporaryDirectory() as temp:
        work = Path(temp)
        shutil.copytree(ROOT / "scripts", work / "scripts", ignore=shutil.ignore_patterns("__pycache__"))
        untested = [line for line in survivors(work) if not any(rule in line for rule in ARCHIVED)]
    for line in untested:
        print(f"{line}\n    no test fails with this rule switched off; add a failing and a passing case")
    print(f"rule tests: {len(untested)} untested rule(s)")
    return 1 if untested else 0


if __name__ == "__main__":
    sys.exit(main())
