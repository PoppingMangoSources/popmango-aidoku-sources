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
  2. Work through .claude/skills/aidoku-source/SKILL.md.
  3. Add this row to README.md, in alphabetical order:

<h3><img src="https://cdn.jsdelivr.net/gh/PoppingMangoSources/popmango-aidoku-sources@main/sources/$id/res/icon.png" width="24" align="top"/>&nbsp; $name &nbsp;<sub><a href="$url">$host</a></sub></h3>

<sub><a href="https://poppingmangosources.github.io/popmango-aidoku-sources/sources/$id-v1.aix"><b>Add to Aidoku</b></a> &nbsp;·&nbsp; <a href="https://poppingmangosources.github.io/popmango-aidoku-sources/sources/$id-v1.aix">.aix (v1)</a></sub>
EOF
