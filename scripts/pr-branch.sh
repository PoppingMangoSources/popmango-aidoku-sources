#!/usr/bin/env bash
# Prepare an upstream submission from a committed snapshot; leave its checkout alone.
# Usage: scripts/pr-branch.sh <id> [--from <ref>] [--branch <name>] [--push]
# Add --update "<subject>" "<body>" for an existing upstream source,
# --fix "<subject>" "<body>" for a review round, or --rebuild to preserve and replace a branch.
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
while [ $# -gt 0 ]; do
    case $1 in
        --from) snapshot=${2:?provide a ref}; shift 2 ;;
        --branch) branch=${2:?provide a branch}; shift 2 ;;
        --fix|--update) mode=${1#--}; subject=${2:?provide a subject}; body=${3:?provide a body}; shift 3 ;;
        --push) push=true; shift ;;
        --rebuild) rebuild=true; shift ;;
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
python3 - "$work/$source" "$version" <<'PY'
import json, re, sys
from pathlib import Path
source, version = Path(sys.argv[1]), int(sys.argv[2])
path = source / "res/source.json"
data = json.loads(path.read_text())
data["info"]["version"] = version
path.write_text(json.dumps(data, indent="\t", ensure_ascii=False) + "\n")
(source / "src/test.rs").unlink(missing_ok=True)
lib = source / "src/lib.rs"
lib.write_text(re.sub(r"\n*#\[cfg\(test\)\]\nmod test;\n", "\n", lib.read_text()))
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
python3 - "$root" "$snapshot" "$work" "$source" <<'PY'
import json, re, subprocess, sys
root, snapshot, work, source = sys.argv[1:]
metadata, lib, test = (source + "/" + name for name in ("res/source.json", "src/lib.rs", "src/test.rs"))
def tree(repo, ref):
    rows = subprocess.check_output(["git", "-C", repo, "ls-tree", "-r", ref, "--", source], text=True).splitlines()
    return [row for row in rows if row.split("\t")[1] not in (metadata, lib, test)]
assert tree(root, snapshot) == tree(work, "HEAD"), "source files drifted"
def show(repo, ref, path):
    return subprocess.check_output(["git", "-C", repo, "show", ref + ":" + path], text=True)
expected = re.sub(r"\n*#\[cfg\(test\)\]\nmod test;\n", "\n", show(root, snapshot, lib))
assert show(work, "HEAD", lib) == expected, "lib.rs drifted beyond the test module"
assert subprocess.run(["git", "-C", work, "cat-file", "-e", "HEAD:" + test], capture_output=True).returncode != 0, "test.rs reached the submission"
def info(repo, ref):
    data = json.loads(subprocess.check_output(["git", "-C", repo, "show", ref + ":" + metadata]))
    data["info"].pop("version")
    return data
assert info(root, snapshot) == info(work, "HEAD"), "source metadata drifted beyond version"
PY
sha=$(git -C "$work" rev-parse HEAD)
if $push; then
    git -C "$work" push --force-with-lease="refs/heads/$branch:$lease" origin "HEAD:refs/heads/$branch"
else
    prepared="chore/$id-prepared-${sha:0:8}"
    git update-ref "refs/heads/$prepared" "$sha"
    echo "Validated $sha; retained as $prepared. Re-run with --push to publish the submission."
fi
