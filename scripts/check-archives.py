#!/usr/bin/env python3
"""Require complete source and template snapshots before removing a crate."""

import argparse
from pathlib import Path
import posixpath
import subprocess
import tomllib


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args], stderr=subprocess.PIPE)


def tree(repo, ref, path):
    return git(repo, "ls-tree", "-r", ref, "--", path)


def dependencies(repo, ref, crate):
    manifest = tomllib.loads(git(repo, "show", f"{ref}:{crate}/Cargo.toml").decode())
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for dependency in manifest.get(section, {}).values():
            if isinstance(dependency, dict) and "path" in dependency:
                yield posixpath.normpath(f"{crate}/{dependency['path']}")


def inspect(repo, base, head, archives):
    deleted = git(repo, "diff", "--no-renames", "--diff-filter=D", "--name-only", f"{base}...{head}").decode().splitlines()
    crates = [str(Path(path).parent) for path in deleted
              if path.startswith("sources/") and len(Path(path).parts) == 3 and path.endswith("/Cargo.toml")]
    errors = []
    for crate in crates:
        paths = {crate}
        pending = [crate]
        while pending:
            for dependency in dependencies(repo, base, pending.pop()):
                if dependency not in paths:
                    paths.add(dependency)
                    pending.append(dependency)
        icon = f"media/sources/{Path(crate).name.split('.', 1)[1]}.png"
        if tree(repo, base, icon):
            paths.add(icon)
        if not any(all(tree(repo, base, path) == tree(repo, archive, path) for path in paths)
                   for archive in archives):
            errors.append(f"{crate}: no archive preserves its complete snapshot, tests, icons and path dependencies")
    return crates, errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--archive-ref", action="append")
    args = parser.parse_args()
    archives = []
    for ref in args.archive_ref or ["origin/merged-sources", "origin/dead-sources"]:
        if subprocess.run(["git", "-C", str(args.repo), "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}"], capture_output=True).returncode == 0:
            archives.append(ref)
    try:
        crates, errors = inspect(args.repo, args.base, args.head, archives)
    except (subprocess.CalledProcessError, tomllib.TOMLDecodeError) as error:
        parser.exit(1, f"Archive validation failed: {error}\n")
    for error in errors:
        print(error)
    print(f"Archive snapshots: {len(crates)} removed source(s), {len(errors)} failure(s)")
    return bool(errors)


if __name__ == "__main__":
    raise SystemExit(main())
