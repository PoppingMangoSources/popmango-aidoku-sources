# Source review evidence

Consult this when creating a source, choosing APIs, preparing an upstream branch, or
answering review comments. Read `AGENTS.md` / `CLAUDE.md` for the repository's rules and the parent
`SKILL.md` for the procedure. Record new observations instead of assuming these snapshots
remain current.

## Official references and boundaries

Inspected 2026-10-26: aidoku-rs `ae6485e3c14ecd1fd5497d170eae884c68d04a87`,
AidokuRunner `2ed3f0d395be0c3ee9ebe238e2ffcead0094613f`, Aidoku app
`ceee58b29d4f48106cf63042ca0cb075a8ac22b4`; community base
`2b0996dcdaf4529b36e059882f8bf50d33d87bb5`. Fetch current revisions before treating a
claim as a current API guarantee. These are references, not extra source dependencies.

| Question | Read | What it establishes |
| --- | --- | --- |
| Contribution and minimum functionality | [CONTRIBUTING](https://github.com/Aidoku-Community/sources/blob/main/CONTRIBUTING.md), [official guide](https://aidoku.github.io/aidoku-rs/book/) | Source expectations, ownership, templates; AI assistance still needs human review. |
| Source contract | [Rust traits and exports](https://github.com/Aidoku/aidoku-rs/blob/main/crates/lib/src/lib.rs), `structs/`, `macros/`, [example](https://github.com/Aidoku/aidoku-rs/tree/main/examples/example-source) | Required functions and registered optional traits; not a live-site API specification. |
| URL/query/date/network behavior | [helpers](https://github.com/Aidoku/aidoku-rs/tree/main/crates/lib/src/helpers), [imports](https://github.com/Aidoku/aidoku-rs/tree/main/crates/lib/src/imports), [helper tests](https://github.com/Aidoku/aidoku-rs/blob/main/crates/lib/tests/helpers.rs) | Prefer the provided URL encoding, parsing and request APIs to handwritten substitutes. |
| JSON format and app-version requirements | [schemas](https://github.com/Aidoku/aidoku-rs/tree/main/crates/cli/src/supporting/schema), [verify](https://github.com/Aidoku/aidoku-rs/blob/main/crates/cli/src/commands/verify.rs), [models](https://github.com/Aidoku/aidoku-rs/tree/main/crates/cli/src/models) | `aidoku verify` validates resources, icon dimensions/opacity and WASM exports/import minimum versions. It does not call the website. |
| Scaffold/package/serve/log commands | [CLI entry](https://github.com/Aidoku/aidoku-rs/blob/main/crates/cli/src/bin/aidoku.rs), [commands](https://github.com/Aidoku/aidoku-rs/tree/main/crates/cli/src/commands), [templates](https://github.com/Aidoku/aidoku-rs/tree/main/crates/cli/src/supporting/templates) | Actual command flags/defaults; use `--help` before copying old invocation examples. |
| Source tests | [test macro](https://github.com/Aidoku/aidoku-rs/blob/main/crates/test-macro/src/lib.rs), [test runner](https://github.com/Aidoku/aidoku-rs/tree/main/crates/test-runner/src) | `#[aidoku_test]` with the wasm runner; not full app parity. |
| Actual app host behavior | [AidokuRunner](https://github.com/Aidoku/AidokuRunner/tree/main/Sources/AidokuRunner), including `Imports/`, `Extensions/`, `Models/Codable/` and `Utilities/Postcard/` | Swift host implementations and serialized model layout. Read alongside the Rust structs when debugging a contract mismatch. |
| App integration | [Aidoku](https://github.com/Aidoku/Aidoku/tree/main/Aidoku), including `Core/`, `App/`, `Features/`, `Extensions/`, [Info.plist](https://github.com/Aidoku/Aidoku/blob/main/Aidoku/Info.plist), [Xcode project](https://github.com/Aidoku/Aidoku/tree/main/Aidoku.xcodeproj) | UI, download, settings and deployment behavior. The app marketing version does not substitute for imported source API version checks. |
| JSON filter and setting defaults | AidokuRunner `Sources/AidokuRunner/Models/Filter.swift` (`init(from:)` of each filter) | What the app assumes when a key is absent from `filters.json`: `canExclude` false, `usesTagStyle` = `isGenre`. Settle any "is this the default?" question here, not from a review comment. |
| Tests that use the host | [AidokuTests](https://github.com/Aidoku/Aidoku/tree/main/AidokuTests), [Support](https://github.com/Aidoku/Aidoku/tree/main/AidokuTests/Support), [Runner tests](https://github.com/Aidoku/AidokuRunner/tree/main/Tests/AidokuRunnerTests) | Useful app/codec examples; `TestableSourceRunner` returns canned search/page results and is not a source-site test runner. |

The Rust runner's `imports/js.rs` has WebView methods returning `-1` in the inspected
revision. A runner pass cannot prove Cloudflare prompts, cookies, navigation, WebView
capture, or rendered reader behavior. Use the app for those paths. AidokuRunner's
`Interpreter.getMangaUpdate` wires partial results to the host; verify chapter-only,
details-only and combined requests separately when the source branches on those flags.

## Verification matrix

| Layer | Run or inspect | Evidence and limits |
| --- | --- | --- |
| Instructions | `python3 scripts/sync-agent-docs.py --check` | Full docs and both physical skill trees match. |
| Source conventions | `python3 scripts/check-sources.py <id>` | Fork rules and README links; use `--upstream --repo <worktree>` for a clean submission without fork files. |
| Submission | `check-upstream-scope.py --repo <worktree> --source <id>` after canonical fetch | Source-only final diff and commit history, no merge/generated files, correct id/version. New version 1; update upstream+1; review rounds retain that version. |
| Rust/package | `scripts/verify-source.sh <source-directory>` | Formatting, Clippy with warnings denied, package exists, schema/icon/WASM verification. |
| Parser/state tests | `scripts/verify-source.sh <source-directory> --test` | Install `aidoku-test-runner`; cover keys, URL encoding, parsers, pagination signals, errors and cleared settings with `#[aidoku_test]`. |
| Website | Probe each relevant endpoint with a timeout and proper headers | Record request and response facts, page 2 and a deliberately impossible filter value. A request that cannot connect is unverified, not a passing empty-result test. |
| App | Install `.aix` by device transfer or `aidoku serve package.aix`; use `aidoku logcat` | Check browse, details/chapter-only updates, deep links, settings, locked/duplicate chapters and an actual readable chapter. |

Use meaningful tests where logic warrants them. Do not claim the guide requires tests for
every source. Live tests remain explicit/opt-in; do not make random network failures a
required gate or silently suppress them. Never use `cargo clippy --all-targets` for sources.

## Reviewer evidence

These are observed review comments; technical corrections and repository conventions are
not all universal API restrictions. Apply them to matching code, not mechanically everywhere.

| Evidence | Lesson |
| --- | --- |
| [624: query encoding](https://github.com/Aidoku-Community/sources/pull/624#discussion_r3605938180) | Use `QueryParameters`; unencoded queries can fail on some iOS versions. Test spaces, punctuation and non-ASCII queries. |
| [624: models](https://github.com/Aidoku-Community/sources/pull/624#discussion_r3908110450), [637: helpers](https://github.com/Aidoku-Community/sources/pull/637#discussion_r3625394787) | Keep trait implementations in `lib.rs`, models/conversions in `models.rs`, free helpers in `helpers.rs`. |
| [624: crypto](https://github.com/Aidoku-Community/sources/pull/624#discussion_r3605922419) | Prefer no_std Rust signing/crypto to JS where practical; check API availability for async JS. |
| [637: query filters](https://github.com/Aidoku-Community/sources/pull/637#discussion_r3625414597) | Hide filters while searching only when the site ignores them; do not process ignored values on that path. |
| [637: response decoding](https://github.com/Aidoku-Community/sources/pull/637#discussion_r3633441173) | Decode JSON through request/response APIs and preserve meaningful errors rather than remapping everything. |
| [824: chapter-only requests](https://github.com/Aidoku-Community/sources/pull/824#discussion_r4308565334) | Self-sufficient keys may include both slug and id so chapters do not require another detail fetch. |
| [824: dates](https://github.com/Aidoku-Community/sources/pull/824#discussion_r4308580887) | Prefer `parse_date`; avoid a dependency just to parse one site timestamp. |
| [824: sorting](https://github.com/Aidoku-Community/sources/pull/824#discussion_r4318364144) | Put the sort filter first as a presentation convention. |
| [824: exclusions](https://github.com/Aidoku-Community/sources/pull/824#discussion_r4329883138) | Dynamic Rust `MultiSelectFilter` must explicitly disable exclusion when unsupported; static JSON has its own defaults. Do not confuse these defaults. |
| [828: unified API](https://github.com/Aidoku-Community/sources/pull/828#discussion_r4329442516) | Prefer a single real API for query and filters over duplicated search paths when supported. |
| [828: config](https://github.com/Aidoku-Community/sources/pull/828#discussion_r4352966663) | `breakingChangeVersion` belongs in top-level `config`; preserve source identity for a rewrite/rebrand. |
| [833: fixed regex set](https://github.com/Aidoku-Community/sources/pull/833#discussion_r4353112488) | A fixed number of compiled regexes can be an array; compile reusable patterns once. |
| [835: page failures](https://github.com/Aidoku-Community/sources/pull/835#discussion_r4353163328) | Propagate a failing key or retain the page URL. Never silently shorten a chapter's page list. |
| [624: previous feedback](https://github.com/Aidoku-Community/sources/pull/624#pullrequestreview-5082686395) | Recheck every prior comment against final code; resolving a thread is not evidence of a fix. |

Accepted reference implementations: [en.toonkotsu](https://github.com/Aidoku-Community/sources/tree/main/sources/en.toonkotsu),
[multi.xcomic](https://github.com/Aidoku-Community/sources/tree/main/sources/multi.xcomic),
and [Kagane rework](https://github.com/Aidoku-Community/sources/commit/54d287d5af7697d4b053d5aeee24283a32b1945b).
Use them for their demonstrated patterns while verifying each site's current contract.
