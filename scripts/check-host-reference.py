#!/usr/bin/env python3
"""Checks that the host reference still describes the app's code.

The skill's references/aidoku-host.md cites files in AidokuRunner, the Aidoku app and
aidoku-rs, pinned in its `pins` block to the blob each was read at. This fetches each
repository's current main and reports every cited file that changed, so the facts drawn from
it get re-read. --update records the current blobs, after the reference has been re-read.

Usage: scripts/check-host-reference.py [--update]
"""

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REFERENCE = ROOT / ".claude/skills/aidoku-source/references/aidoku-host.md"
BLOCK = re.compile(r"(```pins\n)(.*?)(\n```)", re.S)


def parse(block: str) -> list[tuple[str, str, list[tuple[str, str]]]]:
    repos = []
    for line in block.splitlines():
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "repo":
            repos.append((parts[1], parts[2], []))
        else:
            repos[-1][2].append((parts[0], parts[1]))
    return repos


def current(url: str, paths: list[str], work: Path) -> tuple[str, dict[str, str]]:
    clone = work / re.sub(r"\W", "_", url)
    subprocess.run(
        ["git", "clone", "-q", "--depth", "1", "--filter=blob:none", "--no-checkout", url, str(clone)],
        check=True,
    )
    commit = subprocess.check_output(["git", "-C", str(clone), "rev-parse", "HEAD"], text=True).strip()
    rows = subprocess.check_output(["git", "-C", str(clone), "ls-tree", "HEAD", "--", *paths], text=True)
    blobs = {row.split("\t")[1]: row.split()[2] for row in rows.splitlines()}
    return commit, blobs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--update", action="store_true")
    args = parser.parse_args()
    text = REFERENCE.read_text()
    match = BLOCK.search(text)
    if not match:
        sys.exit(f"{REFERENCE.relative_to(ROOT)} has no pins block")
    lines, changed = [], []
    with tempfile.TemporaryDirectory() as temp:
        for url, pinned, files in parse(match.group(2)):
            commit, blobs = current(url, [path for _, path in files], Path(temp))
            lines.append(f"repo {url} {commit if args.update else pinned}")
            for blob, path in files:
                now = blobs.get(path, "missing")
                if now != blob:
                    changed.append(f"{url} {path}: {blob[:10]} -> {now[:10]}")
                lines.append(f"{now if args.update else blob} {path}")
    if args.update:
        REFERENCE.write_text(text[: match.start(2)] + "\n".join(lines) + text[match.end(2) :])
        print(f"Recorded {len(changed)} changed file(s); re-read them before committing.")
        return 0
    for line in changed:
        print(f"CHANGED {line}")
    print(f"{len(changed)} cited file(s) changed since the reference was read")
    return 1 if changed else 0


if __name__ == "__main__":
    sys.exit(main())
