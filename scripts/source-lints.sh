# Lints that are off by default and catch dead or needless code. Sources only: templates are
# upstream's copies and follow its gate. Sourced by verify-source.sh and lint-crates.sh.
# shellcheck disable=SC2034
SOURCE_LINTS=(
    -D unused_extern_crates
    -D unused_import_braces
    -D unused_lifetimes
    -D unused_macro_rules
    -D clippy::redundant_clone
    -D clippy::unused_self
    -D clippy::needless_pass_by_value
    -D clippy::unnecessary_wraps
    -D clippy::used_underscore_binding
    -D clippy::no_effect_underscore_binding
)
