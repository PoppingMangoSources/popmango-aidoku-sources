---
title: Novel Archive (en.novelarchive)
type: subsystem
created: 2026-11-06
updated: 2026-11-11
sources:
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/56e8bb771f5f3a292cde96e1fc3ede22f1545058/sources/en.novelarchive/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/56e8bb771f5f3a292cde96e1fc3ede22f1545058/sources/en.novelarchive/src/helpers.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/56e8bb771f5f3a292cde96e1fc3ede22f1545058/sources/en.novelarchive/src/markdown.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/56e8bb771f5f3a292cde96e1fc3ede22f1545058/sources/en.novelarchive/src/models.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/56e8bb771f5f3a292cde96e1fc3ede22f1545058/sources/en.novelarchive/src/test.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/56e8bb771f5f3a292cde96e1fc3ede22f1545058/sources/en.novelarchive/src/settings.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/56e8bb771f5f3a292cde96e1fc3ede22f1545058/sources/en.novelarchive/res/filters.json
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/56e8bb771f5f3a292cde96e1fc3ede22f1545058/sources/en.novelarchive/res/icon.png
  - ext: https://novelarchive.st/assets/js/libary-page.js
  - ext: https://novelarchive.st/assets/js/index-home.js
verified_at: 46b15d9
---

Novel Archive is a web-novel archive with a plain JSON API under `/api/novels`, read the way the
site's own scripts read it. It moved from `novelarchive.cc` to `novelarchive.st`, and the old host
answers with a 301. Novel ids and the numbered chapter keys did not change, so libraries carry
over without a `breakingChangeVersion`.

> ⚠️ Merged upstream on 2026-11-08 (#873) and removed from `main`. The last copy with its tests is
> `56e8bb7` and the `merged-sources` branch; a later change starts from upstream's crate (section 17
> of the rules), and the code paths cited below are that copy's.

## Search and filters

Search is the library's browse call: `search` with `fuzzy=1`, `sort`, `status`, `genres_include`,
`genres_exclude`, `genre_match=any`, `page` and `per_page=24` (`sources/en.novelarchive/src/helpers.rs:68`).
Recently Updated is the default sort and is never sent. `has_next_page` comes from `pagination`.

Genres are a dynamic filter from `/api/novels/genres`, about 700 merged upstream values, without
the site's category values (`NON_GENRES`) (`sources/en.novelarchive/src/lib.rs:239`). A tapped tag
sends its label lowercased, as the site lowercases every genre.

Hidden Genres is a dynamic setting over the same genre list (`SearchParams::from_settings`,
`sources/en.novelarchive/src/helpers.rs`). It joins every search, listing and sorted Discover row, and
a genre a search includes itself wins over it. A Default Genres setting beside it was removed on the
owner's call. A pasted novel or
reader link is read through the deep-link handler and answered with that novel's card from one
details request; a link to no novel falls back to searching its text. Every API request sends
`Accept: application/json` (`request`), as section 4 of the rules asks.

> ⚠️ The Paperback source also decoded HTML entities, dropped a trailing "show more" from
> descriptions and repaired mirror text read as Latin-1. None of the 200 descriptions sampled on
> 2026-11-07 (the 100 most popular and the 100 newest) had an entity or "show more", and no merged
> source repairs mojibake, so neither is done until a payload shows it.

> ⚠️ Hide Adult Novels is a setting, on by default as the site's library switch is and as a
> source rated 17+ must be, and a check in the search sheet that overrides it for one search: ticked
> sends 1 and hides, excluded sends 2 and shows, unticked isn't sent and the setting decides. Hiding
> adds the site's twelve adult genres to `genres_exclude`, as its library does (`ADULT_GENRES`,
> `sources/en.novelarchive/src/helpers.rs:15`), so the server keeps pages full. The setting also
> covers the listings and the three sorted Discover rows; the fixed feeds take no exclusions.

> ⚠️ The fixed feeds (`trending`, `recently-updated`, `editors-choice`) ignore `genres_exclude`
> (the same list came back with and without it, probed 2026-11-06), and the site's home shows them
> unfiltered. Filtering them locally emptied Editor's Choice (both picks were tagged Mature or Adult)
> and cut Trending to two of five, which is why no row filters locally.

## Home

Six rows, in one parallel request, in the owner's order: Trending (a big scroller), Popular,
Recently Updated (each novel's newest chapter, five a page), Top Rated, Editor's Choice (a big
scroller, since the feed carries each pick's description and genres) and Recently Added
(`sources/en.novelarchive/src/lib.rs:119`). The fixed feeds are read with `limit=50`, the most
they return (asked for 120, trending answered 50). The three sorted rows are browse pages
(`popular`, `rating`, the default sort) and link listings of the same name
(`sources/en.novelarchive/src/lib.rs:220`). Recently Updated names each novel's newest chapter by
its count: lists carry no chapter names, only `total_chapters`, so that chapter has a number and the
novel's `updated_at`, never a title (`sources/en.novelarchive/src/models.rs:151`). The layout goes out
first as placeholders, as merged homes do; a feed that fails drops out of the returned home,
which fails only when all six do.

> ⚠️ The two big scrollers take the site's description unescaped, since the app draws it as plain
> text; `get_manga_update` escapes it for the details page, which renders Markdown
> (`sources/en.novelarchive/src/lib.rs:91`). Escaping both showed `life\,` and `\[` on Discover.

> ⚠️ The site's own home shows only Trending, Editor's Choice and Latest Updates. The six rows are
> the owner's choice, taken from another client of the site, so a review may cite the "order the
> home as the site does" rule; the extra rows are deliberate.

> ⚠️ The browse default sort, labelled "Recently updated" in the site's library, lists novels by
> when the archive added them: on 2026-11-07 its first page was all novels added that day, each
> `updated_at` within seconds of its `created_at`, while `/recently-updated` listed novels added
> in August to October and updated that day. Hence "Recently Added" for that row and listing,
> while the sort filter keeps the site's label. It was "New in the Archive" until review asked
> whether the title came from somewhere, since "New" reads as brand-new series and many
> additions are existing novels newly imported.

## Chapters

Chapter n is the n-th entry of `chapter_names`, keyed by position as the site's reader links
them (`/reader?novel={id}&chapter={n}`), and a novel whose names were never collected still lists
`total_chapters` chapters (`sources/en.novelarchive/src/models.rs:133`). Text comes from
`/chapters/{n}?prefetch=1`, which leaves out the name list; it is plain text, one line per paragraph.

Mirrors come from `/{id}/sources` (`{id, label}`), each with `/{id}/sources/{mirror}/chapters`
(`{number, title}`) and `/{id}/sources/{mirror}/chapters/{number}` (`content_html`, HTML the site's
reader inserts as is), as the owner's Paperback source reads them. They join the
chapter list keyed `{mirror}:{number}`, grouped under their label, with the site's own copies
grouped as "Novel Archive" (`sources/en.novelarchive/src/helpers.rs`, `mirror_chapters`).

> ⚠️ `/{id}/sources` names every mirror with a copy; asking one it leaves out answers 404 "Source has
> no copy of this novel" (probed with NovelFire and fucknovelpia on the Ghost Story novel). The site
> knows seven sources (`/source-list`: Royal Road, NovelFire, Ranobes, fucknovelpia, NovelBin, Tapas,
> noveli-to), and `preferred_source` on a novel is where the archive's own copy came from. The list is
> built lazily, so for a few seconds after a server restart it reports none (`reader-page.js`, which
> retries once after 6 seconds; the source doesn't).

> ⚠️ The library filters by that origin with `source=<id>` (13,266 novels for `royalroad`; an unknown
> id is ignored, not empty), so the Source filter takes its options from `/source-list` beside the
> genres. A mirror chapter's site link is `/reader?novel=…&chapter={number}&source={mirror}`, and the
> deep link reads it back to the `{mirror}:{number}` key.

> ⚠️ A mirror's `number` is only its own index and lists can be partial: Reverend Insanity's Ranobes
> list holds one entry, number 56, titled "Chapter 2315 Nothing Is Certain Except Success". The
> chapter number comes from a leading "Chapter N" in the title, else `number`; a title that is only
> a part number ("2") is dropped.

> ⚠️ `/sources` can hang about 30 seconds before a 504 (the Paperback source's finding), so it has
> an 8-second timeout, and any mirror failure leaves the site's own chapters loading. The timeout
> import is what `aidoku verify` dates to app 0.8.3, hence `minAppVersion`, as on the merged novel
> sources.

> ⚠️ Chapter text escapes only Markdown syntax (`src/markdown.rs`), not all punctuation as the
> merged converters' `escape_markdown` does: cmark makes
> each escape its own node, and a mirror's thousand-character lines would rebuild the crash below.

> ⚠️ Mirror `content_html` is real HTML: the fucknovelpia copy of the Ghost Story novel's chapter 3
> has `h1`, `hr`, `p`, `b`, `i`, `br`, `center`, `div` and `span`, with `&lt;Dark Exploration
> Records&gt;` as text, and chapter 1 opens with an `img`. `html_to_markdown` (`src/markdown.rs`) walks it into Markdown,
> as `en.novelbuddy` and `en.chikari` do, with tags read through `children()`: bold and italics stay open across a `<br>` and nested tags, a heading is a
> `#` heading of its level, `hr` a rule and an image `![](<url>)`, which the reader's MarkdownUI loads. The
> archive's own text is plain, which the site HTML-escapes and prints as is.

> ⚠️ Mirror images are served by the site (`/api/novels/{id}/sources/{mirror}/img/<hash>.jpg`), and
> the rendered page shows them absolute only because Firecrawl resolves links. The fragment is parsed
> with the site as its base and read with `abs:src`, as `zh.jmcomic` does; a non-http result, such as
> an inline `data:` image, is left out.

> ⚠️ `zh.wenku8` keeps illustrations out of its text because "Aidoku's paged text reader cannot load
> remote image URLs" (its `markdown.rs`); `zh.jmcomic`, `zh.zaimanhuanovel`, `en.novelbuddy` and
> `en.chikari` put `![](url)` in the text as here. So a mirror image may show only in the scroll
> reader (unverified in the app).

> ⚠️ Firecrawl strips the tags out of HTML inside a JSON response's strings, so the mirror endpoint
> read through it looks like plain text with paragraphs run together (read that way on 2026-11-07,
> wrongly). Read the markup from the rendered reader page (`formats: ["html"]` with
> `includeTags: ["article"]`), which is what the site inserted.

> ⚠️ The site's bold runs come split, as `<b>[Roulette Event]</b><b><br></b><b>: Create…</b>`
> and `<b>will </b><b><i>not</i></b><b> be</b>`. Written as is that is `****` and `** be`, which
> Markdown prints literally, so a marker is written before a line's first character, closed before a
> line break and trailing spaces, and a run that reopens straight after closing continues instead.

> ⚠️ Three things differ from the merged converters on purpose. They write `<br>` as a hard break
> inside one paragraph and escape every punctuation mark, which rebuilds the stack-overflow crash
> above, so `<br>` ends the paragraph here and only syntax is escaped. They render each emphasis on
> its own and trim it, which drops the space in `will </b><b>not` and prints the split runs above as
> `****`; here markers stay open across children. Lists, links, code and quotes, which they also
> convert, appear in no mirror copy read, so they fall through as text.

> ⚠️ Each line is escaped and becomes its own Markdown paragraph (joined with blank lines). Escaping
> the whole text made one paragraph of hard breaks, and chapter 1 of *Got Dropped into a Ghost Story,
> Still Gotta Work* (`6a162ec14f942c668d69a598`) crashed the app with it: the scroll reader's MarkdownUI
> joins each inline node of a paragraph into one `Text` with `+`, and the crash logs show the main
> thread's stack overflowing in `ConcatenatedTextStorage.resolve` (650 frames, 1,943 more elided).

> ⚠️ A chapter's number is the one its name prints ("Chapter 165.2" gives 165.2), as 19 merged
> sources number from names, with the position as the fallback ("Prologue") and always as the key. The
> Recently Updated row has no names, so its newest chapter keeps the count as its number.

> ⚠️ Chapter names stop matching positions in long novels: that novel's names run "Chapter 1" to
> "Chapter 374", then restart at "Chapter 165.2" for its remaining 261 entries. Keys stay positions, as the
> site's reader links them; the numbers follow the names.

> ⚠️ Native and mirror chapters share one rule (`number_and_title` in `helpers.rs`): a leading
> "Chapter n" becomes the number and leaves the rest as the title, a name without one keeps the
> position or mirror index as its number, and a title that is only digits ("2") is dropped.

> ⚠️ Errors answer `{"error": …}` with a status, such as a 404 "Novel not found"; the message is
> shown as is.

## Icon

> ⚠️ `res/icon.png` is the site's own 1024 px favicon (`/favicon.ico`, served as PNG) scaled whole to
> 128. Cloudflare refuses the sandbox the file, and Firecrawl returns only its text, so it came from
> DuckDuckGo's favicon cache (`icons.duckduckgo.com/ip3/novelarchive.st.ico`), which keeps it at full
> size. Scale it whole: cropping to the logo cuts off the dots in its corners.

## Testing

> ⚠️ Cloudflare refuses this sandbox with a 403 for curl and headless Chromium alike, so the live
> tests in `src/test.rs` fail here on the network. The `review_` tests use responses captured
> through Firecrawl and run anywhere; the live suite needs another network or the app.

> ⚠️ The latest row's date uses the ISO format the app parses (as `en.novelbuddy` does), but the
> test runner drops quoted literals and returns `None`, so the date is only checked in the app.

Not ported: the site's fallback to upstream copies of a chapter (`/sources`), which the reader uses
when the archive's own copy is missing. Such a chapter fails with a message instead.
