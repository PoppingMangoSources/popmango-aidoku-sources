---
title: OniSaga (multi.onisaga)
type: subsystem
created: 2026-11-27
updated: 2026-11-28
sources:
  - code: sources/multi.onisaga/src/api.rs
  - code: sources/multi.onisaga/src/filters.rs
  - code: sources/multi.onisaga/src/home.rs
  - code: sources/multi.onisaga/src/helpers.rs
  - code: sources/multi.onisaga/src/lib.rs
  - code: sources/multi.onisaga/src/models.rs
  - code: sources/multi.onisaga/src/settings.rs
  - code: sources/multi.onisaga/res/filters.json
  - code: sources/multi.onisaga/res/settings.json
  - code: sources/multi.onisaga/res/source.json
  - code: sources/multi.onisaga/src/test.rs
  - ext: https://onisaga.com/browse
verified_at: 7169615
---

OniSaga (`onisaga.com`) is a Next.js site behind Cloudflare with public media on
`media.onisaga.com` and a strict Zod-validated JSON RPC layer under `POST /api/rpc/<method>`.
Everything here was verified against the site's client/server bundle and HTTP captures
(2026-11-21 and 2026-11-27); this sandbox gets Cloudflare's HTTP 403 (`cf-mitigated: challenge`),
so `review_` tests verify all RPC contracts offline while live tests in `src/test.rs` require an
unblocked host.

## RPC endpoints

Every RPC call sends `{"json": input}` with `Origin: https://onisaga.com` and
`Referer: https://onisaga.com/` and reads `{"json": output}` (`sources/multi.onisaga/src/api.rs:162`).

- **Search and listings**: `POST /api/rpc/catalog/browse` (`sources/multi.onisaga/src/api.rs:29`)
  returning `{"items": [...], "nextCursor": ...}`, shared across search and the four `source.json`
  listings (`Latest Update`, `New on Site`, `Finished Series`, `First Published`). Cursors are
  server-signed for the exact query and filter tuple, so `OniSaga` stores bounded
  `(page:serialized_params, cursor)` pairs rather than one shared cursor
  (`sources/multi.onisaga/src/lib.rs:44`). The `readable` check filter uses `"title": "Availability"`
  and `"name": "Readable Chapters"` with `"hideFromHeader": true` to match OniSaga's `/browse`
  panel (`sources/multi.onisaga/res/filters.json:118`).
- **Dynamic taxonomy (`Genres` and `Excluded Genres`)**: `POST /api/rpc/catalog/taxonomy` (`{}`)
  is fetched once on demand and cached in `RefCell<Option<Vec<TermOption>>>`
  (`sources/multi.onisaga/src/filters.rs:15`), populating `Genres` via `DynamicFilters`
  (`sources/multi.onisaga/src/filters.rs:71`) and `Excluded Genres` via `DynamicSettings`
  (`sources/multi.onisaga/src/filters.rs:90`), and routing tapped detail tags across origins,
  demographics, publication formats, genres, themes and freeform tags
  (`sources/multi.onisaga/src/filters.rs:33`).
- **Home**: `POST /api/rpc/catalog/home` (`sources/multi.onisaga/src/home.rs:176`) returns the six
  Discover sections (`Spotlight`, `Trending This Week`, `Start a New Series`, `Latest Updates`,
  `Finished Series`, `Newly Published`) in one JSON call (`sources/multi.onisaga/src/home.rs:19`).
  When `excludedGenres` is non-empty, `origins` is narrowed, or `en` is deselected, the three
  browse-backed shelves (`new`, `finished`, `published`) are fetched in parallel with
  `catalog/home` via `catalog/browse` so server-side genre, origin and language filters apply
  (`sources/multi.onisaga/src/api.rs:129`, `sources/multi.onisaga/src/home.rs:178`).
- **Details**: `POST /api/rpc/catalog/detail` (`{"slug"}`) returns full title metadata, credits,
  spoiler-flagged terms, demographic and `reading` stream/language/group tables
  (`sources/multi.onisaga/src/models.rs:138`).
- **Chapters**: `POST /api/rpc/chapters/index` pages units 200 at a time (`order: "desc"`) per
  selected language scope (`sources/multi.onisaga/src/api.rs:225`,
  `sources/multi.onisaga/src/api.rs:282`). Units whose `selection` is `null` (non-default
  languages or multi-group rows) batch `POST /api/rpc/chapters/select` in 200-ID chunks
  (`sources/multi.onisaga/src/api.rs:239`). Each unit promotes OniSaga's `selectedReleaseId`
  first, and the opt-in `deduplicateChapters` (`Deduplicate Chapters`, default `false`) setting
  truncates each unit to that preferred upload without dropping distinct chapter parts or
  unnumbered units (`sources/multi.onisaga/src/models.rs:528`,
  `sources/multi.onisaga/res/settings.json:76`).
- **Reader pages**: `POST /api/rpc/reader/bootstrap` (`{"releaseId"}`) returns `manifest.pages`
  (`sources/multi.onisaga/src/lib.rs:90`, `sources/multi.onisaga/src/models.rs:608`).

> ⚠️ Aidoku's `HomeBigScrollerView` gates the entire bottom pill row — including the colored
> `Suggestive` / `18+` content-rating pill — behind non-empty `entry.tags`. `catalog/home` Spotlight
> cards omit `terms` (genres require per-card `catalog/detail` RPCs, which Home avoids), so
> `Work::into_details` populates `tags` from `[origin, status, chapter count, demographic, form]`
> with a `"Comic"` fallback (`sources/multi.onisaga/src/models.rs:330`). The row has no status or
> count of its own, so both ride as tags in the site's slide order; the count is the highest
> `languages[].readableUnits` among the reader's languages ("37 chapters", "1 chapter").

> ⚠️ OniSaga's Zod schema on `catalog/browse` is `.strict()` and rejects duplicate array items,
> more than 10 `languages`, more than 20 `include` or `exclude` slugs per taxonomy group,
> `year.from > year.to`, `"relevance"` sort without `q`, and non-ASCII term slugs outside
> `^[a-z0-9]+(?:-[a-z0-9]+)*$` (`sources/multi.onisaga/src/api.rs:94`,
> `sources/multi.onisaga/src/models.rs:44`, `sources/multi.onisaga/src/helpers.rs:210`). `slugify`
> folds Latin diacritics (`Shōnen` → `shonen`) before splitting on ASCII non-alphanumerics, and
> `form_slug` / `origin_slug` map hyphenated pill labels (`One-shot` → `oneshot`, `Art book` →
> `artbook`, `Other Asian` → `other_asian`) back to OniSaga's enum IDs
> (`sources/multi.onisaga/src/helpers.rs:84`).

> ⚠️ `chapters/index` treats `editionId` as optional, whereas `chapters/select` requires
> `"editionId": null` when a scope has no edition (`sources/multi.onisaga/src/api.rs:258`).

## Ratings, origins, languages and media URLs

Source `contentRating` is `1`. The `contentRating` select mirrors the site's own prompt: `All
ages` (safe), `Include suggestive` (safe and suggestive, the default) and `18+ · I'm 18 or older`
(all four), each adding to the one before (`sources/multi.onisaga/src/settings.rs:15`). There
is no Content rating search filter: the site only shows Mature and Explicit titles after its 18+
confirmation, which this setting gives, so a filter could not override it (owner's call,
2026-11-28).
Selecting all seven origins omits `origin` (`None`) so titles with `origin: null` still appear;
clearing `origins` or `languages` returns empty results before issuing any network request.

> ⚠️ `POST /api/age-confirmation` sets a 180-day `__Host-onisaga.age_confirmed` cookie on
> `onisaga.com`. `sync_age` follows the 18+ setting only (confirm on, `action=withdraw` off), so
> a search no longer flips the cookie mid-session. A refused confirmation is an error, not a
> silently remembered state. `detail` and `bootstrap` go through `gated`, which confirms and
> retries once on HTTP 403/404 so saved library entries and deep links still open
> (`sources/multi.onisaga/src/lib.rs:82`, `sources/multi.onisaga/src/api.rs:171`).

> ⚠️ `media.onisaga.com/api/covers/<slug>` answers adult titles with a small blurred placeholder
> (`x-art: blur`) because the host-only age cookie never reaches it; the site itself loads adult
> covers from `onisaga.com/api/covers/`. `build_work` moves Mature and Explicit covers to
> `BASE_URL`, where the app adds the shared cookie to the cover request
> (`sources/multi.onisaga/src/models.rs:285`). Safe and Suggestive covers stay on the media host.

> ⚠️ Once the age is confirmed, the API returns every Mature and Explicit `coverUrl` as a bare
> path (`/api/covers/<slug>?mv=N`), while Safe and Suggestive ones stay absolute media-host urls
> (all 15 adult and 189 other cards in a signed-in capture, 2026-11-28). Aidoku cannot load a
> path, so `build_work` prefixes `BASE_URL`; without that, every adult cover is blank.

> ⚠️ A cover without `w=` is the full original (472 KB for one JPEG, against 65 KB as WebP at
> `w=480`). The site's own cards request `w=160` to `w=480`, so `build_work` appends `w=480` with
> `&` or `?` as the url needs (captured site code, 2026-11-21).

> ⚠️ Some uploads repeat the number in the chapter title ("Chapter 46 46"). `clean_chapter_title`
> treats a title whose words after the chapter prefix are all the chapter number as a duplicate
> (`sources/multi.onisaga/src/helpers.rs:195`).

> ⚠️ Safe and Suggestive page URLs carry `p=sg` and load directly from `media.onisaga.com`;
> age-gated page URLs carry `p=sc` (or lack `p=sg`) and must be rewritten to `onisaga.com` because
> the `__Host-` age cookie is host-only (`sources/multi.onisaga/src/helpers.rs:299`). With the age
> confirmed, adult chapters' page urls come as bare paths (`/api/media/<id>?...&p=sgmx...`), which
> `media_url` prefixes with `BASE_URL` (signed-in capture, 2026-11-28).

> ⚠️ Page URLs carry short-lived `e=<unix_expiry>` signatures. `get_page_list` appends
> `#oni=<releaseId>.<ord>`, and `ImageRequestProvider` strips that fragment, adds
> `Referer: https://onisaga.com/`, and re-bootstraps the chapter once when `now + 60 >= expiry`
> (`sources/multi.onisaga/src/lib.rs:261`, `sources/multi.onisaga/src/helpers.rs:306`).

> ⚠️ A title's `reading.languages` can list the same language under more than one numbering stream
> (for example a monochrome default stream and a partial colour stream). `language_scopes` picks
> the scope matching `defaultScope` or `streams[].isDefault` before falling back to the highest
> `readableUnits` count (`sources/multi.onisaga/src/models.rs:495`).
