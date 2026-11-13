---
title: MangaBall (multi.mangaball)
type: subsystem
created: 2026-11-13
updated: 2026-11-13
sources:
  - code: sources/multi.mangaball/src/api.rs
  - code: sources/multi.mangaball/src/home.rs
  - code: sources/multi.mangaball/src/lib.rs
  - code: sources/multi.mangaball/src/models.rs
  - code: sources/multi.mangaball/src/helpers.rs
  - code: sources/multi.mangaball/src/settings.rs
  - code: sources/multi.mangaball/res/filters.json
  - code: sources/multi.mangaball/res/settings.json
  - ext: https://api.github.com/repos/Smexhy/yomu-aidoku-sources/git/blobs/76bed84198f3eb07c7f2124faedbe0de7301a7f2
  - ext: https://github.com/Aidoku-Community/sources/blob/2924f522ba533e2dc8b3c138b3769c0e29e03aba/sources/en.atsumaru/src/helpers.rs
  - ext: https://mangaball.com/search-advanced
  - ext: https://mangaball.com/api/v1/title/search-advanced
verified_at: a9768d3
---

MangaBall (`mangaball.com`, formerly `mangaball.net`) is a Next.js app over a JSON API at
`/api/v1`. The pages are server-rendered shells; the source uses the API only, as the site's
own client does. Everything below was read from the owner's captures of the site on
2026-11-13 (home, search, a title, a chapter, and the site's script bundles) unless marked.

> ⚠️ Earlier requests were blocked by Cloudflare. A later public API capture on 2026-11-13
> succeeded for all five sorts in both directions and details. Availability varies by client;
> a successful external request does not prove the source decoder or device behavior.
> A parameter marked (unverified) below still needs a positive narrowing probe.

## Endpoints

| Use | Request | Notes |
|---|---|---|
| Search, listings, two home rows | `GET /title/search-advanced` | `keyword`, `sort_by`, `sort_order`, `adult_mode`, `tag_mode`, `chapterLanguage`, `type`, `status`, `publicationDemographic`, `included_tags`, `excluded_tags`, `page`, `limit`; answers `data` and `pagination {page, total_pages}` |
| Hero | `GET /title/featured?limit=10&adult_mode=` | the site's "Featured Manga" |
| Ranking | `GET /title/top-views?period=week&limit=&section_key=sidebar_ranking&adult_mode=` | the site's "Top Ranking", Weekly tab |
| Recommended | `GET /title/recommended?limit=10&adult_mode=` | the site's "Titles Recommended" |
| Details | `GET /title/detail/{id}` | also answers a slug |
| Chapters | `POST /chapter/chapter-listing-by-title-id` with `{"title_id": id}` | every language in one answer; the site's reader and the keiyoushi extension call it |
| Pages | `GET /chapter-detail?chapter_id=` | `data.chapter.pages` are full urls; `title_id` resolves a chapter link |

`sort_by` takes `lastupdate`, `views`, `rating`, `created_at` and `name`: the site's Sort By
list is Latest Updated, Most Viewed, Top Rated, Recently Added and Title A-Z, with one
direction arrow, and Latest Updated descending is its default with or without a keyword.

> ⚠️ The site has no relevance sort. The first version offered one because a reference client
> does (it omits `sort_by`); the owner's screenshot of the site's panel corrected it. A filter
> or sort is taken from the site's own control, never from a reference client alone.

> ⚠️ The title JSON mixes naming styles: `originalLanguage` and `availableTranslatedLanguages`
> beside `recent_chapters` and `is18plus`. A `rename_all` on `Title` silently empties
> whichever half it gets wrong, so each camel-case field is renamed on its own
> (`models.rs`).

> ⚠️ `top_viewed` is switched off in the site's home configuration and answers nothing; the
> ranking the site shows is the same endpoint with `section_key=sidebar_ranking`. The site asks
> for 7; the source asks for 9 to fill three pages of three, and the owner saw all nine in the
> app on 2026-11-13.

## Keys

A title's key is its 24-character hex id. The site's urls are `/title-detail/{id}-{slug}` and
`/chapter-detail/{id}`; `parse_link` reads the id before the first hyphen, so a pasted or
deep link works with or without the slug. Chapter keys are chapter ids.

## Languages

The source lists the site's interface languages and uses the app's own language setting. The
site stores chapters under legacy and regional codes as well (`jp`, `kr`, `cn`, `zh-hk`,
`es-la`), so `site_languages` pairs every site code with the app language that selects it.
The pairs feed `chapterLanguage` in search, pick which chapters are kept, and set each
chapter's `language`.

> ⚠️ A title's `availableTranslatedLanguages` holds junk beside real codes (`vn`, `vie`,
> `english`, `gb`). It is only used to filter the three feeds that take no language
> parameter; never send it anywhere or show it.

## Content modes

The API has one axis, `adult_mode`: `no_18`, `all` or `only_18`, and each title carries only
`is18plus`. Two switches give the three modes (Show 18+ Content, then 18+ Only), and the
Content Rating filter exposes the site's three modes after a neutral Use Source Setting entry.

> ⚠️ The app removes a select left on its default. Keep Use Source Setting first, so Safe Only,
> All Content and 18+ Only are all explicit overrides of the saved preference. Safe mode also
> checks the shared tag-based rating: the API returned two Hentai-tagged titles with is18plus=false.

> ⚠️ Languages belong in the source's language setting only, never in search as well. kkantan
> flagged a Translated Languages filter on `multi.xcomic` ("I'm not convinced this filter needs
> to exist when the source already has the configurable languages setting"), which is the
> translated-languages row in section 3 of the root guide. An Original Language filter copied
> from a reference client broke that row here and was removed on the owner's correction. The
> site's panel has no such control either (owner's screenshots, 2026-11-13).

The source remains rated 1: the site has a Hentai theme, and
section 3 of the root guide fails a Hentai genre on a source rated 0.

## Types

`type` takes one of `manga`, `manhwa`, `manhua`, `comics`, which the site maps to original
languages (`TYPES` in `helpers.rs`, copied from the site's `ORIGINAL_LANGUAGE_CATEGORY_CONFIG`).
The Manga, Manhwa, Manhua and Comic genre tags exist but only 3 of 43 captured Japanese titles
carried one, so they are not used for this.

> ⚠️ Every reference client sends a single `type`. Whether the API accepts a comma list is
> unverified, which is why the Type setting is a select and not a multi-select. Use Source Setting
> inherits that selection; explicit Any clears the type constraint for one search.

## Home

Featured Manga (big scroller), Top Weekly Ranking (ranked list, three per page), Titles
Recommended, Latest Chapter Updates (chapter list) and Recently Added Titles, in the order the
owner asked for. One request each, sent together. The last two are search sorted by
`lastupdate` and `created_at`, so the settings apply on the server and both have listings.
The first three take only `adult_mode`; `SearchParams::allows` applies the type, excluded
tags and languages to them from the fields each title already carries.

Each search result carries `recent_chapters`, three uploads in any language. The chapter list
row takes the first one in a selected language and drops the title when there is none.

## Chapters

One request returns every chapter in every language; the selected languages are kept. The
site's title page pages `GET /title/chapter-listing` instead (`page`, `limit`, `group_by`,
`language`), but its largest `limit` and the `flat` grouping were never observed, so the
source uses the call the keiyoushi extension has in production.

> ⚠️ The chapter request must send `Content-Type: application/json` with no charset; the API
> answers 400 to anything else (noted in the keiyoushi extension, unverified here).

Blank chapter keys are skipped. Only bare numbers and the site's repeated chapter/volume labels are omitted from chapter titles.
Numbered prose such as "12 Heroes" stays intact. The same chapter conversion applies language
and scanlator exclusions before selecting a home chapter or returning the full list.
Timestamps are UTC with microseconds and no zone; the first 19 characters are parsed with the
`T` replaced by a space, because the test runner drops quoted literals from a date format.
The same group can upload a chapter twice; both are kept.

Images need the site as referer (`ImageRequestProvider`); the image host is a separate
Cloudflare zone.

## Search decoding correction

The ascending `lastupdate` response carries `availableTranslatedLanguages` as a single `"en"`
string; newer records use arrays. `string_or_vec` accepts those two observed shapes and retains
errors for other types. Some ascending `views`, `created_at` and `name` responses include an
incomplete record with no `name`. Title decoding permits that absent field, but `TryFrom<Title>`
rejects missing/blank titles and blank keys. Lists and every home layout skip invalid cards without
reordering valid entries or changing server pagination; details propagate the identity error.

> ⚠️ An API 200 and different ascending/descending IDs missed both app failures. Exercise the
> actual decoder and source entry point for every direction, and keep deterministic regressions
> for scalar/list languages, incomplete identities, home conversions and meaningful detail errors.

MangaDotNet provides the merged untagged scalar/list pattern; Flame provides fallible card
conversion. No extra dependency, default empty title or per-card request is needed.

A keyword probe with the single token `zzqxjvnosuchtitlezzqxjv` returns no titles; `hound`
returns the known series. The older negative fixture contained ordinary words ("no such title")
and matched real results, so it was not an impossible query. Keep the empty-result assertion with
an actually impossible token and retain a positive title-search assertion.

Verification after the initial decoder correction: all 14 source tests passed, including every live sort direction,
positive/negative search, paging, details, chapters, pages, listings, home and deep links. The
captured ten sort responses and one detail response also decode through the source models.
Formatting, warning-failing production/test Clippy, packaging and Aidoku verification pass.
Device rendering still needs an app check.

## Final submission audit

Compared the production files and resources against canonical `2924f522`, the fork's
`merged-sources` archive and current site bundles/API captures. Read the 200 most recently
updated inline review comments and the relevant open/merged kkantan review summaries; the
latest applicable feedback concerns propagated errors, metadata scope, compact cards and
avoiding duplicated search logic. HTML age-gate and mirror discovery suggestions are site-specific.

| Files/area | Merged comparison and decision |
| --- | --- |
| lib.rs and API requests | MangaDex, Comix, Toonkotsu and Sumidoku: framework traits, query parameters, owned JSON, partial details and shared request/status helpers. Keep headers/status handling shared across GET, POST and concurrent home requests; no new dependency or cache. |
| models.rs | Flame and Keyoapp: fallible compact cards, full details only on details/featured, trimmed unique credits, unique alphabetical tags. MangaDotNet provides the observed scalar/list language decoder pattern. Keep real numbered chapter titles and reject missing reader pages. |
| helpers.rs/settings.rs | Merged Atsumaru/Xcomic: key-only routing, named site differences and centralized defaults. Keep language aliases/type tables because the API uses legacy codes; an explicitly cleared language selection stays empty. Remove narrative comments and unused public metadata fields. |
| home.rs | Merged Toonkotsu/Atsumaru: declarative section layouts, five concurrent requests, compact links, ranked page size 3 and chapter page size 5. Featured metadata comes from its existing response. Group exclusions now match the full chapter list; all-section failure propagates the underlying error. |
| filters/settings/resources | Every one of the 96 tag ID/name pairs matches the live grouped-tag endpoint: 40 genres, 42 themes, 12 formats and two content warnings. All four support inclusion/exclusion. The site has five reversible sorts; preserve both directions. Neutral source-setting filter options make all explicit overrides usable. Icons remain opaque 128x128 with margin. |
| Manifest/lock/submission | Runtime dependencies remain aidoku and serde. Fork test-only serde_json is removed by the submission normalizer. The earlier audit published main version 8; the new-source submission retains version 1. Tests, documentation, guards and workflows remain off the PR branch. |

Live probes verified completed status, Japanese publication type, shounen demographic, AND/OR
tag inclusion, tag exclusion, each adult mode, page 2 and impossible status/demographic values.
The safe-mode/tag discrepancy is covered by deterministic and live regressions rather than a
compensating fetch loop. A focused checker rejects restoration of MangaBall's empty-language
fallback; its regression test and upstream calibration pass.

The source still makes five home requests, one normal search/listing request, one details
request, one chapter-list request and one reader request. Combined details/chapters uses two
requests with partial details first. Pasted-title search adds a detail lookup and falls back to
ordinary search only on failure; chapter deep links need one lookup. No per-card fan-out was added.

The supplied check-sources.sh is a Python snapshot with a stale rule rejecting explicit
canExclude=false. Current fork rules and merged feedback require that false for unsupported
exclusion controls. The host reference also incorrectly described parse_date as local time;
the current Rust helper explicitly passes UTC, so only the reference needed correcting.

Home order stays Featured Manga, Top Weekly Ranking, Titles Recommended, Latest Chapter Updates
and Recently Added Titles. The latest request did not include a replacement layout after "like",
so it does not authorize an invented redesign. App/device rendering remains a manual check.


## Markdown correction

The reported series, I Want to Trick You (6889f391e00aa4d41d07dfeb), has **Original Webtoon:**
wrapped in Markdown in both its featured and details API responses. Unlike plain-text
NovelArchive descriptions, this Markdown must pass through unescaped, following merged
MangaDex (`sources/multi.mangadex/src/models.rs:267`). The app's ExpandableTextView renders
Markdown; HomeBigScrollerView uses Text. The fix removes the escaping in the details path,
so partial and final updates retain bold/italics and links. A deterministic fixture covers
both emphasis styles, paragraphs and link underscores; the live reported-title regression
compares details-only, combined and chapters-only results.

> ⚠️ Featured descriptions still display Markdown markers literally because Aidoku's
> BigScroller is plain text. This matches merged MangaDex's direct description path. A source
> cannot make that component render bold/italics; do not add a general Markdown parser or
> erase literal punctuation to simulate it. Device verification of the detail correction remains.

The focused checker rejects reintroducing Markdown escaping on MangaBall's details path.
Submission version stays 1; fork publication advances once for this correction.


## Chapter deduplication

Inspected Yomu's published MangaBall v12 AIX (blob 76bed84198f3eb07c7f2124faedbe0de7301a7f2),
including its settings and compiled field names. Its settings advertise official, likes,
views, then newest, with deduplication enabled by default. Compiled settings are evidence
of the advertised contract, not proof of unpublished Rust implementation.

Compared merged Xcomic (server-side unique chapters), Atsumaru (winning group plus parts),
Toonkotsu (site main upload plus upload-quality selection), MangaDotNet (official/preferred/
frequent group scoring) and Kagane (edition-specific book index, no deduplication toggle).
MangaBall's observed full chapter responses supply optional likes and views and created_at,
but no verified official flag. The implementation ranks by likes, views, the complete UTC upload timestamp
and chapter key, without inventing an official classification. Group IDs distinguish uploads;
names are only a fallback. Group exclusions/language filtering happen before ranking.

Deduplicate Chapters defaults off, following Atsumaru/Toonkotsu and preserving current lists.
When enabled, chapter_list retains the winning group's same-number parts, every unnumbered
chapter and separate language/volume slots. Chapter keys are unchanged and no extra requests
are added. The setting footer describes the priority and read-status effect concisely.
Deterministic tests cover priorities, exclusions, parts, languages, volumes and reversed-input
tie breaking; a live test ensures the selected uploads retain existing keys.

> ⚠️ Selecting one upload can hide a previously read upload. Switching back restores that
> original upload's visibility; never rewrite chapter keys to a manufactured chapter number.
