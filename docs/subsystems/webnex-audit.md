---
title: WebNex audit coverage
type: subsystem
created: 2026-11-20
updated: 2026-11-23
sources:
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/2e52acec2d185e608a3f3f390f5566d91fd7e715/sources/en.webnex/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/2e52acec2d185e608a3f3f390f5566d91fd7e715/sources/en.webnex/src/helpers.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/2e52acec2d185e608a3f3f390f5566d91fd7e715/sources/en.webnex/src/models.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/2e52acec2d185e608a3f3f390f5566d91fd7e715/sources/en.webnex/src/home.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/2e52acec2d185e608a3f3f390f5566d91fd7e715/sources/en.webnex/src/reader.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/2e52acec2d185e608a3f3f390f5566d91fd7e715/sources/en.webnex/src/rsc.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/2e52acec2d185e608a3f3f390f5566d91fd7e715/sources/en.webnex/src/settings.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/2e52acec2d185e608a3f3f390f5566d91fd7e715/sources/en.webnex/src/test.rs
verified_at: 2e52ace
---

This recheck covers the entire WebNex crate after removal of the typed Tags filter.
The root reviewer and the standards agent read every text line; the standards agent
also inspected the opaque 128×128 RGB icon. No additional must-fix source defect was
identified. The independent upstream-comparison agent confirmed the old feature tree
contains no unique runtime work, then hit its usage limit before returning its final
report. Do not count that as a completed second audit.

The attached STANDARDS.md has 185 lines and is byte-identical to
`.agents/skills/aidoku-source/references/submission.md`, SHA-256
`0438f369faaa61edf597935928aac4eaf89f57ceb2ffcb6c4b4a22638e3d0e05`.
The separate reviewer standards and current AGENTS/workflow references were also read.
This records coverage, not a guarantee that code has no undiscovered defects.

## Every attachment line

The ranges below are contiguous and include headings, blank lines and examples.
Pass means the named source or validation evidence satisfies the applicable rule;
N/A means its trigger does not occur in this standalone-source update.

| Attached lines | Point checked | Evidence / applicability |
|---|---|---|
| 1–8 | Scope and headings | Gates and upstream-submission reference identified; distinct from reviewer-rule reference. |
| 9–15 | Offline gates and honest reporting | ci-gates, strict source checks and explicit live/device limitations. |
| 16–20 | Shared hooks | Absolute tooling hooks installed; same checks apply to clean submission checkout. |
| 21–24 | Full source review command | review-source invoked; network failures retained and failing live case retried separately. |
| 25–42 | Agent hooks, wrapper and instruction copies | Current files read, wrapper used, copies synchronized; cloud hooks replaced by explicit gates. No personal settings modified. |
| 43–45 | Deterministic push tests | Nine review_ cases, separate from live suite. |
| 46–73 | Every listed source-rule family | check-sources and warning-failing Clippy; compact cards, static JSON capabilities, key-only reader, valid icon, no suppression or dead Tags parser. Chrono note justified by current-year presets. |
| 74–81 | Portable template contracts | Invoked; WebNex depends on no shared template, so zero applicable template contracts. |
| 82–83 | Template consumers | N/A: no template or consumer changed. |
| 84–90 | Workflows and action runtimes | Existing workflows retained; offline workflow checks and their regression suite run. |
| 91–95 | Checker mutation coverage | Added fail/pass regression for removed typed Tags control; rule tests report zero untested rules. |
| 96–98 | Rule calibration | New Tags guard scoped to en.webnex; calibration reports no merged source failure. |
| 99–111 | Merged feature and naming comparison | Feature survey plus explicit function comparisons in [[webnex]]; no generic naming change without equivalent behavior. |
| 112–116 | Host-reference drift | No host-reference implementation changed. Select defaults, section rendering and reader imports checked against existing host code; no claim of device verification. |
| 117–118 | Dead-code lints | verify-source uses source-lints; removal deletes both Tags control and parser arm. |
| 119–120 | Live suite | Full suite attempted; separate retry retains original network failure evidence. |
| 121–125 | Section boundary and scaffolding | Existing source; new-source scaffold N/A. |
| 126–133 | Clean new-source submission | Canonical upstream parent, one feat commit, version 1, no fork tests/tooling/docs/artifacts. Fork publication remains independently versioned. |
| 134–138 | Review-round history | Current fixes first retained on main; feature history folded into one commit, old head preserved and expected SHA checked. |
| 139–143 | Already-merged updates | N/A: WebNex is not present at canonical upstream base. |
| 144–149 | Final diff and every commit scope | Independent scope check on candidate; compare normalized source tree to tested main snapshot. |
| 150–151 | Standalone rewrite isolation | No consumer migrating away from a template in this task. |
| 152–158 | Template submission exceptions | N/A: only sources/en.webnex in submission diff. |
| 159–164 | Branch names and fork-tool exclusion | feat/en.webnex; source-only submission. Audit documentation and custom checker stay on main. |
| 165–170 | GitHub scope reporting | Existing workflow retained. No repository protection settings changed; local checks independently verify actual candidate. |
| 171–174 | Preserved rebuild | c29f933e old feature head retained before replacement. |
| 175–180 | Unique-work protection | Normalized old feature files exactly match main snapshot dd7f22c9dead06a8949dfe0c58d2a63ec8d045b0; no discard-unmerged override. |
| 181–182 | Versions | Main version 12; submission version 1. |
| 183–185 | Leased force push | Branch update uses explicit expected old SHA; inspect/refuse remote drift. |

## Every source text line

Each range starts at line 1 and ends at the final line. Hashes identify the reviewed
main snapshot contents; the submission differs only by version and removal of the
fork-only test module/file. No generated package is included.

| Relative path | Reviewed lines | SHA-256 |
|---|---|---|
| `.cargo/config.toml` | 1–5 | `3a6fd3fb3b18d0628f1e882fa6b360a9cbdccbd7b0f4e3c6f3e8a10c22bc01d7` |
| `Cargo.lock` | 1–376 | `1c7cb8585e844dd75bfb2a89a1ad6a08aded13e7234c5584ef9d531827b9d079` |
| `Cargo.toml` | 1–26 | `120380333a412fb13bfc47ffcbb71d5355469bf487aa50eb92895dfc22ab2a6b` |
| `res/filters.json` | 1–304 | `279bd5cc42e17f67ee11202d20e8ee0faa57a41b980e7e34f6637fdb775bc758` |
| `res/settings.json` | 1–204 | `6de207c9d69781c8d0f8105e55bfab0c2b27124db54e49461086a8a48f386f44` |
| `res/source.json` | 1–16 | `5bf99fbf7ed18822ef0ed5ac3235a6418eb4e0b57fc586898c06a2f5e64a15e3` |
| `src/helpers.rs` | 1–376 | `f91cf8fe5cf48ad195b77bff3f5fe30cd7401ea3853dc6f1f5f2c7db06a71fc8` |
| `src/home.rs` | 1–167 | `cfe4a68e891d80112545bc397c09a2cc08ceefb01d5e292794ad3e5067e24daf` |
| `src/lib.rs` | 1–168 | `a392237cf230ed440ce87ae3b88b50e375667d61942f2dc41f8872e537699039` |
| `src/models.rs` | 1–314 | `8170d3cc0611c6151c2cf6e566237f2c3e01f09e0a468d514860e92bc406b86d` |
| `src/reader.rs` | 1–34 | `e38021faf36c439b63abd009db772db54e78bd1e3d2c16b4024e8b30de4961d6` |
| `src/settings.rs` | 1–77 | `f911d21d57755bc2e90d28c7083e606562315e6abd3b92e887ca7f63fc5b68a3` |
| `src/test.rs` | 1–717 | `d82d171cacdd03c8eea9ba9975594bd4a2f655619594f487303d72eabf2ae3ff` |

## Function and resource decisions

| Area | Functions / contract checked |
|---|---|
| lib.rs | new; get_search_manga_list; get_manga_update; get_page_list; handle_deep_link; get_image_request; trait registration/test-module isolation. Compact pasted results and fallback, partial details before extra pages, source-only keys, real URLs and Referer. |
| helpers.rs | get_html; parse_response; script_cookie; parse_key; text; markdown; card; date; parse_relative_date; search_params; browse. Checked all matches, guards, loops, errors, query values and parser branches. Typed Tags arm removed. |
| home.rs | layout and get_home. Requested order, page size 5, real chapter keys/dates, compact rows, one homepage request, empty selection before request, rating fallback and separate visibility control. |
| models.rs | Taxonomy/ReaderProps/SeriesRef/ReaderPages; complete Page conversion; details; Upload::priority; uploads; select_chapters. Required titles/keys, plain-text vs Markdown boundaries, raw group names vs official display marker, exclusions before ranking, parts and stable tie-breaker. |
| settings.rs | home_sections; Catalogue::load/ceiling/cookie; Groups::load. Unset vs cleared, same-axis overrides, Rust/JSON defaults, ordered preferences. |
| reader.rs | pages. Direct chapter HTML then pages JSON; script cookie and chapter Referer; all pages converted or error, no WebView polling/cache. |
| resources | Every sort/filter ID and option; no typed Tags control; chapter/year presets selectable via explicit empty default; section choices/defaults; source rating 1 and Safe/Suggestive defaults. |
| manifest/lock/config | Runtime serde_json retained because Flight parsing uses it; standard aidoku/aidoku-test dev scaffold retained; pinned SDK, no_std dependencies, profiles and wasm runner. |
| tests | All 17 test bodies reviewed. Nine deterministic cases and eight live cases; meaningful assertions, offline/live split and no tests copied to submission. |

Merged comparisons include Flame Comics, MangaBall, Kagane, XCOMIC, Atsumaru,
MangaK and VineTheme; the existing [[webnex]] page records counterpart decisions and
site-specific reasons. A JSON catalogue API was not inferred merely from another
Next.js implementation. Existing Keiyoushi/Paperback captures informed cookie,
streamed-placeholder and chapter-group handling.

> ⚠️ Runner results do not establish iOS rendering or cookie/WebView parity. The current
> reader does not use WebView, but image display and filter layout still need a device.
> Featured cards only receive metadata available in the homepage response, not per-card
> genre requests. Home chapter cards expose no group metadata, so group preferences
> apply to full chapter lists rather than home previews.
