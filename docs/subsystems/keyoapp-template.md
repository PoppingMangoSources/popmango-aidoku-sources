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
  - code: sources/en.genztoons/res/filters.json
  - code: templates/keyoapp/README.md
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
> console snippet), so tags are tappable without `supportsTagSearch`. Every filter is
> include-only, as on the site and in keiyoushi: a series needs every chosen genre and any one
> chosen type or status, and none can be excluded (the owner's call; the local filtering could
> exclude, but the site offers no such control).

> ⚠️ A card's `tags` is a JSON list (`["Drama","Sports"]`, apostrophes written `___`), and a
> name can hold a comma ("Host Club, Unrequited Love" on Kaizen), so it splits between quotes.
> Some Genz names carry stray spaces and tabs (`" \taction"`, `"shounen  "`), which the site's
> own filter then misses; the template trims them.

> ⚠️ A pasted series link (`<base>/series/<slug>/`) as the query finds that card by its path,
> with or without the trailing slash, from the same catalogue page.

> ⚠️ An unknown series redirects (302) to `/not-found/`, which has no `div.grid > h1`, so the
> details and chapter paths fail with "Series not found" rather than return an empty series.

> ⚠️ A series page links only its first genres above the title (Mechanic: Slayer shows Action and
> Adventure of six), so details tags are those, as the site shows and keiyoushi reads. Its
> `meta[name=keywords]` starts with every genre, then a per-site tail of brand and type words
> (Timelesstoons' tail even lists Action and Shounen), so it can't give the tags, but the
> content rating reads it too: an Eris series linking Romance and Drama lists Mature and Smut
> there, and rated Safe before.

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
> An image without a `uid` fails the page list rather than dropping that page.

> ⚠️ The home is generic: every `h2` over cards becomes a row in the site's order, with the
> untitled featured slider first. Latest Updates is a chapter list of five per page; Pinned
> Series uses the same `.latest-poster` markup, so the heading tells them apart and Pinned is a
> card row like the rest. Some rows repeat for the mobile layout ("Editor's Choice"), so titles
> are kept once; headings with no cards ("Sync Local Library") and Bulk Discounted Series (coin
> chapters for sale) are skipped (the owner's call). Genz's Pinned Series lists the same series
> as its Latest Updates, so Genz hides it with `Params::hidden_rows`; Mist, Timelesstoons and Eris
> keep theirs.

> ⚠️ A row's cards are found by climbing at most three parents from its heading, and the climb
> stops at a block holding another row's heading: otherwise a heading with no cards near the
> top of the page reaches `<body>` and takes every card on it (caught by the offline fixture).
> `parse_home` is apart from the request so `review_` tests can check the layout against a
> cut-down page with no network.

> ⚠️ The featured slider is covers on Genz (`aspect-[0.74/1]`) and 2:1 banners on Mist, Kewn
> and Timelesstoons (`aspect-[2/1]`, 1000px wide), so a banner slider is an image scroller sized
> 400 by 200 and the rest a card row. The banner slides carry only a title and the image: Mist's
> descriptions and genres belong to its Collections carousel (themed groups, not series), so a
> big scroller there would cost a details request per slide. Chromium shows `--clone` slides
> that the script adds; the served page has none.

> ⚠️ Editor's Choice is a big scroller. Its picks are `.editor-pick-button` divs with no link:
> the site's script opens `/series/<data-id>`, loads the cover from `data-img` on the upload
> host and decodes `data-description` from base64. The key keeps the trailing slash so it
> matches the search cards, or the same series would be added to the library twice.

> ⚠️ A chapter path holds the series id, not its slug, so a chapter deep link fetches the chapter
> page for its series link, skipping the menu's bare `/series/` link (Kewn's chapter pages put it
> first).

> ⚠️ The submission branch is `feat/keyoapp-template`. `check-upstream-scope.py` validates a
> `feat/<template>-template` branch through one of its changed consumers, and the pre-push hook
> verifies every crate it changes.
