---
title: Rinko Comics (en.rinkocomics)
type: subsystem
created: 2026-11-28
updated: 2026-11-28
sources:
  - code: sources/en.rinkocomics/src/lib.rs
  - code: sources/en.rinkocomics/src/helpers.rs
  - code: sources/en.rinkocomics/src/home.rs
  - code: sources/en.rinkocomics/src/models.rs
  - code: sources/en.rinkocomics/src/test.rs
  - code: scripts/check-sources.py
  - ext: https://rinkocomics.com/comic/
verified_at: 626ef2d
---

Rinko Comics runs the ComicWorld WordPress theme and serves both comics (`/comic/<slug>/`,
chapters at `/chapter/<slug>-chapter-<n>/`) and light novels (`/novel/<slug>/`, episodes at
`/novel_chapter/…`). Everything was checked against the live site on 2026-11-28, which this
sandbox can reach, so the live tests in `src/test.rs` run here.

> ⚠️ Paid chapter and episode rows link to `#` and carry `data-reason="login_required"`; their
> real address is in `data-permalink`, which is what the chapter key comes from
> (`sources/en.rinkocomics/src/helpers.rs:146`). Paid rows on the home page have no permalink at
> all, so the home list shows each title's newest free chapter.

> ⚠️ A long comic lists 20 chapters and loads the rest from `wp-admin/admin-ajax.php` with
> `action=load_more_chapters`, the nonce from `script#comicworld-loadmore-js-extra` (the page holds
> another nonce too), `offset`, `order=desc` and `ui=v2`. Without `ui=v2` the site answers with
> an older `li.chapter` layout (`sources/en.rinkocomics/src/helpers.rs:208`). The total comes from
> the "Chapters" fact; novels list every episode on the page.

> ⚠️ The test runner's `Element::next()` returns nothing, so the facts (Status, Type, Chapters)
> are read from each `main dl > div` and the genre labels from each `#cwBrowse label`, which work
> in both the app and the runner (`sources/en.rinkocomics/src/helpers.rs:107`).

> ⚠️ A chapter page's first `a[href*='/comic/']` is the menu's Comics link. A comma selector list
> such as `"main a[…], a[…]"` matches in page order, so it picked that menu link and pasted chapter
> links opened a series keyed `comic`. `check-sources.py` notes such lists (shadowing fallbacks).

> ⚠️ Novels link genres under `/novels_genres/` and comics under `/comics_genres/`, and each browse
> page lists its own genre set, so the genre filter merges both. A tapped tag sends the genre's
> name; every slug is that name lowercased with dashes (all 33 checked).

> ⚠️ Dates are "Sep 27, 2026" on series pages and ages ("7h", "1mo") on home; a paid comic row
> ends with a "Free on Dec 27" span, so the date is the last span that parses. The runner returns
> `None` for any date-only format, so tests assert only the ages.

> ⚠️ Novel titles on the site already end in "[Novel]". `display_title`
> (`sources/en.rinkocomics/src/helpers.rs:74`) strips that suffix, as the merged Iken template
> does, then adds "(Novel)" when `labelNovels` is on; appending alone showed "[Novel] (Novel)".
