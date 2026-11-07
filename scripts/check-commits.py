#!/usr/bin/env python3
"""Validate proposed messages and the exact commits a push or CI event introduces."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

NAME = "Popmango Extensions"
EMAIL = "297959108+PoppingMangoSources@users.noreply.github.com"
ATTRIBUTION = re.compile(r"^\s*(?:co-authored-by|assisted-by|signed-off-by|[\w-]*session[\w-]*)\s*:|\bgenerated (?:with|by)\b|https?://|#\d+\b", re.I | re.M)


def git(*args):
    return subprocess.check_output(["git", *args], text=True, stderr=subprocess.PIPE).rstrip("\n")


def inspect_raw(raw, label):
    headers, _, message = raw.partition("\n\n")
    errors = []
    for key in ("author", "committer"):
        if not re.search(rf"^{key} {re.escape(NAME)} <{re.escape(EMAIL)}> \d+ [+-]\d{{4}}$", headers, re.M):
            errors.append(f"{label}: {key} must use the owner identity; run scripts/install-hooks.sh")
    if re.search(r"^gpgsig(?:-sha256)? ", headers, re.M):
        errors.append(f"{label}: commits must be unsigned")
    if ATTRIBUTION.search(message):
        errors.append(f"{label}: remove attribution, session links and issue/PR references")
    return errors


def inspect_push(before, after, branch):
    if re.fullmatch(r"0+", after):
        return []
    errors = []
    if re.match(r"(?:claude|codex|copilot)/", branch, re.I):
        errors.append("Agent-named branches are forbidden")
    if before and not re.fullmatch(r"0+", before):
        if subprocess.run(["git", "cat-file", "-e", before + "^{commit}"], capture_output=True).returncode:
            if not re.fullmatch(r"[0-9a-f]{40,64}", before):
                raise ValueError("Invalid previous commit SHA")
            subprocess.check_call(["git", "fetch", "--no-tags", "origin", before])
        # A rebuilt submission branch moves onto a newer upstream; other contributors' commits there aren't ours.
        upstream = git("for-each-ref", "--format=%(refname)", "refs/remotes/upstream/").split()
        arguments = [after, "--not", before, *upstream]
    else:
        refs = [ref for ref in git("for-each-ref", "--format=%(refname)", "refs/remotes/").splitlines()
                if ref != f"refs/remotes/origin/{branch}"]
        arguments = [after, "--not", *refs]
    for sha in git("rev-list", *arguments).splitlines():
        errors.extend(inspect_raw(git("cat-file", "commit", sha), sha[:12]))
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--message")
    parser.add_argument("--pre-push", action="store_true")
    parser.add_argument("--event", action="store_true")
    parser.add_argument("--base")
    args = parser.parse_args()
    try:
        if args.message:
            raw = f"author {git('var', 'GIT_AUTHOR_IDENT')}\ncommitter {git('var', 'GIT_COMMITTER_IDENT')}\n\n{Path(args.message).read_text()}"
            errors = inspect_raw(raw, "pending commit")
        elif args.pre_push:
            errors = []
            for line in sys.stdin.read().splitlines():
                if not line.strip():
                    continue
                _, after, ref, before = line.split()
                errors.extend(inspect_push(before, after, ref.removeprefix("refs/heads/")))
        elif args.event:
            event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
            if "pull_request" in event:
                pr = event["pull_request"]
                errors = inspect_push(pr["base"]["sha"], pr["head"]["sha"], pr["head"]["ref"])
            elif event.get("before") and event.get("after"):
                errors = inspect_push(event["before"], event["after"], event["ref"].removeprefix("refs/heads/"))
            else:
                errors = inspect_raw(git("cat-file", "commit", "HEAD"), "HEAD")
        else:
            errors = inspect_push(args.base, "HEAD", git("branch", "--show-current")) if args.base else inspect_raw(git("cat-file", "commit", "HEAD"), "HEAD")
        if errors:
            print("\n".join(errors), file=sys.stderr)
            return 1
        print("Commit metadata passed.")
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Commit metadata validation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
