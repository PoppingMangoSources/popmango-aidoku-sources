#!/usr/bin/env python3
"""Keep the requested physical instruction and skill copies identical. --check never writes."""

import argparse
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    primary = ROOT / ".claude/skills"
    mirror = ROOT / ".agents/skills"
    errors = []
    if args.check:
        if (ROOT / "AGENTS.md").read_bytes() != (ROOT / "CLAUDE.md").read_bytes():
            errors.append("AGENTS.md and CLAUDE.md differ")
        if mirror.is_symlink():
            errors.append(".agents/skills must be a physical copy, not a symlink")
        source_files = {p.relative_to(primary): p.read_bytes() for p in primary.rglob("*") if p.is_file()}
        mirror_files = {p.relative_to(mirror): p.read_bytes() for p in mirror.rglob("*") if p.is_file()}
        if source_files != mirror_files:
            errors.append(".claude/skills and .agents/skills differ")
        for path in (ROOT / "AGENTS.md", ROOT / "CLAUDE.md"):
            for reference in (".claude/skills/aidoku-source/SKILL.md", ".agents/skills/aidoku-source/SKILL.md"):
                if reference not in path.read_text():
                    errors.append(f"{path.name} does not point to {reference}")
        for error in errors:
            print(f"FAIL {error}")
        if not errors:
            print("PASS instruction files and skill trees match")
        return int(bool(errors))
    shutil.copyfile(ROOT / "AGENTS.md", ROOT / "CLAUDE.md")
    if mirror.is_symlink():
        mirror.unlink()
    elif mirror.exists():
        shutil.rmtree(mirror)
    shutil.copytree(primary, mirror)
    print("Synchronized CLAUDE.md and .agents/skills")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
