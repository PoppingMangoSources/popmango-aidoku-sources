---
title: WebNex (en.webnex)
type: subsystem
created: 2026-11-20
updated: 2026-11-20
sources:
  - code: sources/en.webnex/src/lib.rs
  - code: sources/en.webnex/src/helpers.rs
  - code: sources/en.webnex/src/home.rs
  - code: sources/en.webnex/src/reader.rs
  - code: sources/en.webnex/src/test.rs
  - code: sources/en.webnex/res/filters.json
  - ext: https://webnex.cc/browse
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/src/en/webnex
verified_at: 993247d
---

WebNex (`webnex.cc`) is a Next.js app with no JSON API: every page is server-rendered HTML
with React streaming chunks (`div[id^='S:']` filling `template[id^='P:']`), which
`helpers::expand` splices back before parsing. Everything below was probed live on
2026-11-20 with plain requests and Chromium.

## Home

The source reads the home page's own shelves in the site's order: Popular (this week's
tab), Readers' favourites, Latest updates, Recently popular, New in 2026, Manhwa, Manga,
Manhua, then the Finished and Just added tabs (`sources/en.webnex/src/home.rs:20`).
Reader shelves, Busiest groups and Latest comments are not series lists and are skipped.

> ⚠️ Browse's Popular sort is the same weekly chart as home's Popular shelf; the shelf's
> Today and This month tabs are `/?popular=day` and `/?popular=month`, and `/browse` ignores
> a `popular` parameter.

## Browse parameters

> ⚠️ `tag=` and `genre=` match the same series (and `xtag=` and `xgenre=`), so genres,
> formats, themes and tag taps all send `tag`/`xtag` (`sources/en.webnex/src/helpers.rs:202`).
> "User Created" matches nothing and is not offered.

> ⚠️ The site reads only the first `chapters` value and ignores one that is not a number,
> so the source sends one: a typed Minimum Chapters, else the Chapters preset. The year
> presets are client-side: This year is `from=<year>`, Last 5 years `from=<year - 4>`,
> 2010s `from=2010&to=2019`, 2000s `from=2000&to=2009`, Before 2000 `to=1999`; a typed
> Release Year end overrides the preset's.

> ⚠️ The app omits a select's default, so a select whose first option is a real value can
> never send it. The Chapters select gained "Any" first for this reason (v9); before, its
> "Readable Now" sent nothing.

> ⚠️ The genre panel's list (with counts and ~10,000 tags) comes from a Next.js server
> action, a POST to `/browse` with a build-specific `next-action` id, so it is not fetched;
> the static genres, formats and themes in `filters.json` matched the panel exactly.

## Chapters

> ⚠️ Chapter `datetime` attributes are UTC (`2026-09-19T23:40:35.000Z`). `helpers::date`
> reshapes them to `yyyy-MM-dd HH:mm:ss`, which the test runner can parse, and passes "UTC"
> to `parse_date_with_options`; plain `parse_date` read them in the reader's zone before v9
> (`sources/en.webnex/src/helpers.rs:139`).

## Reader

> ⚠️ `/read/<key>/pages` (the reader props' `pagesUrl`) answers 403 "Open this chapter on
> webnex.cc to read." without the `wnx_reader_v2` cookie. The chapter page writes it from
> `<script data-wnx-reader>`, and only for a page request: `Accept: text/html` (or
> `Sec-Fetch-Mode: navigate` with `Sec-Fetch-Dest: document`); without either the script is
> absent. The list then needs that cookie and the chapter page as `Referer`
> (`sources/en.webnex/src/reader.rs:8`). No web view is involved.

> ⚠️ The "Opening series" interstitial sets a `wnx_` cookie the same way and reloads;
> `helpers::parse_response` retries with it. It could not be reproduced on 2026-11-20, so
> that path is checked only by the script-cookie fixture test.
