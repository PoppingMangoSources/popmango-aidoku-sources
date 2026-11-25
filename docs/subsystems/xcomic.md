---
title: XCOMIC (multi.xcomic)
type: subsystem
created: 2026-11-20
updated: 2026-11-25
sources:
  - ext: https://github.com/Aidoku-Community/sources/tree/main/sources/multi.xcomic
verified_at: 9f0af67
---

Kept here because the API is undocumented and this took a while to work out. `multi.xcomic`
lives upstream only: #735 added it, #738 added the yona.to mirror, #762 and #771 followed the
site's title api and edition urls, #853 added formats and Merge Editions, and #929 (`4aa33eb`)
dropped the dead `xcomic.net` and `yona.to` mirrors. Its crate and
tests are on `merged-sources` (section 17); a later change restarts from `upstream/main`.

- GraphQL, `POST {base}/query/`. Bato-family schema.
- **Two id namespaces.** `/title/{id}` is the series; `/comic/{id}-{lang}-{slug}` is
  one language edition. Only edition ids work as manga keys. A `/title/` page lists
  its editions in a "Sources" section, one `/comic/` link each.
- The site's own url rewriter maps `/series/{id}` → `/comic/{id}` and
  `/chapter/{id}` → `/comic/_/{id}`, so `_` is its placeholder for an unnamed comic.
- Feeds vs. browse: `get_title_recentlyAdded` and `get_title_latestUploads` are
  their own endpoints and take **no filters**, filter those locally. Browse
  (`get_title_browse_items`) applies the other filters server-side, but **ignores
  `incContentRatings`** (probed 2026-10-31; the site's own rating filter sends it too). Left
  as is on the owner's call, since it worked before and the site is still changing.
- **Editions:** a title has many editions per language (median 14 in English), and the site's
  series page only lists them. `get_comicNode(id) { title_titleNode { comicNodes } }` returns
  every edition in one request; Merge Editions (off by default) costs one chapter list per
  edition. A merged series follows `merged_to` on the site; browse never returns one.
- `ignoreGlobalGenres` / `ignoreGlobalULangs` / `ignoreGlobalBlocks` should stay
  **off**. Turning off the blocklist admits uploads the site hasn't approved.
- Chapter lists come in two forms: `get_comic_chapterList_fullList` (every
  scanlator's upload, paged ~100) and `get_comic_chapterList_uniqList`
  (deduplicated, paged ~1000). Alias both to the same field name so one response
  model reads either.
- Chapter `urlPath` is now `/chapter/{id}` with no language (probed 2026-10-31), so chapter
  lists ask for `comicNode { data { translatedLanguage } }` per item, in the same request.
- The `/search` page no longer renders its genre groups; the categories exist only in the
  server-rendered page state (`_categorys`), and no GraphQL query serves them. Genres, formats
  and original languages are static `filters.json` lists (all matched the live list on
  2026-10-31). Browse matches genres against `rewrite.category_ids`, a broader voted list than
  `genre_ids`, so a filter can match titles whose details don't show that genre.
- Exclusion exists only for genres and formats (`excGenres`); types, demographics, status and
  original languages are include-only, so their filters state `"canExclude": false` (#853).
- `xcomic.me` (default) and `comik.to` remain active after #929 dropped `xcomic.net` and
  `yona.to` (both 301 to `comik.to`); `xcomic.cr` and `xcomic.cs` don't resolve. `mpark.top`
  is a sibling site, not a mirror — different ids.

## Reference implementations

Two other clients for the same site, useful for diffing behaviour:

- keiyoushi `src/all/xcomic` — Kotlin, closest to the API surface.
- The Paperback source in `PoppingMangoSources/general-extensions-mangago`
  (`0.9/test`, `src/XCOMIC`).

When a section returns the wrong entries, diff the **request payload** against
theirs field by field before changing anything. Both times something was wrong it
was a parameter we sent and they didn't, not the endpoint or the sort key.
