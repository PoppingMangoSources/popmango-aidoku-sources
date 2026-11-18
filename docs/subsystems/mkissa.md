---
title: MKissa (en.mkissa)
type: subsystem
created: 2026-11-15
updated: 2026-11-18
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
> The local day comes from `get_utc_offset()`, which is UTC minus local time in seconds, so
> local midnight is `(now - offset)` floored to a day, plus the offset
> (`sources/en.mkissa/src/helpers.rs`, kkantan on #910).

> ⚠️ Descriptions arrive as Markdown (`**Links:**`, `[Official Twitter](…)`, `___`) or as
> HTML with `<br>`, `<b>`, `<i>` and `<a>`; 200 popular series showed both (2026-11-18).
> Details keep the Markdown unescaped, turn bold and italic tags into it and end lines with hard
> breaks; raw newlines only count when the text has no HTML breaks, since `<br>\n` is one break.
> The big scroller gets the same lines as plain text. Escaping showed every marker (#910).

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
> (`sources/en.mkissa/src/reader.rs`).

> ⚠️ v32 to v35 tried a content rule list (images, stylesheets, fonts, media, analytics),
> `WebView::eval_async`, and a kept series page together. Chromium passed all of it, but in the app
> every chapter failed with `JsError(MissingResult)` and then the check message (owner's logs,
> 2026-11-15), so v36 restored v31. Those versions also returned any failed `eval` as an error,
> while v31 skips one that fails while the page is busy; `wait_for_pages` keeps v31's loop.

> ⚠️ v37 adds only the kept page to v31: `Reader` holds the last series page in a `RefCell`, as
> merged Comix keeps its web view, and `open` routes it to the next chapter of any series
> (Chromium: about 1 s per later chapter instead of 3.7-5.6 s). The site answers a chapter that
> page already showed from its own cache, which parses nothing, so a repeat chapter or a failed
> reuse loads a fresh page as v31 did. A capture counts only when its `chapterString` is the
> chapter asked for. Try one change per release in the app.

> ⚠️ In the app (owner, 2026-11-18) the first chapter loads slowly, then every later chapter,
> of any series, loads fast: the kept page works, and the first load is the cost. The page is a
> SvelteKit shell whose `kit.start(app, element)` has no hydration data, and its `start()` awaits
> the first navigation before the router adds its click listener to `document.documentElement`
> (bundle `yF11j1xh.js`, read 2026-11-18). v39 clicks as soon as that listener appears instead of
> waiting for the series page's `[data-href]` links: in Chromium the click moved from 1.7-2.3 s to
> 1.1-1.4 s and the first chapter from 2.7-3.5 s to 2.2-3.1 s, usually before the series page's
> covers download. The owner found it faster in the app, with no failing load across several
> series and chapters (2026-11-18).

> ⚠️ Loading the shell straight at the chapter URL renders the reader without ever requesting its
> pages, and so does an unknown path; client-side navigation from a real route is the only way
> in. A search route works too, but has no `[data-href]` links to wait for.

> ⚠️ AidokuRunner's web view is created with a zero frame and never joins a window
> (`WebViewHandler.swift:25`). `load_html_blocking` is `load_html` plus `wait_for_load`, a
> semaphore only `didFinish` signals: the handler has no `didFail` delegate, so a navigation that
> fails would block the reader forever. v40 loaded without blocking and let the 30 second poll
> expire instead, but in the app chapters then failed with the site's check message, so v41 went
> back to v39's blocking load (owner's logs, 2026-11-18). v41 failed the same way, so v42 restored
> v40's load and added `set_rate_limit(3, 1, TimeUnit::Seconds)`, the limit Inkdex's AllManga uses
> (keiyoushi limits the API host to 1 a second). The API's "Too many requests, please try again
> in N seconds" reaches readers through `graphql.rs`.

> ⚠️ Settings has a Pass Human Check web login on `https://mkissa.to/`, as merged Toonkotsu has.
> The settings sheet and the reader's web view share the source's data store
> (`WKWebsiteDataStore.forSource`). On v42 the owner completed the check in settings and
> reopened a chapter successfully on their iPhone. Plain requests use a separate session.
> v43 prompts after five consecutive challenge polls. Reset Session indicates saved cookies.
> On iOS before 17 the store is not persistent. Merged Comix and Toonkotsu still block.
> Toonkotsu loads an empty document under
> the site's origin, so its wait costs nothing. `sleep` takes whole seconds, so polling adds up to
> a second after the pages are captured.

Home rows, in the order the owner set: Popular Weekly (big scroller from the popular feed's own
description, genres and status), Manga {year}, Trending, Random, Latest Updates (newest chapter
and date), Most Saved, Most Boosted and {Season} {year} Anime, where the season is the calendar
quarter.
