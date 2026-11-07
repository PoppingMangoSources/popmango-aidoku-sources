#!/usr/bin/env bash
# Scaffolds sources/<id> with the aidoku CLI, then drops what the CLI adds that a source in
# this repo does not keep, and prints the README row to paste in.
#
# Usage: scripts/new-source.sh <id> <name> <url> [safe|contains-nsfw|primarily-nsfw] [languages]
#   scripts/new-source.sh en.example Example https://example.com
#   scripts/new-source.sh multi.example Example https://example.com safe "en,ja"

set -euo pipefail

if [ $# -lt 3 ]; then
	sed -n '5,7p' "$0" | sed 's/^# \{0,1\}//'
	exit 1
fi

id=$1
[[ "$id" =~ ^([a-z]{2,3}|multi)\.[a-z0-9][a-z0-9.-]*$ ]] || { echo "invalid source id" >&2; exit 1; }
name=$2
url=${3%/}
rating=${4:-safe}
prefix=${id%%.*}
languages=${5:-$prefix}

if [ "$prefix" = multi ] && [ $# -lt 5 ]; then
	echo "A multi-language source needs its languages, e.g. \"en,ja\"; never \"multi\"." >&2
	exit 1
fi

root=$(git rev-parse --show-toplevel)
tooling=$(cd "$(dirname "$0")/.." && pwd)
branch=$(git branch --show-current)
submission=$(python3 -c 'import runpy,sys; print(runpy.run_path(sys.argv[1])["source_for_branch"](sys.argv[2]) or "")' "$tooling/scripts/check-upstream-scope.py" "$branch")
if [ -n "$submission" ]; then
    [ "$submission" = "$id" ] || { echo "Submission branch is for $submission, not $id." >&2; exit 1; }
    git fetch -q https://github.com/Aidoku-Community/sources.git main:refs/remotes/upstream/main
    python3 "$tooling/scripts/check-upstream-scope.py" --repo "$root" --head HEAD --source "$id"
fi
dir="$root/sources/$id"
if [ -e "$dir" ]; then
	echo "sources/$id already exists" >&2
	exit 1
fi

aidoku init "$dir" -n "$name" -u "$url" -l "$languages" -c "$rating"
# The CLI makes each source its own repository.
rm -rf "$dir/.git" "$dir/.gitignore"

host=${url#*://}
cat <<EOF

Scaffolded sources/$id. Next:
  1. Replace res/icon.png (128x128, opaque, with margin).
  2. Work through .claude/skills/aidoku-source/SKILL.md (identical in .agents/skills).
  3. Add this card to its Route select table in README.md, in alphabetical order
     (two cards per <tr>; set heat-N.svg to the source's contentRating):

<td align="center" width="50%"><img src="sources/$id/res/icon.png" width="64"/><br/><b>$name</b><br/><sub><a href="$url">$host</a></sub><br/><img src="media/heat-0.svg" alt="Safe" height="18"/><br/><br/><a href="https://poppingmangosources.github.io/popmango-aidoku-sources/sources/$id-v1.aix"><img src="media/route-button.svg" alt="Choose route" height="30"/></a><br/><sub><a href="https://poppingmangosources.github.io/popmango-aidoku-sources/sources/$id-v1.aix">.aix (v1)</a></sub></td>
EOF
