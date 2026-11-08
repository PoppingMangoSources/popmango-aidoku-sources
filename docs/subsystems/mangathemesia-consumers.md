---
title: MangaThemesia consumers
type: subsystem
created: 2026-11-07
updated: 2026-11-08
sources:
  - code: templates/mangathemesia/src/imp.rs
  - code: templates/mangathemesia/src/helpers.rs
  - code: templates/mangathemesia/src/child_theme.rs
  - code: sources/en.scythescans/src/lib.rs
  - code: sources/en.lagoonscans/src/lib.rs
  - code: sources/en.ravenscans/src/lib.rs
  - code: sources/en.ragescans/src/lib.rs
  - code: sources/en.violetscans/src/lib.rs
  - code: sources/en.violetscans/src/test.rs
  - code: sources/en.thunderscans/src/lib.rs
  - code: sources/en.madarascans/src/lib.rs
verified_at: 40480d5
---

KingOfShojo and RokariComics are plain consumers of the mangathemesia template: a `BASE_URL` and
`Params { base_url, ..Default::default() }`, registered with `Home`, `ImageRequestProvider` and
`DeepLinkHandler`, the shape of upstream's `en.rizzfables`. Their filters are static
`filters.json` lists read from each site's `/manga/` search form: the sort first (its six options
map to the template's `order` values by position), Status and Type with the site's lowercase ids,
and `genre[]` with the site's own ids (numeric on KingOfShojo, slugs on Rokari).

> ⚠️ They use upstream's template unchanged, and `templates/mangathemesia` on `main` is
> upstream's copy, so Discover, details and search are the template's on `main` and in the
> submissions alike. The fork's old copy added `DynamicFilters`, `ListingProvider` and its own
> home rows, which none of upstream's 15 consumers uses; bringing them back would put a template
> change into each submission. A genre the site adds needs adding to `filters.json` by hand, as
> on the upstream consumers.

> ⚠️ No base URL setting: no merged mangathemesia source has one. If a site moves, change
> `BASE_URL`; keys are paths (`/manga/<slug>/`), so libraries carry over.

> ⚠️ Rokari runs the `manganova` WordPress theme now (KingOfShojo still `mangareader`), but its
> search form, list and series markup are the same, and the live tests pass against upstream's
> template (2026-11-07). Rokari had no completed series then, so its status test compares
> completed with ongoing rather than with the unfiltered list.

> ⚠️ The test runner parses selectors with `scraper`, which rejects jsoup's `:contains(…)`, so the
> template's status, type, author and genre selectors return nothing there and details show no
> status or tags. The app's SwiftSoup supports them, and both sites use the markup they target
> (`.infotable tr:contains(status) td:last-child`, `.seriestugenre a`). Check those in the app.

> ⚠️ The template's deep links open series only: it looks the path up as a series, and a chapter
> link returns `None`.

> ⚠️ Both merged upstream on 2026-11-08 (KingOfShojo #875, RokariComics #874) and left `main`. Their
> last copy with tests is `c667b2d` and the `merged-sources` branch; a later change starts from
> upstream's crate (section 17 of the rules).

## Icons

> ⚠️ Each `res/icon.png` is the site's 512 px circular favicon (`cropped-…png` under
> `wp-content/uploads`), scaled to 116 px and centred on an opaque 128 px square, white for
> KingOfShojo and black for Rokari (the inside of its disc). `media/sources/*.png` repeats it for
> the README.

## The November batch (Scythe, Lagoon, Raven, Rage, Violet, Thunder, Madara)

Seven consumers built from the KingOfShojo crate, each `filters.json` generated from the site's own
search form (the sort's six options map by position to the template's `order` values). Raven is
rated 2: it carries many adult titles and no way to filter them (owner's call). Hades Scans and Eva
Scans showed a maintenance page on 2026-11-08 and have no crate yet.

> ⚠️ The template already turns a standard `div.big-slider` (Scythe, Raven, Rage) into its featured
> `BigScroller`. Violet, Madara and Thunder run a child theme with `.slidernew` image banners (one
> `<img class="slider-content">` link per series, 1983×793 on Violet and Madara, 2048×768 on
> Thunder). The template's `get_home` reads neither those nor the child theme's `.bixbox` rows of
> `.bsx` cards, so the whole home lives in the template's `child_theme::get_home` (banners sized by
> `Params::banner_width` and `banner_height`), which those three call from their own `get_home`.
> One shared module rather than three copies follows the template-first rule and the owner's call
> to leave no duplicates; the
> template's `get_home` stays upstream's. This fork's template copy differs from upstream's by that
> module, the base64 reader and the novel text below. No upstream consumer used `slidernew` on
> 2026-11-08.

> ⚠️ Violet's and Madara's Popular Today box holds only its heading: its cards are in a rail beside
> it, inside the `.hotslid` wrapper. Their Editor Picks is a `<section class="bixbox">` of
> `.violet-ep-panel`s whose covers sit apart, matched by `data-index`; it becomes a `BigScroller`.
> Thunder keeps its cards inside each box and has no Editor Picks panel.

> ⚠️ Latest Comics and Latest Novels cards carry a `.chapter-list` of the series' newest chapters,
> so those rows are `MangaChapterList`s naming the newest chapter without a `.fa-coins` lock, as
> the owner's Paperback Violet source does. A series whose listed chapters are all locked drops
> out (23 of 48 on Thunder's Latest Comics on 2026-11-08): a locked chapter has no `href` in the
> series' list, so it couldn't be opened. Dates are left out, as in the template's own chapter
> rows: the site gives only "NEW" or "1 week". Editor Picks' status comes from its badge.

> ⚠️ Some covers are GIFs. WordPress keeps only the first frame when it resizes one, so a card's
> `…-225x300.gif` is a one-frame GIF while the series page shows the animated original (36 MB on
> Thunder's Full-Time Awakening). The app hands every GIF to Gifu (`SourceImageView.swift`), which
> draws nothing for a GIF that doesn't animate, so those cards show blank. That is the app's bug;
> the source leaves the site's URLs alone.

> ⚠️ Scythe's cache plugin (Autoptimize) moves the reader's `ts_reader.run({..."images":[...]})`
> script into a `src="data:text/javascript;base64,…"` URL, so the template's search for `"images":`
> finds nothing. The template decodes the one script whose base64 starts `dHNfcmVhZGVyLnJ1bih7`
> (`ts_reader.run({`), the selector keiyoushi's base uses for every consumer, and reads the images
> and the novel flag from it; the other sites' chapters carry plain JSON. Scythe keeps only
> keiyoushi's rate limit of three requests a second.

> ⚠️ Violet and Thunder list novels among comics (the other five returned none for `type=novel`).
> The theme's reader data flags a novel chapter with `"is_novel":true` and puts its text in
> `#readerarea`, in `<p>`s on newer chapters and nested `<div>`s on older Violet ones, with
> `"images":[]`. The template checks that flag first and returns one text page from the innermost
> blocks (`helpers::novel_text`), escaping only Markdown syntax, so novels stay in search and home
> with their Novel type option. Inline bold and italic are dropped: keeping them needs
> `child_nodes`, which would raise every consumer's `minAppVersion` to 0.8.3.

> ⚠️ A `<p>` can hold two lines of dialogue split by `<br>` (20 of 99 paragraphs on one Thunder
> chapter), which `text()` runs together, so such a block is split at each `<br>` and each line
> parsed on its own. The runner returns no text from `Element::from(document)`, so each line is
> read from a full parse's `body` (`review_novel_lines_split_at_line_breaks` in Thunder's tests).
> No `#readerarea` held headings on 2026-11-08, although the corruptbytes Paperback fixture has one.

> ⚠️ Beyond the search form, keiyoushi's theme sends `author` and `yearx` text filters and genre
> exclusion as `genre[]=-id`. All seven sites answer a nonsense author and the year 1901 with no
> results (Raven and Rage probed through Firecrawl), and a real author or year finds its series, so
> both filters are in every `filters.json`. Excluding a genre returned the unfiltered list on every
> reachable site, so `canExclude` stays false.

> ⚠️ Rage's, Violet's and Thunder's icons are dark discs, so they sit on black; the others on
> white. Thunder's comes from the inkdex Paperback source's 458 px `static/icon.png`, sharper than
> the site's favicon.

> ⚠️ Coin-locked chapters are `<a data-bs-target="#lockedChapterModal" data-coin=…>` with no `href`,
> and the template drops a chapter without one, so locked chapters stay hidden (no login).

> ⚠️ Raven and Rage answer this sandbox with Cloudflare's challenge, curl and headless Chromium
> alike, so their live tests fail on the network. Their markup was read from Firecrawl and pages
> the owner saved; check both in the app. Rage's icon came from the inkdex Paperback source's
> `static/icon.png`, Raven's through the `i0.wp.com` image CDN.

> ⚠️ The search test takes the title's longest run of letters and digits: these sites' titles carry
> curly quotes and ellipses ("‘Cause", "“Kukuku……") that their search doesn't match.

> ⚠️ violetscans.org redirects to violetmanga.com (keiyoushi has moved too); the id stays
> `en.violetscans`.
