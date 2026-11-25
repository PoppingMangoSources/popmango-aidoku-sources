---
title: Toonz template (Toonz, Toonx, LewdManhwa, WebtoonClub)
type: subsystem
created: 2026-11-19
updated: 2026-11-24
sources:
  - code: templates/toonz/src/lib.rs
  - code: templates/toonz/src/imp.rs
  - code: templates/toonz/src/helpers.rs
  - code: templates/toonz/src/home.rs
  - code: templates/toonz/src/models.rs
  - code: templates/toonz/src/rsc.rs
  - code: templates/toonz/src/settings.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/templates/toonz/src/test.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.webtoonclub/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.webtoonclub/src/helpers.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.webtoonclub/src/models.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.webtoonclub/src/test.rs
  - ext: https://webtoonclub.com
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.toonz/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.toonz/src/test.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.toonz/res/filters.json
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.toonz/res/settings.json
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.toonx/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.toonx/src/test.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.lewdmanhwa/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.lewdmanhwa/src/test.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/3fbdaee/sources/en.lewdmanhwa/res/settings.json
  - ext: https://toonz.to/comics
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/src/en/toonz
verified_at: 119527c
---

Toonz (`toonz.to`), Toonx (`toonx.to`), LewdManhwa (`lewdmanhwa.com`) and WebtoonClub
(`webtoonclub.com`) share one Next.js engine, which keiyoushi serves from one extension. The `toonz` template
holds all of it; each consumer is a `Params` block naming its catalogue (`comics` or
`webtoons`), the path that resolves every series (`comic` or `webtoon`), the paths series are
filed under, its rating default, type count and home rows (`templates/toonz/src/lib.rs`).
Everything below was probed live on 2026-11-19 unless marked.

## Requests

- Search and listings: `/{catalog}?search=&sort=&status=&genres=&excludeGenres=&author=&page=`
  (`helpers.rs` `SearchParams::page`), the same on all three; `q` is ignored. Sorts are the
  site's Latest, Popular, Top Rated, A-Z (`latest`, `popular`, `rating`, `a-z`). `author`
  takes the site's slug; names are slugified the site's way (punctuation dropped, spaces to
  hyphens).
- Cards: each `img[data-cover]`, walked up to the nearest ancestor that is or holds a series
  link, as keiyoushi's `closest` does (`helpers.rs` `card`). Toonz and Toonx wrap the cover in
  the link; LewdManhwa puts an empty overlay link beside it.
- Details: the series path (`/comic/{slug}` redirects to `/{type}/{slug}` on Toonz and to
  `/adult/{slug}` on Toonx). Toonz and Toonx list Type, Status and Author in a `dl`;
  LewdManhwa shows badges and `/author/` links instead (`helpers.rs` `details`). Series tagged
  as `Manga` still open in `Viewer::Webtoon` when their genres/tags mark a strip format, their
  `Origin` row is Korean or Chinese, or their `Original title` / `h1 ~ p` alt title contains
  Hangul (some mistagged Korean manhwas also list `Japan` as their origin).
- Chapters: `/api/comics/{id}/chapters`, so a chapters-only update still fetches the series
  page for the id. Chapter keys are the slug minus `chapter-` (`2.1`, `45-wad21`).
- Pages: the chapter route with `RSC: 1`, reading the `"images"` array (`{filename}` under
  `/uploads/`), as keiyoushi's client does.
- Home: Toonz reads `/manhwa`, `/manga` and the list view, Toonx `/` and the list view, and
  LewdManhwa `/home` (hero slides and Latest Updates; its Trending Now repeats the slides), each
  in one `send_all`.

> ⚠️ Toonz and Toonx send five Top Rated slides per page but render images for only two.
> LewdManhwa's Featured row also renders images for only the first few slides. The template
> keeps the rendered slide titles and matches missing covers by slug against the embedded
> page data. LewdManhwa can also reuse a matching cover from another row. No extra requests
> are needed, and Toonz keeps all five Manga and all five Manhwa when both pages supply them.
> LewdManhwa's slide texts are cut by the site at about 300 characters, and some start with the
> alternate titles (non-Latin names stripped to empty ` /  / ` entries) and end with a cut
> "Original Webtoon" credit label. The Featured row keeps only the synopsis between them
> (`templates/toonz/src/home.rs`). Showing the text as it comes put the alternate titles in
> place of Someone Stop Her!'s synopsis (owner's screenshot, v12); keep the cleanup.

> ⚠️ The content setting is the `reader_prefs` cookie (`{"rating","types","blockedGenres"}`),
> and every page reads it, home included. The site's own URL `rating` and `types` can only
> narrow the cookie, never widen it, and an included genre the cookie blocks returns nothing. So
> the source never sends those URL parameters: each request writes the cookie itself from the
> effective values, with search filters replacing the settings, which lets a filter widen as
> well as narrow (`helpers.rs` `Content::request`).

> ⚠️ `rating` is a level ("Erotica and below"), not a set: Safe, Suggestive, Erotica, Explicit
> (`pornographic`). The setting and the Content Rating filter are both selects, as the site's
> own control is single-choice; the filter's first option, Any, is omitted by the app, so the
> setting applies until a level is picked (`imp.rs`). Toonz defaults to Suggestive, as a source
> rated 1 must; LewdManhwa defaults to Explicit, the whole catalogue, and is rated 2.

> ⚠️ Toonx's catalogue is all Explicit comics: any lower rating or another type in the cookie
> returns nothing, so it has no rating or type controls and always sends `pornographic`
> (`sources/en.toonx/src/lib.rs`). Its excluded genres still apply.

> ⚠️ The list view prints a chapter number but no chapter link, so Recently Updated keys its
> chapter by that number. A slug with a suffix (`chapter-45-wad21`) answers 404 for the bare
> number (probed on LewdManhwa, 2026-11-20), so `get_page_list` then reads the series' chapter
> list once and retries with the real key (`imp.rs`); the LewdManhwa suite reads chapter 45 by
> its number. The reader takes the first `"images"` array that parses as pages, not merely the
> first one named so (`rsc.rs` `reader_images`).

> ⚠️ A cleared Types setting returns an empty browse result or Discover layout without a
> request. A full selection sends `types:[]`, which the site interprets as every type.

> ⚠️ Next.js streams the page into hidden `S:` chunks after `<main>`, so selectors scoped to
> `main` find nothing. Listing and details selectors take the whole document; home sections
> are self-contained and need no chunk swap.

> ⚠️ Only the list view (`view=list`) prints update times; some of its cards print none, so
> Recently Updated keeps those rows undated. LewdManhwa has no list view, so its Latest
> Updates row names the chapter without a date. Times are the site's `timeAgo` units: just
> now, m, h, d, w, mo, y. Dates from the chapter API are parsed in UTC.

> ⚠️ Series synopses can contain Markdown and paragraph breaks in the text of the HTML element.
> Details preserve that text for the app's Markdown renderer. Discover uses the plain-text path.

> ⚠️ Home rows read each page's own section: Featured (Top rated this month) merged by rating,
> New Titles merged by age, and the visible Today panel of Trending and Popular Updates. An
> absent section drops its row rather than borrowing another page's cards. LewdManhwa's Most
> Read This Week, Top Rated, Uncensored and Yaoi carousels carry covers without series links
> in the server HTML, so they cannot be rows.

> ⚠️ Toonz series and chapter pages answer `403` with `cf-mitigated: challenge`, a Cloudflare
> managed challenge that the app solves in a web view, as for merged sources; the sandbox
> cannot. `helpers.rs` `check_response` turns an unsolved challenge into "The site answered HTTP 403"
> rather than a misleading "Series not found". Toonx and LewdManhwa pages answer the sandbox.

> ⚠️ The app appends a source's `Cookie` header after its stored cookies, so the Cloudflare
> clearance survives; a `reader_prefs` stored by a web view visit would come first and win
> (inferred from `AidokuRunner.swift` `modify`).

> ⚠️ Genres come from each site's `/api/genres` at runtime (Toonz 199, Toonx 607 including
> X-Ray, LewdManhwa 78 on 2026-11-22): the template's `DynamicFilters` builds the Tags filter
> and `DynamicSettings` the Excluded Genres setting, as merged `en.toonkotsu` does from its
> facets and keiyoushi's Toonz does with `fetchFilterData`. The API answers without the
> Cloudflare challenge Toonz pages have. The consumers' JSON no longer lists genres, Toonx has
> no static settings left, and the site's own names replace our title-cased slugs ("3D Family",
> not "3d Family") (`templates/toonz/src/imp.rs`). Tapping a tag in the details view searches the
> template's `slugify` of its name, with no request: it gives the exact slug for all 884 genres
> on the three sites ("D.Va" is `dva`, checked 2026-11-23). The live
`details_chapters_and_pages` test fails on Toonz in the sandbox on the challenge page and needs
an in-app check; Toonx and LewdManhwa run their whole suites.

## WebtoonClub

WebtoonClub uses the same catalogue, cookie, genre API, chapter API and streamed reader
as the other consumers. Its `Params` uses `/webtoons`, `/webtoon/{slug}` and two home
requests. Rows use the owner-requested Toonx order: Top Rated This Month, New Titles,
Recently Updated, Trending, Popular Updates. The hero's own label supplies its title. Social rows are omitted.
Filters and defaults match Toonz, including its five types and Suggestive ceiling.
The icon is the site's `/branding/webtoonclub/icon-180.png`, reduced with margin.

> ⚠️ WebtoonClub reader arrays can have explicit `sortOrder` values. `sources/en.webtoonclub`
> overrides `Impl::parse_page_list` to order those positions while preserving response order when
> positions are missing or tied, leaving the shared template and its other three consumers on the
> unsorted filename-only reader path. Keiyoushi's WebtoonClub reader sorts the same field.

> ⚠️ Browser inspection of Mood Disorder chapter 101 returned warning images from the
> site's own `/uploads/` URLs on 2026-11-23. A valid chapter URL and nonempty image list
> do not prove that every uploaded page is a comic panel. The sandbox also answers 403
> for direct requests, while browser and captured responses load. Browser inspection of
> The Painter Who Paints Dungeons chapter 34 found comic panels after its promotional
> splash. Device verification remains necessary.
