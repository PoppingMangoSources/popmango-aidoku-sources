---
title: Flame Comics (en.flamecomics)
type: subsystem
created: 2026-11-01
updated: 2026-11-01
sources:
  - code: sources/en.flamecomics/src/lib.rs
  - code: sources/en.flamecomics/src/helpers.rs
  - code: sources/en.flamecomics/src/models.rs
  - code: sources/en.flamecomics/src/test.rs
  - code: sources/en.flamecomics/res/filters.json
  - ext: https://github.com/Aidoku-Community/sources/issues/855
verified_at: 75828b2
---

Flame Comics (`flamecomics.xyz`) is a Next.js site. Its `/api` has only narrow endpoints:
`/api/spotlight` lists every title's id, name and cover for the search box, with no tags, types
or years to filter on, and `/api/novel/chapter/list` repeats the chapters a novel's page
already carries. Every page embeds its data as `__NEXT_DATA__`, and the source reads that from
the page it needs
(`sources/en.flamecomics/src/helpers.rs:15`): one request per operation. The site's
`/_next/data/{buildId}/…json` route carries the same data, but the build id only comes from
another page, so it would cost two. Over the wire `/browse` is about 70 KB.

The old source scraped Mantine class names by position. The site's markup moved on, which
gave issue 855: untitled latest rows, novels that would not open, and a crash from a date
parser that called `expect`.

## Keys

Series and novels have separate id spaces that overlap (`/series/12` and `/novel/12` are
different titles), so keys are the site's paths, `/series/{id}` and `/novel/{id}`
(`sources/en.flamecomics/src/models.rs:58`), and chapter keys are `/{token}`. These are the
keys the old source used, so libraries and read history carry over without a
`breakingChangeVersion`.

## Home

The banner carousel comes first as an image scroller, then the sections in the site's
`blocksConfig` order: Novels, Popular, Staff Picks, Latest and Latest Novels
(`sources/en.flamecomics/src/lib.rs:168`). The site's blog blocks are left out.

## Novels

Novel chapters are html `content`, read as paragraphs. Every novel chapter carries a price;
which ones are locked follows the site's own code, copied in `locked_chapters`
(`sources/en.flamecomics/src/helpers.rs:52`).

> ⚠️ Read the embedded data with `data()`, falling back to `html()`, as the merged
> `ja.soraraw` and `mangak` template do. The test runner finds the `__NEXT_DATA__` script but
> returns `None` from `data()`, so `data()` alone fails every live test.

> ⚠️ The lock rule matched the site's rendered `a[data-locked=true]` on all 13 novels
> (2026-11-01), and `novel_locks_match_the_site` checks it on every test run. Reference
> clients disagree here: one calls the price fields placeholders and reads `data-locked`
> instead; another marks every priced chapter without a `public_release` as unknown. Both
> conflict with this evidence, so trust the test over them.

> ⚠️ A chapter's price doesn't mean it is locked. A priced chapter is public once its
> `public_release` passes, or under `ROLLING_LATEST` once it is out of the newest N, where
> N is the larger of `locked_chapter_count` (10 if unset) and `locked_chapter_percent` of all
> chapters, capped by `locked_chapter_max`. The `review_` tests pin this rule.

> ⚠️ Page urls end in `?{edit_time}`, as the site's reader builds them, so pages re-uploaded
> under the same names aren't served from a cache. The reader's "read on Flame" banners are
> `decoy` entries it inserts between pages itself; chapter data carried none in five series
> probed, so the source has no decoy filter. Add one if a chapter's `images` ever flags one.

> ⚠️ Browse calls the tags `categories`, while the home blocks and details call them `tags`;
> the model reads both through one alias. No entry carries both, which the alias needs.

> ⚠️ The site's browse has no genre filter; the source keeps one, matched locally, so tags on
> a details page stay tappable. Year options are a static list and need a new year added.
