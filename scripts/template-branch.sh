#!/usr/bin/env bash
# Prepare a template submission from a committed snapshot: one commit for the template, then one
# per source that uses it, each without fork tests and at version 1. Run it as
# scripts/pr-branch.sh --template <name> <id>... --body "<template>" --body "<id>"... [--from <ref>] [--push]
set -euo pipefail
root=$(git rev-parse --show-toplevel)
tooling=$(cd "$(dirname "$0")/.." && pwd)
source "$tooling/scripts/toolchain.sh"
name=${1:?provide a template name}
shift
snapshot=HEAD
push=false
ids=()
bodies=()
while [ $# -gt 0 ]; do
    case $1 in
        --from) snapshot=${2:?provide a ref}; shift 2 ;;
        --body) bodies+=("${2:?provide a body}"); shift 2 ;;
        --push) push=true; shift ;;
        -*) echo "unknown option: $1" >&2; exit 1 ;;
        *) ids+=("$1"); shift ;;
    esac
done
[[ "$name" =~ ^[a-z0-9_]+$ ]] || { echo "invalid template name" >&2; exit 1; }
[ ${#ids[@]} -gt 0 ] || { echo "list the sources that use the template" >&2; exit 1; }
[ ${#bodies[@]} -eq $((${#ids[@]} + 1)) ] || { echo "give one --body for the template and one per source, in order" >&2; exit 1; }
branch="feat/$name-template"
template="templates/$name"
snapshot=$(git rev-parse --verify "$snapshot^{commit}")
git cat-file -e "$snapshot:$template/Cargo.toml"
for id in "${ids[@]}"; do
    [[ "$id" =~ ^([a-z]{2,3}|multi)\.[a-z0-9][a-z0-9.-]*$ ]] || { echo "invalid source id: $id" >&2; exit 1; }
    git show "$snapshot:sources/$id/Cargo.toml" | grep -q "templates/$name\"" \
        || { echo "sources/$id does not use $template" >&2; exit 1; }
done
git diff --quiet -- "$template" "${ids[@]/#/sources/}" && git diff --cached --quiet -- "$template" "${ids[@]/#/sources/}" \
    || { echo "Commit template and source changes before preparing a submission." >&2; exit 1; }
git fetch -q https://github.com/Aidoku-Community/sources.git main:refs/remotes/upstream/main
lease=$(git ls-remote origin "refs/heads/$branch" | cut -f1)
work=$(mktemp -d)
trap 'git -C "$root" worktree remove --force "$work" >/dev/null 2>&1 || true' EXIT
git worktree add -q --detach "$work" upstream/main
export GIT_AUTHOR_NAME="Popmango Extensions"
export GIT_AUTHOR_EMAIL="297959108+PoppingMangoSources@users.noreply.github.com"
export GIT_COMMITTER_NAME="$GIT_AUTHOR_NAME"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"

# Copies one crate from the snapshot without fork tests, at version 1, and commits it.
add() {
    local dir=$1 subject=$2 body=$3
    rm -rf "${work:?}/$dir"
    git archive "$snapshot" "$dir" | tar -x -C "$work"
    python3 - "$work/$dir" "$tooling" <<'PY'
import json, runpy, sys
from pathlib import Path
crate, tooling = Path(sys.argv[1]), sys.argv[2]
without_tests = runpy.run_path(tooling + "/scripts/submission-files.py")["without_tests"]
files = {str(path.relative_to(crate)): path.read_bytes() for path in crate.rglob("*") if path.is_file()}
prepared = without_tests(files)
for path in files.keys() - prepared.keys():
    (crate / path).unlink()
for path, data in prepared.items():
    (crate / path).write_bytes(data)
metadata = crate / "res/source.json"
if metadata.exists():
    data = json.loads(metadata.read_text())
    data["info"]["version"] = 1
    metadata.write_text(json.dumps(data, indent="\t", ensure_ascii=False) + "\n")
PY
    (cd "$work/$dir" && cargo fmt)
    git -C "$work" add -A -- "$dir"
    git -C "$work" diff --cached --quiet && { echo "No changes to submit in $dir." >&2; exit 1; }
    git -C "$work" -c commit.gpgsign=false commit -q --no-gpg-sign -m "$subject" -m "$body"
}

add "$template" "feat: add $name template" "${bodies[0]}"
for i in "${!ids[@]}"; do
    add "sources/${ids[$i]}" "feat: add ${ids[$i]}" "${bodies[$((i + 1))]}"
done

(cd "$work/$template" && cargo fmt --check && cargo clippy --target wasm32-unknown-unknown -- -D warnings)
for id in "${ids[@]}"; do
    python3 "$tooling/scripts/check-upstream-scope.py" --repo "$work" --source "$id" --branch "$branch"
    python3 "$tooling/scripts/check-sources.py" --repo "$work" --upstream "$id"
    "$tooling/scripts/verify-source.sh" "$work/sources/$id"
done
# Each crate must match the snapshot apart from fork tests, formatting and the version.
python3 - "$tooling" "$root" "$snapshot" "$work" "$template" "${ids[@]/#/sources/}" <<'PY'
import json, re, runpy, sys
tooling, root, snapshot, work, *crates = sys.argv[1:]
helpers = runpy.run_path(tooling + "/scripts/submission-files.py")
def normal(files):
    files = dict(files)
    if "res/source.json" in files:
        metadata = json.loads(files["res/source.json"])
        metadata["info"].pop("version")
        files["res/source.json"] = metadata
    return {path: re.sub(rb"\s+", b"", data) if isinstance(data, bytes) else data for path, data in files.items()}
for crate in crates:
    expected = normal(helpers["without_tests"](helpers["snapshot_files"](root, snapshot, crate)))
    actual = normal(helpers["snapshot_files"](work, "HEAD", crate))
    assert expected == actual, f"{crate} drifted beyond version, formatting and fork-only tests"
PY
sha=$(git -C "$work" rev-parse HEAD)
git -C "$work" log --oneline upstream/main..HEAD
if $push; then
    if [ -n "$lease" ]; then
        backup="chore/$name-template-before-rebuild-${lease:0:8}"
        git update-ref "refs/heads/$backup" "$lease"
        git push --force-with-lease="refs/heads/$backup:" origin "$lease:refs/heads/$backup"
        echo "Preserved $lease as $backup"
    fi
    git -C "$work" push --force-with-lease="refs/heads/$branch:$lease" origin "HEAD:refs/heads/$branch"
else
    prepared="chore/$name-template-prepared-${sha:0:8}"
    git update-ref "refs/heads/$prepared" "$sha"
    echo "Validated $sha; retained as $prepared. Re-run with --push to publish the submission."
fi
