#!/usr/bin/env python3
"""Claude Code hooks and review stamps that keep source work on the AGENTS.md / CLAUDE.md workflow.

Usage:
    scripts/agent-hooks.py pre-edit     PreToolUse hook: point at the workflow before a source edit
    scripts/agent-hooks.py stop         Stop hook: refuse to finish while a changed source fails its
                                        rules or has no review of its current files
    scripts/agent-hooks.py stamp <id> <status>
                                        record that scripts/review-source.sh ran on <id>'s files
    scripts/agent-hooks.py checklist    print the section 3 rules to read against the diff
"""

import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SKIPPED = {"target", "package.aix"}


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True).stdout


def crate_of(path: str) -> str | None:
    """`sources/<id>` or `templates/<name>` for a path inside one."""
    parts = Path(path).parts
    if len(parts) > 2 and parts[0] in ("sources", "templates"):
        return f"{parts[0]}/{parts[1]}"
    return None


def changed_crates() -> list[str]:
    base = git("merge-base", "HEAD", "origin/main").strip() or "HEAD"
    names = git("diff", "--name-only", base, "--", "sources", "templates").split()
    names += git("ls-files", "--others", "--exclude-standard", "--", "sources", "templates").split()
    return sorted({crate for name in names if (crate := crate_of(name))})


def tree_hash(crate: str) -> str:
    """A hash of the crate's files as they are now, so any later edit invalidates a review."""
    digest = hashlib.sha256()
    folder = ROOT / crate
    for path in sorted(folder.rglob("*")):
        relative = path.relative_to(folder)
        if path.is_file() and not SKIPPED & set(relative.parts):
            digest.update(str(relative).encode() + b"\0" + path.read_bytes() + b"\0")
    return digest.hexdigest()


def stamp_path(crate: str) -> Path:
    git_dir = Path(git("rev-parse", "--absolute-git-dir").strip() or ROOT / ".git")
    return git_dir / "aidoku-review" / crate.replace("/", "__")


def read_stamp(crate: str) -> dict:
    try:
        return json.loads(stamp_path(crate).read_text())
    except (OSError, ValueError):
        return {}


def pre_edit(event: dict) -> None:
    try:
        relative = Path(event.get("tool_input", {}).get("file_path", "")).resolve().relative_to(ROOT)
    except ValueError:
        return
    crate = crate_of(str(relative))
    if not crate:
        return
    seen = Path(tempfile.gettempdir()) / "aidoku-hooks" / f"{event.get('session_id', 'session')}.json"
    reminded = json.loads(seen.read_text()) if seen.exists() else []
    if crate in reminded:
        return
    seen.parent.mkdir(parents=True, exist_ok=True)
    seen.write_text(json.dumps(reminded + [crate]))
    name = crate.split("/", 1)[1]
    review = f"`scripts/review-source.sh {name}`" if crate.startswith("sources/") else \
        "`scripts/review-source.sh` on each source that uses it"
    message = (
        f"You are editing {crate}. Follow the aidoku-source skill (.claude/skills/aidoku-source/SKILL.md; "
        "load it if it isn't loaded), compare with merged sources built the same way (AGENTS.md / CLAUDE.md section 6), "
        f"and before calling the work done run {review} and read AGENTS.md / CLAUDE.md section 3 against the diff row by row."
    )
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": message}}))


def stop(event: dict) -> None:
    crates = [crate for crate in changed_crates() if (ROOT / crate / "Cargo.toml").exists()]
    sources = [crate.split("/", 1)[1] for crate in crates if crate.startswith("sources/")]
    if not sources:
        return
    problems = []
    result = subprocess.run([sys.executable, str(ROOT / "scripts/check-sources.py"), *sources],
                            cwd=ROOT, capture_output=True, text=True)
    if result.returncode:
        problems.append("check-sources.py fails:\n" + result.stdout.strip())
    for name in sources:
        stamp = read_stamp(f"sources/{name}")
        if stamp.get("tree") != tree_hash(f"sources/{name}"):
            problems.append(f"sources/{name} changed since its last review: run scripts/review-source.sh {name}.")
        elif stamp.get("status") != "pass":
            problems.append(f"scripts/review-source.sh {name} last failed: fix it, or tell the user which "
                            "failure is the live site rather than the source.")
    if not problems:
        return
    if event.get("stop_hook_active"):
        # Already asked once this turn; report rather than loop.
        print("\n\n".join(problems), file=sys.stderr)
        return
    print(json.dumps({"decision": "block", "reason": "\n\n".join(problems)}))


def stamp(name: str, status: str) -> None:
    crate = f"sources/{name}"
    path = stamp_path(crate)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"tree": tree_hash(crate), "status": status}))


def checklist() -> None:
    text = (ROOT / "AGENTS.md").read_text()  # identical to CLAUDE.md
    section = text[text.index("## 3. Review feedback"):text.index("## 4. ")]
    rows = re.findall(r"^\| (?!What was flagged|---)(.+?) \| (.+?) \|", section, re.M)
    bullets = re.findall(r"^- \*\*(.+?)\*\*", section, re.M)
    print("Read each against the diff (AGENTS.md / CLAUDE.md section 3):")
    for flagged, rule in rows:
        print(f"- [ ] {rule}  ({flagged})")
    for bullet in bullets:
        print(f"- [ ] {bullet} (see section 3)")


def main() -> int:
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command in ("pre-edit", "stop"):
        # A hook that breaks must not stop edits or the end of a turn.
        try:
            event = json.loads(sys.stdin.read() or "{}")
            (pre_edit if command == "pre-edit" else stop)(event)
        except Exception as error:  # noqa: BLE001
            print(f"agent-hooks {command}: {error}", file=sys.stderr)
            if command == "stop":
                print(json.dumps({"decision": "block", "reason": f"Workflow validation could not run: {error}"}))
    elif command == "stamp" and len(sys.argv) == 4:
        stamp(sys.argv[2], sys.argv[3])
    elif command == "checklist":
        checklist()
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
