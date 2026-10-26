#!/usr/bin/env bash
# Usage: scripts/verify-source.sh /path/to/sources/en.example [--test]
set -euo pipefail
dir=$(cd "${1:?provide a source directory}" && pwd)
test -f "$dir/Cargo.toml"
cd "$dir"
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown -- -D warnings
RUSTFLAGS=-Dwarnings aidoku package
test -s package.aix
aidoku verify package.aix
if [ "${2:-}" = --test ]; then
	cargo test --target wasm32-unknown-unknown
fi
