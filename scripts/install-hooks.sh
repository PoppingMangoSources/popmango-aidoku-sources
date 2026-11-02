#!/usr/bin/env bash
# Use an absolute path so the hook remains active in clean upstream worktrees.
set -euo pipefail
root=$(git rev-parse --show-toplevel)
test -f "$root/.githooks/pre-push"
chmod +x "$root/.githooks/pre-push" "$root/.githooks/commit-msg"
git config --local core.hooksPath "$root/.githooks"
git config --local user.name "Popmango Extensions"
git config --local user.email "297959108+PoppingMangoSources@users.noreply.github.com"
git config --local commit.gpgsign false
echo "Installed hooks from $root/.githooks; keep this tooling checkout available."
