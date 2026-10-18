#!/usr/bin/env python3
"""Checks sources against the rules in AGENTS.md that need no network.

Usage:
    scripts/check-sources.py                  every source
    scripts/check-sources.py en.foo multi.bar  just those
    scripts/check-sources.py --base origin/main
        the sources changed since that ref, plus the version-bump rule

Exits non-zero when any rule fails. Each failure names the file and the rule.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = ROOT / "sources"

failures: list[str] = []


def fail(path: Path, message: str) -> None:
    failures.append(f"{path.relative_to(ROOT)}: {message}")


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def changed_sources(base: str) -> list[str]:
    names = git("diff", "--name-only", f"{base}...HEAD", "--", "sources").split()
    return sorted({Path(name).parts[1] for name in names if len(Path(name).parts) > 2})


def check_text_file(path: Path) -> None:
    data = path.read_bytes()
    if data and not data.endswith(b"\n"):
        fail(path, "does not end with a newline")


def check_json(path: Path) -> object:
    check_text_file(path)
    text = path.read_text()
    try:
        value = json.loads(text)
    except json.JSONDecodeError as error:
        fail(path, f"is not valid JSON ({error})")
        return None
    for number, line in enumerate(text.splitlines(), 1):
        if line.startswith(" "):
            fail(path, f"line {number} is indented with spaces; JSON is indented with tabs")
            break
    return value


def check_source_json(path: Path, source_id: str) -> dict:
    info = check_json(path)
    if not isinstance(info, dict):
        return {}
    meta = info.get("info", {})
    if meta.get("id") != source_id:
        fail(path, f"info.id is {meta.get('id')!r}, the folder is {source_id!r}")
    if "multi" in meta.get("languages", []):
        fail(path, 'languages lists "multi"; list each language instead')
    if info.get("config", {}).get("languageSelectType") == "multi":
        fail(path, 'languageSelectType "multi" makes the source fail to install; omit it')
    ids = [listing.get("id") for listing in info.get("listings", [])]
    if len(ids) != len(set(ids)):
        fail(path, "listing ids repeat")
    return info


def check_filters_json(path: Path) -> None:
    filters = check_json(path)
    if not isinstance(filters, list):
        return
    for item in filters:
        if not isinstance(item, dict):
            continue
        name = item.get("id") or item.get("title")
        if item.get("type") == "sort" and item.get("default") == {"index": 0, "ascending": False}:
            fail(path, f"sort {name!r} sets the default the framework already uses")
        if item.get("type") == "multi-select" and item.get("canExclude") is False:
            fail(path, f"multi-select {name!r} sets canExclude false, which is the default")
        if item.get("type") == "check" and not item.get("hideFromHeader"):
            fail(path, f"check {name!r} shows in the filters header; set hideFromHeader")


SECTION_BANNER = re.compile(r"^\s*//+\s*[-=*#~]{3,}")
PAGE_SIZE = re.compile(r"page_size:\s*Some\((\d+)\)")
CHAPTER_LANGUAGE = re.compile(r"\blanguage:\s*Some\(")


def check_rust(source: Path, single_language: bool) -> None:
    has_settings = (source / "src" / "settings.rs").exists()
    for path in sorted((source / "src").rglob("*.rs")):
        check_text_file(path)
        if path.name == "test.rs":
            continue
        for number, line in enumerate(path.read_text().splitlines(), 1):
            where = f"line {number}"
            if SECTION_BANNER.match(line):
                fail(path, f"{where} is a decorative divider")
            size = PAGE_SIZE.search(line)
            if size and int(size.group(1)) > 5:
                fail(path, f"{where} sets page_size {size.group(1)}; home components take 5 at most")
            if single_language and CHAPTER_LANGUAGE.search(line):
                fail(path, f"{where} sets a chapter language on a single-language source")
            if has_settings and path.name != "settings.rs" and "defaults_get" in line:
                fail(path, f"{where} reads a setting outside settings.rs")


def readme_version(source_id: str) -> int | None:
    readme = (ROOT / "README.md").read_text()
    found = set(re.findall(rf"sources/{re.escape(source_id)}-v(\d+)\.aix", readme))
    shown = set(re.findall(rf"{re.escape(source_id)}-v\d+\.aix\">\.aix \(v(\d+)\)", readme))
    if len(found | shown) > 1:
        fail(ROOT / "README.md", f"{source_id} links more than one version: {sorted(found | shown)}")
    return int(next(iter(found))) if found else None


def check_version_bump(source: Path, version: int, base: str) -> None:
    path = source / "res" / "source.json"
    try:
        before = json.loads(git("show", f"{base}:{path.relative_to(ROOT)}"))
    except subprocess.CalledProcessError:
        return
    if version <= before.get("info", {}).get("version", 0):
        fail(path, f"version {version} was not bumped from {base}")


def check_source(source_id: str, base: str | None) -> None:
    source = SOURCES / source_id
    if not (source / "Cargo.toml").exists():
        return
    for name in ("Cargo.toml", ".cargo/config.toml"):
        if not (source / name).exists():
            fail(source, f"has no {name}")
        else:
            check_text_file(source / name)
    info = check_source_json(source / "res" / "source.json", source_id)
    for name in ("filters.json",):
        if (source / "res" / name).exists():
            check_filters_json(source / "res" / name)
    if (source / "res" / "settings.json").exists():
        check_json(source / "res" / "settings.json")
    languages = info.get("info", {}).get("languages", [])
    check_rust(source, len(languages) == 1)

    version = info.get("info", {}).get("version")
    shown = readme_version(source_id)
    if shown is None:
        fail(ROOT / "README.md", f"has no row for {source_id}")
    elif shown != version:
        fail(ROOT / "README.md", f"{source_id} links v{shown}, source.json is v{version}")
    if base and isinstance(version, int):
        check_version_bump(source, version, base)


def main() -> int:
    args = sys.argv[1:]
    base = None
    if "--base" in args:
        index = args.index("--base")
        base = args[index + 1]
        del args[index : index + 2]
    names = args or (changed_sources(base) if base else sorted(p.name for p in SOURCES.iterdir()))

    agents, claude = ROOT / "AGENTS.md", ROOT / "CLAUDE.md"
    if claude.exists() and agents.read_bytes() != claude.read_bytes():
        fail(claude, "differs from AGENTS.md; keep the two identical")

    for name in names:
        check_source(name, base)

    for failure in failures:
        print(f"FAIL {failure}")
    print(f"checked {len(names)} source(s): {len(failures)} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
