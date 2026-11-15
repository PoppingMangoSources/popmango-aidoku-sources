---
title: MKissa (en.mkissa)
type: subsystem
created: 2026-11-15
updated: 2026-11-15
verified_at: 96de49c
sources:
  - code: sources/en.mkissa/src/graphql.rs
  - code: sources/en.mkissa/src/home.rs
  - code: sources/en.mkissa/src/lib.rs
  - code: sources/en.mkissa/src/models.rs
  - code: sources/en.mkissa/src/helpers.rs
  - code: sources/en.mkissa/src/webview.rs
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

> ⚠️ The search panel's genres and tags only include, so neither filter offers exclusion, though
> `SearchInput` has `excludeGenres` and `excludeTags`. Its Tags section is the 16 trending tags
> (`queryTags` with `queryType: Home`), sent by slug (`theme:family_life`), so it is a dynamic
> filter. A tapped tag arrives as its display name and is matched back to a slug with a
> `queryTags` lookup; a name with no matching slug searches as a genre.

> ⚠️ Update Day sends one local day as `dateRangeStart`/`dateRangeEnd` (midnight to 23:59:59).
> Chapters ranges go out as `epRangeStart`/`epRangeEnd`, the last (2001+) without an end. The
> site's season picker always pairs a season with a year, so a season alone sends the current
> year. Its Origin list shows Taiwan, but the API rejects `TW`, so the filter leaves it out.
> Quick picks are shortcuts for the Update sort, the current season and the Top sort.

> ⚠️ `pageInfo.total` is capped at 100 pages of the requested size, as on the site.

> ⚠️ Episode ids in `episodeInfos` arrive as numbers or strings; compare them by value with the
> `availableChaptersDetail` strings, which are the chapter keys.

> ⚠️ The reader decodes pages in the page's script and pins a pristine `JSON.parse` taken from a
> new iframe under a generated window key. The web view claims that key first and hides iframe
> windows, then captures `chapterPages`. The site's own human check never clears in a hidden view,
> so after five sightings the reader asks the user to solve it in the web view. The test runner
> stubs web views, so reading is checked in the app only.

Home rows, in the order the owner set: Popular Weekly (big scroller from the popular feed's own
description, genres and status), Manga {year}, Trending, Random, Latest Updates (newest chapter
and date), Most Saved, Most Boosted and {Season} {year} Anime, where the season is the calendar
quarter.
