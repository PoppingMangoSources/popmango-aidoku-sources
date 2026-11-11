#!/usr/bin/env python3
"""Run portable review regressions against a candidate template's public API."""

import argparse
import http.server
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile
import threading
import tomllib

ROOT = Path(__file__).resolve().parent.parent
CONTRACTS = ROOT / "scripts/review-contracts"
TEMPLATES = {"keyoapp", "mangathemesia"}


def select_templates(repo, crates):
    selected = set()
    for crate in crates:
        path = repo / crate
        if not (path / "Cargo.toml").is_file():
            continue
        if crate.startswith("templates/") and path.name in TEMPLATES:
            selected.add(path.name)
        manifest = tomllib.loads((path / "Cargo.toml").read_text())
        for name, dependency in manifest.get("dependencies", {}).items():
            if isinstance(dependency, dict) and "path" in dependency:
                template = Path(dependency["path"]).name
                if template in TEMPLATES:
                    selected.add(template)
    return sorted(selected)


def run_contract(repo, template):
    candidate = repo / "templates" / template
    manifest = tomllib.loads((candidate / "Cargo.toml").read_text())
    sdk = manifest["dependencies"]["aidoku"]
    locator = ", ".join(f"{key} = {json.dumps(sdk[key])}" for key in ("git", "rev", "branch", "tag") if key in sdk)
    if not locator:
        raise ValueError(f"{candidate}: expected an explicit aidoku Git dependency")
    handler = runpy.run_path(str(CONTRACTS / "fixtures.py"))["Responses"]
    with tempfile.TemporaryDirectory(prefix="aidoku-review-") as directory, \
            http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        harness = Path(directory)
        (harness / "src").mkdir()
        shutil.copy2(CONTRACTS / f"{template}.rs", harness / "src/lib.rs")
        if (candidate / "Cargo.lock").exists():
            shutil.copy2(candidate / "Cargo.lock", harness / "Cargo.lock")
        (harness / "Cargo.toml").write_text(
            '[package]\nname = "review-contract"\nversion = "0.1.0"\nedition = "2024"\n\n'
            '[dependencies]\n'
            f'{template} = {{ path = {json.dumps(str(candidate))} }}\n'
            f'aidoku = {{ {locator}, features = ["test"] }}\n'
            f'aidoku-test = {{ {locator} }}\n'
        )
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        env = {**os.environ, "AIDOKU_REVIEW_BASE_URL": f"http://127.0.0.1:{server.server_port}",
               "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER": "aidoku-test-runner",
               "CARGO_TARGET_DIR": str(candidate / "target/review-contracts"), "NO_PROXY": "127.0.0.1"}
        env["RUSTFLAGS"] = (env.get("RUSTFLAGS", "") + " -Dwarnings").strip()
        for name in subprocess.check_output(["git", "rev-parse", "--local-env-vars"], cwd=repo, text=True).splitlines():
            env.pop(name, None)
        try:
            print(f"Review contracts: {template} ({candidate})", flush=True)
            result = subprocess.run(["cargo", "test", "--manifest-path", str(harness / "Cargo.toml"),
                                     "--target", "wasm32-unknown-unknown", "--lib", "--", "--nocapture"], env=env)
        finally:
            server.shutdown()
            thread.join()
        return result.returncode


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--template", action="append", choices=sorted(TEMPLATES))
    parser.add_argument("--base")
    parser.add_argument("--crate", action="append")
    parser.add_argument("--crates-file", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()
    if args.template:
        selected = args.template
    elif args.crate:
        selected = select_templates(repo, args.crate)
    elif args.crates_file:
        selected = select_templates(repo, args.crates_file.read_text().splitlines())
    elif args.base:
        crates = subprocess.check_output([sys.executable, str(ROOT / "scripts/changed-crates.py"),
                                          "--base", args.base], cwd=repo, text=True).splitlines()
        paths = subprocess.check_output(["git", "diff", "--name-only", f"{args.base}...HEAD"], cwd=repo, text=True).splitlines()
        if any(path == "scripts/review-contracts.py" or path.startswith("scripts/review-contracts/") for path in paths):
            crates += [f"templates/{name}" for name in TEMPLATES]
        selected = select_templates(repo, crates)
    else:
        selected = [name for name in sorted(TEMPLATES) if (repo / "templates" / name / "Cargo.toml").is_file()]
    try:
        failed = [name for name in selected if run_contract(repo, name)]
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Review contracts failed: {error}\n")
    print(f"Review contracts: {len(selected)} template(s), {len(failed)} failure(s)")
    return bool(failed)


if __name__ == "__main__":
    raise SystemExit(main())
