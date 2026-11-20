---
title: Toonkotsu (en.toonkotsu)
type: subsystem
created: 2026-11-20
updated: 2026-11-20
sources:
  - ext: https://github.com/Aidoku-Community/sources/tree/main/sources/en.toonkotsu
verified_at: 3ac574e
---

`en.toonkotsu` lives upstream only; it was removed from `main` once merged there, and a later
change is a new branch from `upstream/main`, as for Kagane. The last copy and its tests are
listed in section 17.
Kept here because the API is undocumented. Everything below was confirmed by console
probes or captures unless marked.

- JSON throughout: the catalogue under `/cdn/v1/`, reader and rankings under `/api/`.
  Send `Accept: application/json` on every request — see section 4 for what happens
  without it.
- **Keys** are the 5-character `hid` (a lowercase letter, then four lowercase letters or
  digits). Series url `/title/{hid}-{slug}`; chapter url
  `/title/{hid}-{slug}/{id}-chapter-{n}`, and `/chapter/{id}` resolves too. Both deep-link.
- **Browse** `/cdn/v1/browse`: `q`, `sort`, `order`, `status`, `demographic`, `genres`,
  `exclude`, `genreMode`, `excludeMode`, `type`, `originalLanguage`, `author`,
  `yearFrom`/`yearTo`, `chaptersMin`/`chaptersMax`, `discovery`, `sfw=1`. Every list
  endpoint returns `hasNextPage`.
  - Sort ids: `relevance`, `latest` (updated and uploaded date), `new` (created),
    `score`, `popular` (views and follows), `chapters`, `year`, `title`. The site lists
    sorts that send the same id; they were kept because the person asked for every site
    sort, and a reviewer may question them.
  - `status` is capitalised: `Ongoing`, `Complete`, `Hiatus`, `Canceled`. Lowercase
    shipped broken once.
  - `type` (case-insensitive): `manga`, `manhwa`, `manhua`, `oel`, `other`.
  - `originalLanguage` takes a comma list. Live codes are `ja`, `ko`, `zh`, `en`; the
    `-ro` codes are placeholders the site keeps for later, and `zh-hk`/`zh-ro` answer as
    `zh`. About 3,400 titles have none of the four.
- Content ratings: `sfw=1` is the only rating control the API has — it ignores
  `contentRating`, and the site's own search sends just `sfw=1` whatever ratings are
  picked (probed). Safe, Suggestive and Erotica are told apart from the tags, so they are
  filtered after the page arrives.
- "Has chapters" sends `hasChapters=true`, as the site's search does. Every title had a
  chapter when probed, so the flag changed no totals then, but the catalogue is new.
- **Latest** `/cdn/v1/latest` lists every new chapter, so dedupe per title. It honours
  `type`, `exclude` and `sfw` (probed); the search filters don't apply to it.
- **Facets** `/cdn/v1/facets?sfw=0|1`: `genres` (456 with `sfw=0`, ~390 with `sfw=1`;
  the 67 explicit ones carry `isAdult`), `types`, `statuses`, `years`. The site's picker
  shows categorised tags under Theme, including `Kids`, which is categorised as a
  demographic.
- **Rankings** `/api/v1/rankings`: `GET ?metric=views` for Most Read; `POST
  {"params": {"discovery": …, "metric": …}}` for trending and hidden gems.
- **Details** `/cdn/v1/series/{hid}` carries the chapter list in the same response.
  Entries have `id`, `n`, `label`, `vol`, `at` (milliseconds), `w`, `mb`, `group`, and
  `alts` (other uploads, without `n`/`vol`). Official releases restart numbering each
  volume, whole-volume uploads have no `n`, and one group can upload a chapter twice —
  keep its widest, then largest, upload.
- **Reader.** `/api/auth/me` hands out the guest `sid` cookie. The unsigned
  `/api/manga/chapter/{id}` usually returns the page keys unmasked; the signed
  fallback fetches `/api/manga/key` and sends `x-sig`. All of them go through the web
  view (see below). Pages are AES-CTR (64-bit
  big-endian counter) with a per-page key and nonce.
- **Read limits** come back as 429 with `code: "manga-limit"`: `resign: true` wants a
  fresh key, `transient: true` a short wait (`retryAfterSec`), `reason: "wait"` a long
  one, and `reason: "verify"` a human check. The site says the limit is per IP or device, but
  in practice the unlock follows the web view's session. `verify` wants a Turnstile check, which the site's reader posts to
  `/api/manga/unlock` and which often passes unattended in a hidden WebView. Detect the
  unlock from the page's resource timing — **never modify the check page**: injecting a
  content blocker and a fetch hook made nothing load at all.
- **Keep every reader request inside the web view.** v38 tried the unsigned read over
  a plain request first and fell back to the web view when refused; in the app, the human
  check then never passed and reading looped on "Confirm you're human". Plain requests
  carry a different session from the web view the check unlocks, so the two must not be
  mixed. Reverted in v39.
- The WebView calls push `minAppVersion` to 0.9.0 (section 7).

## Reference implementations

- keiyoushi `src/en/toonkotsu` — Kotlin. Its Latest is browse with `sort=latest`; its
  Type filter is built from the facets.
- `PoppingMangoSources/popmango-extensions-mana`, `src/Toonkotsu` — our own Mana source;
  where the unsigned reader path came from.
