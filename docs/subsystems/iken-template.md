---
title: Iken template and consumers
type: subsystem
created: 2026-11-11
updated: 2026-11-12
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
  - code: sources/es.eternalmangas/res/source.json
  - ext: https://github.com/Aidoku-Community/sources/commit/a5b9802a25780ba5db5dbe2590793e88bfd3d355
  - ext: https://github.com/Aidoku-Community/sources/commit/d88a384f99bcccff7e2e4340b5b19bd6ea66210f
  - ext: https://github.com/Aidoku-Community/sources/commit/289b5613bb47f62c6eb91de3d8b2282643586533
  - ext: https://github.com/Aidoku-Community/sources/commit/79f1619cc0d14e853619b4e13d48b5331cd099b1
verified_at: 14fe54dc
---

The template serves Hive Scans, Nyx Scans, Vortex Scans, Magus Manga and EternalMangas.
Shared behavior follows the Params/Impl arrangement used by [[mangak-template]]. The fork
restores these upstream sources temporarily for publication and testing. The submission
branch is `fix-iken`, based on canonical `289b5613`, with production files only.
The submission history separates Eternal's site migration from shared Iken behavior.

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
Shared API and home requests use the same JSON headers, referer and 30-second timeout;
the home document overrides Accept to text/html. Responses are checked before owned JSON decoding.

> ⚠️ The details response can contain an empty chapter list despite a nonzero chapter count.
> Never use it as the complete inventory. Empty pagination before completion or a repeated chapter id anywhere in the inventory
> must fail instead of returning a shortened list (`helpers::chapters`).

> ⚠️ Numeric keys retain existing libraries. Home slug paths resolve to those keys before
> details/deep-link results. Chapter-only updates need no details lookup for numeric keys,
> and reader calls require no cached metadata (`imp.rs`).

> ⚠️ Deep links are already dispatched by the app. Parse their path without requiring the
> configured URL's exact scheme, and decode the chapter slug as well as the series slug.
> Keep the complete chapter suffix. Pasted search links still require the configured base/path boundary.

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

## Review audit, November 12

The submission is based on canonical upstream `289b5613bb47f62c6eb91de3d8b2282643586533`.
Every changed production module and resource was compared with that tree, current fork
instructions, attached snapshots and the merged Keyoapp, Flame, MangaK and MangaThemesia changes.
The submission has three commits: shared Iken fixes, Eternal's site migration, then novel support.
The former fourth audit commit is distributed by responsibility. Shared request timeout imports
already require app 0.8.3 in the first commit; every intermediate consumer must compile.
Submission versions remain upstream + 1 across review rounds. Eternal stays rated 1.

| Merged reference | Applied pattern |
| --- | --- |
| [Keyoapp](https://github.com/Aidoku-Community/sources/pull/888) | Split comma/ampersand credits, trim, omit blanks, deduplicate per role without losing dual roles. Keep optional short synopses. Name shared entry points get_home/parse_home. |
| [Flame](https://github.com/Aidoku-Community/sources/pull/860) | Fallible required identities, details conversion with context, paragraph/emphasis preservation and inaccessible chapter handling. |
| [MangaK](https://github.com/Aidoku-Community/sources/pull/850) | Params/Impl template layout, local overrides, compact cards and metadata conversions in models.rs. |
| [MangaThemesia](https://github.com/Aidoku-Community/sources/pull/884) | Stateful HTML novel formatting, separate reader module and shared-consumer app minimum. |
| Merged NovelArchive, ManhwaWeb and Olympus | Reusable checked request setup and get_json naming; keep error-aware pagination and lookup helpers substantial enough to justify their existence. |

The new Eternal API sample contains comma-separated credits. The prior implementation returned
one combined name. `helpers::names` now follows Keyoapp's verified separators; both roles use it.
The regression also found plain-text API synopses lost by the SDK newline helper's fragment
reparse. Fall back to the selected element's text only when that helper returns None, retaining
paragraph-aware parsing for HTML. No additional conversion path or HTTP request is needed.

Trait forwarding is required by the template. `Post::key` shares required-identity validation;
context-bearing into_details/into_chapter methods match merged patterns rather than artificial
TryFrom tuples. Pagination and post lookup helpers retain meaningful error/404 behavior.
The remaining cover and slug clones preserve values still needed by their callers. No redundant
sorting, dependency, public model surface, suppression, decorative divider or narration was added.

Current kkantan feedback was read in the four linked PRs and the latest 100 repository inline
comments, including compact details/cards, blank keys, independent credit roles, URI/date helpers,
full chapter suffixes, optional values, include-only filters, complete pagination and home order.
The following matrix covers every current section-three feedback row individually. A convention
is not treated as a universal requirement when API evidence or a meaningful context differs.

| Row | Recorded feedback | Iken disposition / evidence |
| --- | --- | --- |
| 1 | Search returned tags + descriptions | Compact TryFrom<Post> and Link::from in search, pasted links and home. |
| 2 | Short synopses disappeared without an expand control (Keyoapp review) | Plain/HTML/empty descriptions covered by deterministic regressions. |
| 3 | Combined credits and duplicate names reached the app (Keyoapp review) | Shared names parser; independent author and artist regressions. |
| 4 | A comment containing "Author" became a title's author (Rage review) | API credit fields, no broad text metadata selector. |
| 5 | A webtoon labeled "Manga" opened right-to-left (Rage review) | API type enum selects viewer; no evidence requiring a site override. |
| 6 | Novel emphasis was stripped to keep an older app minimum (shared-template review) | Separate Markdown reader and app minimum 0.8.3 on all five consumers. |
| 7 | Home rows showed "Manhwa · ★ 8.3" (our own miss, `en.atsumaru`) | Compact ordinary cards; richer data only for the hero. |
| 8 | Big scroller descriptions showed `life\, while` and `\[Domestic` (our own miss, `en.novelarchive`) | Hero uses HTML text; escaping stays on details. |
| 9 | A chapter escaped whole, one paragraph of 600 hard breaks, crashed the app with a stack overflow (our own miss, `en.novelarchive`, crash logs) | Reader escapes syntax only and separates paragraphs; device check remains. |
| 10 | A "chapter" invented for `MangaChapterList` from a feed with no chapters | Batch API supplies actual chapter identities. |
| 11 | A pasted link answered with the full series | Pasted link returns compact Manga with text-search fallback. |
| 12 | Explicit `"default"` on a sort filter | No explicit sort default. |
| 13 | Icon filled the whole canvas | Existing icons unchanged; package validation retained. |
| 14 | Madara’s dark artwork sat on white (#884) | Madara-specific artwork not applicable to these unchanged icons. |
| 15 | A translated-languages filter | Single-language sources; no duplicate language control. |
| 16 | Genre filter presentation | Static genre JSON uses isGenre. |
| 17 | 14 listings | No redundant listing registration. |
| 18 | A helper sat in `lib.rs` | Free helpers live outside lib.rs. |
| 19 | `String::new()` / `Vec::new()` everywhere | Default structs retained; no repeated empty initialization. |
| 20 | Temporary vectors before assigning to params | Direct QueryParameters assignment. |
| 21 | Settings lookups outside `settings.rs` | No persistent settings in this template. |
| 22 | `page_size: Some(10)` on home components | Latest five, popular three. |
| 23 | Latest Comics used `page_size: None` when it was meant to show five rows (owner's correction) | Explicit latest page_size Some(5). |
| 24 | Web view urls 404'd | API slug URLs; fallback chapter post slug when details are absent. |
| 25 | `language: Some("en")` on every chapter | No chapter language assignment. |
| 26 | "Does this not support the filtered types setting?" on the latest feed | No content-preference settings requiring feed propagation. |
| 27 | Dynamic genres without `supportsTagSearch` | Static genres; no fetched list or dynamic tag capability. |
| 28 | The "Has Chapters" check sat in the filters header | No secondary filter toggles. |
| 29 | `query.filter(\|q\| !q.is_empty())` | No redundant optional query emptiness guard. |
| 30 | A big scroller with only titles and covers | Hero fills metadata available in HTML without per-card requests. |
| 31 | Everything in `lib.rs` | Traits/lib, behavior/imp, DTOs/models, network/helpers, home and Markdown modules. |
| 32 | `fn into_manga(self)` / hand-built `Chapter` in a loop | TryFrom for compact cards; context-bearing details/chapter conversion matches Flame. |
| 33 | `AidokuError::message(..)`, `.map_err(\|_\| ..)` on `Request::post` | error!/bail!; network errors propagated. |
| 34 | `.string()?` then `serde_json::from_str` | Owned response JSON; Hive embedded script is not an HTTP JSON response. |
| 35 | A hand-written ISO date parser | SDK parse_date_with_options UTC conversion. |
| 36 | HMAC signing in `JsContext` | No signing or JS context. |
| 37 | `Home` registered returning `Unimplemented` | Home implemented for every registered consumer. |
| 38 | `if !needs_details && !needs_chapters` | No impossible neither-flag guard. |
| 39 | `query.as_deref().map(\|q\| !q.is_empty())` | No optional query map-to-bool. |
| 40 | `if !included.is_empty()` in a multi-select arm | Include-only genre arm sends joined values directly. |
| 41 | `url.starts_with(base)` then slicing | strip_prefix for pasted link base boundary. |
| 42 | Matching status strings as-is | Captured API uses uppercase enums; keep exact enum matching, unlike variable HTML labels. |
| 43 | A name→id map in Rust for a filter | Genre ids in JSON. |
| 44 | `canExclude` left on where the API can't exclude | Explicit canExclude false throughout. |
| 45 | A description that can be `""` | Descriptions trim and filter empty text. |
| 46 | Two code paths for query and no query | One query endpoint for search and browse. |
| 47 | Listings that are really search options (Uncensored, Raw) | Type/status are filters, not redundant listings. |
| 48 | Scraping `__NEXT_DATA__` when an API exists | API used for catalogue/details/chapters; HTML only supplies actual home presentation. |
| 49 | A source on a known theme written from scratch | All five consume Iken with small overrides. |
| 50 | A rebranded site added under a new id | Existing source ids retained. |
| 51 | `Vec<Regex>` built per call | No regex allocation. |
| 52 | Pages silently dropped when a per-page key fails | Missing/blank image pages fail; no filter_map reader truncation. |
| 53 | A per-item cache on the source struct | No cache state. |
| 54 | `"HQ-now"`, `struct Theblank` | Eternal struct renamed to match its own source. |
| 55 | No `minAppVersion` | All consumers declare 0.8.3. |
| 56 | Comments marked resolved without a change | No PR threads posted or resolved by this audit. |
| 57 | `#[serde(rename_all = "camelCase")]` on JSON that isn't ("have you verified this?", #846) | camelCase DTO fields checked against current API captures. |
| 58 | `sort` and `order` sent to an endpoint that ignores them ("this doesn't seem to do anything", #846) | Eternal unsupported sorts removed after direction probes; supported Params stay local. |
| 59 | The sort filter built last, titled "Sort" (#846) | Sort first. |
| 60 | `hideNsfw` defaulting to `false` with `contentRating` 1 (#845) | Rating 1 preserved by explicit owner decision; no hide-adult setting exists. Do not claim opt-in filtering. |
| 61 | Discover sections in a different order from the site (#845) | DOM order preserved; localized row headings and novel placement. |
| 62 | `.unwrap_or("")` repeated down a chain (zh.suduguu, November: "keep this an optional result. it doesn't make sense for callers to check if the result is an empty string rather than `None`") | Missing metadata stays optional. |
| 63 | A page limit ending a loop with `break` (zh.suduguu, November: "it would be better to return an error here") | Incomplete/overlapping chapter inventory returns an error. |
| 64 | Genre exclusion offered on Genz Toons, whose site and keiyoushi only include genres ("are genres even excludable in genz site?", owner's correction) | Genres include only; unsupported exclusions disabled. |
| 65 | Eva Scans rated 17+ because its genre list has Adult, Smut and Mature (our own miss, owner's correction) | Existing catalogue ratings retained; per-title rating computed independently. |
| 66 | Scans.GG rated 0 though its genres include Hentai (kkantan: "given that there is a 'Hentai' genre and such, this should be marked as containing nsfw content", 0 to 1) | Eternal remains 1; English catalogues remain 0 per reviewed inventory. |
| 67 | Comics and novels behind a source setting (#842, also dyphire) | Novel type filter and feed, no catalogue-switch setting. |
| 68 | `page_url_hd` dropped in a rework (#842) | Original cover fallback and all image pages retained. |
| 69 | `requiresFalse: "contentType == novel"` on a setting (#842) | No dependent settings. |
| 70 | A slug looked up by searching the whole series list each time (#840) | Stable numeric keys and direct numeric lookup; no full-catalogue slug search. |
| 71 | `ImageScroller` with `width: None` (#819) | Hive script banners specify 400 by 150. |
| 72 | A banner linking a series page with `LinkValue::Url` (#819) | Series banners link internally. |
| 73 | The two newest chapters of a long series didn't load (#819) | Live source tests read available chapters; newest device reading remains unverified. |
| 74 | A chapter that failed to load returned "(empty chapter)" text (#819, fix commit) | Empty/locked reader content returns meaningful errors. |
| 75 | Missing titles became empty strings (#865) | Blank keys/slugs/titles rejected by shared fallible validation. |
| 76 | Rechecking the host in a deep-link handler (#869) | Dispatched deep links parse path; pasted search checks base boundary. |
| 77 | Chapter ids with suffixes were truncated (#872) | Complete decoded chapter suffix retained. |
| 78 | Extra helpers and long explanations in `lib.rs` (#872) | Only traits/template setup in lib; six existing why comments calibrated. |
| 79 | A contributor with both writing and drawing credits lost one role (#865) | Author and artist parsed independently. |
| 80 | `impl From<Entry> for Manga` with `key(..).unwrap_or_default()` ("we don't want to return manga objects with empty keys") | TryFrom returns identity error, details propagates it. |
| 81 | A "Show Locked Chapters" switch defaulting on | No locked-chapter toggle; inaccessible entries are omitted. |
| 82 | A home row and listing titled "New in the Archive" for the library's default sort ("does this title come from somewhere?") | createdAt feed named New Releases; existing site headings preserved. |
| 83 | Genz Toon's cover slider showed as cropped banners in the app, while the runner and its `review_` test read it as covers (owner's screenshots) | No quoted bracket substring selector; app selector parity still requires device evidence. |
| 84 | V2 Trending Now was overridden with Month despite the site's Today default (owner's correction) | Keyoapp-specific periods not present in Iken. |
| 85 | Newly Added fell back to a document-wide grid (owner's review) | Each heading uses its bounded own card scope. |
| 86 | The Keyoapp V1 section ancestor bound was widened from three to five without evidence (owner's correction) | Three-level traversal; regression prevents neighboring-row borrowing. |
| 87 | Blocked requests looked like missing catalogues or invalid JSON (owner’s CI log) | Checked shared responses; loopback covers HTTP diagnostics and bounded body prefix. |
| 88 | `SEARCH_URL`, `BASE_URL` plus a path and only ever followed by `?` | Only API/base constants; single-use paths inline. |
| 89 | `manga_page_url`, `chapters_url`, `read_url` over an `api_url` helper, each used once | No one-use endpoint wrapper chain; substantive request/post/pagination helpers retained. |
| 90 | Five Typesense parameters as constants, each sent once | No one-use Typesense constants. |
| 91 | `FEED_LIMIT`, `SEARCH_LIMIT` for page sizes | PAGE_SIZE for catalogue page size. |
| 92 | `fn is_novel(&self) { is_novel(self.medium…) }` | No field-only is_novel forwarding method. |
| 93 | `aidoku::imports::std::current_date()` in an expression | No fully qualified current_date expression. |
| 94 | Deduplication discarded same-number parts from one scanlator | No group deduplication or group-preference controls; pagination duplicate IDs are errors. |
| 95 | The novels row mapped search hits to cards a second way | Novel feed reuses compact TryFrom + Link::from. |
| 96 | A comment on the fallback sort described the feeds | Existing comments reread beside their actual code. |
| 97 | Full `mediums`/`types` lists sent to the feeds | No full-selection media setting; one optional type filter. |
| 98 | Ranking periods defaulted to a week and applied to the home page | No ranking-period setting. |
| 99 | `actions/cache@v4` (and `checkout@v4`, `upload-artifact@v4`, a pages action) on Node 20 | Submission changes no workflows; main offline workflow checker verifies repository actions. |
| 100 | `aidoku-test-runner` install failed on GitHub's runner | Runner available; current workflow dependency checker retained. |

## Audit verification

Deterministic template regressions include credit splitting, short/HTML/empty descriptions,
blank required identities, independent locks, numeric strings, compact cards, bounded home
sections, actual chapters and balanced novel emphasis. Loopback tests cover non-2xx responses,
key-only reading, incomplete/overlapping chapter pages and dispatched deep-link variants.
Current API captures confirm all 356 static genre IDs/names across Nyx, Vortex, Magus and
Eternal; whitespace in labels matches the API. Hive's direct API returned HTTP 403.

The attached checker is stale: current main permits explicit canExclude=false and requires
complete synchronized instruction copies. Attached pr-branch.sh also predates test/dependency
stripping and template scope validation. Use current repository scripts.

Fresh results: all 26 consumer tests passed (20 live tests and six deterministic source checks).
All five fork reviews passed formatting, production/test Clippy with warnings denied, packaging,
Aidoku verification and source rules. All five normalized submission packages passed those
production gates too; both intermediate shared and Eternal snapshots compiled cleanly.
The final submission scope/history check reports exactly three commits, with no fork tests,
tooling or unused test dependencies. Its final tree equals the preserved audited snapshot.

Eight deterministic template regressions passed. The loopback suite passed its three active
HTTP/reader/layout tests; the optional captured-home entry returned without exercising captures
because no capture directory was configured. Repository checks passed 132 Python regressions,
nine offline workflow checks, instruction-copy synchronization and documentation validation.
Direct Hive API access returned 403, while the actual runner-based Hive suite passed.
Runner results do not establish app rendering, WebView sessions or selector parity. Those
remain device checks.

| Consumer | Fork publication version | Submission version | Catalogue rating |
| --- | --- | --- | --- |
| Hive | 10 | 3 | 0 |
| Nyx | 9 | 2 | 0 |
| Vortex | 11 | 4 | 0 |
| Magus | 10 | 3 | 0 |
| Eternal | 11 | 3 | 1 |

The [submission branch](https://github.com/PoppingMangoSources/popmango-aidoku-sources/tree/fix-iken)
was published with an explicit expected-SHA force lease. Its three commits keep shared fixes,
Eternal's site migration and novels separate. Fork regressions and this record remain on main.
