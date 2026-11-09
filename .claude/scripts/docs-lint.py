#!/usr/bin/env python3
"""Compatibility entry point for the shared wiki linter."""

from pathlib import Path
import runpy

if __name__ == "__main__":
    runpy.run_path(str(Path(__file__).resolve().parents[2] / "scripts/docs-lint.py"), run_name="__main__")
