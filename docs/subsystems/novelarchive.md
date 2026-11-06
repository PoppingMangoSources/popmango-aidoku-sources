---
title: Novel Archive (en.novelarchive)
type: subsystem
created: 2026-11-06
updated: 2026-11-06
sources:
  - code: sources/en.novelarchive/src/lib.rs
  - code: sources/en.novelarchive/src/helpers.rs
  - code: sources/en.novelarchive/src/models.rs
  - code: sources/en.novelarchive/src/test.rs
  - code: sources/en.novelarchive/res/filters.json
  - ext: https://novelarchive.st/assets/js/libary-page.js
  - ext: https://novelarchive.st/assets/js/index-home.js
verified_at: c8b21b1
---

Novel Archive is a web-novel archive with a plain JSON API under `/api/novels`, read the way the
site's own scripts read it. It moved from `novelarchive.cc` to `novelarchive.st`, and the old host
answers with a 301. Novel ids and the numbered chapter keys did not change, so libraries carry
over without a `breakingChangeVersion`.

## Search and filters

Search is the library's browse call: `search` with `fuzzy=1`, `sort`, `status`, `genres_include`,
`genres_exclude`, `genre_match=any`, `page` and `per_page=24` (`sources/en.novelarchive/src/helpers.rs:68`).
Recently Updated is the default sort and is never sent. `has_next_page` comes from `pagination`.

Genres are a dynamic filter from `/api/novels/genres`, about 700 merged upstream values, without
the site's category values (`NON_GENRES`) (`sources/en.novelarchive/src/lib.rs:264`). A tapped tag
sends its label lowercased, as the site lowercases every genre.

> ⚠️ Hide Adult Content is on by default on the site and here. On search it adds the site's twelve
> adult genres to `genres_exclude`, as the library script does (`ADULT_GENRES`,
> `sources/en.novelarchive/src/helpers.rs:16`), so the server keeps pages full.

> ⚠️ The home feeds (`trending`, `editors-choice`, `recently-updated`) ignore `genres_exclude`;
> the same twelve came back with and without it (probed 2026-11-06). So the home filters adult titles
> out locally from each novel's genres instead (`sources/en.novelarchive/src/lib.rs:110`).

## Home

Six rows, in one parallel request: Trending (the site's hero, five rotating every 8 seconds),
Editor's Choice, Popular, Recently Updated, Top Rated and New in the Archive. The three sorted rows
are browse pages (`popular`, `rating`, the default sort) and link listings of the same name
(`sources/en.novelarchive/src/lib.rs:246`). Recently Updated names each novel's newest chapter by
its count: lists carry no chapter names, only `total_chapters`, so that chapter has a number and the
novel's `updated_at`, never a title (`sources/en.novelarchive/src/models.rs:155`). A feed that
fails drops out; the home fails only when all six do.

> ⚠️ The site's own home shows only Trending, Editor's Choice and Latest Updates. The six rows are
> the owner's choice, taken from another client of the site, so a review may cite the "order the
> home as the site does" rule; the extra rows are deliberate.

> ⚠️ The browse default sort, labelled "Recently updated" in the site's library, lists newly added
> novels first (the captured page's `created_at` and `updated_at` match), hence "New in the
> Archive" for that row while the sort filter keeps the site's label.

## Chapters

Chapter n is the n-th entry of `chapter_names`, numbered by position as the site's reader links
them (`/reader?novel={id}&chapter={n}`), and a novel whose names were never collected still lists
`total_chapters` chapters (`sources/en.novelarchive/src/models.rs:134`). Text comes from
`/chapters/{n}?prefetch=1`, which leaves out the name list; it is plain text, escaped as Markdown.

> ⚠️ A chapter title drops a leading "Chapter n" only when n is its own position, so
> "Chapter 10" at position 1 stays whole (`sources/en.novelarchive/src/helpers.rs:144`).

> ⚠️ Errors answer `{"error": …}` with a status, such as a 404 "Novel not found"; the message is
> shown as is.

## Testing

> ⚠️ Cloudflare refuses this sandbox with a 403 for curl and headless Chromium alike, so the live
> tests in `src/test.rs` fail here on the network. The `review_` tests use responses captured
> through Firecrawl and run anywhere; the live suite needs another network or the app.

> ⚠️ The latest row's date uses the ISO format the app parses (as `en.novelbuddy` does), but the
> test runner drops quoted literals and returns `None`, so the date is only checked in the app.

Not ported: the site's fallback to upstream copies of a chapter (`/sources`), which the reader uses
when the archive's own copy is missing. Such a chapter fails with a message instead.
