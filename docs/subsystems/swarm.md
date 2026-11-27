---
title: Swarm (multi.swarm)
type: subsystem
created: 2026-11-19
updated: 2026-11-27
sources:
  - code: sources/multi.swarm/src/lib.rs
  - code: sources/multi.swarm/src/api.rs
  - code: sources/multi.swarm/src/helpers.rs
  - code: sources/multi.swarm/src/home.rs
  - code: sources/multi.swarm/src/models.rs
  - code: sources/multi.swarm/src/settings.rs
  - code: sources/multi.swarm/src/test.rs
  - code: sources/multi.swarm/res/filters.json
  - code: sources/multi.swarm/res/settings.json
  - code: sources/multi.swarm/res/source.json
  - ext: https://swarm.ws/browse
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/src/all/swarm
verified_at: 9fcd3ad
---

Swarm (`swarm.ws`) is a Next.js site over a JSON API, reachable from the sandbox; the live
suite runs. Probed on 2026-11-19, 2026-11-26 and 2026-11-27.

## Requests

- Browse `/api/manga/filtered`, text search `/api/comic/search` (`title=`), both with `limit`,
  `offset`, `order` (`last_updated`, `title`, `newest`), `genres`, `types`, `tags`, `nsfw`,
  `suggestive`, `yearStartMin`/`yearStartMax` and `minChapters` (`api.rs`).
- Series and chapters: `/api/comic/slug/{slug}/details` (`api.rs` `comic`, decoding `ComicDetails`
  when chapters are omitted and `ComicPage` when chapters are requested). Chapter keys are upload ids.
- Pages: `POST /api/sp` with `{"payload":"ivHex:cipherHex"}` encrypting
  `{"chapterId":"...","clean":true}` under SHA-256 of a constant from the site's reader
  (`helpers.rs` `encrypt_payload`); relative entries and `dataSaver` fallbacks resolve against
  `baseUrl` (`models.rs` `Pages::into_urls`).
- Novels: `/api/novel`, `/api/novel/{slug}`, `/api/novel/{slug}/chapter/{n}`; keys `novel/{slug}`.
- Home: popular and new-comics analytics (`window=7d&limit=25`), `/api/latest?filter=hot|new`,
  novels (`home.rs`).

> ⚠️ `/api/latest` intermittently stalls on cache misses (up to Cloudflare 524). Match the site's
> query order (`limit=50&filter=...&nsfw=...&suggestive=...&page=1`) and send all five Home
> requests in a single `Request::send_all` batch with a 2.5s timeout on `/api/latest` (6s on the
> rest) so `get_home` releases the `Interpreter` WASM actor lock promptly instead of blocking
> subsequent series/chapter calls. Precompute chapter sort keys once per filtered item before
> `sort_unstable_by` so large catalogues do not repeat label parsing in interpreted `Wasm3`.

> ⚠️ Without `"clean":true`, `/api/sp` can return expired `/noah/<hash>/` URLs that 404 on
> `cash.swarm.ws` (or temporary unstitched slices deleted once stitching finishes). Cold stitched
> WebPs on `cash.swarm.ws` also take 5–20s to pull from origin: `HEAD` leaves `cf-cache-status: MISS`,
> whereas `GET` with `Range: bytes=0-0` (`.timeout(0.4)`) populates `cf-cache-status: HIT`.
> `ImageRequestProvider` (`lib.rs`, `api.rs` `warm`, `helpers.rs` `next_warm_range`) warms the next
> 10 pages ahead in one 0.4s batch once page 0 starts downloading, and sets a 60s timeout on reader
> pages (25s on covers) without `Referer` (matching the site's `referrerPolicy="no-referrer"`).

> ⚠️ Any filter collapses `total` to offset plus one page plus one, so the next page is
> "total exceeds what was shown", not a real count.

> ⚠️ The API has no exclusion parameters and no `maxChapters`; excluded genres and tags, and
> the Chapters range maximum, are applied to whatever each record carries. Search results have
> genres but no tags or counts; Hot and New Updates carry no genres, tags or types, only a
> language; only the popular rows carry tags. The settings footer says so.

> ⚠️ Popular and new-comics ignore every rating parameter and hold no adult titles; `/api/latest`
> honours `nsfw`, `nsfwOnly` and `suggestive` as the site's home sends them. `suggestive`
> changes nothing on browse.

> ⚠️ `order=created` answers an error on `/api/manga/filtered` without a query; `newest` is the
> same sort and works on browse and search alike (`api.rs` `SORTS`).

> ⚠️ Some records hold `genres`, `tags` and `publishers` as Python list reprs
> (`"['Action', \"Boys' Love\"]"`); `models.rs` `list` reads both shapes. The popular feed sends
> `chapterCount` and `chapter_count` together, so only one may be named.

> ⚠️ Chapter languages use regional and legacy codes (`es-la`, `kr`, `pt-br`); `helpers.rs`
> `app_language` maps them to the app's. Large series list thousands of uploads (Solo Leveling
> 4,716), one per uploader copy (`official2` is a second copy of `official`); the site's own pick is the
> lowest rank letter, then the first copy, which deduplication follows.

> ⚠️ A blocked uploader blocks its numbered copies too, in the chapter list (`models.rs`
> `chapter_list`) and in the Hot and New Updates rows (`home.rs` `updates`), which keep each
> series' first chapter in the site's order; `review_home_updates_follow_languages_and_blocked_uploaders`
> holds the home side.

> ⚠️ The Content Rating filter mirrors the site's All, Safe and NSFW with nothing picked, so the
> setting applies until one is; All, or Safe and NSFW together, is the site's All
> (`lib.rs`, `ratings_follow_the_setting_and_filter`). Aidoku never sends a multi-select with
> nothing picked, so an empty selection needs no arm.
