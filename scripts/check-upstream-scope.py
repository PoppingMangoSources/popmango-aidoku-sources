#!/usr/bin/env python3
"""Check a submission's tree AND commit history against upstream, without checking it out."""

import argparse
import json
import re
import subprocess
from pathlib import Path

SOURCE_ID = re.compile(r"(?:[a-z]{2,3}|multi)\.[a-z0-9][a-z0-9.-]*\Z")
TEMPLATE_DEPENDENCY = re.compile(r'path\s*=\s*"\.\./\.\./templates/([A-Za-z0-9_-]+)"')
LEGACY_BRANCHES = {"fix/kagane-chapter-fetch": "multi.kagane"}


def source_for_branch(branch):
    if branch in LEGACY_BRANCHES:
        return LEGACY_BRANCHES[branch]
    prefix, _, source = branch.partition("/")
    return source if prefix in {"feat", "fix"} and SOURCE_ID.fullmatch(source) else None


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args], stderr=subprocess.PIPE)


def templates_used(repo, ref, source):
    try:
        manifest = git(repo, "show", f"{ref}:sources/{source}/Cargo.toml").decode("utf-8", "replace")
    except subprocess.CalledProcessError:
        return set()
    return set(TEMPLATE_DEPENDENCY.findall(manifest))


def source_version(repo, ref, source):
    try:
        return json.loads(git(repo, "show", f"{ref}:sources/{source}/res/source.json"))["info"]["version"]
    except subprocess.CalledProcessError:
        return None


def inspect(repo, base, head, source):
    if not SOURCE_ID.fullmatch(source):
        raise ValueError("invalid source id")
    base = git(repo, "rev-parse", "--verify", base + "^{commit}").decode().strip()
    head = git(repo, "rev-parse", "--verify", head + "^{commit}").decode().strip()
    git(repo, "merge-base", base, head)  # Missing/shallow history must fail closed.
    commits = git(repo, "rev-list", base + ".." + head).decode().splitlines()
    if not commits:
        return {"sha": head, "source": source, "errors": [], "description": "Already upstream; no submission commits"}
    prefix = f"sources/{source}/"
    errors = set()
    # A template change may ride with a source using it, together with its other consumers.
    templates = templates_used(repo, head, source)
    consumers = set()

    def allowed(path):
        parts = path.split("/")
        if path.startswith(prefix):
            return True
        if len(parts) > 2 and parts[0] == "templates":
            return parts[1] in templates
        if len(parts) > 2 and parts[0] == "sources" and templates_used(repo, head, parts[1]) & templates:
            consumers.add(parts[1])
            return True
        return False

    def check_paths(data):
        for raw in data.split(b"\0"):
            if not raw:
                continue
            path = raw.decode("utf-8", "replace")
            if not allowed(path):
                errors.add(f"outside {prefix}: {path}")
            elif path.endswith(".aix") or any(part in {"target", "public", ".git"} for part in path.split("/")):
                errors.add(f"generated/nested repository file: {path}")

    delta = git(repo, "diff", "--no-renames", "--name-only", "-z", base + "..." + head)
    check_paths(delta)
    if not delta:
        errors.add("submission has commits but no source changes")
    for commit in commits:
        parents = git(repo, "rev-list", "--parents", "-n", "1", commit).split()
        if len(parents) > 2:
            errors.add(f"merge commit {commit[:12]}; rebase the submission instead")
        check_paths(git(repo, "diff-tree", "--root", "--no-commit-id", "--no-renames", "--name-only", "-r", "-z", commit))
    for consumer in sorted(consumers):
        previous = source_version(repo, base, consumer)
        version = source_version(repo, head, consumer)
        if previous is None:
            errors.add(f"{consumer} is not upstream; submit it on its own branch")
        elif version != previous + 1:
            errors.add(f"{consumer} changes with the template, so it must use version {previous + 1}, found {version}")
    metadata_path = prefix + "res/source.json"
    metadata = json.loads(git(repo, "show", head + ":" + metadata_path))["info"]
    if metadata.get("id") != source:
        errors.add("source.json id does not match the submission source")
    version = metadata.get("version")
    try:
        previous = json.loads(git(repo, "show", base + ":" + metadata_path))["info"]["version"]
    except subprocess.CalledProcessError:
        previous = None
    if not errors and previous is not None:
        identical = subprocess.run(["git", "-C", str(repo), "diff", "--quiet", base, head, "--", prefix]).returncode == 0
        if identical:
            return {"sha": head, "source": source, "errors": [], "description": "Source tree already matches upstream (squash merge)"}
    if type(version) is not int or (previous is None and version != 1):
        errors.add("a new upstream source must start at version 1")
    elif previous is not None and version != previous + 1:
        errors.add(f"an upstream update must use version {previous + 1}, found {version}")
    return {"sha": head, "source": source, "errors": sorted(errors), "description": f"{len(commits)} submission commit(s)"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".")
    parser.add_argument("--base", default="upstream/main")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--source")
    parser.add_argument("--branch", help="Infer source from feat/<id>, fix/<id> or the documented legacy Kagane branch")
    parser.add_argument("--all", action="store_true", help="Inspect every origin submission branch")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    targets = []
    if args.all:
        refs = git(args.repo, "for-each-ref", "--format=%(refname:strip=3)", "refs/remotes/origin").decode().splitlines()
        targets = [(branch, "refs/remotes/origin/" + branch, source_for_branch(branch)) for branch in refs if source_for_branch(branch)]
    else:
        branch = args.branch or git(args.repo, "branch", "--show-current").decode().strip()
        source = args.source or source_for_branch(branch)
        if not source:
            parser.error("provide --source <id> or a recognized --branch")
        targets = [(branch, args.head, source)]
    reports = []
    for branch, head, source in targets:
        try:
            report = inspect(args.repo, args.base, head, source)
        except (subprocess.CalledProcessError, ValueError, KeyError, TypeError) as error:
            resolved = subprocess.run(["git", "-C", args.repo, "rev-parse", "--verify", head + "^{commit}"], text=True, capture_output=True)
            sha = resolved.stdout.strip() if resolved.returncode == 0 else None
            report = {"sha": sha, "source": source, "errors": [f"cannot validate submission: {error}"], "description": "Validation unavailable"}
        report["branch"] = branch
        reports.append(report)
    if args.json:
        print(json.dumps(reports))
    else:
        for report in reports:
            print(f"{'FAIL' if report['errors'] else 'PASS'} {report['branch'] or report['source']}: {report['description']}")
            for error in report["errors"][:20]:
                print(f"  {error}")
            if len(report["errors"]) > 20:
                print(f"  ... {len(report['errors']) - 20} more scope violations")
    return int(any(report["errors"] for report in reports))


if __name__ == "__main__":
    raise SystemExit(main())
