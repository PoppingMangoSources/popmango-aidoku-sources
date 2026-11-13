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
verified_at: 54be504
---

MangaBall (`mangaball.com`, formerly `mangaball.net`) is a Next.js app over a JSON API at
`/api/v1`. The pages are server-rendered shells; the source uses the API only, as the site's
own client does. Everything below was read from the owner's captures of the site on
2026-11-13 (home, search, a title, a chapter, and the site's script bundles) unless marked.

> ⚠️ Cloudflare refuses the sandbox and GitHub's runners outright (403 "Just a moment" for
> curl, headless and headed Chromium, probed 2026-11-13). The live tests in `src/test.rs` have
> never run. Every response model was checked by parsing the captured responses with the
> source's own structs, and `review_` tests cover the rest offline. A parameter marked
> (unverified) below needs a console probe on the site before it is relied on.

## Endpoints

| Use | Request | Notes |
|---|---|---|
| Search, listings, two home rows | `GET /title/search-advanced` | `keyword`, `sort_by`, `sort_order`, `adult_mode`, `tag_mode`, `chapterLanguage`, `type`, `status`, `publicationDemographic`, `originalLanguage`, `included_tags`, `excluded_tags`, `page`, `limit`; answers `data` and `pagination {page, total_pages}` |
| Hero | `GET /title/featured?limit=10&adult_mode=` | the site's "Featured Manga" |
| Ranking | `GET /title/top-views?period=week&limit=&section_key=sidebar_ranking&adult_mode=` | the site's "Top Ranking", Weekly tab |
| Recommended | `GET /title/recommended?limit=10&adult_mode=` | the site's "Titles Recommended" |
| Details | `GET /title/detail/{id}` | also answers a slug |
| Chapters | `GET /title/chapter-listing?title_id=&page=&limit=&group_by=flat&sort_order=desc&language=` | `language` is a comma list; `pagination.total_pages` ends the loop |
| Pages | `GET /chapter-detail?chapter_id=` | `data.chapter.pages` are full urls; `title_id` resolves a chapter link |

`sort_by` takes `lastupdate`, `created_at`, `name`, `views` and `rating`. Without one the API
orders by relevance, which only means something with a keyword, so a search without either
sends `lastupdate`, the site's own default.

> ⚠️ The title JSON mixes naming styles: `originalLanguage` and `availableTranslatedLanguages`
> beside `recent_chapters` and `is18plus`. A `rename_all` on `Title` silently empties
> whichever half it gets wrong, so each camel-case field is renamed on its own
> (`models.rs`).

> ⚠️ `top_viewed` is switched off in the site's home configuration and answers nothing; the
> ranking the site shows is the same endpoint with `section_key=sidebar_ranking`. The site asks
> for 7; the source asks for 9 to fill three pages of three (unverified: the server may cap it).

## Keys

A title's key is its 24-character hex id. The site's urls are `/title-detail/{id}-{slug}` and
`/chapter-detail/{id}`; `parse_link` reads the id before the first hyphen, so a pasted or
deep link works with or without the slug. Chapter keys are chapter ids.

## Languages

The source lists the site's interface languages and uses the app's own language setting. The
site stores chapters under legacy and regional codes as well (`jp`, `kr`, `cn`, `zh-hk`,
`es-la`), so `site_languages` pairs every site code with the app language that selects it.
The pairs feed `chapterLanguage` in search, `language` in the chapter list, and each
chapter's `language`.

> ⚠️ A title's `availableTranslatedLanguages` holds junk beside real codes (`vn`, `vie`,
> `english`, `gb`). It is only used to filter the three feeds that take no language
> parameter; never send it anywhere or show it.

## Content modes

The API has one axis, `adult_mode`: `no_18`, `all` or `only_18`, and each title carries only
`is18plus`. Two switches give the three modes (Show 18+ Content, then 18+ Only), and the
Content Rating filter overrides them for one search. A four-way rating setting was left out:
Safe and Suggestive could only be told apart by filtering each page locally from tags.
Per-title ratings still follow the app's scale: `is18plus` or an Adult, Hentai, Manhwa 18+ or
Smut tag is NSFW, Ecchi or Mature is Suggestive.

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

One request per 1,000 chapters. Names mostly repeat the number ("Ch.22", "Chapter 188",
"Vol.5 Ch.22"), so a name containing the number is dropped and only a real title is kept.
Timestamps are UTC with microseconds and no zone; the first 19 characters are parsed with the
`T` replaced by a space, because the test runner drops quoted literals from a date format.
The same group can upload a chapter twice; both are kept.

Images need the site as referer (`ImageRequestProvider`); the image host is a separate
Cloudflare zone.
