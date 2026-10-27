#!/usr/bin/env bash
# Runs check-sources.py over every merged upstream source and counts the failures per rule.
# Run it when adding or changing a rule: a rule that fails code kkantan merged recently needs
# evidence it is still the reviewer's position (AGENTS.md / CLAUDE.md section 9) before it ships.
# Usage: scripts/calibrate-rules.sh [rule text to list the sources it fails]
set -euo pipefail
root=$(git rev-parse --show-toplevel)
git -C "$root" fetch -q https://github.com/Aidoku-Community/sources.git main:refs/remotes/upstream/main
work=$(mktemp -d)
trap 'git -C "$root" worktree remove --force "$work" >/dev/null 2>&1 || true' EXIT
git -C "$root" worktree add -q --detach "$work" upstream/main
mapfile -t ids < <(find "$work/sources" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort)
report=$(python3 "$root/scripts/check-sources.py" --repo "$work" --upstream "${ids[@]}" 2>&1 || true)
if [ $# -gt 0 ]; then
    grep -F -- "$1" <<< "$report" || echo "No upstream source fails that rule."
    exit 0
fi
grep '^FAIL' <<< "$report" \
    | sed -E 's/^FAIL [^:]+: //; s/line [0-9]+ //; s/'"'"'[^'"'"']*'"'"'/X/g; s/[0-9]+x[0-9]+/NxN/; s/page_size [0-9]+/page_size N/' \
    | sort | uniq -c | sort -rn
tail -1 <<< "$report"
