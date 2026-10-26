#!/usr/bin/env bash
# Usage: scripts/verify-source.sh /path/to/sources/en.example [--test]
set -euo pipefail
source "$(dirname "$0")/source-lints.sh"
dir=$(cd "${1:?provide a source directory}" && pwd)
test -f "$dir/Cargo.toml"
cd "$dir"
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown -- -D warnings "${SOURCE_LINTS[@]}"
# The plain lint skips the test module; lint it too when there is one.
if grep -qs 'aidoku_test' src/*.rs; then
	cargo clippy --tests --target wasm32-unknown-unknown -- -D warnings "${SOURCE_LINTS[@]}"
fi
RUSTFLAGS=-Dwarnings aidoku package
test -s package.aix
aidoku verify package.aix
if [ "${2:-}" = --test ]; then
	cargo test --target wasm32-unknown-unknown
fi
