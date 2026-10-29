#!/usr/bin/env bash
# Prepare an upstream submission from a committed snapshot; leave its checkout alone.
# Usage: scripts/pr-branch.sh <id> [--from <ref>] [--branch <name>] [--push]
# Add --update "<subject>" "<body>" for an existing upstream source,
# --fix "<subject>" "<body>" for a review round, or --rebuild to preserve and replace a branch
# (refused when the branch holds work main doesn't have, unless --discard-unmerged).
set -euo pipefail
root=$(git rev-parse --show-toplevel)
tooling=$(cd "$(dirname "$0")/.." && pwd)
id=${1:?provide a source id}
shift
snapshot=HEAD
branch="feat/$id"
mode=new
subject="feat: add $id"
body=
push=false
rebuild=false
discard=false
while [ $# -gt 0 ]; do
    case $1 in
        --from) snapshot=${2:?provide a ref}; shift 2 ;;
        --branch) branch=${2:?provide a branch}; shift 2 ;;
        --fix|--update) mode=${1#--}; subject=${2:?provide a subject}; body=${3:?provide a body}; shift 3 ;;
        --push) push=true; shift ;;
        --rebuild) rebuild=true; shift ;;
        --discard-unmerged) discard=true; shift ;;
        *) echo "unknown option: $1" >&2; exit 1 ;;
    esac
done
[[ "$id" =~ ^([a-z]{2,3}|multi)\.[a-z0-9][a-z0-9.-]*$ ]] || { echo "invalid source id" >&2; exit 1; }
git check-ref-format --branch "$branch" >/dev/null
destination=$(python3 -c 'import runpy,sys; print(runpy.run_path(sys.argv[1])["source_for_branch"](sys.argv[2]) or "")' "$tooling/scripts/check-upstream-scope.py" "$branch")
[ "$destination" = "$id" ] || { echo "Destination must be a recognized submission branch for $id; refusing $branch." >&2; exit 1; }
source="sources/$id"
snapshot=$(git rev-parse --verify "$snapshot^{commit}")
git cat-file -e "$snapshot:$source/Cargo.toml"
git diff --quiet -- "$source" && git diff --cached --quiet -- "$source" || { echo "Commit source changes before preparing a submission." >&2; exit 1; }
git fetch -q https://github.com/Aidoku-Community/sources.git main:refs/remotes/upstream/main
lease=$(git ls-remote origin "refs/heads/$branch" | cut -f1)
if [ -n "$lease" ]; then
    git fetch -q origin "refs/heads/$branch"
fi
if [ "$mode" = fix ]; then
    [ -n "$lease" ] || { echo "no existing submission branch" >&2; exit 1; }
    python3 "$tooling/scripts/check-upstream-scope.py" --repo "$root" --head "$lease" --source "$id"
    base=$lease
else
    base=$(git rev-parse upstream/main)
    if [ -n "$lease" ] && ! $rebuild; then
        if ! git merge-base --is-ancestor "$lease" "$base" && ! git diff --quiet "$lease" "$base" -- "$source"; then
            echo "Existing unmerged branch: use --fix, or --rebuild to preserve it and start fresh." >&2
            exit 1
        fi
    fi
fi
# A rebuild replaces the branch, so refuse when it holds source work no commit on main has:
# someone may have pushed to it directly. --discard-unmerged replaces it anyway.
if $rebuild && [ -n "$lease" ] && ! $discard; then
    python3 - "$root" "$lease" "$source" "$tooling" <<'PY' || exit 1
import runpy, subprocess, sys
root, lease, source, tooling = sys.argv[1:]
helpers = runpy.run_path(tooling + "/scripts/submission-files.py")
def files(ref):
    data = helpers["without_tests"](helpers["snapshot_files"](root, ref, source))
    data.pop("res/source.json", None)
    modes = helpers["snapshot_modes"](root, ref, source)
    modes.pop("src/test.rs", None)
    return data, modes
branch_files = files(lease)
snapshots = subprocess.check_output(["git", "-C", root, "log", "--format=%H", "-n", "200", "origin/main", "--", source], text=True).split()
if not any(branch_files == files(ref) for ref in snapshots):
    sys.exit(f"{lease[:8]} on the branch matches no commit of {source} on main: someone pushed work to it.\n"
             "Bring that work into main first (or pass --discard-unmerged to replace it anyway).")
PY
fi
if $rebuild && [ -n "$lease" ]; then
    backup="chore/$id-before-rebuild-${lease:0:8}"
    git update-ref "refs/heads/$backup" "$lease"
    if $push; then
        git push --force-with-lease="refs/heads/$backup:" origin "$lease:refs/heads/$backup"
    fi
    echo "Preserved $lease as $backup"
fi
work=$(mktemp -d)
trap 'git -C "$root" worktree remove --force "$work" >/dev/null 2>&1 || true' EXIT
git worktree add -q --detach "$work" "$base"
case $mode in
    new)
        if git cat-file -e "upstream/main:$source/res/source.json" 2>/dev/null; then
            echo "Source already upstream: use --update." >&2; exit 1
        fi
        version=1 ;;
    update)
        version=$(git show "upstream/main:$source/res/source.json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["info"]["version"] + 1)') ;;
    fix)
        version=$(git show "$lease:$source/res/source.json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["info"]["version"])') ;;
esac
rm -rf "$work/$source"
git archive "$snapshot" "$source" | tar -x -C "$work"
# Tests stay on main: they call the live site, and the submission is the source alone.
python3 - "$work/$source" "$version" "$tooling" <<'PY'
import json, runpy, sys
from pathlib import Path
source, version, tooling = Path(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
without_tests = runpy.run_path(tooling + "/scripts/submission-files.py")["without_tests"]
files = {str(path.relative_to(source)): str(path.readlink()).encode() if path.is_symlink() else path.read_bytes()
         for path in source.rglob("*") if path.is_file() or path.is_symlink()}
prepared = without_tests(files)
for path in files.keys() - prepared.keys():
    (source / path).unlink()
for path, data in prepared.items():
    if not (source / path).is_symlink():
        (source / path).write_bytes(data)
path = source / "res/source.json"
data = json.loads(path.read_text())
data["info"]["version"] = version
path.write_text(json.dumps(data, indent="\t", ensure_ascii=False) + "\n")
PY
git -C "$work" add -A -- "$source"
git -C "$work" diff --cached --quiet && { echo "No source changes to submit." >&2; exit 1; }
export GIT_AUTHOR_NAME="Popmango Extensions"
export GIT_AUTHOR_EMAIL="297959108+PoppingMangoSources@users.noreply.github.com"
export GIT_COMMITTER_NAME="$GIT_AUTHOR_NAME"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"
message=(-m "$subject")
[ -z "$body" ] || message+=(-m "$body")
git -C "$work" -c commit.gpgsign=false commit -q --no-gpg-sign "${message[@]}"
python3 "$tooling/scripts/check-upstream-scope.py" --repo "$work" --source "$id" --branch "$branch"
python3 "$tooling/scripts/check-sources.py" --repo "$work" --upstream "$id"
"$tooling/scripts/verify-source.sh" "$work/$source"
python3 - "$root" "$snapshot" "$work" "$source" "$tooling" <<'PY'
import json, runpy, sys
root, snapshot, work, source, tooling = sys.argv[1:]
helpers = runpy.run_path(tooling + "/scripts/submission-files.py")
expected = helpers["without_tests"](helpers["snapshot_files"](root, snapshot, source))
actual = helpers["snapshot_files"](work, "HEAD", source)
for files in (expected, actual):
    metadata = json.loads(files["res/source.json"])
    metadata["info"].pop("version")
    files["res/source.json"] = metadata
assert expected == actual, "source files drifted beyond version and fork-only tests"
expected_modes = helpers["snapshot_modes"](root, snapshot, source)
expected_modes.pop("src/test.rs", None)
assert expected_modes == helpers["snapshot_modes"](work, "HEAD", source), "source file modes drifted"
PY
sha=$(git -C "$work" rev-parse HEAD)
if $push; then
    git -C "$work" push --force-with-lease="refs/heads/$branch:$lease" origin "HEAD:refs/heads/$branch"
else
    prepared="chore/$id-prepared-${sha:0:8}"
    git update-ref "refs/heads/$prepared" "$sha"
    echo "Validated $sha; retained as $prepared. Re-run with --push to publish the submission."
fi
