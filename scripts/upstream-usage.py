#!/usr/bin/env python3
"""Shows how much of a source's code the merged upstream sources share.

For every aidoku-rs item the source uses and every name it defines, counts the upstream
sources that use or define the same, and names a few. An item no merged source uses is worth
a look: either the site needs it, or there is a usual way to do the same thing (find it among
the sources built like this one, AGENTS.md / CLAUDE.md section 6). A report, not a gate.

Usage: scripts/upstream-usage.py <id> [--rare N]   (N: list items used by at most N sources, default 2)
"""

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFINITION = r"\b(?:fn|const|struct|enum|static|trait|type)\s+{}\b"


def rust(directory: Path) -> str:
    return "\n".join(p.read_text(errors="ignore") for p in sorted(directory.glob("src/**/*.rs")) if p.name != "test.rs")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("source")
    parser.add_argument("--rare", type=int, default=2)
    args = parser.parse_args()
    ours = rust(ROOT / "sources" / args.source)
    if not ours:
        sys.exit(f"no Rust in sources/{args.source}")

    imported = set()
    for block in re.findall(r"use aidoku::\{(.*?)\};", ours, re.S) + re.findall(r"use aidoku::([\w:]+);", ours):
        imported |= set(re.findall(r"\b[A-Za-z_]\w+\b", block))
    defined = set(re.findall(r"\b(?:fn|const|struct|enum|static|trait|type)\s+([A-Za-z_]\w*)", ours))
    imported -= defined | {"alloc", "imports", "helpers", "prelude", "std", "net", "defaults", "uri", "string", "self"}

    subprocess.run(["git", "-C", str(ROOT), "fetch", "-q", "https://github.com/Aidoku-Community/sources.git",
                    "main:refs/remotes/upstream/main"], check=True)
    with tempfile.TemporaryDirectory() as work:
        subprocess.run(["git", "-C", str(ROOT), "worktree", "add", "-q", "--detach", work, "upstream/main"], check=True)
        try:
            upstream = {d.name: rust(d) for d in sorted((Path(work) / "sources").iterdir()) if d.name != args.source}
        finally:
            subprocess.run(["git", "-C", str(ROOT), "worktree", "remove", "--force", work], check=True)

    def report(title: str, names: set[str], pattern: str) -> None:
        rows = []
        for name in sorted(names):
            hits = [s for s, text in upstream.items() if re.search(pattern.format(re.escape(name)), text)]
            if len(hits) <= args.rare:
                rows.append(f"{len(hits):4}  {name:28} {' '.join(hits[:4])}")
        print(f"{title} used by at most {args.rare} of {len(upstream)} upstream sources:")
        print("\n".join(rows) if rows else "     none")

    report("aidoku items", imported, r"\b{}\b")
    report("Names defined", defined, DEFINITION)
    return 0


if __name__ == "__main__":
    sys.exit(main())
