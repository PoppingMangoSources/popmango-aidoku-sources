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
verified_at: 43ad4ba
---

Flame Comics (`flamecomics.xyz`) is a Next.js site. Its `/api` has only narrow endpoints:
`/api/spotlight` lists every title's id, name and cover for the search box, with no tags, types
or years to filter on, and `/api/novel/chapter/list` repeats the chapters a novel's page
already carries. Every page embeds its data as `__NEXT_DATA__`, and the source reads that from
the page it needs
(`sources/en.flamecomics/src/helpers.rs:19`): one request per operation. The site's
`/_next/data/{buildId}/…json` route carries the same data, but the build id only comes from
another page, so it would cost two. Over the wire `/browse` is about 70 KB.

The old source scraped Mantine class names by position. The site's markup moved on, which
gave issue 855: untitled latest rows, novels that would not open, and a crash from a date
parser that called `expect`.

## Keys

Series and novels have separate id spaces that overlap (`/series/12` and `/novel/12` are
different titles), so keys are the site's paths, `/series/{id}` and `/novel/{id}`
(`sources/en.flamecomics/src/models.rs:59`), and chapter keys are `/{token}`. These are the
keys the old source used, so libraries and read history carry over without a
`breakingChangeVersion`.

## Home

The banner carousel comes first as an image scroller, then Popular, Latest Updates, Novels, Staff
Picks and Latest Novels (`sources/en.flamecomics/src/lib.rs:144`). That is the owner's order,
not the site's (`blocksConfig` puts Novels first and Latest after Staff Picks), and the latest
rows page five at a time. The site's blog blocks are left out.

> ⚠️ A review may cite the "order the home as the site does" rule; the order here is
> deliberate. "Latest Updates" is the site's own heading for that row.

## Novels

Novel chapters are html `content`. Descriptions and chapter text are converted to Markdown by
`markdown` (`sources/en.flamecomics/src/helpers.rs:71`), which escapes the text and keeps
`<em>`/`<strong>` as `*`/`**`, since the app renders both as Markdown. Every novel chapter carries a price;
which ones are locked follows the site's own code, copied in `locked_chapters`
(`sources/en.flamecomics/src/helpers.rs:134`).

Show Locked Chapters (`showLocked`, on by default, read in `settings.rs`) leaves locked chapters
out of the list when it is off, the same switch en.asurascans and the gigaviewer template have.

> ⚠️ `markdown` walks text nodes with `child_nodes`, which needs app 0.8.3, so `minAppVersion`
> is 0.8.3, as on the merged novel sources. Bare-text descriptions (3 of 26 sampled) are wrapped
> in a `<p>` first; the test runner's fragments have no `body` to fall back to.

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
