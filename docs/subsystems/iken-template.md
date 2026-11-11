---
title: Iken template and consumers
type: subsystem
created: 2026-11-11
updated: 2026-11-11
sources:
  - code: templates/iken/src/lib.rs
  - code: templates/iken/src/imp.rs
  - code: templates/iken/src/models.rs
  - code: templates/iken/src/helpers.rs
  - code: templates/iken/src/home.rs
  - code: templates/iken/src/markdown.rs
  - code: templates/iken/src/test.rs
  - code: templates/iken/src/source_test.rs
  - code: sources/en.hivescans/src/helpers.rs
  - ext: https://github.com/Aidoku-Community/sources/commit/a5b9802a25780ba5db5dbe2590793e88bfd3d355
  - ext: https://github.com/Aidoku-Community/sources/commit/d88a384f99bcccff7e2e4340b5b19bd6ea66210f
verified_at: e07a9ac
---

The template serves Hive Scans, Nyx Scans, Vortex Scans, Magus Manga and EternalMangas.
Shared behavior follows the Params/Impl arrangement used by [[mangak-template]]. The fork
restores these upstream sources temporarily for publication and testing. The submission
branch is `fix-iken`, based on canonical `4ac8414`, with production files only.

## API and keys

All five consumers explicitly set both base_url and api_url, as in the merged MangaK template.
Search uses `/api/query` with page/perPage, archive view, orderBy/orderDirection, seriesType,
seriesStatus and included genreIds. The default sorts are lastChapterAddedAt, totalViews, createdAt,
chaptersCount and postTitle. Eternal honors updatedAt, totalViews, createdAt and postTitle, so its Params and static
control expose those four. Its first option is last modification, rather than claiming
chapter-update ordering. Static genres come from each site's `/api/genres` response. Eternal also exposes the
verified adult/BL comic types from its current browse page and API. The native BL Manhwa
spelling is not an accepted API enum; MANHWA_BL is the working value.
Exclusion is disabled. Details use `/api/post?postId=` or postSlug; chapters use
`/api/chapters?postId=&skip=&take=` and totalChapterCount. Reading uses chapterId alone.
Four APIs were reachable directly on November 11; Hive's API returned 403. All five public
home pages were available through Firecrawl. Direct home requests returned 403.

> ⚠️ The details response can contain an empty chapter list despite a nonzero chapter count.
> Never use it as the complete inventory. Empty pagination before completion or a repeated chapter id anywhere in the inventory
> must fail instead of returning a shortened list (`helpers::chapters`).

> ⚠️ Numeric keys retain existing libraries. Home slug paths resolve to those keys before
> details/deep-link results. Chapter-only updates need no details lookup for numeric keys,
> and reader calls require no cached metadata (`imp.rs`).

> ⚠️ Lock flags are independent. `false.or(Some(true))` loses a time lock. Explicit accessible
> responses override the price fallback, not a true lock flag (`IkenChapter::locked`).

## Home and novels

Home requests the HTML, New Releases, Novels and one chapter-metadata API feed concurrently. Newly added series use
createdAt descending, and their compact row sits before Most Popular/Tendencias. The
consumer can localize both headings. Novels uses seriesType=NOVEL and the consumer's verified default sort. isNovel is ignored by these APIs and is never sent. Empty novel
feeds are omitted; an existing novel component keeps its original position. Failed rows are omitted; all failures return an error.
Home reads real HTML cards, recognizes verified headings in DOM order and bounds the ancestor
walk before a neighboring section. It does not construct artificial chapters or fetch each
card. Latest Releases/Latest Updates/Últimas Actualizaciones uses chapter rows with five rows per page.
One `/api/posts?page=1&perPage=100` batch resolves real series/chapter IDs while preserving the HTML row order, including pinned titles. It selects the first accessible chapter and uses becameFreeAt, updatedAt, then createdAt for its timestamp. If metadata cannot resolve the complete row, the original series list remains intact.
Most Popular/Tendencias is a ranked list with three rows per page.
Looped featured slides are ordered by their slide indexes and deduplicated. Vortex's original
cover fallback is preferred over a wide banner. Eternal uses its own Spanish headings and
hero selector. Hive's custom four-banner script data is parsed only in that consumer.

> ⚠️ A blank section must not borrow cards from its next section. Bounded ancestor traversal
> and neighboring-heading detection are pinned by `review_home_order_compact_cards_and_section_boundaries`.

> ⚠️ Novel text preserves paragraphs and inline emphasis, including adjacent mixed spans.
> Only Markdown syntax is escaped, keeping long prose from multiplying inline nodes.
> `child_nodes` requires minAppVersion 0.8.3 on all five packages. Device rendering remains
> a separate check. This follows the reader work described in [[flamecomics]].

## Verification

`templates/iken/src/test.rs` contains deterministic model/layout/novel tests and loopback
HTTP/reader/pagination tests run by `scripts/test-http.py templates/iken`. Set
AIDOKU_IKEN_EVIDENCE_DIR to a local directory of captured home HTML to additionally exercise
current layouts without sending captures elsewhere. Source tests share a fork-only fixture
in `templates/iken/src/source_test.rs` and run static contracts separately from live suites.
Do not include these test files or their unused dev dependencies in the submission.

Hive stores the title `crocodile` in lowercase in both the HTML and public API; preserve its stored spelling rather than changing every catalogue title.

Reference clients reviewed: Keiyoushi Iken, InkDex HiveToons, LucifersCircle Mana and Pirate
Vodka VortexScans, Seyden VortexScans/Iken, CorruptBytes VortexScans and Kitty NyxScans/vcomics.
Their endpoints, sort ids, pagination and access flags are leads confirmed against API/HTML
responses. Their JS state parsers, caching and authentication scaffolding are not needed for
these public API paths. See [[source-checks]] for the repository gates.

> ⚠️ Eternal silently ignores lastChapterAddedAt and chaptersCount sorting. Reversed direction
> probes returned the same fallback order. Do not re-enable these options without fresh evidence.

## Review audit

The November 11 audit compared every production file changed by `fix-iken` with canonical
`4ac8414`, the current main instructions and attached snapshots. MangaK supplied the Params/Impl
and conversion reference; Flame and NovelArchive supplied the HTML emphasis references.
The recent reviewer threads checked were 890, 889, 884, 883, 881, 882, 872 and 869.

| Area | Outcome |
| --- | --- |
| Layout, names and helpers | Trait forwarding remains in lib.rs, behavior in imp.rs, DTO conversions in models.rs, network operations in helpers.rs, and the substantial home/reader logic in their own modules. Removed the unused API fallback accessor. |
| Ownership and duplication | Reuse request URLs, decode Hive banner JSON while its script buffer is borrowed, and avoid duplicate identity validation and unnecessary slug clones. The chapter date fields use take because the remaining chapter is subsequently consumed by its normal conversion. |
| Pagination and errors | Reject duplicate ids within or between pages, including overlaps after the first entry. Loopback regressions retain incomplete-page, HTTP, missing-page and key-only reader checks. |
| Cards and filters | Compact cards, blank required identity rejection, sort first, static tag taps, explicit unsupported exclusions and source-local sort/type overrides remain. No settings or dynamic-genre machinery is needed. |
| Home and reader | Preserve pinned HTML order, real chapter ids, latest five/popular three, source-local banners, partial success and separate novel support. Both emphasis nesting orders have deterministic coverage. |
| Submission | Keep the two requested commits, upstream+1 submission versions, and no fork tests, docs or unused test dependencies. |

Before this cleanup, all five already-published consumers passed their live suites: Hive 6,
Nyx 5, Vortex 5, Magus 5 and Eternal 5. The novel test reports a catalogue with no novels or
accessible chapters explicitly; a passing suite does not mean every site currently has novels.
App layout, WebView sessions and device novel rendering require separate device evidence.
