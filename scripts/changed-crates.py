#!/usr/bin/env python3
"""List existing changed crates and the source consumers of changed templates."""

import argparse
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--sources-only", action="store_true")
    args = parser.parse_args()
    names = subprocess.check_output(["git", "diff", "--name-only", "--no-renames", args.base + "...HEAD"], text=True).splitlines()
    crates = {"/".join(name.split("/")[:2]) for name in names if name.startswith(("sources/", "templates/")) and len(name.split("/")) > 2}
    templates = {Path(crate).name for crate in crates if crate.startswith("templates/")}
    for manifest in Path("sources").glob("*/Cargo.toml"):
        if any(f"templates/{template}" in manifest.read_text() for template in templates):
            crates.add(str(manifest.parent))
    for crate in sorted(crates):
        if (Path(crate) / "Cargo.toml").is_file() and (not args.sources_only or crate.startswith("sources/")):
            print(crate)


if __name__ == "__main__":
    main()
