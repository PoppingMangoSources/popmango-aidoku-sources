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
  - ext: https://mangaball.com/search-advanced
  - ext: https://mangaball.com/api/v1/title/search-advanced
verified_at: cc4a119
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
Content Rating filter, the site's own three-option select, overrides them for one search.

> ⚠️ The Content Rating filter's default is Safe Only, and the app does not send a select left
> on its default. So a reader whose setting shows 18+ titles cannot pick Safe Only for one
> search; the other two options always override.

> ⚠️ Languages belong in the source's language setting only, never in search as well. kkantan
> flagged a Translated Languages filter on `multi.xcomic` ("I'm not convinced this filter needs
> to exist when the source already has the configurable languages setting"), which is the
> translated-languages row in section 3 of the root guide. An Original Language filter copied
> from a reference client broke that row here and was removed on the owner's correction. The
> site's panel has no such control either (owner's screenshots, 2026-11-13).

The source is rated 1, not 0: the site has a Hentai theme with about 69,000 titles, and
section 3 of the root guide fails a Hentai genre on a source rated 0.

## Types

`type` takes one of `manga`, `manhwa`, `manhua`, `comics`, which the site maps to original
languages (`TYPES` in `helpers.rs`, copied from the site's `ORIGINAL_LANGUAGE_CATEGORY_CONFIG`).
The Manga, Manhwa, Manhua and Comic genre tags exist but only 3 of 43 captured Japanese titles
carried one, so they are not used for this.

> ⚠️ Every reference client sends a single `type`. Whether the API accepts a comma list is
> unverified, which is why the Type setting is a select and not a multi-select. A Type filter
> left on Any keeps the setting, so a search cannot widen a narrowed setting.

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

Names mostly repeat the number ("Ch.22", "Chapter 188",
"Vol.5 Ch.22"), so a name containing the number is dropped and only a real title is kept.
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

Verification after the correction: all 14 source tests pass, including every live sort direction,
positive/negative search, paging, details, chapters, pages, listings, home and deep links. The
captured ten sort responses and one detail response also decode through the source models.
Formatting, warning-failing production/test Clippy, packaging and Aidoku verification pass.
Device rendering still needs an app check.
