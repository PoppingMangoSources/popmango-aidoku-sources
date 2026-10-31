---
title: mangak template (MangaK and ToonTop)
type: subsystem
created: 2026-10-31
updated: 2026-10-31
sources:
  - code: templates/mangak/src/lib.rs
  - code: templates/mangak/src/imp.rs
  - code: templates/mangak/src/helpers.rs
  - code: templates/mangak/src/models.rs
  - code: sources/en.mangak/src/lib.rs
  - code: sources/en.mangak/res/filters.json
  - code: sources/en.mangak/src/test.rs
  - ext: https://github.com/Aidoku-Community/sources/pull/850
verified_at: 07f5fdc
---

MangaK (`mangak.io`, `en.mangak`) and ToonTop (`toontop.io`, `en.toonilyme`, upstream only) run the
same API and search page, so everything they share lives in `templates/mangak`. A consumer
is a `Params` value: base url, API url, and the two things the sites' home pages differ
in, the big scroller's title and feed and the latest row's name
(`templates/mangak/src/lib.rs:15`). kkantan asked for this split on #850: features both
sites have go in the template, per-site differences become `Params` fields.

## Search

`templates/mangak/src/imp.rs:17` builds `/titles/search` from the filters. The ten sorts map
through `SORTS` (`templates/mangak/src/helpers.rs:7`), where Best Match is `None` and sends
no sort, as the site does.

> ⚠️ Only the genre multi-select sends excluded values (`genres` / `exclude`,
> `templates/mangak/src/imp.rs:39`). The API has no exclusion parameter for type, content
> rating, demographic or format (probed), so those filters set `"canExclude": false` in
> `sources/en.mangak/res/filters.json`. kkantan asked for that on #850; enabling exclusion
> on another filter needs both a probed API parameter and a branch in
> `templates/mangak/src/imp.rs`.

> ⚠️ An empty status or author returns a 400 from the API, so empty select and text values are
> skipped rather than sent (`templates/mangak/src/imp.rs:49`). The `alphabetical` sort and
> the `hiatus` status also return 400s, which is why A-Z is gone; Hiatus stays in the filter.

> ⚠️ `demographic` and `format` match nothing on ToonTop, so they exist only in
> `sources/en.mangak/res/filters.json`. Check the other consumer's site before moving a filter into the
> shared template.

## Home and listings

`get_home` (`templates/mangak/src/imp.rs:133`) sends the layout as a partial, loads every row
with one `send_all`, then returns only the rows that loaded, so a failed row drops out
instead of leaving an empty placeholder. Rows: the `Params` big scroller (10 entries), Trending
This Week, Popular Updates, New Manga and the latest row. Listings reuse each row's request
through `listing_path` (`templates/mangak/src/helpers.rs:20`).

> ⚠️ The trending feed has no pagination and ignores `page`, so it reads as one page
> (`templates/mangak/src/models.rs:29`). Its items carry no genres or chapters.

> ⚠️ Only the big scroller carries details (`into_featured`,
> `templates/mangak/src/models.rs:66`). Every other card is key, title and cover, the rule
> kkantan applies to every listing. Popular Updates and the latest row are chapter lists
> because the site shows each title's newest chapter there (`into_manga_with_chapter`).

> ⚠️ MangaK's search response omits some genres that details list (Webtoon, Girls Love and a
> few others in about 8 of 20 titles), so a big-scroller card's tags can grow when it is
> opened. The owner accepted that over a per-card lookup; the test checks the card's tags are
> an ordered subset of the details' (`sources/en.mangak/src/test.rs`).

## Ratings

`content_rating` (`templates/mangak/src/helpers.rs:43`) reads the genres: Adult, Hentai,
Mature or Smut is NSFW, Ecchi is Suggestive. The source is rated 18+ because the trending
feeds ignore `content_rating`, so Discover can't be kept safe.

## Testing

`sources/en.mangak/src/test.rs` holds 17 live tests on `main` only; `scripts/pr-branch.sh`
keeps them out of submissions. ToonTop is not on `main`, so its changes are made and tested
in the PR worktree.
