#!/usr/bin/env python3
"""Run live suites and retain failures with HTTP evidence in the CI summary."""

import argparse
import os
from pathlib import Path
import re
import subprocess


def http_evidence(log):
    evidence = []
    if re.search(r"^URL: .*; HTTP 403;.*Just a moment", log, re.M):
        evidence.append("Cloudflare 403 challenge observed")
    if re.search(r"^URL: .*; HTTP 522;", log, re.M):
        evidence.append("HTTP 522 origin timeout observed")
    return "; ".join(evidence) or "See log"


def run(crates, output):
    output.mkdir(parents=True, exist_ok=True)
    rows = []
    failed = False
    for crate in crates:
        if not re.fullmatch(r"(?:sources|templates)/[A-Za-z0-9_.-]+", crate) or not (Path(crate) / "Cargo.toml").is_file():
            raise ValueError(f"Unknown crate: {crate}")
        print(f"::group::Testing {crate}", flush=True)
        log = output / (crate.replace("/", "_") + ".log")
        with log.open("w") as stream:
            result = subprocess.run(["cargo", "test", "--", "--skip", "review_"], cwd=crate,
                                    stdout=stream, stderr=subprocess.STDOUT)
        text = log.read_text(errors="replace")
        print(text, end="", flush=True)
        print("::endgroup::", flush=True)
        failed |= result.returncode != 0
        rows.append(f"| `{crate}` | {'Failed' if result.returncode else 'Passed'} | {http_evidence(text) if result.returncode else ''} |")
        if result.returncode:
            print(f"::error title=Tests failed::{crate}")
    summary = "### Live source tests\n\n| Crate | Result | HTTP evidence |\n|---|---|---|\n" + "\n".join(rows) + "\n"
    (output / "summary.md").write_text(summary)
    if path := os.environ.get("GITHUB_STEP_SUMMARY"):
        with Path(path).open("a") as stream:
            stream.write(summary)
    return int(failed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crates", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        return run(args.crates.read_text().splitlines(), args.output)
    except (OSError, ValueError) as error:
        parser.exit(1, f"Source tests failed: {error}\n")


if __name__ == "__main__":
    raise SystemExit(main())
