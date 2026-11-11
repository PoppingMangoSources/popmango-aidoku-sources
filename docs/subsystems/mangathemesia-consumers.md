---
title: MangaThemesia consumers
type: subsystem
created: 2026-11-07
updated: 2026-11-11
sources:
  - code: templates/mangathemesia/src/imp.rs
  - code: templates/mangathemesia/src/helpers.rs
  - code: templates/mangathemesia/src/test.rs
  - code: scripts/test-http.py
  - code: .github/workflows/test.yaml
  - code: templates/mangathemesia/src/child_theme.rs
  - code: sources/en.scythescans/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.lagoonscans/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/5882af8cedaea94150b49d4207c37ece5309a1fa/sources/en.ravenscans/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.ragescans/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.violetscans/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.violetscans/src/test.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.thunderscans/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.madarascans/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.madarascans/res/filters.json
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.evascans/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.thunderscans/src/test.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.madarascans/src/test.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/ce4c29829e74a69ca10d9061551ef968a0fe6c1f/sources/en.evascans/src/test.rs
  - ext: https://github.com/Aidoku/Aidoku/blob/main/Aidoku/Features/Source/Home/HomeComponents/HomeChapterListView.swift
verified_at: 5882af8
---

Portable review gates and their coverage are recorded in [[source-checks]].

KingOfShojo and RokariComics are plain consumers of the mangathemesia template: a `BASE_URL` and
`Params { base_url, ..Default::default() }`, registered with `Home`, `ImageRequestProvider` and
`DeepLinkHandler`, the shape of upstream's `en.rizzfables`. Their filters are static
`filters.json` lists read from each site's `/manga/` search form: the sort first (its six options
map to the template's `order` values by position), Status and Type with the site's lowercase ids,
and `genre[]` with the site's own ids (numeric on KingOfShojo, slugs on Rokari).

> ⚠️ KingOfShojo and RokariComics originally submitted upstream's unchanged template.
> Main now has the shared child-theme module and novel reader described below. Keep those
> additions when synchronizing the template; the old dynamic-filter/listing implementation is
> separate and should not return. Static genre lists still need updating from the site's form.

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

> ⚠️ The default template deep-link handler opens series only. Violet, Thunder, Madara and
> Eva override it with `child_theme::handle_deep_link`. Chapter pages are identified by
> `#readerarea`, and `.ts-breadcrumb a` supplies their series key under the configured
> `/comics/` or `/series/` directory. Skip the bare catalogue link. Canonical URLs and trailing
> slashes keep series/chapter keys consistent with lists when incoming URLs have tracking
> queries or fragments. This is one page request and does not fetch series details again.

> ⚠️ Both merged upstream on 2026-11-08 (KingOfShojo #875, RokariComics #874) and left `main`. Their
> last copy with tests is `c667b2d` and the `merged-sources` branch; a later change starts from
> upstream's crate (section 17 of the rules).

## Icons

> ⚠️ Each `res/icon.png` is the site's 512 px circular favicon (`cropped-…png` under
> `wp-content/uploads`), scaled to 116 px and centred on an opaque 128 px square, white for
> KingOfShojo and black for Rokari (the inside of its disc). `media/sources/*.png` repeats it for
> the README.

## The November batch (Scythe, Lagoon, Raven, Rage, Violet, Thunder, Madara, Eva)

Consumers built from the KingOfShojo crate, each `filters.json` generated from the site's own
search form (the sort's six options map by position to the template's `order` values). Eva Scans
came back from maintenance later the same day and is a copy of Madara's crate: the same child theme
and `/series` path. Hades Scans, in maintenance on 2026-11-08, has no crate yet.

> ⚠️ A source's rating follows what most of the site carries, not the genres it lists (the
> owner's call); each title still gets its own rating from its tags. Madara is rated 1 because
> its catalogue includes Hentai. Raven is rated 2 because adult titles fill its lists; the
> remaining consumers in this batch are rated 0. Adult/Smut genre names alone do not require
> increasing a source rating.

> ⚠️ Eva's images go through WordPress.com's proxy (`i1.wp.com/evascans.net/…`), which answers 404
> to Python's and curl's default clients but 200 to a browser user agent, as the app sends. Fetch
> its assets with a browser user agent, or from evascans.net directly.

> ⚠️ The template already turns a standard `div.big-slider` (Scythe, Raven, Rage) into its featured
> `BigScroller`. Violet, Madara, Thunder and Eva run a child theme with `.slidernew` image banners (one
> `<img class="slider-content">` link per series, 1983×793 on Violet, Madara and Eva, 2048×768 on
> Thunder). The template's `get_home` reads neither those nor the child theme's `.bixbox` rows of
> `.bsx` cards, so the whole home lives in the template's `child_theme::get_home` (banners sized by
> `Params::banner_width` and `banner_height`), which those four call from their own `get_home`.
> One shared module rather than four copies follows the template-first rule and the owner's call
> to leave no duplicates; the
> template's `get_home` stays upstream's. This fork's template copy differs from upstream's by that
> module, the base64 reader and the novel text below. No upstream consumer used `slidernew` on
> 2026-11-08.

> ⚠️ Violet's and Madara's Popular Today box holds only its heading: its cards are in a rail beside
> it, inside the `.hotslid` wrapper. Their Editor Picks is a `<section class="bixbox">` of
> `.violet-ep-panel`s whose covers sit apart, matched by `data-index`; it becomes a `BigScroller`.
> Thunder keeps its cards inside each box and has no Editor Picks panel.

> ⚠️ Latest Comics (`#manga-posts`) cards carry a `.chapter-list` of the series' newest chapters,
> so that row is a `MangaChapterList` naming the newest chapter without a `.fa-coins` lock, as the
> owner's Paperback Violet source does. Latest Novels (`#novel-posts`) has the same markup but
> stays a `Scroller` (the owner's call): its newest chapters are mostly locked and the remaining
> link is often the "all chapters" one with no `.epxs`, so a chapter row came out empty and
> vanished from Violet's home in the app. A series whose listed chapters are all locked drops
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

> ⚠️ Violet and Thunder list novels among comics. The shared reader recognizes
> `"is_novel":true` and walks `#readerarea` as Markdown paragraphs. `<strong>`/`<b>` and
> `<em>`/`<i>` keep their emphasis, including nested spans. `<br>` and block boundaries close
> and reopen active emphasis, preventing markers from leaking into later paragraphs. Only
> Markdown syntax is escaped, avoiding excessive inline nodes in long prose.

> ⚠️ The owner chose a shared reader without an opt-in feature. `child_nodes` raises the
> minimum app version to 0.8.3 for every dependent package, even an image-only site.
> The current upstream list has 19 existing consumers, including the newly merged Lagoon
> and Raven sources, plus the four submissions in the child-theme review. Existing sources
> receive upstream version + 1 in that submission; new sources keep version 1. Fork main
> bumps each of its eight consumers once for publication.

> ⚠️ Flame walks text nodes alongside `children`, whose elements the runner can name.
> NovelArchive supplies the emphasis-boundary and syntax-escaping approach used here;
> its mirror images and headings are separate needs and were not copied. Atsumaru receives
> plain paragraph strings and has no HTML emphasis parser. References were checked against
> current merged source files on 2026-11-10.

> ⚠️ A `<p>` can hold dialogue split by `<br>`; each line remains its own Markdown paragraph.
> Regression tests cover nested emphasis, adjacent matching spans in both nesting orders, whitespace, empty text,
> long punctuation-heavy prose, and reading a text page with only the chapter key. Tests stay
> on fork main. App rendering still needs an in-app check.

> ⚠️ The theme supports `author` and `yearx` parameters, but an impossible author returning no
> results does not prove useful author data exists. Rage has no author metadata or author control
> on its search form, so its Author filter is removed. Other consumers need their own evidence;
> the earlier claim that every site's author filter was justified was too broad. Year remains
> separate. Excluding a genre returned the unfiltered list on every reachable site, so
> `canExclude` stays false.

> ⚠️ Rage's details container includes comments. The default template fallback
> `span:contains(Author)` matched a comment discussing the author on the reported series.
> Rage scopes both author and artist selectors to `.tsinfo .imptdt` metadata rows. The app
> supports `:contains`; the runner does not, so a runner returning no author is not evidence
> that an unrestricted selector is safe.

> ⚠️ Rage labels webtoons as Manga. `Params::viewer_override` is `None` by default, preserving
> the template's existing type mapping, and Rage sets it to `Some(Viewer::Webtoon)`.
> The loopback HTTP regression checks that the override wins over a Manga label and that
> default Manga, Manhwa and Comic mapping still works.

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

> ⚠️ Latest Comics sets `page_size: Some(5)`. The app renders `None` as an unpaged vertical
> list, so it does not apply a default five-row cap. Keep all entries and page them instead of
> truncating the feed. The Violet `review_` fixtures cover this, blank card/banner titles,
> compact banner payloads and canonical series/chapter keys; each of the four sources also
> checks a captured free chapter URL and reads it with only its series/chapter keys.

> ⚠️ Madara's catalogue exposes the Novel type even when that selection has no titles.
> Keep both `Novel` and `novel` in its static type options/IDs, matching the form.

> ⚠️ Main checks HTTP status before parsing search, details, reader, home and child-theme
> deep-link responses (`templates/mangathemesia/src/helpers.rs:12`). Non-2xx responses report
> their HTTP status and log URL, content type and at most 240 Unicode characters of the body.
> A child-theme deep link returning 404 remains `None`, since the missing page cannot name a
> series or chapter; its response is checked without parsing the error page.
> Image request providers return an unsent request for the host to execute and keep their
> existing image headers. A Cloudflare challenge must not look like an empty catalogue.

`scripts/test-http.py` exercises both live-tested templates against a local fixture.
The MangaThemesia regression checks every request path above for 403/404/429/503 before
parsing, plus successful 2xx responses and bounded diagnostic output. It is separate from
live-site suites and needs no external site.

## Madara icon review

Madara's original artwork is retained on an opaque `#0e0f13` background, matching the site
header, with its existing 128×128 size and margin. The old white background is removed.

> ⚠️ Adjacent bold-and-italic spans close two markers. Remembering only the last marker
> corrupts their Markdown when the next span opens; the reader keeps the closed markers in
> order. Child-theme banners, cards, chapters and Editor Picks share `link_key` to reject
> blank paths. Editor Picks also trims titles and omits blank descriptions.

## Merged review record and archive

Violet, Thunder, Madara and Eva merged in [884](https://github.com/Aidoku-Community/sources/pull/884),
Rage in [883](https://github.com/Aidoku-Community/sources/pull/883), and Lagoon in
[881](https://github.com/Aidoku-Community/sources/pull/881). Their last main snapshot is
`ce4c298`; complete crates/tests and the shared template are kept on
[merged-sources](https://github.com/PoppingMangoSources/popmango-aidoku-sources/tree/merged-sources).
Raven merged in [882](https://github.com/Aidoku-Community/sources/pull/882); its complete
`5882af8` main snapshot, including tests and icon, is archived there too.

| Review evidence | Result and retained regression |
|---|---|
| Rage: author filter lacked real author metadata; comment prose became the author | Removed the filter and scoped both credit selectors to metadata. The archived Rage contract checks selectors and the omitted control. |
| Rage: webtoons were labelled Manga | Optional viewer override; the template loopback test checks the override and default Manga/Manhwa/Comic mappings. |
| Shared novels: preserving emphasis requires newer imports | Shared support with minimum app 0.8.3 for every dependent consumer; optional Cargo gating was an offered alternative. Main retains paragraph/emphasis and key-only reader tests. |
| Madara icon: white background did not fit the artwork | Kept the artwork/margin and used the reviewed opaque header colour `#0e0f13`. |
| Lagoon | Approved without requested changes. Keep its static site filter IDs and standard template wiring as a reference; no extra reviewer requirement was inferred. |
| Owner's layout/key checks | Archived child-theme fixtures cover home order, five-row paging, compact internal banners, blank titles/keys and canonical chapter links. |

> ⚠️ The archive needs the template version these tests used, not just each consumer's
> directory. Main retains the shared template for Scythe. The archive's older
> KingOfShojo/Rokari metadata minimums move to 0.8.3 for its shared reader imports; archive
> package versions stay unchanged. Future source updates start from current canonical upstream.
