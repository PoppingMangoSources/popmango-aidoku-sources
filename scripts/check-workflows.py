#!/usr/bin/env python3
"""Checks the workflows for problems GitHub's runner only shows once a run fails.

- Every `uses:` action runs on a Node version GitHub still supports. The runtime is read from
  the action's own action.yml at the pinned ref, so a new deprecation is caught without a list
  to keep up to date. Needs the network.
- A step that installs aidoku-test-runner installs the fontconfig and freetype headers first;
  the hosted runner has neither, and the install fails without them.

Usage: scripts/check-workflows.py [--offline]   (--offline skips the runtime check)
"""

import argparse
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEPRECATED = {"node12", "node16", "node20"}
USES = re.compile(r"^\s*(?:-\s*)?uses:\s*['\"]?([^\s'\"#]+)", re.M)
RUNS_USING = re.compile(r"^runs:\s*\n(?:[ \t]+.*\n)*?[ \t]+using:\s*['\"]?([\w-]+)", re.M)
TEST_RUNNER = "aidoku-test-runner"
FONT_HEADERS = ("libfontconfig-dev", "libfreetype-dev")


def workflow_files(root: Path) -> list[Path]:
    github = root / ".github"
    return sorted([*github.glob("workflows/*.y*ml"), *github.glob("actions/*/action.y*ml")])


def remote_actions(text: str) -> list[str]:
    return [ref for ref in USES.findall(text) if "@" in ref and not ref.startswith(("./", "docker://"))]


def fetch_action(ref: str) -> str | None:
    name, version = ref.split("@", 1)
    owner, repo, *path = name.split("/")
    folder = "/".join(path) + "/" if path else ""
    for file in ("action.yml", "action.yaml"):
        url = f"https://raw.githubusercontent.com/{owner}/{repo}/{version}/{folder}{file}"
        try:
            with urllib.request.urlopen(url, timeout=20) as response:
                return response.read().decode()
        except urllib.error.HTTPError as error:
            if error.code != 404:
                raise
    return None


def runtime(action_yml: str) -> str | None:
    found = RUNS_USING.search(action_yml)
    return found.group(1) if found else None


def check_runtimes(files: list[Path], fetch=fetch_action) -> list[str]:
    problems = []
    seen: dict[str, str | None] = {}
    for path in files:
        for ref in remote_actions(path.read_text()):
            if ref not in seen:
                text = fetch(ref)
                seen[ref] = runtime(text) if text else None
                if text is None:
                    problems.append(f"{path.name}: {ref} has no action.yml at that ref")
            if seen[ref] in DEPRECATED:
                problems.append(f"{path.name}: {ref} runs on {seen[ref]}, which GitHub has deprecated; move to a release on a newer Node")
    return problems


def check_test_runner(files: list[Path]) -> list[str]:
    problems = []
    for path in files:
        text = path.read_text()
        install = text.find(TEST_RUNNER)
        if install < 0 or "cargo install" not in text:
            continue
        missing = [name for name in FONT_HEADERS if not 0 <= text.find(name) < install]
        if missing:
            problems.append(f"{path.name}: installs {TEST_RUNNER} without first installing {', '.join(missing)}")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    files = workflow_files(ROOT)
    problems = check_test_runner(files)
    if not args.offline:
        problems += check_runtimes(files)
    for problem in problems:
        print(f"FAIL {problem}")
    print(f"checked {len(files)} workflow file(s): {len(problems)} failure(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
