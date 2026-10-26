#!/usr/bin/env bash
# Read one crate path per line; retain exit codes, annotations and diagnostics on failure.
set -euo pipefail
root=$(git rev-parse --show-toplevel)
logs=$(mktemp -d)
trap 'rm -rf "$logs"' EXIT
failed=0
index=0
while IFS= read -r crate; do
    [ -n "$crate" ] || continue
    index=$((index + 1))
    echo "::group::Linting $crate"
    if ! (cd "$crate"; cargo fmt --check); then failed=1; fi
    if ! (cd "$crate"; cargo clippy --target wasm32-unknown-unknown --message-format=json -- -D warnings) > "$logs/$index.json"; then failed=1; fi
    if ! jq --arg WORKING_DIRECTORY "$crate" -rsf "$root/.github/workflows/supporting/annotations.jq" "$logs/$index.json"; then failed=1; fi
    echo "::endgroup::"
done
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    if [ "$index" -gt 0 ]; then
        jq --arg RUSTC_VERSION "$(rustc --version)" --arg CARGO_VERSION "$(cargo --version)" --arg CLIPPY_VERSION "$(cargo clippy --version)" -rsf "$root/.github/workflows/supporting/summary.jq" "$logs/"*.json >> "$GITHUB_STEP_SUMMARY"
    else
        echo "No Rust crates changed." >> "$GITHUB_STEP_SUMMARY"
    fi
fi
exit "$failed"
