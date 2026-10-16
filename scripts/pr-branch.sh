#!/usr/bin/env bash
# Builds the branch an upstream Aidoku-Community PR is opened from, in a separate worktree so
# the current checkout is left alone. The source is copied from main and the gates run before
# anything is pushed.
#
# Usage:
#   scripts/pr-branch.sh <id>                               a new source: "feat: add <id>", version 1
#   scripts/pr-branch.sh <id> --update "<subject>" "<body>"  a source already upstream: its version + 1
#   scripts/pr-branch.sh <id> --fix "<subject>" "<body>"     a review round: one more commit on top
#   add --push to push it (with a lease on what origin holds)

set -euo pipefail

usage() {
	sed -n '5,10p' "$0" | sed 's/^# \{0,1\}//'
	exit 1
}

[ $# -ge 1 ] || usage
id=$1
shift
mode=new
subject=
body=
push=false
while [ $# -gt 0 ]; do
	case $1 in
	--fix | --update)
		[ $# -ge 3 ] || usage
		mode=${1#--}
		subject=$2
		body=$3
		shift 3
		;;
	--push)
		push=true
		shift
		;;
	*) usage ;;
	esac
done

root=$(git rev-parse --show-toplevel)
branch="feat/$id"
source="sources/$id"
[ -f "$root/$source/Cargo.toml" ] || { echo "no $source on this branch" >&2; exit 1; }

export GIT_AUTHOR_NAME="Popmango Extensions"
export GIT_AUTHOR_EMAIL="297959108+PoppingMangoSources@users.noreply.github.com"
export GIT_COMMITTER_NAME=$GIT_AUTHOR_NAME
export GIT_COMMITTER_EMAIL=$GIT_AUTHOR_EMAIL
commit() { git -c commit.gpgsign=false commit -q --no-gpg-sign "$@"; }

main=$(git -C "$root" rev-parse HEAD)
git -C "$root" fetch -q upstream main
lease=$(git -C "$root" ls-remote origin "refs/heads/$branch" | cut -f1)

work=$(mktemp -d)
trap 'git -C "$root" worktree remove --force "$work" >/dev/null 2>&1 || true' EXIT

# A branch whose PR was merged is restarted from upstream rather than built on.
merged=false
if [ -n "$lease" ]; then
	git -C "$root" fetch -q origin "$branch"
	git -C "$root" merge-base --is-ancestor "$lease" upstream/main && merged=true
fi
if [ "$mode" = fix ]; then
	[ -n "$lease" ] && ! $merged || { echo "origin has no open $branch to add a commit to" >&2; exit 1; }
	git -C "$root" worktree add -q -B "$branch" "$work" "origin/$branch"
else
	[ -z "$lease" ] || $merged || { echo "origin has an unmerged $branch; use --fix for a review round" >&2; exit 1; }
	git -C "$root" worktree add -q -B "$branch" "$work" upstream/main
fi

cd "$work"
case $mode in
new)
	[ ! -e "$source" ] || { echo "$source is already upstream; use --update" >&2; exit 1; }
	version=1
	;;
update)
	[ -e "$source" ] || { echo "$source is not upstream yet; open it as a new source" >&2; exit 1; }
	version=$(($(sed -nE 's/.*"version": ([0-9]+),.*/\1/p' "$source/res/source.json" | head -1) + 1))
	;;
fix) version=$(sed -nE 's/.*"version": ([0-9]+),.*/\1/p' "$source/res/source.json" | head -1) ;;
esac
# Only what an upstream source directory holds: no package.aix, no target, no README row.
rm -rf "$source"
mkdir -p "$source"
git checkout -q "$main" -- \
	"$source/.cargo" "$source/Cargo.toml" "$source/Cargo.lock" "$source/res" "$source/src"
sed -i -E "0,/\"version\": [0-9]+,/s//\"version\": $version,/" "$source/res/source.json"
git add -A "$source"

if [ "$mode" = new ]; then
	commit -m "feat: add $id"
else
	git diff --cached --quiet && { echo "nothing to commit: the source already matches main" >&2; exit 1; }
	commit -m "$subject" -m "$body"
fi

(
	cd "$source"
	cargo fmt --all -- --check
	cargo clippy --release --target wasm32-unknown-unknown 2>&1 | tee /dev/stderr | grep -qE '^(warning|error)' && exit 1
	aidoku package >/dev/null
	aidoku verify package.aix
	rm -f package.aix
)

others=$(git diff --name-only upstream/main HEAD | grep -v "^$source/" || true)
[ -z "$others" ] || { echo "the branch touches files outside $source:" >&2; echo "$others" >&2; exit 1; }
drift=$(git diff "$main" HEAD -- "$source" | grep -E '^[+-][^+-]' | grep -vE '"version": [0-9]+,' || true)
[ -z "$drift" ] || { echo "the branch differs from main beyond the version:" >&2; echo "$drift" >&2; exit 1; }

git log --oneline upstream/main..HEAD
if $push; then
	git push --force-with-lease="$branch:$lease" origin "$branch"
else
	echo "Ready. Push with: git push --force-with-lease=$branch:$lease origin $branch"
fi
