#!/usr/bin/env python3
"""Keep the requested physical instruction and skill copies identical. --check never writes."""

import argparse
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INSTRUCTION_PAIRS = (("AGENTS.md", "CLAUDE.md"), ("docs/AGENTS.md", "docs/CLAUDE.md"))
DOC_WORKFLOWS = ("docs-sync", "docs-lint", "docs-query", "docs-status")


def command_wrapper(name):
    return (f"# /{name}\n\nFollow `.claude/skills/{name}/SKILL.md`.\n"
            "Pass the user question and command arguments to that workflow.\n\nArguments: $ARGUMENTS\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    primary = ROOT / ".claude/skills"
    mirror = ROOT / ".agents/skills"
    errors = []
    if args.check:
        for source, target in INSTRUCTION_PAIRS:
            if not (ROOT / source).is_file() or not (ROOT / target).is_file():
                errors.append(f"{source} or {target} is missing")
            elif (ROOT / source).read_bytes() != (ROOT / target).read_bytes():
                errors.append(f"{source} and {target} differ")
        for name in DOC_WORKFLOWS:
            command = ROOT / f".claude/commands/{name}.md"
            if not (primary / name / "SKILL.md").is_file():
                errors.append(f"{name} skill is missing")
            if not command.is_file() or command.read_text() != command_wrapper(name):
                errors.append(f"{name} command does not match its skill wrapper")
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
        # Both files hold the same rules, so a pointer names both, for whichever an agent reads.
        for base in (".claude/skills", "scripts", ".github", ".githooks"):
            for path in sorted((ROOT / base).rglob("*")):
                if not path.is_file() or path.suffix not in ("", ".md", ".py", ".sh", ".yaml") or path == Path(__file__).resolve():
                    continue
                for number, line in enumerate(path.read_text(errors="ignore").splitlines(), 1):
                    if ("AGENTS.md" in line) != ("CLAUDE.md" in line) and "write_text" not in line:
                        errors.append(f"{path.relative_to(ROOT)}:{number} names one of AGENTS.md and CLAUDE.md; name both")
        for error in errors:
            print(f"FAIL {error}")
        if not errors:
            print("PASS instruction files and skill trees match")
        return int(bool(errors))
    for source, target in INSTRUCTION_PAIRS:
        shutil.copyfile(ROOT / source, ROOT / target)
    for name in DOC_WORKFLOWS:
        command = ROOT / f".claude/commands/{name}.md"
        command.parent.mkdir(parents=True, exist_ok=True)
        command.write_text(command_wrapper(name))
    if mirror.is_symlink():
        mirror.unlink()
    elif mirror.exists():
        shutil.rmtree(mirror)
    shutil.copytree(primary, mirror)
    print("Synchronized AGENTS.md / CLAUDE.md pairs, skill trees and docs command wrappers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
