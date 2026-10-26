#!/usr/bin/env python3
"""Monitor source branches from the fork's default branch; never execute branch content."""

import json
import os
import subprocess
import sys
import urllib.request
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent.parent
    command = [sys.executable, str(root / "scripts/check-upstream-scope.py"), "--repo", str(root), "--json"]
    branch = os.environ.get("SCOPE_BRANCH", "")
    if branch:
        command += ["--branch", branch, "--head", "refs/remotes/origin/" + branch]
    else:
        command.append("--all")
    result = subprocess.run(command, text=True, capture_output=True)
    if result.returncode not in (0, 1) or not result.stdout:
        sys.stderr.write(result.stderr)
        return 1
    reports = json.loads(result.stdout)
    repository = os.environ.get("GITHUB_REPOSITORY")
    token = os.environ.get("GH_TOKEN")
    failed = False
    for report in reports:
        errors = report["errors"]
        failed |= bool(errors)
        state = "failure" if errors else "success"
        description = f"{len(errors)} scope/version violation(s)" if errors else report["description"]
        print(f"{state}: {report['branch']}: {description}")
        for error in errors[:10]:
            print(f"  {error}")
        if repository and token and report["sha"]:
            payload = json.dumps({"state": state, "context": "Upstream source scope", "description": description[:140], "target_url": f"{os.environ['GITHUB_SERVER_URL']}/{repository}/actions/runs/{os.environ['GITHUB_RUN_ID']}"}).encode()
            request = urllib.request.Request(f"https://api.github.com/repos/{repository}/statuses/{report['sha']}", data=payload, headers={"Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json"})
            with urllib.request.urlopen(request, timeout=30) as response:
                response.read()
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
