#!/usr/bin/env python3
"""Shows how merged upstream sources do what a source here does.

For every feature the source uses (deduplication, partial results, home components, date
parsing, web views and the rest of FEATURES), lists the merged sources that implement the same
thing, newest-changed first, with the file and line to read; recent rewrites are the current
standard (AGENTS.md / CLAUDE.md section 6). Then, for every aidoku-rs item and every name the
source defines, counts the upstream sources sharing it: a name no merged source uses either has
a reason in the site or a usual equivalent. A report, not a gate.

Usage: scripts/upstream-usage.py <id> [--rare N]   (N: list items used by at most N sources)
       scripts/upstream-usage.py --feature <regex>   (who upstream does what you are about to add)
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REF = "upstream/main"
DEFINITION = r"\b(?:fn|const|struct|enum|static|trait|type)\s+{}\b"
# What merged sources call the same thing, from surveys of all of them (AGENTS.md / CLAUDE.md
# section 8). Printed beside a rare name that matches, so the swap is a decision, not a search.
EQUIVALENTS = [
    (r"_(?:LIMIT|PER_PAGE|COUNT)$", "a page size is PAGE_SIZE (9 sources) or <THING>_PAGE_SIZE"),
    (r"^(?:SEARCH|QUERY|LIST|DETAIL|CHAPTERS?)_(?:URL|ENDPOINT|PATH)$",
     "endpoints are formatted inline from BASE_URL or API_URL (11 sources)"),
    (r"^RATINGS$", "the full rating list is CONTENT_RATINGS (multi.xcomic)"),
    (r"^(?:FILTERS_URL|FILTER_URL)$", "fetch the filter list in a function, as en.toonkotsu's listed_facets"),
]
# Features worth comparing with merged code before writing them: the SDK surface and the
# patterns reviews keep asking about.
FEATURES = [
    ("Chapter deduplication", r"(?i)dedup(?:licat)\w*_?chapters|deduplicat"),
    ("Partial details while chapters load", r"send_partial_result\(&manga"),
    ("Partial home layout", r"HomePartialResult::Layout"),
    ("Big scroller", r"HomeComponentValue::BigScroller"),
    ("Chapter list row", r"HomeComponentValue::MangaChapterList"),
    ("Image scroller (banners)", r"HomeComponentValue::ImageScroller"),
    ("Parallel requests", r"Request::send_all"),
    ("Deep links", r"impl\b.*\bDeepLinkHandler for"),
    ("Listings", r"impl\b.*\bListingProvider for"),
    ("Dynamic filters", r"impl\b.*\bDynamicFilters for"),
    ("Dynamic settings", r"impl\b.*\bDynamicSettings for"),
    ("Image request headers", r"impl\b.*\bImageRequestProvider for"),
    ("Page image processing", r"impl\b.*\bPageImageProcessor for"),
    ("Base url / mirrors", r"impl\b.*\bBaseUrlProvider for|defaults_get::<String>\(\"url\"\)"),
    ("Web view", r"\bWebView::"),
    ("Rate limit", r"set_rate_limit"),
    ("Date parsing", r"parse_date(?:_with_options)?\("),
    ("Relative dates (\"2 hours ago\")", r"(?i)\"(?:\w+ )?ago\"|strip_suffix\(\" ago\"\)|hours? ago"),
    ("Status mapping", r"MangaStatus::(?:Ongoing|Completed)"),
    ("Content rating from tags", r"ContentRating::(?:NSFW|Suggestive)"),
    ("Viewer from type", r"Viewer::(?:Webtoon|RightToLeft)"),
    ("Markdown descriptions", r"escape_markdown\(\)"),
    ("Novel text pages", r"PageContent::text\("),
    ("Locked chapters", r"locked:\s*(?:true|[a-z])"),
    ("Settings reads", r"defaults_get::<"),
    ("JSON API models", r"json_owned::<|get_json_owned::<"),
    ("Next.js page data", r"__NEXT_DATA__|\"RSC\"|self\.__next_f"),
    ("GraphQL", r"(?i)graphql|\"query\":"),
    ("HTML scraping", r"\.select_first\("),
    ("Response status checks", r"status_code\(\)"),
    ("Query string builder", r"QueryParameters::new\(\)"),
    ("Tag search (\"genre\" select)", r"FilterValue::Select\s*\{[^}]*\}\s*if id == \"genre\"|\"genre\" =>"),
    ("Cookies", r"\"Cookie\"|get_cookies|set_cookie"),
    ("Encryption / hashing", r"(?i)aes|sha2|hmac|md5"),
    ("Regex", r"Regex::new\("),
    ("Template", r"register_source!\(\w+<"),
]


def git(*args: str, data: bytes | None = None) -> bytes:
    return subprocess.run(["git", "-C", str(ROOT), *args], input=data, capture_output=True, check=True).stdout


def upstream_files() -> dict[str, str]:
    """Every Rust file under upstream's sources, read through one `git cat-file --batch`."""
    names = [n for n in git("ls-tree", "-r", "--name-only", REF, "sources").decode().splitlines()
             if n.endswith(".rs") and not n.endswith("/test.rs")]
    output = git("cat-file", "--batch", data="".join(f"{REF}:{n}\n" for n in names).encode())
    files, offset = {}, 0
    for name in names:
        end = output.index(b"\n", offset)
        size = int(output[offset:end].split()[2])
        files[name] = output[end + 1:end + 1 + size].decode(errors="ignore")
        offset = end + 1 + size + 1
    return files


def last_touched() -> dict[str, str]:
    dates, current = {}, ""
    for line in git("log", "--format=%x00%cs", "--name-only", REF, "--", "sources").decode().splitlines():
        if line.startswith("\0"):
            current = line[1:]
        elif line.startswith("sources/"):
            dates.setdefault(line.split("/")[1], current)
    return dates


def source_of(path: str) -> str:
    return path.split("/")[1]


def usage(files: dict[str, str], pattern: str) -> dict[str, str]:
    regex, hits = re.compile(pattern), {}
    for path, text in files.items():
        if source_of(path) in hits:
            continue
        for number, line in enumerate(text.splitlines(), 1):
            if regex.search(line):
                hits[source_of(path)] = f"{path}:{number}"
                break
    return hits


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("source", nargs="?")
    parser.add_argument("--rare", type=int, default=2)
    parser.add_argument("--feature", help="regex: list merged sources that already do this")
    args = parser.parse_args()
    if not args.source and not args.feature:
        parser.error("give a source id or --feature <regex>")

    git("fetch", "-q", "https://github.com/Aidoku-Community/sources.git", f"main:refs/remotes/{REF}")
    files = upstream_files()
    touched = last_touched()

    def newest(hits: dict[str, str]) -> list[str]:
        order = sorted(hits, key=lambda source: touched.get(source, ""), reverse=True)
        return [f"{hits[source]} ({touched.get(source, '?')})" for source in order[:5]]

    print(f"Upstream {REF} ({git('log', '-1', '--format=%h %cs', REF).decode().strip()}), newest-changed sources first.")
    if args.feature:
        for line in newest(usage(files, args.feature)):
            print(f"  {line}")
        return 0

    paths = sorted((ROOT / "sources" / args.source).glob("src/**/*.rs"))
    ours = "\n".join(p.read_text(errors="ignore") for p in paths if p.name != "test.rs")
    if not ours:
        sys.exit(f"no Rust in sources/{args.source}")
    # A template consumer does what its template does.
    for template in re.findall(r'path\s*=\s*"\.\./\.\./templates/([\w-]+)"',
                               (ROOT / "sources" / args.source / "Cargo.toml").read_text()):
        ours += "\n".join(p.read_text(errors="ignore") for p in sorted((ROOT / "templates" / template).glob("src/**/*.rs"))
                          if p.name != "test.rs")
    if any(source_of(path) == args.source for path in files):
        print(f"\n{args.source} exists upstream. Start from it: git diff {REF} HEAD -- sources/{args.source}")

    print(f"\nFeatures {args.source} uses, and merged sources that do the same (read the newest before writing yours):")
    for label, pattern in FEATURES:
        if not re.search(pattern, ours):
            continue
        hits = usage({p: t for p, t in files.items() if source_of(p) != args.source}, pattern)
        print(f"- {label}: {len(hits)} merged" + ("" if hits else "; no merged source does this, so say why in the PR"))
        for line in newest(hits)[:3]:
            print(f"    {line}")

    upstream = {}
    for path, text in files.items():
        if source_of(path) != args.source:
            upstream[source_of(path)] = upstream.get(source_of(path), "") + "\n" + text
    imported = set()
    for block in re.findall(r"use aidoku::\{(.*?)\};", ours, re.S) + re.findall(r"use aidoku::([\w:]+);", ours):
        imported |= set(re.findall(r"\b[A-Za-z_]\w+\b", block))
    defined = set(re.findall(r"\b(?:fn|const|struct|enum|static|trait|type)\s+([A-Za-z_]\w*)", ours))
    imported -= defined | {"alloc", "imports", "helpers", "prelude", "std", "net", "defaults", "uri", "string", "self"}

    def report(title: str, names: set[str], pattern: str) -> None:
        rows = []
        for name in sorted(names):
            hits = [s for s, text in upstream.items() if re.search(pattern.format(re.escape(name)), text)]
            if len(hits) <= args.rare:
                usual = next((advice for rule, advice in EQUIVALENTS if re.search(rule, name)), "")
                rows.append(f"{len(hits):4}  {name:28} {' '.join(hits[:4])}" + (f"  -> {usual}" if usual else ""))
        print(f"\n{title} used by at most {args.rare} of {len(upstream)} upstream sources:")
        print("\n".join(rows) if rows else "     none")

    report("aidoku items", imported, r"\b{}\b")
    report("Names defined", defined, DEFINITION)
    return 0


if __name__ == "__main__":
    sys.exit(main())
