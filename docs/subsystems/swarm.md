---
title: Swarm (multi.swarm)
type: subsystem
created: 2026-11-19
updated: 2026-11-19
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
verified_at: afbc5da
---

Swarm (`swarm.ws`) is a Next.js site over a JSON API, reachable from the sandbox; the live
suite runs. Probed on 2026-11-19.

## Requests

- Browse `/api/manga/filtered`, text search `/api/comic/search` (`title=`), both with `limit`,
  `offset`, `order` (`last_updated`, `title`, `newest`), `genres`, `types`, `tags`, `nsfw`,
  `suggestive`, `yearStartMin`/`yearStartMax` and `minChapters` (`api.rs`).
- Series and chapters: `/comic/{slug}?_rsc` with `RSC: 1`, the `initialData` object
  (`api.rs` `comic_page`). Chapter keys are upload ids.
- Pages: `POST /api/sp` with `{"payload":"ivHex:cipherHex"}`, AES-256-CBC under SHA-256 of a
  constant from the site's reader (`helpers.rs` `encrypt_payload`).
- Novels: `/api/novel`, `/api/novel/{slug}`, `/api/novel/{slug}/chapter/{n}`; keys `novel/{slug}`.
- Home: popular and new-comics analytics (`window=7d`), `/api/latest?filter=hot|new`, novels.

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
