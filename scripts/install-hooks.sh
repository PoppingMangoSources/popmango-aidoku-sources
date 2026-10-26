#!/usr/bin/env bash
# Use an absolute path so the hook remains active in clean upstream worktrees.
set -euo pipefail
root=$(git rev-parse --show-toplevel)
test -f "$root/.githooks/pre-push"
git config core.hooksPath "$root/.githooks"
echo "Installed hooks from $root/.githooks; keep this tooling checkout available."
