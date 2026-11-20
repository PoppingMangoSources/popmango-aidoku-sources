---
title: yaoi.me (multi.yaoime)
type: subsystem
created: 2026-11-17
updated: 2026-11-20
sources:
  - code: sources/multi.yaoime/src/lib.rs
  - code: sources/multi.yaoime/src/browse.rs
  - code: sources/multi.yaoime/src/helpers.rs
  - code: sources/multi.yaoime/src/home.rs
  - code: sources/multi.yaoime/src/models.rs
  - code: sources/multi.yaoime/src/cipher.rs
  - code: sources/multi.yaoime/src/settings.rs
  - code: sources/multi.yaoime/src/filters.rs
  - code: sources/multi.yaoime/src/tags.rs
  - code: sources/multi.yaoime/res/filters.json
  - code: sources/multi.yaoime/res/settings.json
  - ext: https://yaoi.me/browse
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/src/all/yaoime
verified_at: afbc5da
---

yaoi.me is a Next.js site rendered on the server, so the source reads its HTML; the only JSON
endpoint is the reader. The sandbox reaches it directly, and everything below was probed live
on 2026-11-17 unless marked.

## Settings live in a cookie

The site keeps its settings in a `yf.settings` cookie (URL-encoded JSON, with `yf.adult=1`
beside it) and applies most of them on the server. The source sends the same cookie on every
page request (`SiteSettings::cookie`).

| Key | Applied by the server to | Probe |
| --- | --- | --- |
| `hiddenTypes`, `hiddenRatings`, `hiddenTags` | home, browse, Discover | results changed; `tags=smut` with `smut` hidden answers nothing |
| `titleLang` (`site`, `en`, `romaji`, `native`) | every title on every page | browse titles changed to native script |
| `officialOnly` | the series page's chapter data | variants 729 to 222 on series 4578 |
| `chapterLangs` | home only | browse ignores it; browse takes `langs=` instead |
| `home` | which home sections render, in order | one request renders all nine rows |

- Every rating hidden reads as none hidden on the site, so a cleared Content Ratings setting
  returns nothing in the source instead of everything.
- A search's Type or Content rating filter clears that axis from the cookie, and an included
  genre or tag is removed from `hiddenTags`, so a filter overrides its setting for that search.
- Hidden tags are names, lowercased by the site, at most 40.

> ⚠️ Browse ignores `chapterLangs`; the translated languages go to browse as `langs=` and to
> the home page in the cookie. Sending only one of them leaves the other unfiltered.

> ⚠️ A filter only overrides its setting because the source removes that axis from the cookie
> (`sources/multi.yaoime/src/browse.rs`). With the axis left in, the site ANDs both and an
> included tag that is also hidden answers nothing.
- `chapterClean` (dedupe and clean order) is applied by the site's client, not the server, so
  the source reimplements it (`clean_list`, from the site's own function in its chapter bundle).

> ⚠️ A clean row skips a chapter only when every upload is dead, so the upload it opens must be
> the first live one, not the first listed: up to v9 a dead first copy opened as a broken
> chapter (`sources/multi.yaoime/src/models.rs:281`, pinned in `review_clean_chapter_list`).

## Browse

`/browse` takes `q`, `langs` (comma list), `genre`, `without`, `tags`, `withoutTags` (comma lists,
ANDed; `tags` matches genre names too, which is why tapped tags go there), `type`, `status`,
`content`, `demographic`, `yearFrom`/`yearTo`, `minChapters`/`maxChapters`, `minRating`/`maxRating`
(0 to 100), `minPages`, `updatedWithin` (`1`, `7`, `30`, `90`, `365`), `anime=1`, `licensed=1`,
`author`, `artist`, `sort` (`update`, `create`, `popular`, `rating`, `chapters`, `az`), `dir` and
`page`. Sixty cards a page; the next page is a "Next" link in the page's `nav`.

Genres, tags and the year range come live from the browse page's data (`sources/multi.yaoime/src/filters.rs`),
for the filters and for the Excluded Genres and Excluded Tags settings. The site sends no group
per tag: its panel files them by a name table and 24 rules in its browse script, copied as data
into `sources/multi.yaoime/src/tags.rs` (ten groups, 1330 tags on 2026-11-17).

> ⚠️ The app puts dynamic filters after `filters.json` and dynamic settings after
> `settings.json` (AidokuRunner `Source.swift`). Everything from Genres onward is built in
> Rust so the filters keep the site's order, and the Exclusions group comes last in settings.

> ⚠️ A new tag the rules don't know lands in OTHER until `tags.rs` is refreshed from the
> site's script; `review_tag_groups` pins samples of each group.

## Discover

One request renders the home page: the source writes the wanted section ids into the
settings cookie's `home` list, then each one that is turned off and the site sections it does
not show (`continue`, `reviews`, `creators`, `about`) prefixed with `-`, and reads each row
under its `h2.section-title` (`sources/multi.yaoime/src/home.rs:85`). The order is the owner's:
Popular Updates, Most Bookmarked, Latest Releases (a chapter list of five), Trending Now, Top
Rated, Rising, Most Talked About (a ranked list of four), Hidden Gems and Binge-Worthy, every
other row a scroller (`sources/multi.yaoime/src/home.rs:21`, pinned by `review_home_layout`).

> ⚠️ The site renders the sections in the order the cookie lists them and drops the ones it
> leaves out, so a section missing from both the wanted and the `-` lists comes back in its
> default place and can push a row out of order. Every id in `SECTIONS` and `SITE_SECTIONS` must
> be named one way or the other.

> ⚠️ Headings are matched ignoring case because the site writes them in sentence case
> ("Trending now"); a renamed heading drops its row silently rather than failing the page.
> Popular Updates has no listing page on the site, so it is the one row without a listing.

## Series and chapters

- Series keys are the title id from `/series/{id}`. A series also opens under any of its source
  series ids (`/series/196432` is title 4578), so deep links and pasted links read the
  canonical link from the page.
- The chapter data is a Flight object with `groups` (one per chapter, each with `variants`, one
  per upload) and `sources`. A variant's `s` is its source series; chapter keys are `{s}/{id}`,
  which the reader needs, and chapter urls are `/chapter/{s}/{id}`.
- Source display names come only from the reader response, so a chapter list costs one reader
  request for the first variant (none in clean mode, which shows no names).
- Credits read `Story by A, B · Art by C` or `Story & art by A` under the title.

> ⚠️ An extra group's `num` is often a running count, not its place: 4-Week Lovers (4971) ends
> at 133, and its "Bounus.1" to "Bounus.7" extras carry 158 to 164 with `numReal` 1. Taken as
> chapter numbers they sat among chapters 1 to 8 in the app. An extra keeps a number only when
> its label names the same one ("Ch.26.5"); otherwise it shows by its label in the site's order.

> ⚠️ Undated uploads have `createdAt` 0, which the app shows as 31 December 1969; they get no date.

## Reader

`GET /api/reader/chapter?s={s}&c={id}` needs `x-yf-t` (milliseconds; seconds times 1000 is
accepted, ten minutes old is refused) and `x-yf-s`, the unpadded base64url HMAC-SHA256 of
`"{path} {t}"`. Page hashes come sealed: a 16-byte salt and 12-byte IV, then AES-256-GCM under
`sha256(SEAL_KEY ‖ salt)`, holding `{"p": [hashes]}`. Images are `/i/p/{hash}?w={width}` with the
first offered width at least the page's own; they need no cookie or referer. "Novels" on the site
are image pages through the same reader.

> ⚠️ The two keys come from the site's reader bundle and will change if it rotates them; the
> reader then answers 403 "Not a request from the reader."
