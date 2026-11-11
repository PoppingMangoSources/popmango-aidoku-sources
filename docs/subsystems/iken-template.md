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
verified_at: 779de0f
---

The template serves Hive Scans, Nyx Scans, Vortex Scans, Magus Manga and EternalMangas.
Shared behavior follows the Params/Impl arrangement used by [[mangak-template]]. The fork
restores these upstream sources temporarily for publication and testing. The submission
branch is `fix-iken`, based on canonical `4ac8414`, with production files only.

## API and keys

Search uses `/api/query` with page/perPage, archive view, orderBy/orderDirection, seriesType,
seriesStatus and included genreIds. The sorts are lastChapterAddedAt, totalViews, createdAt,
chaptersCount and postTitle. Static genres come from each site's `/api/genres` response.
Exclusion is disabled. Details use `/api/post?postId=` or postSlug; chapters use
`/api/chapters?postId=&skip=&take=` and totalChapterCount. Reading uses chapterId alone.
Four APIs were reachable directly on November 11; Hive's API returned 403. All five public
home pages were available through Firecrawl. Direct home requests returned 403.

> ⚠️ The details response can contain an empty chapter list despite a nonzero chapter count.
> Never use it as the complete inventory. Empty or repeating pagination before completion
> must fail instead of returning a shortened list (`helpers::chapters`).

> ⚠️ Numeric keys retain existing libraries. Home slug paths resolve to those keys before
> details/deep-link results. Chapter-only updates need no details lookup for numeric keys,
> and reader calls require no cached metadata (`imp.rs`).

> ⚠️ Lock flags are independent. `false.or(Some(true))` loses a time lock. Explicit accessible
> responses override the price fallback, not a true lock flag (`IkenChapter::locked`).

## Home and novels

Home reads real HTML cards, recognizes verified headings in DOM order and bounds the ancestor
walk before a neighboring section. It does not construct artificial chapters or fetch each
card. Latest Releases is a compact scroller because the HTML links carry chapter slugs, not
stable chapter ids. Most Popular/Tendencias is a ranked list with five visible entries.
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

Reference clients reviewed: Keiyoushi Iken, InkDex HiveToons, LucifersCircle Mana and Pirate
Vodka VortexScans, Seyden VortexScans/Iken, CorruptBytes VortexScans and Kitty NyxScans/vcomics.
Their endpoints, sort ids, pagination and access flags are leads confirmed against API/HTML
responses. Their JS state parsers, caching and authentication scaffolding are not needed for
these public API paths. See [[source-checks]] for the repository gates.
