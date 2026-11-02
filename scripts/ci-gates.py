#!/usr/bin/env python3
"""Run the repository-owned, offline checks before source validation and publication."""
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent


def run(*args):
    subprocess.check_call(args, cwd=ROOT)


def main():
    try:
        base = sys.argv[2] if len(sys.argv) == 3 and sys.argv[1] == "--base" else "origin/main"
        if sys.argv[1:] == ["--event"]:
            run(sys.executable, "scripts/check-commits.py", "--event")
            event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
            base = event.get("pull_request", {}).get("base", {}).get("sha") or event.get("before") or base
            if set(base) == {"0"}:
                base = "origin/main"
        else:
            run(sys.executable, "scripts/check-commits.py")
        run("git", "cat-file", "-e", base + "^{commit}")
        run(sys.executable, "scripts/sync-agent-docs.py", "--check")
        run(sys.executable, "-m", "unittest", "discover", "-s", "scripts/tests", "-v")
        run(sys.executable, "scripts/check-workflows.py", "--offline")
        run(sys.executable, "scripts/check-sources.py", "--base", base)
        print("All offline repository checks passed. Rust, live and device checks remain explicit gates.")
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Repository validation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
