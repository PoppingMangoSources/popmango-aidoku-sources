#!/usr/bin/env python3
"""Claude Code/Codex hooks and review stamps for the AGENTS.md / CLAUDE.md workflow.

Usage:
    scripts/agent-hooks.py pre-edit     PreToolUse hook: point at the workflow before a source edit,
                                        and print the home rules before a home edit
    scripts/agent-hooks.py pre-bash     PreToolUse hook: refuse a git commit outside scripts/commit.sh,
                                        and print the commit rules or section 3 before a commit or push
    scripts/agent-hooks.py digest       SessionStart hook after compaction: print the hard rules
    scripts/agent-hooks.py stop         Stop hook: refuse to finish while a changed source fails its
                                        rules or has no review of its current files
    scripts/agent-hooks.py stamp <id> <status>
                                        record that scripts/review-source.sh ran on <id>'s files
    scripts/agent-hooks.py checklist [--compact]
                                        print the section 3 rules to read against the diff
"""

import contextlib
import hashlib
import io
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


def edited_files(event: dict) -> list[tuple[str, str]]:
    tool_input = event.get("tool_input", {})
    if event.get("tool_name") == "apply_patch":
        patch = tool_input.get("command", "")
        headers = list(re.finditer(r"^\*\*\* (?:Add File|Update File|Delete File|Move to): (.+)$", patch, re.M))
        return [(match[1], patch[match.end():headers[i + 1].start() if i + 1 < len(headers) else len(patch)])
                for i, match in enumerate(headers)]
    path = tool_input.get("file_path")
    return [(path, tool_input.get("new_string", "") + tool_input.get("content", ""))] if path else []


def pre_edit(event: dict) -> None:
    messages = []
    for path, written in edited_files(event):
        try:
            relative = (Path(event.get("cwd") or ROOT) / path).resolve().relative_to(ROOT)
        except ValueError:
            continue
        crate = crate_of(str(relative))
        if not crate:
            continue
        if remind_once(event, crate):
            name = crate.split("/", 1)[1]
            review = f"`scripts/review-source.sh {name}`" if crate.startswith("sources/") else \
                "`scripts/review-source.sh` on each source that uses it"
            messages.append(
                f"You are editing {crate}. Follow the aidoku-source skill "
                "(.agents/skills/aidoku-source/SKILL.md in Codex, .claude/skills/aidoku-source/SKILL.md in Claude; "
                "load it if it isn't loaded), compare with merged sources built the same way (AGENTS.md / CLAUDE.md section 6), "
                f"and before calling the work done run {review} and read AGENTS.md / CLAUDE.md section 3 against the diff row by row."
            )
        if ("HomeComponent" in written or relative.name == "home.rs") and remind_once(event, f"{crate}#home"):
            messages.append(section("### Home components", "### Global mutable state"))
    if messages:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                                 "additionalContext": "\n\n".join(messages)}}))


def section(heading: str, end: str) -> str:
    text = (ROOT / "AGENTS.md").read_text()  # identical to CLAUDE.md
    start = text.index(heading)
    return text[start:text.index(end, start + len(heading))].strip()


GIT_COMMIT = re.compile(r"(?:^|[\s;&|(`])git(?:\s+-[cC]\s+\S+)*\s+commit(?![\w-])", re.M)
SCRIPT_PATH = re.compile(r"(?:^|\s)((?:/|\.{0,2}/?)[\w./-]*\.sh)\b")
TRUSTED = ("scripts", ".githooks")


def commands_run(command: str) -> list[tuple[str, str]]:
    """The command, plus each shell script it runs from outside the trusted tooling folders."""
    found = [("the command", command)]
    for name in SCRIPT_PATH.findall(command):
        path = (ROOT / name).resolve()
        try:
            trusted = path.relative_to(ROOT).parts[0] in TRUSTED
        except ValueError:
            trusted = False
        if path.is_file() and not trusted:
            found.append((str(path), path.read_text(errors="replace")))
    return found


def remind_once(event: dict, key: str) -> bool:
    seen = Path(tempfile.gettempdir()) / "aidoku-hooks" / f"{event.get('session_id', 'session')}.json"
    reminded = json.loads(seen.read_text()) if seen.exists() else []
    if key in reminded:
        return False
    seen.parent.mkdir(parents=True, exist_ok=True)
    seen.write_text(json.dumps(reminded + [key]))
    return True


def pre_bash(event: dict) -> None:
    command = event.get("tool_input", {}).get("command", "")
    commits = [where for where, text in commands_run(command) if GIT_COMMIT.search(text)]
    if commits:
        reason = (
            f"Refused: {', '.join(commits)} runs git commit directly. Commit with "
            "scripts/commit.sh -m \"<subject>\" -m \"<body>\" instead.\n\n"
            + section("## 0. Hard rules", "\n---")
        )
        print(json.dumps({"hookSpecificOutput": {
            "hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": reason,
        }}))
        return
    context = []
    if re.search(r"(?:^|[\s;&|(])(?:\./)?scripts/commit\.sh\s+-", command):
        rules = section("## 9. This repo's workflow", "### How a rule")
        context.append("Commit rules (AGENTS.md / CLAUDE.md section 9):\n" +
                       "\n".join(b for b in re.split(r"\n(?=- )", rules) if "ommit" in b.split("\n", 1)[0]))
    pushes_source = re.search(r"\bgit\s+push\b", command) and changed_crates()
    if pushes_source or re.search(r"pr-branch\.sh\s", command):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            checklist(compact=True)
        context.append(
            "Before this push: list each item the person asked for and what shows it, then read "
            "every rule below against the diff (full rows: scripts/agent-hooks.py checklist).\n" + out.getvalue()
        )
    if context:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                                 "additionalContext": "\n\n".join(context)}}))


def digest() -> None:
    """Printed after a compaction, which otherwise keeps only a pointer to AGENTS.md / CLAUDE.md."""
    print(section("## 0. Hard rules", "\n---"))
    print("\nRe-read the AGENTS.md / CLAUDE.md sections for the work in progress before continuing, "
          "and the aidoku-source skill before a source edit.")


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


def checklist(compact: bool = False) -> None:
    """Section 3's rules; compact keeps each rule's first sentence, so a hook's output stays inline."""
    text = (ROOT / "AGENTS.md").read_text()  # identical to CLAUDE.md
    section = text[text.index("## 3. Review feedback"):text.index("## 4. ")]
    rows = re.findall(r"^\| (?!What was flagged|---)(.+?) \| (.+?) \|", section, re.M)
    bullets = re.findall(r"^- \*\*(.+?)\*\*", section, re.M)
    print("Read each against the diff (AGENTS.md / CLAUDE.md section 3):")
    for flagged, rule in rows:
        if compact:
            print(f"- [ ] {re.split(r'(?<=[.;])\s', rule, maxsplit=1)[0]}")
        else:
            print(f"- [ ] {rule}  ({flagged})")
    for bullet in bullets:
        print(f"- [ ] {bullet} (see section 3)")


def main() -> int:
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    hooks = {"pre-edit": pre_edit, "pre-bash": pre_bash, "stop": stop}
    if command in hooks:
        # A hook that breaks must not stop edits, commands or the end of a turn.
        try:
            event = json.loads(sys.stdin.read() or "{}")
            hooks[command](event)
        except Exception as error:  # noqa: BLE001
            print(f"agent-hooks {command}: {error}", file=sys.stderr)
            if command == "stop":
                print(json.dumps({"decision": "block", "reason": f"Workflow validation could not run: {error}"}))
    elif command == "stamp" and len(sys.argv) == 4:
        stamp(sys.argv[2], sys.argv[3])
    elif command == "checklist":
        checklist(compact="--compact" in sys.argv)
    elif command == "digest":
        digest()
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
