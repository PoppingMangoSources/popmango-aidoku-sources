---
title: Flame Comics (en.flamecomics)
type: subsystem
created: 2026-11-01
updated: 2026-11-02
sources:
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/tree/171e770833b81843aa13c36514d13d6ca11ca25a/sources/en.flamecomics
  - ext: https://github.com/Aidoku-Community/sources/pull/860
  - ext: https://github.com/Aidoku-Community/sources/issues/855
verified_at: d1c881a
---

Flame Comics (`flamecomics.xyz`) is a Next.js site. Its `/api` has only narrow endpoints:
`/api/spotlight` lists every title's id, name and cover for the search box, with no tags, types
or years to filter on, and `/api/novel/chapter/list` repeats the chapters a novel's page
already carries. Every page embeds its data as `__NEXT_DATA__`, and the source reads that from
the page it needs
(`next_data` in `helpers.rs`): one request per operation. The site's
`/_next/data/{buildId}/…json` route carries the same data, but the build id only comes from
another page, so it would cost two. Over the wire `/browse` is about 70 KB.

The old source scraped Mantine class names by position. The site's markup moved on, which
gave issue 855: untitled latest rows, novels that would not open, and a crash from a date
parser that called `expect`.

> ⚠️ The crate is not on `main`. It was removed while upstream PR 860 is open; the submission is
> the `fix/en.flamecomics` branch, and the last copy on `main`, with its live tests in
> `src/test.rs`, is commit `171e770`. A review round builds from that commit:
> `scripts/pr-branch.sh en.flamecomics --branch fix/en.flamecomics --from 171e770 --fix …`, after
> restoring the crate on `main` if the change needs testing there. Function names below refer
> to that commit.

## Keys

Series and novels have separate id spaces that overlap (`/series/12` and `/novel/12` are
different titles), so keys are the site's paths, `/series/{id}` and `/novel/{id}`
(`key` in `helpers.rs`), and chapter keys are `/{token}`. These are the
keys the old source used, so libraries and read history carry over without a
`breakingChangeVersion`.

## Home

The banner carousel comes first as an image scroller, then Popular, Latest Updates, Novels, Staff
Picks and Latest Novels (`get_home` in `lib.rs`). That is the owner's order,
not the site's (`blocksConfig` puts Novels first and Latest after Staff Picks), and the latest
rows page five at a time. The site's blog blocks are left out.

> ⚠️ A review may cite the "order the home as the site does" rule; the order here is
> deliberate. "Latest Updates" is the site's own heading for that row.

## Novels

Novel chapters are html `content`. Descriptions and chapter text are converted to Markdown by
`markdown` in `helpers.rs`, which escapes the text and keeps
`<em>`/`<strong>` as `*`/`**`, since the app renders both as Markdown. Every novel chapter carries a price;
which ones are locked follows the site's own code, copied in `locked_chapters`
in `helpers.rs`.

Show Locked Chapters (`showLocked`, off by default, read in the settings module) leaves locked chapters
out of the list unless it is on, the same switch en.asurascans and the gigaviewer template have.

> ⚠️ Off by default because the source has no login: locked chapters can't be opened, so showing
> them confuses readers (kkantan on review). Keep the Rust fallback and the JSON default equal.

> ⚠️ Cards and details are built with `TryFrom<Entry> for Manga` and `Entry::into_details`, which
> fail on an entry with no id rather than return a manga with an empty key. Lists drop such an
> entry; none exists in the live data.

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

> ⚠️ Browse defaults to Title A to Z, like the site. Popularity runs the other way from Title:
> the site's descending order starts at rank 1, and ties (most entries are unranked) stay A to
> Z whichever way the arrow points, so the sort doesn't just reverse the list
> (the search sort in `lib.rs`). `popularity_sort_puts_the_top_ranked_first`
> checks it against the site's ranks.
