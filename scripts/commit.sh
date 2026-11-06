#!/usr/bin/env bash
# Commits as the owner, unsigned, in the form AGENTS.md / CLAUDE.md section 9 gives.
# Usage: scripts/commit.sh -m "<conventional subject>" -m "<body>" [other git commit options]
set -euo pipefail
name="Popmango Extensions"
email="297959108+PoppingMangoSources@users.noreply.github.com"
GIT_AUTHOR_NAME="$name" GIT_AUTHOR_EMAIL="$email" \
    git -c commit.gpgsign=false -c user.name="$name" -c user.email="$email" \
    commit --no-gpg-sign "$@"
