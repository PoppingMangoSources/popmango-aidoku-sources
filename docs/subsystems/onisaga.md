---
title: OniSaga (multi.onisaga)
type: subsystem
created: 2026-11-27
updated: 2026-11-27
sources:
  - code: sources/multi.onisaga/src/api.rs
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
verified_at: 966ddd3
---

OniSaga (`onisaga.com`) is a Next.js site behind Cloudflare with public media on
`media.onisaga.com` and a strict Zod-validated JSON RPC layer under `POST /api/rpc/<method>`.
Everything here was verified against the site's client/server bundle and HTTP captures
(2026-11-21 and 2026-11-27); this sandbox gets Cloudflare's HTTP 403 (`cf-mitigated: challenge`),
so `review_` tests verify all RPC contracts offline while live tests in `src/test.rs` require an
unblocked host.

## RPC endpoints

Every RPC call sends `{"json": input}` with `Origin: https://onisaga.com` and
`Referer: https://onisaga.com/` and reads `{"json": output}` (`sources/multi.onisaga/src/api.rs:159`).

- **Search and listings**: `POST /api/rpc/catalog/browse` (`sources/multi.onisaga/src/api.rs:24`)
  returning `{"items": [...], "nextCursor": ...}`, shared across search and the four `source.json`
  listings (`Latest Update`, `New on Site`, `Finished Series`, `First Published`). Cursors are
  server-signed for the exact query and filter tuple, so `OniSaga` stores bounded
  `(page:serialized_params, cursor)` pairs rather than one shared cursor
  (`sources/multi.onisaga/src/lib.rs:105`). The `readable` check filter uses `"title": "Availability"`
  and `"name": "Readable Chapters"` with `"hideFromHeader": true` to match OniSaga's `/browse`
  panel (`sources/multi.onisaga/res/filters.json:135`).
- **Dynamic taxonomy (`Genres` and `Excluded Genres`)**: `POST /api/rpc/catalog/taxonomy` (`{}`)
  is fetched once on demand and cached in `RefCell<Option<Vec<TermOption>>>`
  (`sources/multi.onisaga/src/lib.rs:50`), populating `Genres` via `DynamicFilters`
  (`sources/multi.onisaga/src/lib.rs:408`) and `Excluded Genres` via `DynamicSettings`
  (`sources/multi.onisaga/src/lib.rs:427`), and routing tapped detail tags across origins,
  demographics, publication formats, genres, themes and freeform tags
  (`sources/multi.onisaga/src/lib.rs:68`).
- **Home**: `POST /api/rpc/catalog/home` (`sources/multi.onisaga/src/home.rs:176`) returns the six
  Discover sections (`Spotlight`, `Trending This Week`, `Start a New Series`, `Latest Updates`,
  `Finished Series`, `Newly Published`) in one JSON call (`sources/multi.onisaga/src/home.rs:19`).
  When `excludedGenres` is non-empty, `origins` is narrowed, or `en` is deselected, the three
  browse-backed shelves (`new`, `finished`, `published`) are fetched in parallel with
  `catalog/home` via `catalog/browse` so server-side genre, origin and language filters apply
  (`sources/multi.onisaga/src/api.rs:126`, `sources/multi.onisaga/src/home.rs:178`).
- **Details**: `POST /api/rpc/catalog/detail` (`{"slug"}`) returns full title metadata, credits,
  spoiler-flagged terms, demographic and `reading` stream/language/group tables
  (`sources/multi.onisaga/src/models.rs:148`).
- **Chapters**: `POST /api/rpc/chapters/index` pages units 200 at a time (`order: "desc"`) per
  selected language scope (`sources/multi.onisaga/src/lib.rs:184`). Units whose `selection` is
  `null` (non-default languages or multi-group rows) batch `POST /api/rpc/chapters/select` in
  200-ID chunks (`sources/multi.onisaga/src/lib.rs:198`). Each unit promotes OniSaga's
  `selectedReleaseId` first, and the opt-in `deduplicateChapters` (`Deduplicate Chapters`, default
  `false`) setting truncates each unit to that preferred upload without dropping distinct chapter
  parts or unnumbered units (`sources/multi.onisaga/src/models.rs:585`,
  `sources/multi.onisaga/res/settings.json:76`).
- **Reader pages**: `POST /api/rpc/reader/bootstrap` (`{"releaseId"}`) returns `kind` (`"images"`
  or `"text"`) and `manifest.pages` (`sources/multi.onisaga/src/lib.rs:151`,
  `sources/multi.onisaga/src/models.rs:640`).

> ⚠️ Aidoku's `HomeBigScrollerView` gates the entire bottom pill row — including the colored
> `Suggestive` / `18+` content-rating pill — behind non-empty `entry.tags`. `catalog/home` Spotlight
> cards omit `terms` (genres require per-card `catalog/detail` RPCs, which Home avoids), so
> `Work::into_details` populates `tags` from `[origin, demographic, form]` with a `"Comic"`
> fallback (`sources/multi.onisaga/src/models.rs:349`).

> ⚠️ `catalog/browse` mixes comics and text novels unless `medium: ["comic"]` is sent on every
> search and listing payload (`sources/multi.onisaga/src/api.rs:67`), and `Work`, `WorkDetail` and
> `Bootstrap` reject non-comic or `"text"` entries so prose chapters never fail silently as empty
> image lists.

> ⚠️ OniSaga's Zod schema on `catalog/browse` is `.strict()` and rejects duplicate array items,
> more than 10 `languages`, more than 20 `include` or `exclude` slugs per taxonomy group,
> `year.from > year.to`, `"relevance"` sort without `q`, and non-ASCII term slugs outside
> `^[a-z0-9]+(?:-[a-z0-9]+)*$` (`sources/multi.onisaga/src/api.rs:91`,
> `sources/multi.onisaga/src/models.rs:44`, `sources/multi.onisaga/src/helpers.rs:201`). `slugify`
> folds Latin diacritics (`Shōnen` → `shonen`) before splitting on ASCII non-alphanumerics, and
> `form_slug` / `origin_slug` map hyphenated pill labels (`One-shot` → `oneshot`, `Art book` →
> `artbook`, `Other Asian` → `other_asian`) back to OniSaga's enum IDs
> (`sources/multi.onisaga/src/helpers.rs:84`).

> ⚠️ `chapters/index` treats `editionId` as optional, whereas `chapters/select` requires
> `"editionId": null` when a scope has no edition (`sources/multi.onisaga/src/lib.rs:219`).

## Ratings, origins, languages and media URLs

Source `contentRating` is `1`, defaulting `contentRatings` to `["safe", "suggestive"]` (`Mature`
and `Explicit` opt-in). Selecting all seven origins omits `origin` (`None`) so titles with
`origin: null` still appear; clearing `contentRatings`, `origins` or `languages` returns empty
results before issuing any network request (`sources/multi.onisaga/src/settings.rs:10`,
`sources/multi.onisaga/src/api.rs:113`).

> ⚠️ `POST /api/age-confirmation` sets a 180-day `__Host-onisaga.age_confirmed` cookie on
> `onisaga.com`. `sync_age` sends `action=withdraw&next=%2Frobots.txt` whenever the active request
> drops back to Safe/Suggestive so adult home cards do not leak afterward, while `detail` and
> `bootstrap` retry once with `sync_age(true)` on HTTP 403/404 so saved library entries and deep
> links still open (`sources/multi.onisaga/src/lib.rs:42`, `sources/multi.onisaga/src/api.rs:169`).

> ⚠️ Safe and Suggestive page URLs carry `p=sg` and load directly from `media.onisaga.com`;
> age-gated page URLs carry `p=sc` (or lack `p=sg`) and must be rewritten to `onisaga.com` because
> the `__Host-` age cookie is host-only (`sources/multi.onisaga/src/helpers.rs:286`).

> ⚠️ Page URLs carry short-lived `e=<unix_expiry>` signatures. `get_page_list` appends
> `#oni=<releaseId>.<ord>`, and `ImageRequestProvider` strips that fragment, adds
> `Referer: https://onisaga.com/`, and re-bootstraps the chapter once when `now + 60 >= expiry`
> (`sources/multi.onisaga/src/lib.rs:472`, `sources/multi.onisaga/src/helpers.rs:297`).

> ⚠️ A title's `reading.languages` can list the same language under more than one numbering stream
> (for example a monochrome default stream and a partial colour stream). `language_scopes` picks
> the scope matching `defaultScope` or `streams[].isDefault` before falling back to the highest
> `readableUnits` count (`sources/multi.onisaga/src/models.rs:527`).
