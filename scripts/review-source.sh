#!/usr/bin/env bash
# Runs every review step for one source and records the result for the Stop hook.
# Usage: scripts/review-source.sh <id>
set -uo pipefail
id=${1:?usage: scripts/review-source.sh <id>}
root=$(git rev-parse --show-toplevel)
dir="$root/sources/$id"
[ -f "$dir/Cargo.toml" ] || { echo "no source at sources/$id" >&2; exit 2; }
status=pass
step() {
    echo
    echo "== $1"
    shift
    "$@" || status=fail
}

tests=()
[ -f "$dir/src/test.rs" ] && tests=(--test)
step "Gates: fmt, clippy, dead-code lints, package, verify${tests:+, live tests}" \
    "$root/scripts/verify-source.sh" "$dir" "${tests[@]}"
step "Portable template review contracts" python3 "$root/scripts/review-contracts.py" --repo "$root" --crate "sources/$id"
step "Source rules" python3 "$root/scripts/check-sources.py" --base origin/main "$id"
echo
echo "== Names few merged sources share (swap each for its usual equivalent, or say why not)"
python3 "$root/scripts/upstream-usage.py" "$id" || true

for branch in "feat/$id" "fix/$id"; do
    if git -C "$root" ls-remote --exit-code -q origin "refs/heads/$branch" >/dev/null; then
        git -C "$root" fetch -q origin "$branch"
        git -C "$root" fetch -q https://github.com/Aidoku-Community/sources.git main:refs/remotes/upstream/main
        step "Upstream scope of $branch" python3 "$root/scripts/check-upstream-scope.py" \
            --source "$id" --head "origin/$branch"
    fi
done

echo
python3 "$root/scripts/agent-hooks.py" checklist
python3 "$root/scripts/agent-hooks.py" stamp "$id" "$status"
echo
echo "review-source: $id $status"
[ "$status" = pass ]
