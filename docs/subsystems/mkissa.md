---
title: MKissa (en.mkissa)
type: subsystem
created: 2026-11-15
updated: 2026-11-15
verified_at: db1ade7
sources:
  - code: sources/en.mkissa/src/graphql.rs
  - code: sources/en.mkissa/src/home.rs
  - code: sources/en.mkissa/src/lib.rs
  - code: sources/en.mkissa/src/models.rs
  - code: sources/en.mkissa/src/helpers.rs
  - code: sources/en.mkissa/src/reader.rs
  - code: sources/en.mkissa/res/filters.json
  - ext: https://mkissa.to
---

MKissa serves its catalogue from a GraphQL API at `api.mkissa.net/api`. Full POST queries work,
so the persisted-query hashes the site sends are not needed. Queries and the request helper live
in `graphql.rs`, as merged GraphQL sources keep them; response types stay in `models.rs`.

> ⚠️ `queryLatestPageStatus` (Trending) answers `total: null`, so a full page is the only sign
> of another. `sortBy: Random` on browse silently returns Latest; Random is its own
> `queryRandomRecommendation`. `sortDirection` is an enum and is left out.

> ⚠️ Details carry `type` (Manga, Manhwa, Manhua or null) and `countryOfOrigin` (JP, KR, CN,
> EN, OTHER or null). About a fifth of recent series have no type, so the viewer falls back to the
> country, after a "Webtoons" genre or a "Long Strip" tag. Tags arrive as slugs such as
> `format:long_strip`; `tag_names` turns them into display names.

> ⚠️ The search panel's genres only include, but `SearchInput.excludeGenres` works (probed:
> excluding Action left none of 40), so the genre filter excludes by the owner's choice. The
> panel's Tags section (16 rotating trending tags from `queryTags`) is left out at the owner's
> request. A tapped tag arrives as its display name and is matched back with a `queryTags` lookup;
> a name with no match searches as a genre.

> ⚠️ Tags include magazines (`ridi_webtoon-magazine`), which match nothing as `tags` or by slug;
> `SearchInput.magazine` takes the magazine's display name, so a tapped magazine searches that.

> ⚠️ Update Day sends one local day as `dateRangeStart`/`dateRangeEnd` (midnight to 23:59:59).
> Chapters ranges go out as `epRangeStart`/`epRangeEnd`, the last (2001+) without an end. The
> site's season picker always pairs a season with a year, so a season alone sends the current
> year. Its Origin list shows Taiwan, but the API rejects `TW`, so the filter leaves it out.
> Quick picks are shortcuts for the Update sort, the current season and the Top sort.

> ⚠️ `pageInfo.total` is capped at 100 pages of the requested size, as on the site.

> ⚠️ Episode ids in `episodeInfos` arrive as numbers or strings; compare them by value with the
> `availableChaptersDetail` strings, which are the chapter keys.

> ⚠️ `chapterPages` edges are the schema's `Chapter` type: `pictureUrlHead` plus `pictureUrls`.
> There is no `serverUrl` (the API rejects the field), and querying pages directly answers
> `AA_CRYPTO_MISSING`, which is why pages are captured from the site's own reader.

> ⚠️ The reader decodes pages in the page's script and pins a pristine `JSON.parse` taken from a
> new iframe under a generated window key. The web view claims that key first and hides iframe
> windows, then captures `chapterPages`. The site's own human check never clears in a hidden view,
> so after five seconds of it the reader asks the user to solve it in the web view. The test runner
> stubs web views, so reading is checked in the app only.

> ⚠️ The chapter route answers Cloudflare's 403 to a fresh browser while the series page loads,
> so the reader opens the series page and routes to the chapter inside it
> (`sources/en.mkissa/src/reader.rs`). The hidden view blocks images, stylesheets, fonts, media
> and analytics (Chromium, 2026-11-15: load event 0.3-0.5 s instead of 0.8-1.4 s, about 9.6 MB
> fewer bytes, routing unchanged). It must not block `challenges.cloudflare.com`.

> ⚠️ `open` starts the route with `eval` and Rust polls `poll()` once a second. v32 to v34 awaited
> a promise with `WebView::eval_async`, and every chapter failed in the app with
> `JsError(MissingResult)` (owner's logs), so don't go back to it; see
> [[source-checks]] and the host notes in the aidoku-source skill.

> ⚠️ The loaded series page stays on the source in a `RefCell`, as merged Comix keeps its web
> view, and later chapters of any series route inside it (Chromium, 2026-11-15: 3.2-4.2 s for the
> first chapter, about 1 s for each one after with polling). The site answers a chapter that page already showed
> from its own cache, which parses nothing, so `Reader` loads a fresh page for a chapter it opened
> before, and for any reuse that fails or times out. A capture only counts when its
> `chapterString` is the chapter asked for.

Home rows, in the order the owner set: Popular Weekly (big scroller from the popular feed's own
description, genres and status), Manga {year}, Trending, Random, Latest Updates (newest chapter
and date), Most Saved, Most Boosted and {Season} {year} Anime, where the season is the calendar
quarter.
