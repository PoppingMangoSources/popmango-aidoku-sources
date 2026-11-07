---
title: MangaThemesia consumers (en.kingofshojo, en.rokaricomics)
type: subsystem
created: 2026-11-07
updated: 2026-11-07
sources:
  - code: sources/en.kingofshojo/src/lib.rs
  - code: sources/en.kingofshojo/res/filters.json
  - code: sources/en.kingofshojo/src/test.rs
  - code: sources/en.rokaricomics/src/lib.rs
  - code: sources/en.rokaricomics/res/filters.json
  - code: templates/mangathemesia/src/imp.rs
verified_at: 40480d5
---

KingOfShojo and RokariComics are plain consumers of the mangathemesia template: a `BASE_URL` and
`Params { base_url, ..Default::default() }`, registered with `Home`, `ImageRequestProvider` and
`DeepLinkHandler`, the shape of upstream's `en.rizzfables`. Their filters are static
`filters.json` lists read from each site's `/manga/` search form: the sort first (its six options
map to the template's `order` values by position), Status and Type with the site's lowercase ids,
and `genre[]` with the site's own ids (numeric on KingOfShojo, slugs on Rokari).

> ⚠️ They use upstream's template unchanged. The fork's template adds `DynamicFilters` and
> `ListingProvider`, which none of upstream's 15 consumers registers; registering them here would
> put a template change into each submission. A genre the site adds needs adding to
> `filters.json` by hand, as on the upstream consumers.

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

## Icons

> ⚠️ Each `res/icon.png` is the site's 512 px circular favicon (`cropped-…png` under
> `wp-content/uploads`), scaled to 116 px and centred on an opaque 128 px square, white for
> KingOfShojo and black for Rokari (the inside of its disc). `media/sources/*.png` repeats it for
> the README.
