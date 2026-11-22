---
title: VyManga (en.vymanga)
type: subsystem
created: 2026-11-17
updated: 2026-11-20
sources:
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/046a7f2a6f93f99ba3566a09a42b1ec28dd57487/sources/en.vymanga/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/046a7f2a6f93f99ba3566a09a42b1ec28dd57487/sources/en.vymanga/src/helpers.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/046a7f2a6f93f99ba3566a09a42b1ec28dd57487/sources/en.vymanga/src/home.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/046a7f2a6f93f99ba3566a09a42b1ec28dd57487/sources/en.vymanga/src/settings.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/046a7f2a6f93f99ba3566a09a42b1ec28dd57487/sources/en.vymanga/src/test.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/046a7f2a6f93f99ba3566a09a42b1ec28dd57487/sources/en.vymanga/res/filters.json
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/046a7f2a6f93f99ba3566a09a42b1ec28dd57487/sources/en.vymanga/res/settings.json
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/046a7f2a6f93f99ba3566a09a42b1ec28dd57487/sources/en.vymanga/res/source.json
  - ext: https://mangavyvy.com/search
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/src/en/vyvymanga
verified_at: afbc5da
---

VyManga (`mangavyvy.com`, mirror `mangavyvy.net`) is server-rendered HTML with no API and no
theme template. The sandbox reaches it with plain requests, so everything below was probed
live on 2026-11-17 and the source's live tests run. Both hosts serve the same pages; the
reader picks one through the app's base url select (`urls` in `source.json`). It is submitted
upstream from `feat/en.vymanga`, cut from `upstream/main`.

## Pages

| Use | Request | Notes |
|---|---|---|
| Search, listings, two home rows | `GET /search` | 36 cards a page; `[rel=next]` is the next-page signal; the last page and an unmatched query answer no cards |
| Popular Manga | `GET /` | 16 slides in `.slick-detail > a`, each with the fields of a series page |
| Series and chapters | `GET /manga/<slug>` | 404 for an unknown slug; the chapter list is in the same page |
| Reader | the chapter's link | one redirect to a rotating host; images are `.carousel-item > img[data-src]` |

A card (`.comic-item`) carries the link, title, cover (`img[data-src]`), a `.comic-completed`
marker and the latest chapter's name. A series is Completed or Ongoing, nothing else.

> ⚠️ The card's `.comic-completed` marker can disagree with the series page (Hyakujuu Kingdom,
> kkantan on #913), so cards set no status and details read it from the series page.

Requests need no headers: every page, the reader chain, covers and page images answered 200
without a browser user agent or a Referer.

## Search parameters

Probed one at a time against the page count; a missing parameter is the site's default.

| Parameter | Values | Result |
|---|---|---|
| `q` | text | matches titles |
| `check_search_desc` | `1` | also matches descriptions ("pirate": 1 page without, 4 with) |
| `author`, `author_po` | text; `0` contain, `1` begin, `2` end | all three positions change the results |
| `completed` | `2` all, `1` completed, `0` ongoing | an unknown value answers nothing |
| `sort` | `viewed` (default), `scored`, `created_at`, `updated_at` | an unknown value is the default |
| `sort_type` | `desc` (default), `asc` | works on every sort |
| `genre[]`, `exclude_genre[]` | the whole `data-value`, such as `Action-1-action` | the number or the name alone is ignored for `genre[]` |
| `page` | from 1 | |

Every filter also works together with a text query (probed with `q=love`: status, each sort
and direction, genre include and exclude, author and its match, and description search all
changed the page count or order), so `hidesFiltersWhileSearching` stays unset.

> ⚠️ `search_po` (the Contain, Begin, End select beside the title box) changes nothing: "one
> piece" and "piece" answered the same pages for all three values. The source leaves it out;
> don't add it back without a probe that shows it narrowing.

> ⚠️ A genre both included and excluded answers nothing, so `search_url` drops a genre from
> the exclusions when the search includes it. That is also how a search overrides the
> Excluded Genres setting.

> ⚠️ `filters.json` and `settings.json` hold the same 244 genres, generated from the search
> page's `.checkbox-genre[data-value]` boxes. The site adds genres freely (three ids contain a
> line break), so regenerate both together; `review_genre_lists_match` fails when they differ.

## Chapters

> ⚠️ A chapter's link is encrypted anew on every page load and expires within hours, and the
> `id="chapter-<n>"` attributes repeat (`chapter-0` 96 times on one series). The chapter key is
> therefore its name as the site prints it, with `#2`, `#3` counted from the oldest when a name
> repeats. `get_page_list` reads the series page again and follows the fresh link of the
> chapter with that key. Keying by the link, as versions up to 9 did, changed every key on
> every refresh. Each chapter keeps the site's link as its url, as the reference client does,
> although it stops opening within hours (owner's choice in v16, after v13 and v14 set none
> and v15 the series page).

> ⚠️ Four genre ids carry the site's raw whitespace (`Adventure\n'`, `MONSTERS\nACTION`,
> `Shounen\nType`, `Sci-fi  Shounen`), while a home slide's tag text is normalised. The Popular
> Manga filter compares names word by word (`sources/en.vymanga/src/home.rs:64`); an exact
> comparison never hid those genres.

Names come as "Chapter 212 : Ch 212 : S2 Finale", "Chapter 211 : Ch 211", "Vol.15 Chapter
136: Hometown Song", "Vol.2 Fifth Course: …" and bare names such as "Extra". The volume and
chapter numbers are taken from those prefixes; a remainder that is only "Ch <the number>" is
dropped. Dates print as "Dec 09, 2025", or "N hour ago" and "N minutes ago" for the last day.

> ⚠️ The runner parses no date-only format, so the absolute dates are only checked in the app.

The latest chapter named on a card can be one ahead of the series page for a while (seen
once in 14), so the Latest Update row is a plain scroller, not a chapter list.

## Details and ratings

A home slide and a series page share their markup (`h1.title`, `.img-manga img`, `.pre-title`
rows, `.label-badge` genres), so `parse_manga` reads both; only the description's class
differs. A series page links each author; a slide prints them joined by " , ". Names such as
"Kariya, Tetsu" keep their comma.

The site blurs covers it counts as adult (`cover-adult`). That flag, or an Erotica,
Pornographic, R-18 or Smut genre, rates a series NSFW; Ecchi or Mature rates it Suggestive.
The source is rated 2 (the owner's call). The site has no control for hiding adult titles, so the source adds none; Excluded Genres
covers it. Smut titles are not always flagged by the site.

## Home

Popular Manga comes from the home page; Latest Update and New Release are the first page of
the search sorted by `updated_at` and `created_at`, so the Excluded Genres setting applies to
them on the server. The slides are filtered by genre name, read from the id's first part.

## Icon

The site's logo is `icon-green.png` on its image host, 60px, and no larger copy exists on
either mirror or on vymanga.site. The icon is that logo scaled to 88px on white.

> ⚠️ `icon.png` beside it is 256px but is an older, different mark. It shipped for one version
> and the owner rejected it; a larger file is not the site's logo just because it is larger.
