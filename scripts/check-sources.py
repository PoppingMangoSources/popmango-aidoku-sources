#!/usr/bin/env python3
"""Checks sources against the rules in AGENTS.md / CLAUDE.md that need no network.

Usage:
    scripts/check-sources.py                  every source
    scripts/check-sources.py en.foo multi.bar  just those
    scripts/check-sources.py --base origin/main
        the sources changed since that ref, plus the version-bump rule

Exits non-zero when any rule fails. Each failure names the file and the rule.
"""

import argparse
import json
import re
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = ROOT / "sources"

failures: list[str] = []
# Judgement calls merged sources disagree on: printed, never failing. Change each or say why not.
notes: list[str] = []


def fail(path: Path, message: str) -> None:
    failures.append(f"{path.relative_to(ROOT)}: {message}")


def note(path: Path, message: str) -> None:
    notes.append(f"{path.relative_to(ROOT)}: {message}")


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
        if re.match(r"^\t* +", line):
            fail(path, f"line {number} is indented with spaces; JSON is indented with tabs")
            break
    return value


def check_source_json(path: Path, source_id: str) -> dict:
    info = check_json(path)
    if not isinstance(info, dict):
        return {}
    meta = info.get("info", {})
    if "breakingChangeVersion" in meta:
        fail(path, "breakingChangeVersion belongs in the top-level config object, not info")
    if "minAppVersion" not in meta:
        fail(path, 'has no minAppVersion; "0.7.1" unless verify asks for more')
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
        if item.get("type") in ("multi-select", "select") and item.get("isGenre") and item.get("usesTagStyle") is True:
            fail(path, f"{name!r} sets usesTagStyle, which isGenre already implies")
        if item.get("type") == "check" and not item.get("hideFromHeader"):
            fail(path, f"check {name!r} shows in the filters header; set hideFromHeader")


def check_icon(path: Path) -> None:
    if not path.exists():
        fail(path, "is missing")
        return
    header = path.read_bytes()[:24]
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n":
        fail(path, "is not a PNG")
    elif struct.unpack(">II", header[16:24]) != (128, 128):
        fail(path, "is {}x{}; icons are 128x128".format(*struct.unpack(">II", header[16:24])))


SECTION_BANNER = re.compile(r"^\s*//+\s*[-=*#~]{3,}")
PAGE_SIZE = re.compile(r"page_size:\s*Some\((\d+)\)")
CHAPTER_LANGUAGE = re.compile(r"\blanguage:\s*Some\(")
# Rules from review comments on other contributors' PRs (AGENTS.md / CLAUDE.md section 3).
LINE_RULES = [
    (
        re.compile(r"!needs_details\s*&&\s*!needs_chapters|!needs_chapters\s*&&\s*!needs_details"),
        "guards against neither flag being set; get_manga_update is never called that way",
    ),
    (
        re.compile(r"\bquery[\w.()]*\.(?:filter|map)\(\|\w+\|\s*!\w+\.is_empty\(\)"),
        "checks for an empty query; the app passes None instead",
    ),
    (
        re.compile(r"AidokuError::(?:message|Message)\("),
        "builds an error by hand; use error! or bail!",
    ),
    (
        re.compile(r"AidokuError::Unimplemented"),
        "returns Unimplemented; leave the trait out of register_source! instead",
    ),
]


LINK_BLOCK = re.compile(r"\bLink\s*\{((?:[^{}]|\{[^{}]*\})*)\}")
LINK_SUBTITLE = re.compile(r"\bsubtitle\b(?!:\s*None)")
SEARCH_BODY = re.compile(r"fn get_search_manga_list\b(.*?)(?=\n\tfn |\n\}\n)", re.S)


def check_cards(path: Path, text: str) -> None:
    """Search results and home rows carry key, title, cover, status and rating only."""
    for block in LINK_BLOCK.finditer(text):
        if LINK_SUBTITLE.search(block.group(1)):
            line = text.count("\n", 0, block.start()) + 1
            fail(path, f"line {line} gives a home row a subtitle; only the big scroller shows details")
    body = SEARCH_BODY.search(text)
    if body and "get_manga_update(" in body.group(1):
        fail(path, "search returns a series' full details as a card; look it up compactly instead")


SILENCED = re.compile(r"#!?\[\s*(?:allow|expect)\s*\([^)]*\b(?:dead_code|unused\w*)")
FULL_PATH_CALL = re.compile(r"\baidoku::(?:imports|helpers|alloc)::[\w:]*::[a-z_][a-z0-9_]*\(")
URL_CONSTANT = re.compile(r'\bconst (\w+): &str = "(https?://[^"]+)";')
STRING_CONSTANT = re.compile(r'\bconst (\w+): &str =\s*"[^"]*";')
QUERY_VALUE = re.compile(r'\.push\(\s*"[^"]+",\s*Some\((\w+)\)\s*\)')
PAGE_SIZE_SENT = re.compile(
    r'push\(\s*"(?:limit|per_page|perPage|size|page_size|pageSize)",\s*Some\(&?([A-Z][A-Z0-9_]*)\b'
    r'|[?&](?:limit|per_page|perPage|size|page_size|pageSize)=\{([A-Z][A-Z0-9_]*)\}'
)
SINGLE_CALL_FN = re.compile(
    r"^(\t*)(?:pub(?:\([^)]*\))? )?fn (\w+)(?:<[^>]*>)?\(([^)]*)\)[^{;]*\{\n\t*([\w:]+)\(([^;{}]*)\)\n\1\}",
    re.M,
)


def crate_text(source: Path) -> str:
    return "\n".join(p.read_text() for p in sorted((source / "src").rglob("*.rs")) if p.name != "test.rs")


def check_naming(source: Path) -> None:
    """Constants and wrappers merged sources write differently (AGENTS.md / CLAUDE.md section 8)."""
    text = crate_text(source)
    # An import isn't a use: a helper imported once and called once is still used once.
    code = re.sub(r"^\s*(?:pub(?:\([^)]*\))? )?use [^;]*;", "", text, flags=re.M | re.S)
    uses = lambda name: len(re.findall(rf"\b{name}\b", code)) - 1
    urls = dict(URL_CONSTANT.findall(text))
    for name, value in urls.items():
        base = next((other for other, prefix in urls.items() if other != name and value.startswith(prefix + "/") and value != prefix + "/"), None)
        endpoint = uses(name) and len(re.findall(rf"\{{{name}\}}\?", text)) == uses(name)
        if base and endpoint:
            fail(source, f"{name} is {base} plus a path, used only with a query; format it inline from {base}")
    strings = set(STRING_CONSTANT.findall(text))
    for name in QUERY_VALUE.findall(text):
        if name in strings and uses(name) == 1:
            fail(source, f"{name} is sent once as a query value; write the value where it is sent")
    for groups in PAGE_SIZE_SENT.findall(text):
        name = groups[0] or groups[1]
        if not name.endswith("SIZE"):
            note(source, f"{name} is sent as a page size; most merged sources call it PAGE_SIZE or <THING>_PAGE_SIZE")
    for path in sorted((source / "src").rglob("*.rs")):
        if path.name in ("test.rs", "settings.rs"):
            continue
        file_text = path.read_text()
        for match in SINGLE_CALL_FN.finditer(file_text):
            indent, name, params, callee, args = match.groups()
            before = file_text[: match.start()]
            impl = re.findall(rf"^{indent[:-1]}impl\b[^\n]*", before, re.M) if indent else []
            if (impl and " for " in impl[-1]) or callee[0].isupper() or name == "new":
                continue  # Trait methods, constructors and Ok(..) bodies aren't wrappers.
            line = before.count("\n") + 1
            own_fields = args and all(a.strip().startswith("self.") for a in args.split(","))
            if "self" in params and "." not in callee and own_fields:
                note(path, f"line {line} fn {name} only passes its own fields to {callee}; consider calling {callee} directly")
            elif uses(name) == 1:
                note(path, f"line {line} fn {name} only calls {callee} and is used once; consider inlining it")


def check_rust(source: Path, single_language: bool) -> None:
    has_settings = (source / "src" / "settings.rs").exists()
    for path in sorted((source / "src").rglob("*.rs")):
        check_text_file(path)
        lines = path.read_text().splitlines()
        for number, line in enumerate(lines, 1):
            # A justified exception, such as a field serde needs to tell untagged variants
            # apart, carries a comment on the line above.
            justified = number > 1 and lines[number - 2].strip().startswith("//")
            if SILENCED.search(line) and not justified:
                fail(path, f"line {number} silences an unused-code warning; delete the code, or comment why it must stay")
        if path.name == "test.rs":
            continue
        check_cards(path, path.read_text())
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
            call = FULL_PATH_CALL.search(line)
            if call and not line.lstrip().startswith(("use ", "pub use ", "//")):
                note(path, f"{where} calls {call.group(0)[:-1]} by its full path; import it")
            for pattern, message in LINE_RULES:
                if pattern.search(line):
                    fail(path, f"{where} {message}")


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


RELEASE_PROFILE = ('panic = "abort"', 'opt-level = "s"', "strip = true", "lto = true")
AIDOKU_TEST = re.compile(r"#\[aidoku_test\]\s*fn (\w+)\(\)[^{]*\{(.*?)\n\}", re.S)


def check_cargo(path: Path) -> None:
    """Every upstream source builds with the same release profile."""
    text = path.read_text()
    release = text.split("[profile.release]", 1)[1] if "[profile.release]" in text else ""
    missing = [line for line in RELEASE_PROFILE if line not in release]
    if missing:
        fail(path, f"[profile.release] lacks {', '.join(missing)}, which every upstream source sets")


def check_tests(source: Path) -> None:
    """A test that asserts nothing passes whatever the source does."""
    for path in sorted((source / "src").rglob("*.rs")):
        for test in AIDOKU_TEST.finditer(path.read_text()):
            if not re.search(r"\bassert|\.unwrap(?:_err)?\(|\.expect\(", test.group(2)):
                fail(path, f"test {test.group(1)} asserts nothing")


def check_source(source_id: str, base: str | None, upstream: bool = False) -> None:
    source = SOURCES / source_id
    if not (source / "Cargo.toml").exists():
        return
    for name in ("Cargo.toml", ".cargo/config.toml"):
        if not (source / name).exists():
            fail(source, f"has no {name}")
        else:
            check_text_file(source / name)
    check_cargo(source / "Cargo.toml")
    check_tests(source)
    info = check_source_json(source / "res" / "source.json", source_id)
    check_icon(source / "res" / "icon.png")
    for name in ("filters.json",):
        if (source / "res" / name).exists():
            check_filters_json(source / "res" / name)
    if (source / "res" / "settings.json").exists():
        check_json(source / "res" / "settings.json")
    languages = info.get("info", {}).get("languages", [])
    check_rust(source, len(languages) == 1)
    check_naming(source)

    version = info.get("info", {}).get("version")
    if upstream:
        if base and isinstance(version, int):
            check_version_bump(source, version, base)
        return
    shown = readme_version(source_id)
    if shown is None:
        fail(ROOT / "README.md", f"has no row for {source_id}")
    elif shown != version:
        fail(ROOT / "README.md", f"{source_id} links v{shown}, source.json is v{version}")
    if base and isinstance(version, int):
        check_version_bump(source, version, base)


def main() -> int:
    global ROOT, SOURCES
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base")
    parser.add_argument("--repo", type=Path, default=ROOT)
    parser.add_argument("--upstream", action="store_true", help="Source rules only; no fork README or agent files")
    parser.add_argument("sources", nargs="*")
    args = parser.parse_args()
    ROOT = args.repo.resolve()
    SOURCES = ROOT / "sources"
    names = args.sources or (changed_sources(args.base) if args.base else sorted(p.name for p in SOURCES.iterdir()))
    if not args.upstream:
        result = subprocess.run([sys.executable, str(ROOT / "scripts/sync-agent-docs.py"), "--check"], capture_output=True, text=True)
        if result.returncode:
            failures.append(result.stdout.strip() or result.stderr.strip())
    for name in names:
        check_source(name, args.base, args.upstream)

    for item in notes:
        print(f"NOTE {item}")
    for failure in failures:
        print(f"FAIL {failure}")
    print(f"checked {len(names)} source(s): {len(failures)} failure(s), {len(notes)} note(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
