---
title: Keyoapp template
type: subsystem
created: 2026-11-08
updated: 2026-11-08
sources:
  - code: templates/keyoapp/src/lib.rs
  - code: templates/keyoapp/src/imp.rs
  - code: templates/keyoapp/src/helpers.rs
  - code: sources/en.genztoons/src/lib.rs
  - code: sources/en.genztoons/src/test.rs
  - code: scripts/check-upstream-scope.py
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/lib-multisrc/keyoapp
verified_at: 7510d93
---

The Keyoapp template ports keiyoushi's first-generation `Keyoapp` library: a site lists its whole
catalogue on `/series/`, a series page carries its chapters, and a chapter page its images. Genz
Toons is the first consumer; the same live checks (catalogue, details, chapters, pages, home,
chapter deep links) passed against Grim, Mist, Eris, Kewn and Timelesstoons on 2026-11-08.

> ⚠️ Keiyoushi has a second generation, `KeyoappV2` (`?_ajax=search` with offset paging,
> `?_ajax=recent-chapters`, `?_chapters_html=1&series_slug=`, series at the site root). Asmodeus
> and Siren run it, and so do Kaizen and Luna Toons (both refuse headless Chromium with a
> Cloudflare challenge; Firecrawl reads them, and Kaizen's `?_ajax=search` answers
> `{html, has_more}`), though keiyoushi still files Kaizen under the first generation. They
> need their own implementation. Art Lapsa, Kitsune Dawn and Rithar are custom in keiyoushi
> (`/series/` is a 404). MeiToon answers "the website has been stopped".

> ⚠️ The site ignores `q`, `genre`, `type` and `status` on `/series/` (every request returns all
> 263 Genz cards), and filters in the browser from each card's `title`, `tags`, `data-type` and
> `data-status`. So search reads the one page and filters it the same way, with no next page,
> as keiyoushi does. A card's `title` attribute also carries the alternative titles, so a query
> matches those too.

> ⚠️ Genres, types and statuses are each site's own lists from the `initializeDropdownMenu`
> script, written to the consumer's static `filters.json` (the template's README has the
> console snippet), so tags are tappable without `supportsTagSearch`. Genre exclusion works,
> since the filtering is local; type and status are include-only, as on the site.

> ⚠️ Novels appear among comics (`data-type="novel"`, or a `novel` badge on home cards). The
> template has no novel reader, so it leaves them out of search and home, as keiyoushi does.

> ⚠️ Details labels ("Status", "Author", "Artist", "Type") are read with selectors only: a
> `div.grid` whose `div.font-medium > span` names the label, and its `div.font-medium + div`
> value. The test runner returns nothing from `parent()` and `next()`, and the page's search
> menu uses the same words with no value beside them.

> ⚠️ About half the series on Grim, Mist and Timelesstoons have no synopsis anywhere (no
> `#expand_content`, no meta description), so a missing description is `None`, not an error.

> ⚠️ Coin chapters carry `img[alt=Coin]` and need a login, so chapter lists and Latest Updates
> skip them; "Upcoming" rows are links with a future date and an `Upcoming` badge, also skipped.
> Dates are "6 hours ago" or "Aug 3, 2026".

> ⚠️ Pages are `#pages > img[uid]`, and the reader builds each url from a `realUrl` template in a
> script (`https://cdn.meowing.org/uploads/${uid}` on every site checked), which needs no referer.

> ⚠️ The home is generic: every `h2` over cards becomes a row in the site's order, `.latest-poster`
> rows (Latest Updates, Pinned Series) as chapter lists and the rest as card rows, with the
> untitled featured slider first. Some rows repeat for the mobile layout ("Editor's Choice"), so
> titles are kept once; headings with no cards ("Sync Local Library") are skipped.

> ⚠️ A chapter path holds the series id, not its slug, so a chapter deep link fetches the chapter
> page for its series link, skipping the menu's bare `/series/` link (Kewn's chapter pages put it
> first).

> ⚠️ The submission branch is `feat/keyoapp-template`. `check-upstream-scope.py` validates a
> `feat/<template>-template` branch through one of its changed consumers, and the pre-push hook
> verifies every crate it changes.
