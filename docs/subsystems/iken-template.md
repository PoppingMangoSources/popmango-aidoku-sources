---
title: Iken template and consumers
type: subsystem
created: 2026-11-11
updated: 2026-11-13
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
  - code: sources/es.eternalmangas/src/helpers.rs
  - code: sources/es.eternalmangas/src/home.rs
  - code: sources/es.eternalmangas/src/test.rs
  - code: sources/es.eternalmangas/res/source.json
  - code: sources/en.hijalascans/src/lib.rs
  - code: sources/en.hijalascans/res/filters.json
  - code: sources/en.hijalascans/Cargo.toml
  - code: sources/en.sanascans/src/lib.rs
  - code: sources/en.sanascans/res/filters.json
  - code: sources/en.sanascans/Cargo.toml
  - ext: https://github.com/Aidoku-Community/sources/pull/900#pullrequestreview-5897319010
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/lib-multisrc/iken
  - ext: https://github.com/LucifersCircle/pirate.vodka-extensions/tree/0.9/stable/src/Hijala
  - ext: https://github.com/kittycatgit/kitty-extensions-0.9/tree/main/src/SanaScans
  - ext: https://github.com/Aidoku-Community/sources/commit/a5b9802a25780ba5db5dbe2590793e88bfd3d355
  - ext: https://github.com/Aidoku-Community/sources/commit/d88a384f99bcccff7e2e4340b5b19bd6ea66210f
  - ext: https://github.com/Aidoku-Community/sources/commit/289b5613bb47f62c6eb91de3d8b2282643586533
  - ext: https://github.com/Aidoku-Community/sources/commit/79f1619cc0d14e853619b4e13d48b5331cd099b1
verified_at: 73fde51
---


Iken serves Hive, Nyx, Vortex, Magus, Hijala and Sana through the Params/Impl arrangement
used by [[mangak-template]]. Eternal now has an independent current-site HTML implementation.
The production-only `fix-iken` submission is refreshed onto canonical `a607bebe`; four commits
separate shared fixes, novel settings, Hijala and Sana. Eternal's independent migration is one
commit on `fix-eternal` from the same canonical base. Fork tests and tooling stay on main.

## API, keys and current controls

The shared API uses `/api/query`, `/api/post`, `/api/chapters` and `/api/chapter`.
Numeric library keys survive; home paths resolve by slug, including complete URL-decoded chapter
suffixes. Numeric chapter-only updates avoid a details request. Reader calls need no cached metadata
(`templates/iken/src/imp.rs:130`). Checked responses retain bounded status/body diagnostics.

> ⚠️ Post details can omit chapters despite a nonzero total. Collect skip/take chapter pages until
> totalChapterCount is reached; empty or overlapping pages must error. Independent lock flags stay
> independent; accessible overrides only the price fallback (`templates/iken/src/helpers.rs:60`).

Hijala and Sana use the public API directly; no JS state decoder or cache is necessary.
Current source controls include sort, tri-state genres, single status, multiple types,
chapter count and creation-year ranges. Earlier site probes verified saleFilter (omitted from the source under the commercial-control
policy), comma-separated seriesType and single seriesStatus. Positive and impossible API probes
established actual narrowing and genre exclusion. Existing four consumers retain include-only controls.
Sana sets page_size=30 and filename page sorting, matching its current Keiyoushi override;
Hijala uses 18 results. Both use official site icons with an opaque 128-pixel canvas and margin.

> ⚠️ A full-catalogue first response is terminal even when it exceeds perPage. Compare response
> length with totalCount as well as the requested offset (`templates/iken/src/imp.rs:75`).

## Home and novels

Home concurrently requests HTML, createdAt New Releases, NOVEL results and one metadata batch.
HTML determines the existing section order and real chapter rows. Latest Releases uses MangaChapterList with page_size=5 when chapters are present. The separate
paid Latest Updates row is excluded while still bounding adjacent sections.
Pinned entries absent from the first metadata page survive with readable path keys, chapter
numbers and HTML timestamps. Relative footer dates are supported, and multiple chapter updates
for the same series remain separate (`templates/iken/src/home.rs:170`).
Optional batch enrichment may replace a matching chapter with numeric IDs; a missing match must
never downgrade a chapter row to covers. There is no per-card request fan-out. Hive alone keeps its
custom script banners. Popular rows use three entries per page. Ordinary cards stay compact.

> ⚠️ Bound each heading's ancestor traversal before another section. A blank row must never borrow
> the next row's cards; the deterministic home tests pin these boundaries.

> ⚠️ Novel paragraphs and inline emphasis remain separate and balanced. Escape Markdown syntax,
> not ordinary punctuation. All consumers declare minimum app 0.8.3 for child_nodes imports.
> Actual reader rendering remains a device check; see [[flamecomics]].

## Eternal migration

The former JSON catalogue is stale: the reported miracle title is absent, while the current site
has chapters. The reported divorce title has more chapters on the site than its old API record.
The new implementation reads current HTML catalogue, details, chapters and reader content.
Old numeric keys use the API only to recover the slug, then load current HTML. They retain their
original library key (`sources/es.eternalmangas/src/helpers.rs:52`). Current descriptions use the
site's displayed synopsis, avoiding literal escaped quotes and carriage-return strings.
The original EM logo replaces the old icon. Catalogue contentRating is **2**, explicitly requested
on November 12; this supersedes earlier rating-1 instructions. Individual title ratings remain separate.

> ⚠️ Search results being plausible does not establish API freshness. Keep positive regression
> fixtures for both reported titles and compare chapter inventories with current HTML.

> ⚠️ The .org site can redirect through sistemasoperativos.info. Keep canonical series paths in
> keys and .org web links. Reader images use original data-src before src, never placeholder pages.

The current HTML archive supports query/type/status/one genre, with no visible sort control.
Removed old API sort options and genre IDs must not be restored without fresh evidence.

## Review evidence and verification

The earlier audit against merged Keyoapp, Flame, MangaK and MangaThemesia remains recorded in
history. Applied patterns include fallible identities, per-role unique credit names, short/HTML
synopses, compact Link::from cards, local Params differences and a separate Markdown reader.
See root section 3 for the individual review rules and [[source-checks]] for automatic gates.
Named Hijala and Sana clients plus Keiyoushi were read, then their endpoints and controls checked
against current HTML, API pages and browser network requests. Their caches and state parsers are
not needed for the verified public endpoints.

Deterministic template tests cover chapter rows, pinned metadata misses, repeated-series updates,
relative timestamps, credit splitting, synopses, identities, locks and novel formatting. Loopback
HTTP tests cover errors, complete decoded chapter paths, key-only reading, filename ordering and
pagination failures. Optional captured-home tests require AIDOKU_IKEN_EVIDENCE_DIR; a missing
directory does not count as captured-site evidence. Live home suites check every latest component.

The prior full live suites passed for Eternal, Hive, Vortex, Magus, Hijala and Sana. Nyx passed four tests,
but the original sort/pagination run had a network request failure. Its retry instead failed an
exact equality assertion on two latest-update snapshots: the same title moved to the front.
Filter-clearing comparisons now use alphabetical order for both requests. Sort request errors
identify the sort index and direction. The corrected follow-up passed all five Nyx tests,
including all sort directions, paging, clearing, novels, home and chapter reading.
This does not establish a global outage and does not justify removing the assertion or hiding
its failure. Deterministic checks and publication gates remain separate from live availability.
No runner result establishes app selector parity, cookies, WebView sessions or device rendering.

| Consumer | Fork version | Submission version | Catalogue rating |
| --- | --- | --- | --- |
| Hive | 14 | 3 | 0 |
| Nyx | 13 | 2 | 0 |
| Vortex | 15 | 4 | 0 |
| Magus | 14 | 3 | 0 |
| Eternal | 15 | 3 | 2 |
| Hijala | 5 | 1 | 0 |
| Sana | 5 | 1 | 0 |


## Nyx reference follow-up

Current Keiyoushi Nyx extends Iken and enables hideNovelsListedAsComics. That option compensates
for its comic-only reader. This source supports novel content, so copying that exclusion would
remove supported reading. Its five sort fields, API host, genre query encoding and chapter-ID
reader requests match ours. Kitty's Nyx extends vcomics; the shared engine also uses the public
query/post/chapters/chapter routes. Its 27-item page size is a client choice, while Keiyoushi
uses 18. Its additional home rails and views/averageRating sorts are not evidence that those
sorts work on this site. No production change is warranted from these differences alone.

References checked November 12:
[Keiyoushi Nyx](https://github.com/keiyoushi/extensions-source/blob/main/src/en/nyxscans/src/eu/kanade/tachiyomi/extension/en/nyxscans/NyxScans.kt),
[Keiyoushi Iken](https://github.com/keiyoushi/extensions-source/tree/main/lib-multisrc/iken),
[Kitty Nyx](https://github.com/kittycatgit/kitty-extensions-0.9/tree/main/src/NyxScans) and
[Kitty vcomics](https://github.com/kittycatgit/kitty-extensions-0.9/tree/main/src/vcomics).

> ⚠️ Live catalogue order can change while a multi-request test runs. Filter clearing needs a
> stable sort rather than an earlier latest-update snapshot. Fork-only regression changes do
> not require package version bumps; a simultaneous runtime change still does.


## Hijala and Sana audit

Reviewed every introduced source Rust file, resource, manifest and fork test against the current
root rules, matching skill references, attached snapshots, main scripts/workflows and canonical
upstream `289b5613`. The shared search/filter, page-size and image-order additions were read with
their callers and consumers (`templates/iken/src/imp.rs:19`, `templates/iken/src/imp.rs:159`).
The consumer wiring lives at `sources/en.hijalascans/src/lib.rs:8` and
`sources/en.sanascans/src/lib.rs:8`; static controls stay in each res/filters.json. No additional production wrapper, parser, dependency or naming change
was needed. Their source files contain no narrative comments or decorative dividers.

| Area | Merged comparison and result |
| --- | --- |
| Source names, traits and file split | Existing Hive/Iken and MangaK consumers: minimal lib.rs trait wiring, site constants, direct Params construction and the shared generic engine. Both retain their site names and ids. |
| Site differences | Madara/MangaK Params defaults: Sana sets only page_size=30 and sort_pages_by_filename=true. Hijala inherits defaults. No new override chain or site-only parser. |
| Queries and filters | MangaK/MangaThemesia match FilterValue directly and use QueryParameters. Iken sends the verified CSV genre/type values, excludedGenreIds and actual range/availability fields. Static genres keep ids in JSON; exclusions are disabled for types and enabled only for the verified genre control. |
| Pagination and reading | Original Iken uses API totalCount and ordered images. Sana's demonstrated filename exception uses one inline numeric sort, without adding a regex dependency, cache or extra helper. A loopback case distinguishes page 2 from page 10 and verifies key-only reading. |
| Models, details and errors | Flame's fallible identity and existing Iken DTOs/conversions: reused compact TryFrom cards, owned response decoding, checked HTTP errors and independent lock flags. No duplicate source-specific models. |
| Review feedback | Root section 3 checked for applicable compact-card, credits, synopsis, novel, filters, keys, error, comment, request-cost, layout, rating and app-minimum rules. No extra listings, settings or per-card fetches. Static raw genre labels match API detail tags, preserving host tag-to-filter lookup. |
| Tests and tooling | Consolidated the identical advanced-filter tests into the existing shared fixture. Positive included/excluded genre assertions require actual detail tags. The HTTP contract checks all advanced parameters, non-ASCII encoding and 18/30-item paging. Fork tests, dev dependencies and tooling remain outside the five production submission commits. |

Source-scoped review gates cover formatting, production/test Clippy with warnings denied,
packaging, schema/icon/WASM verification, source checks and current live suites. SDK/runner checks
cannot establish device rendering or app selector parity; those remain explicit manual checks.


Both source-scoped reviews passed all five current tests after test consolidation, plus
formatting, production/test Clippy, packaging and Aidoku verification. The final positive
included/excluded-genre checks were rerun after strengthening their assertions. Static resources
match all 55 Hijala and nine Sana API genre id/name pairs. The five-commit submission still
passes independent scope checks; no runtime changes or additional PR commit were required.


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

### Icon assets

Use the original header logos, downsampled with margin onto the site's dark background,
rather than enlarging favicons. Verified natural sizes on November 12, 2026:

- [Hijala logo](https://storage.en-hijala.com/upload/2025/06/24/final-523b7e2e4fb3a659.webp): 751×751.
- [Sana logo](https://media.sanascans.com/upload/2025/08/07/1000301312-836f79641ae0985a.webp): 2354×2564.

Final package icons remain opaque 128×128 PNGs, with the artwork fitted inside 112×112.


## November 13 review corrections

All seven inline comments and the review summary were checked individually: Hijala/Sana
item/method spacing, Hive method spacing, Eternal helper spacing, Eternal isGenre, the optional
metadata selector simplification and retained standard dev-dependencies. Changes are folded
into the existing four Iken commits and the separate Eternal migration commit. No review-fix
commit is appended. The Iken source/template dev scaffolds are restored, their lockfiles are
synchronized, and main keeps the fork tests.

> ⚠️ Until the independent migration merges, canonical Eternal still consumes Iken. Its old
> Params initializer needs the new API URL type and its package needs app 0.8.3. These minimal
> compatibility changes and the normal submission bump remain with Iken; the independent
> migration does not. Rebase the remaining branch after either PR merges.

Eternal metadata rows now use :has(strong) and an exact label-prefix match, keeping selectors
inside .col-lg-3. Fixtures cover an unlabelled row, a near-matching label and metadata outside
the sidebar. The genre resource keeps isGenre; a deterministic test guards that flag.
Automatic guards cover reviewed spacing, preservation of standard test dependencies, the
standalone source branch alias and rejection of independent migrations inside template PRs.
Spacing calibration found merged counterexamples, so unrelated files receive notes rather
than failures. The standard scaffold rule preserves upstream entries instead of inventing
an unconditional dependency requirement for all older sources.


Review verification: the Eternal selector/genre and existing deterministic fixtures pass;
the template's deterministic suite and loopback HTTP tests pass. Captured-home tests still
need their optional evidence directory, so the loopback run does not establish fresh captured
site rendering. The seven fork source versions advance once for this publication; submission
versions remain upstream plus one (new Hijala/Sana remain 1).
