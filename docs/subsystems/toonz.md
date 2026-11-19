---
title: Toonz (en.toonz)
type: subsystem
created: 2026-11-19
updated: 2026-11-19
sources:
  - code: sources/en.toonz/src/lib.rs
  - code: sources/en.toonz/src/helpers.rs
  - code: sources/en.toonz/src/home.rs
  - code: sources/en.toonz/src/models.rs
  - code: sources/en.toonz/src/settings.rs
  - code: sources/en.toonz/src/test.rs
  - code: sources/en.toonz/res/filters.json
  - code: sources/en.toonz/res/settings.json
  - code: sources/en.toonz/res/source.json
  - ext: https://toonz.to/comics
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/src/en/toonz
verified_at: 24fb7b5
---

Toonz (`toonz.to`) is a Next.js site. Listings (`/comics`, `/manhwa`, `/manga`) and the APIs
(`/api/comics/{id}/chapters`, `/api/genres`, `/api/search`) answer plain requests; series and
chapter pages sit behind a Cloudflare managed challenge, which the app solves and the sandbox
cannot. Everything below was probed live on 2026-11-19 unless marked.

## Requests

- Search and listings: `/comics?search=&sort=&status=&genres=&excludeGenres=&author=&page=`
  (`helpers.rs` `SearchParams::page`). Sorts are the site's Latest, Popular, Top Rated, A-Z
  (`latest`, `popular`, `rating`, `a-z`). `author` takes the site's slug; names are slugified
  the site's way (punctuation dropped, spaces to hyphens), which matched 93% of `/api/authors`
  by a plain rule and the listed exceptions by this one.
- Details: `/comic/{slug}` redirects to `/{type}/{slug}`; title, `dl` rows, genre links, the
  `p.whitespace-pre-line` synopsis and `[data-comic-id]` come from the HTML.
- Chapters: `/api/comics/{id}/chapters`, so a chapters-only update still fetches the series
  page for the id. Chapter keys are the slug minus `chapter-` (`2.1`, `84.02`).
- Pages: the chapter route with `RSC: 1`, reading the `"images"` array (`{filename}` under
  `/uploads/`), as keiyoushi's client does (unverified from the sandbox).
- Home: `/manhwa`, `/manga` and `/comics?sort=latest&view=list` in one `send_all`.

> ⚠️ The content setting is the `reader_prefs` cookie (`{"rating","types","blockedGenres"}`),
> and every page reads it, home included. The URL `rating` and `types` can only narrow the
> cookie, never widen it, and an included genre the cookie blocks returns nothing. So each
> request builds the cookie from the effective values, with search filters replacing the
> settings (`helpers.rs` `Content::request`).

> ⚠️ `rating` is a level ("Erotica and below"), not a set: Safe, Suggestive, Erotica, Explicit
> (`pornographic`). The setting is therefore a select, defaulting to the site's own default,
> Erotica; a ticked set with gaps could not be honoured. No cookie also means Erotica.

> ⚠️ A cleared Types setting sends `types:["none"]`, which the site answers with nothing; a
> full selection sends `[]`.

> ⚠️ Next.js streams the page into hidden `S:` chunks after `<main>`, so selectors scoped to
> `main` find nothing. Listing and details selectors take the whole document; home sections
> are self-contained and need no chunk swap.

> ⚠️ Only the list view (`view=list`) prints update times; some of its cards print none, so
> Recently Updated keeps those rows undated. Times are the site's `timeAgo` units: just now,
> m, h, d, w, mo, y.

> ⚠️ Home rows read each page's own section: Featured (Top rated this month) from both type
> pages merged by rating, New Titles merged by age, and the visible Today panel of Trending and
> Popular Updates. An absent section drops its row rather than borrowing another page's cards.

> ⚠️ The app appends a source's `Cookie` header after its stored cookies, so the Cloudflare
> clearance survives; a `reader_prefs` stored by a web view visit would come first and win
> (inferred from `AidokuRunner.swift` `modify`).

The live `details_chapters_and_pages` test fails in the sandbox on the challenge page and needs
an in-app check. Toonx and LewdManhwa run the same engine with different catalogues and home
layouts, so no template was made with one understood consumer.
