---
title: Atsumaru (en.atsumaru)
type: subsystem
created: 2026-11-20
updated: 2026-11-20
sources:
  - ext: https://github.com/Aidoku-Community/sources/tree/main/sources/en.atsumaru
verified_at: 3ac574e
---

`en.atsumaru` lives upstream only; it was removed from `main` once merged (#843), and a later
change is a new branch from `upstream/main`, as for Toonkotsu. Its tests went with it; the last
copy is at `43255ad` (section 17).

`en.atsumaru` (atsu.moe): comics and novels. Confirmed by live requests from the sandbox, which
reaches the site, unless marked.

- **Site type.** A client-rendered React app: the page is an empty `#root` and a Vite bundle
  (`/assets/index-<hash>.js`), with no server-rendered data to scrape. Everything comes from a
  JSON API under `/api/` and a Typesense search at `/collections/manga/documents/search`. So
  the sources to compare against are the JSON-API ones — `en.toonkotsu`, `multi.kagane`,
  `en.chikari` (also novels), `en.comix`, `multi.mangadex` — not the HTML or GraphQL ones.
- **What it shows.** Details only in the big scroller (Popular This Week), filled from one
  search lookup. Every other row, listing and search card is key, title, cover, status and
  rating. Recently Updated is a plain row, since the feed names no chapter.
- **Filters vs settings.** Content Types and Content Ratings are settings, and both are also
  search filters that override the setting for one search (the owner's call: readers pick
  novels or one comic type from Discover). The site's tag groups are **not**
  search filters (the owner's call): a tapped tag still searches by its id, and each group
  has a Hidden Tags list in settings. The site has 6,140 tags, too many for one list, so the
  19 group lists sit on a Hidden Tags page under Exclusions, which keeps the settings short
  (the owner's choice over listing them inline). Excluded Scanlators leaves chapters from
  named groups out.
- **Home** sends the layout as a partial, then returns the sections that loaded, so a failed
  one drops out rather than leaving an empty row; with every section failing it reports an
  error.
- **Sorts.** Popularity, Title, Trending, Recently Added, Release Date, Top Rated, then the nine
  ranked feeds (Hot Updates to Hidden Gems), then Random. As both reference clients do, the
  sort applies to text searches too, and the arrow maps straight through: off is `desc`, on is
  `asc`, for every indexed sort including Title (Typesense takes `field:asc` on each, probed).
  The feeds and Random have one order and ignore the arrow.
- **Rankings.** Only `popular`, `mostBookmarked` and `mostTalkedAbout` take `timeframe`
  (`daily`, `weekly`, `monthly`, `all`; anything else is a 400); the other feeds accept and
  ignore it (probed). The site's Explore offers a period on the Popularity, Most Bookmarked and
  Most Talked About sorts, All by default; with All, Popularity is the index's `views` sort, and
  otherwise the `popular` feed. Each has a setting, `<feed>Timeframe`, defaulting to weekly,
  that applies only to its Discover section and linked listing. Search ignores these settings:
  Popularity uses the index's `views` sort, and Most Bookmarked and Most Talked About use the
  all-time feeds. A feed sort reads the feed with the content settings, then narrows it with one
  search lookup over its ids, so every search filter applies and the
  feed's order is kept. Random sorts on the index's `shuffleA`/`shuffleB` fields from a seed
  the source picks at launch, so paging stays in one order until the app restarts.
- **Search** is Typesense: `q` (`*` for none), `filter_by`, `sort_by`, `page`, `per_page`,
  `include_fields`. `filter_by` takes `&&`, `||` and parentheses; values are backtick-quoted
  (escape backticks and backslashes). `tags` holds genre names, `genreIds`/`tagIds` the ids;
  tags only match by id, so a tapped tag name is looked up in
  `/api/explore/availableFilters`. `id:=[…]` fetches several series in one request. Its typo
  tolerance can match a nonsense query; that's the site, not a bug.
- **Feeds** `/api/home2/{id}` with `offset`, `limit`, `mediums`, `types`, `adult=1` (adult
  only), `contentRatings` and `excludedTags` (genre and tag ids together; the two never share
  an id). `types` and `mediums` are ANDed, so with novels on the types are filtered locally.
  `contentRatings` only filters rated titles: unrated ones pass, and official translations
  can't be filtered at all. A missing `mediums` or `types` means every one, so a full selection
  is left out; a missing `contentRatings` hides adult titles, so it is always sent (probed).
  No total, so a full page is the only next-page signal.
- **Ratings.** `mbContentRating` (`Safe`, `Suggestive`, `Erotica`, `Pornographic`) plus
  `isAdult`; most titles are unrated (579 of 2,669 novels have one). Unrated titles are rated
  from genres: Hentai (`10`) as Pornographic, Adult (`46`) and Smut (`41`) as Erotica.
  Defaults are Safe and Suggestive; Erotica and Pornographic remain selectable.
- **Series** `/api/manga/page?id=` (`mangaPage: null` for an unknown id, with a 200) carries the
  first chapters and `hasMoreChapters`; `/api/manga/allChapters?mangaId=` has the rest. Both
  list chapters newest first, so only deduplicating sorts. One chapter can appear once per
  group; `scanlators` carry a `score`. The page has no rating, so details also look the
  series up in search.
- **Deduplication.** Remove excluded scanlators first. With deduplication enabled, pick the
  first available name in the ordered Group Preference list (case-insensitive); when none
  match, use the selected Keep measure, then newest upload and chapter id as tie-breakers.
  Keep all same-number entries from that group, plus all unnumbered chapters; the disabled
  mode preserves the site order. Test reversed input order, split chapters, ties, all three
  measures, preference order and exclusions.
- **Keys.** Novel series keys are `novel/<id>`; comic series and all chapter keys remain
  site ids. Search, Discover, details and deep links use the same encoding. Strip the series
  prefix before every API id lookup, including pasted-link search, chapter lists and reading.
  Reader routing depends only on the series key, never URLs or other metadata.
- **Reader** `/api/read/chapter` (pages in order, `/static/…` paths on `cdn.atsu.moe`) and
  `/api/read/novelChapter` (`paragraphs`). Covers are AVIF with a WebP copy beside each.
- **Errors.** Search answers `{"message": …}`; the other endpoints answer plain text
  (`Chapter not found`) or a validation object, so only search's message is shown.
