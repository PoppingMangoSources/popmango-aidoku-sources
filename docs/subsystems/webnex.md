---
title: WebNex (en.webnex)
type: subsystem
created: 2026-11-20
updated: 2026-11-21
sources:
  - code: sources/en.webnex/src/lib.rs
  - code: sources/en.webnex/src/helpers.rs
  - code: sources/en.webnex/src/home.rs
  - code: sources/en.webnex/src/reader.rs
  - code: sources/en.webnex/src/rsc.rs
  - code: sources/en.webnex/src/test.rs
  - code: sources/en.webnex/res/filters.json
  - ext: https://webnex.cc/browse
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/src/en/webnex
verified_at: 88ccce31
---

WebNex (`webnex.cc`) is a Next.js app without a general catalogue JSON API: every page is server-rendered HTML
with React streaming chunks (`div[id^='S:']` filling `template[id^='P:']`), which
`helpers::expand` splices back before parsing. Everything below was probed live on
2026-11-20 with plain requests and Chromium.

> ⚠️ The site spells itself "Webnex" (page titles, the logo's label), so `source.json`'s name and
> the struct follow it (v15); the id stays `en.webnex`.

> ⚠️ `Element::html()` is `None` for an empty element, and the site streams empty `S:` chunks,
> so `rsc::expand` treats a missing html as empty rather than failing
> (`sources/en.webnex/src/rsc.rs:22`).

## Home

The source reads the home page's own shelves. The owner's order is Popular This Week,
Readers Favourites, New in 2026, Recently Popular, Latest Updates, Manhwa, Manga,
Manhua, Finished and Just Added (`sources/en.webnex/src/home.rs:21`). Latest Updates
uses five rows per carousel page. The site itself puts Latest Updates before Recently
Popular and New in 2026; the source order is an explicit owner override.
Reader shelves, Busiest groups and Latest comments are not series lists and are skipped.

> ⚠️ Readers Favourites is found by its "Readers…" heading, not by `taste-heading`: a
> signed-in page (the app shares the in-app browser's cookies) gives "Your taste" that id,
> while an anonymous page streams Readers' favourites under it and "Your taste" is only the
> loading skeleton (`sources/en.webnex/src/home.rs:35`). When no Readers' favourites row
> exists, the "Your taste" row is shown under that title rather than mislabelled (owner's
> choice, v20). The signed-in markup is inferred from the owner's screenshots, not captured.

> ⚠️ Rows with a site "View all" link (Latest Updates, New in 2026, Recently Popular, Manhwa,
> Manga, Manhua) carry it as their `Listing` id, as VineTheme does, and `ListingProvider` reads
> its `sort`, `kind` and `from` back into browse filters so the catalogue settings still apply
> (`sources/en.webnex/src/lib.rs:160`). An unknown parameter fails the list rather than
> widening it. The site's View all pages do not start with the row's titles (Latest Updates
> and `/browse?sort=updated` differ on the site itself), so the live test compares the listing
> with the site's View all page, not with the row.

> ⚠️ Readers Favourites and Finished are ranked lists of three per page (owner's choice, v16).
> The home page carries no description or genres for the Popular cards (cover, title and type
> only, checked in its HTML and flight data on 2026-11-21); either needs each series page, one
> request per card (16 on the week's shelf), so the hero shows neither.

> ⚠️ Browse's Popular sort is the same weekly chart as home's Popular shelf; the shelf's
> Today and This month tabs are `/?popular=day` and `/?popular=month`, and `/browse` ignores
> a `popular` parameter.

> ⚠️ Sections is a static multi-select like Atsumaru/Toonkotsu. An unset value enables
> every section; a cleared value returns before making a request. Filtered Catalogue
> is a separate option for the existing nonconsecutive-rating fallback. Disabling a
> shelf skips its parsing, but enabled shelves share one home request. Latest Updates
> carries no group metadata, so hidden/preferred groups apply to the full chapter list,
> not home previews. Never add per-card detail requests to compensate.

## Browse parameters

> ⚠️ `tag=` and `genre=` match the same series (and `xtag=` and `xgenre=`), so genres,
> formats, themes and tag taps all send `tag`/`xtag` (`sources/en.webnex/src/helpers.rs:218`).
> "User Created" matches nothing and is not offered.

> ⚠️ The site reads only the first `chapters` value and ignores one that is not a number,
> so the source sends one: a typed Minimum Chapters, else the Chapters preset. Typed values
> must be integers from 1 through 10,000, matching the site input; invalid values return
> an error instead of broadening the search. The year
> presets are client-side: This year is `from=<year>`, Last 5 years `from=<year - 4>`,
> 2010s `from=2010&to=2019`, 2000s `from=2000&to=2009`, Before 2000 `to=1999`; a typed
> Release Year end overrides the preset's.

> ⚠️ The app omits a select's default. Chapters and Released have no visible Any option,
> so their explicit default is the empty string, which is not one of their real preset IDs.
> Removing it silently makes Readable Now and This Year unsendable. Both use tag style;
> Chapters follows Minimum Chapters. Reset Filters clears the presets.

> ⚠️ The genre panel's list (with counts and ~10,000 tags) comes from a Next.js server
> action, a POST to `/browse` with a build-specific `next-action` id, so it is not fetched;
> the static genres, formats and themes in `filters.json` matched the panel exactly.

## Details

> ⚠️ Each author or artist link is one credit as the site defines it: its `href` searches that
> whole string. Some hold separators, but `/` joins one person's native and romanized names
> (`오솟 / Osot` on A Painter Who Draws Dungeons) and commas mix with them
> (`겨울잎 / Jeokbal, purik`), so splitting would duplicate people. Credits stay whole and are
> only deduplicated (checked on 166 links across 25 popular series, 2026-11-21).

> ⚠️ The site types many long strips "Manga": of five popular Manga titles three had pages
> 800 wide and 3,000 to 4,000 tall (2026-11-21), Korean ones and Japanese ones alike. Each of
> those carries a "Long Strip"/"Longstrip" (or "Webtoon") tag, and the paged one carried none,
> so `details` opens a title as a webtoon when its tags say so or its original language is
> Korean or Chinese, and otherwise keeps the type's viewer (v17). Merged MKissa uses the same
> tags but its country only when a title has no type; here the origin overrides "Manga",
> because an untagged Korean "Manga" title measured 800 by 14,941. An untagged Japanese strip
> (800 by 5,000) still opens right to left: nothing on its page tells it from the paged ones.

> ⚠️ Without the web view, the newest imports are `child_nodes`, `kind` and `get_url`, so
> `minAppVersion` is 0.8.3 (v17); it was 0.8.4 only for the old reader's `add_user_script`.

## Chapters

> ⚠️ Chapter `datetime` attributes are UTC (`2026-09-19T23:40:35.000Z`). `helpers::date`
> reshapes them to `yyyy-MM-dd HH:mm:ss`, which the test runner can parse, and passes "UTC"
> to `parse_date_with_options`; plain `parse_date` read them in the reader's zone before v9
> (`sources/en.webnex/src/helpers.rs:155`).

## Reader

> ⚠️ `/read/<key>/pages` (the reader props' `pagesUrl`) answers 403 "Open this chapter on
> webnex.cc to read." without the `wnx_reader_v2` cookie. The chapter page writes it from
> `<script data-wnx-reader>`, and only for a page request: `Accept: text/html` (or
> `Sec-Fetch-Mode: navigate` with `Sec-Fetch-Dest: document`); without either the script is
> absent. The list then needs that cookie and the chapter page as `Referer`
> (`sources/en.webnex/src/reader.rs:9`). No web view is involved.

> ⚠️ The "Opening series" interstitial sets a `wnx_` cookie the same way and reloads;
> `helpers::parse_response` retries with it. It could not be reproduced on 2026-11-20, so
> that path is checked only by the script-cookie fixture test.

## Review evidence (20 November 2026)

Browser inspection confirmed 16 cards per main shelf, 15 Latest Updates cards and eight
entries in each side tab. The combined `/browse?from=2010&to=2019&chapters=50` request
showed 3,746 matches; clearing returned to `/browse`. Firecrawl independently read the
browse control labels. Genres, Formats and Themes support inclusion and exclusion. The owner removed the
typed Tags advanced-search control; arbitrary tags remain tappable on series details.
Reader shelves, groups, comments and grid/table presentation have no corresponding
series-reading control to port. Popular also offers Today and This Month on the site;
the source deliberately retains the owner's Popular This Week selection.

The read-only instruction audit checked every source/resource file against AGENTS,
both workflow skills and every shared reference, including standards row by row.
The feature-usage survey covered all 180 merged sources at `ed872ff0`. Concrete
comparisons and the reason for retaining site-specific code:

| WebNex area | Merged comparison / decision |
|---|---|
| `Source`, partial updates, keys and deep links | MangaBall, Flame Comics, Atsumaru: traits in lib, compact cards, keys sufficient for reading; send partial details only before extra chapter-page requests. |
| `Catalogue`, `Groups`, section settings | XCOMIC, Atsumaru, Toonkotsu: settings reads in settings.rs; preserve cleared lists, search overrides preferences, section defaults include all choices. |
| `search_params`, `browse`, `card` | MangaBall, MangaK, EternalMangas: QueryParameters and compact cards. Current site controls win over Keiyoushi's older coverage sort and User Created tag. |
| `layout` | MangaBall, Kagane, Atsumaru: real home chapters, compact links, five-row page size. No description/genre fan-out. |
| `chunks`, `expand`, `chapter_list` | Paperback's streamed placeholders are HTML fragments. Reassemble them in place; a generic Next.js JSON extractor cannot replace these. |
| `rsc.rs` (`chunks`, `expand`, `chapter_list`, `extract`) | VineTheme is the closest merged build (Next.js App Router flight data) and keeps this parsing in its own `rsc.rs`; `extract` finds an object by its required keys, as Vine's `rsc::extract` does, rather than by key order. WebNex needs neither Vine's text references nor build-ID requests. |
| `details`, `markdown` | Flame Comics' paired text-node/element traversal, FreeWebNovel: escape prose, retain bold/italic, breaks, links and paragraph boundaries; omit empty descriptions. |
| `uploads`, `priority`, `select_chapters` | MangaBall/Atsumaru and the reference clients: remove hidden groups first, keep winning-group parts, deterministic key tie-breaker and official display marker without changing raw group matching. |
| `date`, `parse_relative_date` | VineTheme, EternalMangas: UTC parsing through the host; site-specific compact suffixes include `mo`. Chrono only supplies the current year for presets. |
| `get_html`, `parse_response`, `script_cookie`, reader | Keiyoushi/Paperback agree on chapter navigation plus script cookie and pages endpoint. Keep plain requests, explicit image Referer and complete-page validation. A failed gate retry preserves its HTTP status. |

`review_` regressions cover requested order, headings, real chapter keys/dates, streamed
fragments, page size, preset ordering/defaults, section defaults/clearing, catalogue
clearing/overrides, tags/exclusions, official credits, complete page lists, prose markup
and reversed-input deduplication ties. Live tests also exercise every sort and enabled
genre/format/theme exclusion through the source decoder. App filter layout and image
rendering still require device verification.


The complete attachment/source coverage recheck is recorded in [[webnex-audit]].
