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
  - code: sources/multi.onisaga/src/rsc.rs
  - code: sources/multi.onisaga/src/settings.rs
  - code: sources/multi.onisaga/res/filters.json
  - code: sources/multi.onisaga/res/settings.json
  - code: sources/multi.onisaga/res/source.json
  - code: sources/multi.onisaga/src/test.rs
  - ext: https://onisaga.com/browse
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/src/all/onisaga
verified_at: 45c50ed
---

OniSaga (`onisaga.com`) is a Next.js site behind Cloudflare with images on `media.onisaga.com`.
Everything here was read from the owner's capture of the site (2026-11-21) and from Firecrawl
scrapes of live pages on 2026-11-27; the sandbox itself gets Cloudflare's 403, so the live tests
in `src/test.rs` could not run here.

## Requests

- **Search and listings**: `POST /api/rpc/catalog/browse` with `{"json": input}`; the answer is
  `{"json": {"items": [...], "nextCursor": ...}}` (`sources/multi.onisaga/src/api.rs`). The input
  fields were read from the server-built input the browse page embeds for a URL with every filter
  set: `q`, `origin`, `form`, `status`, `demographic`, `ratings`, `languages`,
  `genres`/`themes`/`tags` as `{include, exclude}`, `year` as `{from, to}`, `readable`, `sort`,
  `limit`. `cursor` comes from the keiyoushi client (unverified here).
- **Home**: one `GET /` plus two browse calls (`sort` `new` and `published`, 14 titles each), all
  in one `send_all` (`sources/multi.onisaga/src/home.rs`). The spotlight (`titles`/`thumbs`
  props) and latest updates (`entries`/`scope` props) come from the page's Flight data; trending
  and finished series are read from `section[aria-labelledby=…]` lists.
- **Details and chapters**: `GET /title/<slug>?chapters=all&order=desc&lang=<code>`, one per
  selected language, following the `nav[aria-label="Chapter pages"]` `cc=` link 100 rows at a
  time (`sources/multi.onisaga/src/lib.rs`).
- **Pages**: `GET /read/<slug>/<release>`, every `figure[data-page] img` (all pages are in the
  HTML).

The site's JavaScript also calls `catalog.home`, `catalog.detail` and `reader.bootstrap` RPCs
(seen in another client's build). Their inputs could not be probed from here, so details and
pages stay on the HTML, which was verified live.

> ⚠️ The Start a new series and Newly published shelves equal `browse?sort=new` and
> `browse?sort=published` title for title (checked 2026-11-27), so they come from the browse API
> and follow every setting. The Finished series shelf does **not** equal
> `browse?status=completed`; it is curated, so it stays the home page's own list.

> ⚠️ The home page ignores the reader's settings. Spotlight and latest-update cards carry
> `contentRating`, `origin` and `languages`, so they are filtered locally; finished-series cards
> carry only an origin (`data-medium`) and trending rows carry nothing. Excluded genres cannot be
> applied to any page-sourced row, since none carries genres.

> ⚠️ Flight sends a card that already appeared as a spotlight slide as a back-reference such as
> `"$4:props:children:1:props:thumbs:0"`, and long strings as `T` rows that can run into the next
> row on the same line. `rsc.rs` reads rows by byte length, as the merged vinetheme template does,
> and resolves both kinds of reference before decoding; without it two latest-update cards drop.

## Ratings, origins and languages

The guest audience is Safe and Suggestive. Mature and Explicit titles need the site's age
confirmation and their pages are a `main[data-ui=content-gate]`; the source offers only Safe and
Suggestive, Safe by default, and fails a gated title or chapter with a message. A title page
shows a "Suggestive" fact chip and nothing for Safe, which is how details set the rating
(`sources/multi.onisaga/src/helpers.rs`).

> ⚠️ Every origin selected sends no `origin`: titles with a null origin (Fire Ring) would
> otherwise vanish. A cleared Origin, Content Rating or language selection browses nothing, before
> any request (`sources/multi.onisaga/src/settings.rs`, `SearchParams::is_empty`).

> ⚠️ Rows unavailable in the page's language print "Not available here" and carry no
> other-language panel, so one title page cannot list several languages; each language is its own
> request. Only the main row of a language page is read, never its `langPanel` variants.

## Chapter rows

A row is `a[data-unit]` (single-language title), `div[data-unit]` with a stretched read link, or
`details[data-unit]` whose `ul > li` items are versions by different groups. Labels read
`Ch 700.5`; titles repeat the label in an `sr-only` span when the chapter has none.

> ⚠️ The test runner's `own_text` returns child text, unlike the app, so labels, titles and credit
> roles are read with `text()` and have the `sr-only` text or link names stripped.

Dates are ISO with six fractional digits and `Z`; the runner's `parse_date` drops fractions, so
dates are `None` there and must be checked in the app (the runner's limits are in
`.claude/skills/aidoku-source/references/testing.md`).
