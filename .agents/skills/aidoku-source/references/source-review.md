# Source review evidence

Consult this when creating a source, choosing APIs, preparing an upstream branch, or
answering review comments. Read `AGENTS.md` / `CLAUDE.md` for the repository's rules and the parent
`SKILL.md` for the procedure. Record new observations instead of assuming these snapshots
remain current.

Automatic enforcement: main's `scripts/review-contracts.py` runs portable public-API
contracts for these templates, including clean PR worktrees. `check-sources.py` guards
known rejected patterns and reviewed consumer configuration. The coverage matrix and
manual-evidence boundaries live in `docs/operations/source-checks.md` on main.

Keyoapp review: [short synopsis](https://github.com/Aidoku-Community/sources/pull/888#discussion_r4542766502),
[combined credits](https://github.com/Aidoku-Community/sources/pull/888#pullrequestreview-5864800084).
V1 short synopses have a preformatted paragraph inside `div.overflow-hidden` without
`#expand_content`; keep the expanded layout too. Split verified comma/ampersand credits,
trim, omit blanks and deduplicate in order, independently for authors and artists. FreeWebNovel
and MangaPlus demonstrate delimiter splitting; ReManga demonstrates order-preserving name
deduplication. Loopback fixtures cover the V1 and both V2 metadata layouts and empty byline fallback.

Rage Scans review: [author filter](https://github.com/Aidoku-Community/sources/pull/883#discussion_r4529796128),
[comment mistaken for author and viewer override](https://github.com/Aidoku-Community/sources/pull/883#pullrequestreview-5848419962).
The reported page has no author metadata; `span:contains(Author)` selects comment prose within
the broad details container. Scope the source's author/artist selectors to its metadata box,
omit author search, and use an optional viewer override when the site's type label is wrong.
A nonsense-name query returning zero results establishes neither real author data nor a useful filter.

Merged MangaThemesia batch: Lagoon was approved without requested changes in
[881](https://github.com/Aidoku-Community/sources/pull/881). Rage's author/viewer fixes above
were approved in [883](https://github.com/Aidoku-Community/sources/pull/883). Violet, Thunder,
Madara and Eva merged together in [884](https://github.com/Aidoku-Community/sources/pull/884).
That review accepted the shared 0.8.3 minimum for novel emphasis and requested
[Madara's dark/header-colour icon background](https://github.com/Aidoku-Community/sources/pull/884#discussion_r4529913487).
The feature-gated reader was an offered alternative, not a requirement; the owner chose shared support.
Preserve source tests with their template when archiving. The main template retains its
paragraph/emphasis, key-only reader and viewer-override loopback regressions.

Cached-reader comparison: merged [Madara](https://github.com/Aidoku-Community/sources/blob/289b5613bb47f62c6eb91de3d8b2282643586533/templates/madara/src/imp.rs#L317)
decodes data-URL JavaScript inline; [Keyoapp](https://github.com/Aidoku-Community/sources/blob/289b5613bb47f62c6eb91de3d8b2282643586533/templates/keyoapp/src/home.rs#L227)
uses `STANDARD.decode` followed by `String::from_utf8`. Compare selector contracts before
copying prefix checks or fallbacks. After shared-reader rebases, test plain/cached image
and novel scripts: a conflict-free merge can leave novel detection using the original body.
Main's MangaThemesia HTTP regressions cover those paths and invalid decode fallback.

Shared-template novel review: [minimum-version discussion](https://github.com/Aidoku-Community/sources/pull/884#issuecomment-6777916715).
The owner chose shared emphasis support with a 0.8.3 minimum for all consumers, without a Cargo
feature. Flame demonstrates walking text nodes alongside element children; NovelArchive keeps
emphasis balanced across line breaks and escapes only syntax. Atsumaru receives plain paragraph
strings, so it is a paragraph-boundary reference rather than an HTML emphasis parser.
The follow-up audit found that remembering only one closed marker corrupts adjacent spans
containing both styles. Keep the closed markers in order and cover both nesting orders.

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
| Parser/state tests | `scripts/verify-source.sh <source-directory> --test` | Install `aidoku-test-runner`; cover key-only reader inputs, chapter/deep-link key agreement, enabled filter exclusions, URL encoding, parsers, pagination signals, errors and cleared settings with `#[aidoku_test]`. |
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
| [843: exclusions](https://github.com/Aidoku-Community/sources/pull/843#discussion_r4393718216) | Enable exclusions only when their values are handled. Explicit `canExclude: false` is valid for unsupported filters even though false is the host default. |
| [843: reader keys](https://github.com/Aidoku-Community/sources/pull/843#discussion_r4393742205) | Reader routing must work with only manga/chapter keys; encode necessary type information there and keep chapter lists and deep links consistent. Test without metadata. |
| [843: rating defaults](https://github.com/Aidoku-Community/sources/pull/843#discussion_r4417489344) | Atsumaru with source rating 1 defaults to Safe/Suggestive; Erotica stays opt-in. Match JSON and Rust defaults. This is review policy, not an enforced host restriction. |
| [843: test dependency](https://github.com/Aidoku-Community/sources/pull/843#discussion_r4417564350) | Omit unused test-only dependencies from submissions. Keep `serde_json` on main when its tests deserialize fixtures. Normalize the root Cargo.lock dependency entry too. |
| [843: featured tags](https://github.com/Aidoku-Community/sources/pull/843#discussion_r4417596622) | Alphabetize featured tags to match details and avoid a tag reorder on opening a card. Check both conversions using shuffled input. |
| [828: unified API](https://github.com/Aidoku-Community/sources/pull/828#discussion_r4329442516) | Prefer a single real API for query and filters over duplicated search paths when supported. |
| [828: config](https://github.com/Aidoku-Community/sources/pull/828#discussion_r4352966663) | `breakingChangeVersion` belongs in top-level `config`; preserve source identity for a rewrite/rebrand. |
| [833: fixed regex set](https://github.com/Aidoku-Community/sources/pull/833#discussion_r4353112488) | A fixed number of compiled regexes can be an array; compile reusable patterns once. |
| [835: page failures](https://github.com/Aidoku-Community/sources/pull/835#discussion_r4353163328) | Propagate a failing key or retain the page URL. Never silently shorten a chapter's page list. |
| [624: previous feedback](https://github.com/Aidoku-Community/sources/pull/624#pullrequestreview-5082686395) | Recheck every prior comment against final code; resolving a thread is not evidence of a fix. |

Accepted reference implementations: [en.toonkotsu](https://github.com/Aidoku-Community/sources/tree/main/sources/en.toonkotsu),
[multi.xcomic](https://github.com/Aidoku-Community/sources/tree/main/sources/multi.xcomic),
and [Kagane rework](https://github.com/Aidoku-Community/sources/commit/54d287d5af7697d4b053d5aeee24283a32b1945b).
Use them for their demonstrated patterns while verifying each site's current contract.

November review cross-checks:

- [872: unnecessary allocations](https://github.com/Aidoku-Community/sources/pull/872#discussion_r4507635520): reuse constructed request URLs and borrow script JSON while decoding instead of copying its remainder.
- [872: helpers and comments](https://github.com/Aidoku-Community/sources/pull/872#discussion_r4507630284): move free helpers out of trait wiring; retain only non-obvious explanations.
- [872: chapter keys](https://github.com/Aidoku-Community/sources/pull/872#discussion_r4503603425): preserve suffixes rather than interpreting chapter identity as a number.
- [869: deep-link dispatch](https://github.com/Aidoku-Community/sources/pull/869#discussion_r4503542558): matching hosts are already selected by the app; this does not cover arbitrary pasted search input.
- [865: missing titles](https://github.com/Aidoku-Community/sources/pull/865#discussion_r4497272622): omit untitled cards rather than returning empty titles.
- [865: host matching](https://github.com/Aidoku-Community/sources/pull/865#discussion_r4497295451): a hostname substring can also occur inside another hostname.
- [865: shared credits](https://github.com/Aidoku-Community/sources/pull/865#discussion_r4503529194): writing and drawing are independent roles.
- [666: unified search](https://github.com/Aidoku-Community/sources/pull/666#discussion_r3707511508): use the browse endpoint for queries too when it supports them. Keyoapp V1 instead filters the one catalogue locally because its site ignores those parameters; V2 uses its single search endpoint.

## Iken maintenance evidence

See `docs/subsystems/iken-template.md` for the five consumers and current endpoint contracts.
Test chapter-only requests with numeric series keys without a detail request, independent
lock flags, complete pagination, empty page URL errors, compact home cards and paragraph-safe
novel emphasis. Hive's script-backed banners stay local. The owner explicitly requested
`fix-iken` from canonical upstream and publication on fork main before the live suites.

Iken New Releases and Novels use the query API with createdAt and seriesType=NOVEL respectively.
Test ordering, compact links, empty novel catalogues and one/all failed requests. Keep novel
sections and formatted reading together in their own submission commit when requested.


Commercial price, sale, discount and purchasing controls are omitted from source forms,
filters and settings. Chapter visibility controls remain useful: Show Locked Chapters
uses `showLocked`, defaults off and only appears where chapter lock metadata exists.
Mixed comic/novel catalogues use `labelNovels`, default on, to append `(Novel)` without
changing keys or reader routing. Eternal has novels but no locked chapters; its novel
catalogue form uses `type=novel`.

Iken keeps Latest Releases and New Releases, excluding the separate paid Latest Updates
section. Excluded headings still bound their neighbors. Featured carousels use all metadata
already available from home HTML and existing responses, without per-card requests. True
banner artwork uses ImageScroller and known dimensions; portrait-cover hero slides remain
BigScroller. Vortex banners are 1920x1080; Hive keeps its custom script banners local.

Home enrichment must keep ranking rows unchanged and preserve available HTML metadata when API fields are absent. Novel labels apply to HTML cards as well as API cards, and featured/detail tag ratings use the same classifier. Deterministic regressions cover these boundaries.

When a shared template adds a dependency, synchronize its own lockfile and every consumer lockfile. Verify the complete graphs with `cargo metadata --locked --filter-platform wasm32-unknown-unknown`. Upstream peer-consumer changes limited to Cargo.lock may retain their versions; behavior or source/resource changes still require the normal bump. Fork publication rules remain unchanged.

## November 13 review follow-up

Read the latest inline feedback and review summaries on 899, 894 and 878 alongside the merged
888, 860, 843, 850, 853 and 827 decisions. Applicable patterns are accurate propagated failures,
metadata-scoped parsing, compact listing cards, shared search logic, short comments and static
filter defaults. Age-gate HTML and auto-discovered mirror recommendations apply to their sites,
not to every JSON source. Source-specific requests are not universal API restrictions.

The MangaBall audit also verified current SelectFilterView: default values are removed from the
request. A neutral source-setting entry keeps every explicit override usable. Captured scalar/list
schemas, safe flags inconsistent with tags, empty language preferences and home chapter group
exclusions now have deterministic source regressions.


MangaBall's API already supplies Markdown. Preserve it on details, following merged MangaDex,
instead of applying the plain-text escaping used by NovelArchive. The owner's screenshots
showed formatted text before the response and literal markers after it. Test details-only,
combined and chapters-only updates against the reported title. BigScroller uses Text rather
than Markdown; its literal markers are an app rendering limitation, not justification for
escaping detail text or adding a Markdown parser to the source.


For MangaBall deduplication, the published Yomu AIX advertises official/likes/views/newest,
but our observed endpoint has no official flag. Use the observed likes/views/UTC timestamp
fields, not an invented official classification. Retain the winning group's same-number
parts, all unnumbered chapters and separate languages/volumes, as Atsumaru's preservation
rule requires. Optional deduplication must keep original chapter keys and add no requests.


## Iken review, November 13

[Review 900](https://github.com/Aidoku-Community/sources/pull/900#pullrequestreview-5897319010)
separates EternalMangas' independent current-site rewrite into its own PR. The template branch
keeps only the legacy consumer's API field/minimum-version compatibility. Its migration,
icon, rating, filters and parser belong on fix-eternal. The new alias is included in the same
scope and publication gates as conventional source branches.

Preserve standard aidoku/aidoku-test dev dependencies even when excluding fork tests
([dependency comment](https://github.com/Aidoku-Community/sources/pull/900#discussion_r4568257560)).
They are scaffolding for future tests; unused test-only serde_json is a separate question.
The scope checker compares upstream manifests, and normalization tests preserve both standard
entries. Keep isGenre on Eternal's genre select. Separate top-level Rust items and methods
with blank lines; the spacing rule is advisory for unrelated merged counterexamples, strict
for the reviewed Iken/Eternal files.

The selector suggestion was explicitly optional, then requested by the owner. Use
.col-lg-3 li:has(strong), with exact label-prefix matching, to scope metadata and reduce the
row walk. Do not copy :contains indiscriminately: the app supports it while the runner's
selector engine may reject it. Test labelled/unlabelled rows and near-matching labels.


## VineTheme evidence

Read `docs/subsystems/vinetheme-template.md` before changing Vine consumers. The current
public JSON catalogue and facets are shared; details use the public JSON API and readers use Next.js Flight.
Material flags can remain after protection is disabled; test the normal imageUrl fallback
against protectionConfig rather than rejecting all encrypted/strip metadata.
The reader metadata object is not the payload with pages. Cover/title links duplicate a
series identity inside a card. Portable Vine contracts pin these reports and novel text
references, complete chapter counts, settings defaults, compact cards and source paths.
The merged #888/#884/#882/#860 reviews were reread alongside open #900; its dev-scaffold,
isGenre, item-spacing and separate-migration rules remain enforced on main.
