# Pins cargo to this repository's Rust version, also in upstream worktrees, which have no
# rust-toolchain.toml and would otherwise format with the machine's default. Upstream CI uses
# the same version, so formatting that passes here passes there. Sourced, not run.
if [ -z "${RUSTUP_TOOLCHAIN:-}" ]; then
    RUSTUP_TOOLCHAIN=$(sed -n 's/^channel = "\(.*\)"$/\1/p' "$(dirname "${BASH_SOURCE[0]}")/../rust-toolchain.toml")
    export RUSTUP_TOOLCHAIN
fi
