# Conventions

Sections 2 and 4 to 8 of the root `AGENTS.md` / `CLAUDE.md`, kept here whole and under their
numbers, so "section 6" anywhere in the repository still finds its text: the aidoku-rs API
in practice, reviewing behaviour and request cost, what transfers from Paperback habits, the
conventions of merged sources, framework defaults that bite, and comment and naming style.

## 2. The aidoku-rs API in practice

The crate's default features are `talc` (wasm allocator), `imports` (the host
functions) and `helpers`. `json` adds serde_json deserialisation of responses;
`test` disables the panic handler for tests.

A source implements `Source` and registers itself with `register_source!`, listing
every extra trait it implements:

```rust
register_source!(MySource, Home, ListingProvider, DeepLinkHandler, BaseUrlProvider);
```

### The four required `Source` functions

- `new` — setup only (rate limits, struct init).
- `get_search_manga_list` — one page of results for a query plus filter values.
  Filter values correspond to `filters.json`; **pages start at 1**. Return
  `has_next_page: true` and it is called again with the next page. Entries only
  need `key`, `title` and `cover` populated.
- `get_manga_update` — fill as much of `Manga` as the site gives when
  `needs_details`; set `chapters` when `needs_chapters`. It is called with either
  or both true, **never neither**, so a guard for that case is dead code.
- `get_page_list` — pages for one chapter.

Most sources should also implement `DeepLinkHandler`, so replacing `http` with
`aidoku` in a site url opens the app on that series or chapter. It only receives
urls matching the source's own base urls.

### Errors

Everything returns `Result<_, AidokuError>`, so `?` works throughout. `error!`
builds an error, `bail!` returns one immediately; both format like `println!` and
the message reaches the UI. Prefer them over silent `unwrap_or_default` on
something the reader needs.

### Requests

`Request::get(url)?.header(..).send()?` gives a `Response`. Terminal calls are
`data`, `string`, `html`, and `json_owned` with the `json` feature. **All of them
block.** Independent requests should go through `Request::send_all` — our home
layout issues its five section requests that way.

### HTML

SwiftSoup behind CSS selectors: `select`, `select_first`, `text`, `attr`.
`attr("abs:src")` resolves a relative url against the document, which saves doing
it by hand. Attribute selectors (`a[href^='/x']`) work and are used widely.

### Tag and author search (verified)

`"config": { "supportsTagSearch": true }` in `source.json` makes a tag on a series page
tappable. The app then calls search with `FilterValue::Select { id: "genre", value: <tag
name> }`; route that value wherever the site takes tags. Genres declared statically in
`filters.json` get this by default; **dynamic genres don't** (kkantan on #815), so a
source with `DynamicFilters` must opt in. `multi.lanraragi`, `ja.soraraw`, `en.madokami`,
`multi.ehentai` and `multi.hitomi` all handle the `"genre"` arm.

The app decides which tags are tappable in AidokuRunner's `Source.matchingGenreFilter`.
With the flag set, **every** tag is tappable and sends the `"genre"` select above, even when
the source also has an `isGenre` filter. Without it, only tags among the options of a
**static** `isGenre` select or multi-select are tappable. Those send that filter's own value
(its id, if it has `ids`), as a multi-select include or a select. Dynamic filters are never
checked. So a source with static genres and a separate tag list (Kagane, Atsumaru) can make
every tag tappable by setting the flag. It then resolves the name itself: `en.atsumaru` looks
it up in the site's filter list, because its search only matches tags by id.

`supportsAuthorSearch` and `supportsArtistSearch` are only for sources **without** an
`author`/`artist` text filter (aidoku-rs book); a source that has one gets author search
from it, so setting the flag as well is redundant.

### Other `source.json` config keys (verified, CLI schema)

- `hidesFiltersWhileSearching` — set it when the site ignores filters alongside a query, and
  then don't read the filters on that path (#637).
- `breakingChangeVersion` — goes in the top-level `config`, **not** `info` (#828). Users
  below it are asked to migrate. When a site is rebranded or rewritten, keep the old source
  id and set this, rather than adding a new id (#828).
- `allowsBaseUrlSelect` — mirrors in `urls`, the chosen one read as `defaults_get("url")`.
- `maximumParallelRequests` — caps parallel requests, such as page downloads.

## 4. Reviewing behaviour and request cost, not just shape

Reading the diff is not enough. For each change, walk through:

- **Count the network requests** for every operation — search page, details,
  chapters, page list, and each home section. Know the number and why.
- **Pagination**: does page 2 send a different page value? Does `has_next_page`
  come from the API's own contract (a pager, a total, or a cursor) rather than
  from the number of items that survived local filtering?
- **Filtering**: does each filter reach the right API field? Does an empty
  selection mean "no constraint" rather than "nothing"?
- **Content ratings**: is each rating independently respected, and is nothing
  shown that the reader excluded?
- **Locked chapters**: are they visible when they should be, hidden when not?
- **URL-paste search and deep links**: do they resolve to the right entry, and do
  they fall back to ordinary search when they can't?
- **Errors and fallbacks**: non-2xx, malformed JSON, an API error array, missing
  data, an empty page list — each should fail clearly rather than silently produce
  an empty object.

**Probe before concluding.** When the sandbox can't reach the site, ask for a probe
from the site's own DevTools console (`.claude/skills/aidoku-source/references/console-probes.md`)
instead of reasoning from a reference client. Send `Accept: application/json` and put a
timeout on every request: without the header, Toonkotsu's filtered latest-feed
requests hung and looked exactly like "this endpoint takes no filters" — with it they
answered in 200 ms, correctly filtered. Include a value that should match nothing; an
unchanged result means the parameter is ignored.

Two failure modes that cost the most:

- **N+1 fan-out.** Never issue a detail request per card when the listing response
  already carries what the card shows. It delays the whole carousel and multiplies
  rate-limit and Cloudflare failures.
- **Hidden request multipliers.** Filtering locally after the fact, then fetching
  more to compensate, is worse than asking the server correctly once.

---

## 5. Paperback/Inkdex habits, what transfers to Rust

Several of these came from Inkdex source guidelines. Most carry over; two don't.

| Guideline | Verdict for Aidoku |
|---|---|
| Parser/API logic as free exported arrow functions | **Reframe.** Rust has no arrow functions or classes in this sense. Use module-private `fn`, `pub(crate)` only when another module calls it. The source struct exists because the framework requires it. |
| Inline any function whose body just reshapes or delegates one call | **Applies.** Renames, `unwrap_or_default`, a single property access, one delegated call — inline them. |
| Fold single-use guards into what they guard | **Applies.** |
| Collapse wrapper-of-wrapper helpers; return new objects rather than mutating inputs | **Applies.** Prefer consuming an owned response and constructing `Manga`/`Chapter` over `mut` plus `.take()` just to move fields out. |
| Reuse the base response type with optional fields | **Applies.** One response model with `Option` fields serves both compact and detailed queries. Don't add a narrower struct that restates the same fields as required. |
| Self-sufficient IDs, no side-channel | **Applies, and is free.** Aidoku's `Manga` has no `additionalInfo` equivalent, so keys must carry what re-fetching needs. Don't put the mirror host in a key — base url must stay switchable. |
| Use listing payloads directly, never fan out per item | **Applies.** See the request-cost section. |
| Paginate from the API contract | **Applies.** Read the incoming page number, request the configured size, return next-page state from the API. Don't hardcode page 1, don't `.take(...)` a section into shape, don't invent a page size the site doesn't use. |
| No speculative or defensive guards | **Applies with judgement.** Every special case should trace to a real payload, a fixture, or a framework requirement. This is not licence to trust arbitrary network data — guards for nulls, blank covers and malformed urls are evidence-backed. |
| No low-value or context-bound caches | **Applies.** A source instance is short-lived; a cache inside it dies with it. |
| Session-level memo promises on the extension instance | **Does not transfer.** That's a JS-context pattern. Fetch the live filter page once per invocation instead. |
| No base classes for sharing small utilities | **Moot.** Free functions and modules already. |
| No legacy-migration code in new sources | **Mostly applies.** Self-healing loops for old id formats are dead weight. A one-line guard against a url shape the site itself still generates is not. |

---

## 6. Conventions seen across the community sources

### Site type picks the sources to compare against

Find out how the site is built before writing anything, then read merged sources built the
same way. View the page source and watch the Network tab:

- **Server-rendered HTML** (the data is in the page): scrape it with selectors —
  `en.weebcentral`. A WordPress theme (`wp-content` in the source) is a template (section 12).
- **Next.js** (`/_next/`, `__NEXT_DATA__`): look for the API the pages call and use that
  instead of the embedded data (section 3). Without one, read `script#__NEXT_DATA__` from the
  page you need, as `ja.soraraw` and the `mangak` template do: deserialize
  `props.pageProps`, and take the script's `data()` with `html()` as the fallback, since the
  test runner returns `None` from `data()` (verified on `en.flamecomics`). Don't use
  `/_next/data/<build id>/…`: the build id changes with every deploy and is only found on
  another page (`ja.soraraw`'s home says so).
- **A client-rendered app over a JSON API** (an empty root element, a hashed bundle such as
  `/assets/index-<hash>.js`, XHRs returning JSON): `en.toonkotsu`, `multi.kagane`,
  `en.chikari`, `en.comix`, `multi.mangadex`, `en.atsumaru` (section 16).
- **GraphQL** (one `POST` endpoint taking a `query`): `ru.senkuro`, `zh.komiic`,
  `multi.xcomic`.

### Module layout

`ru.senkuro` is `graphql.rs` / `helpers.rs` / `lib.rs` / `models.rs`.
`multi.mangadex` adds `home.rs` and `settings.rs`. `en.weebcentral` uses singular
`filter.rs` / `helper.rs` / `model.rs`. Any of these is fine; be consistent.

Rough sizes for calibration: `en.weebcentral` 574, `multi.kagane` 709,
`ru.senkuro` 950, `zh.komiic` 1276, `multi.mangadex` 1703 lines of Rust.

### Search payloads

`ru.senkuro` has a compact `SEARCH_QUERY` (identity + cover + status + rating) and
a separate fuller `MANGA_QUERY` for details. `multi.ehentai` goes further with two
types, `EHGalleryItem` for listings and `EHGallery` for details. Do the same: one
lean selection for browse, a full one for details.

### Series and chapter urls (verified)

Most sources build the url deterministically from the key and set it
unconditionally — `ja.rawdevart` and `ja.soraraw` have `manga_url(&key)` /
`chapter_url(manga_key, &key)` helpers, `multi.mangadex` has `val.url()`, and about
twenty others inline a `format!`. A minority pass through a url the API gives them
(`multi.ehentai`, `ja.nicovideoseiga`, `ru.desu`).

Nobody does "use the API's path, else construct one" — that's a keiyoushi idiom.
Prefer a deterministic url when the shape is derivable from the key. Only lean on
an API-provided path when it isn't, and then treat an empty string as absent.

### Settings vs. filters

Don't offer the same axis twice. Pick one:

- `en.comix` models it as **exclusions** (`hidden_types`), omits the parameter when
  nothing is hidden, and filters locally by exclusion. An exclusion filter can never
  hide an entry the site hasn't classified.
- keiyoushi-style sources make it a **filter only**, sent only when applied.

Whichever you pick, a selection that covers every option constrains nothing — send
nothing rather than the full list. Sending an explicit "all" list to an inclusive
API parameter silently drops every record with a null value in that field, which is
mostly newly added entries. `en.toonkotsu` (`type`) and `multi.xcomic` (`incTypes`) leave a
full selection out; `multi.kagane` and `multi.mangadex` send their full lists, which those
APIs treat the same. Probe what a missing parameter means first. Ratings are the usual
exception: every merged source whose API reads a missing rating as safe-only sends it on every
request, all ratings included — `multi.mangadex` (`contentRating[]`), `multi.xcomic`
(`incContentRatings`), `multi.kagane`, `multi.mangadotnet` (`adult=both`) and `zh.noyacg` (an
`allow-adult` header). Atsumaru's feeds read a missing `contentRatings` that way too.
`en.toonkotsu` is the other kind: a missing `sfw` means everything, so it sends `sfw=1` only
to hide adult titles.

The opposite end matters too: a setting the reader **cleared** must show nothing, not
everything. If the API has no "none" value, send one it can't match — Toonkotsu answers an
unknown `type` or `originalLanguage` with zero results (verified by probe), so a cleared
Types setting sends `type=none`. Don't add an empty-search special case when a sentinel does it.

**Moving an axis between a setting and a filter renames the setting key.** Old stored
values keep their old meaning; reusing the key sends them under the new one (stored
type ids sent as language codes match nothing).

**Let the site gate adult content where it can.** If an endpoint has its own safe mode
(Toonkotsu's `facets?sfw=1`), ask it rather than keeping a local list of adult tags —
`zh.noyacg` does the same with its `allow-adult` header. Most sources never hide tags
at all (`en.comix`, `multi.xcomic`); only ratings affect results.

### `DynamicSettings` (verified)

Only three sources use it, and the legitimate case is genuinely dynamic content —
`zh.zaimanhua` renders per-user login state. Building a **static** list at runtime
is what reviewers object to. If a list is static, put it in `settings.json`. If it
has to come from the site, `DynamicSettings` is fine, and it also avoids keeping the
same taxonomy in both Rust and JSON.

Fetch the live filter page once per invocation. No TTL caches — the source instance
is short-lived, so a cache dies with it.

### Home components (verified from aidoku-rs docs)

- `BigScroller` renders title, author, **cover, description, content rating and
  tags** — so it's legitimate to fetch a description for it. `multi.mangadex` does
  exactly this. Give it its own query rather than a boolean on the shared one;
  `ru.senkuro` has a named query per home section.
- `Scroller` and `MangaList` take `Link`s and show neither. They are cards: give them
  `Link::from(compact Manga)` and nothing else — no type, score, view count or author line.
- `MangaChapterList` renders a relative timestamp from the chapter's date. Only for a feed
  that names the chapter; never invent one from the series.
- `Link::from(Manga)` derives its subtitle from authors, falling back to
  description — with a compact payload it's always `None`, which is what a row should show.
  Don't set a subtitle yourself; `check-sources.py` fails a `Link` that does.
- `HomePartialResult` + `send_partial_result` let one failed section not kill the
  whole page. But the app matches a component partial to the layout **by title**, and keeps
  the layout's empty placeholder for a section that never arrives, which renders as a title
  over nothing once loading ends. When sections load together (one `send_all`), send the
  layout as a partial and **return** the sections that loaded: a returned home replaces the
  partial one (`en.atsumaru`). Details in the skill's `references/aidoku-host.md`.
- `MangaList` rows also show the **manga's tags** when it has any, so a row's manga carries
  none.
- The details page and text pages render **Markdown** (`en.hentai2read` says so for
  descriptions), so escape plain text with `escape_markdown` and turn `<em>`/`<strong>` into
  `*`/`**`, as `en.novelbuddy`, `en.chikari` and `en.freewebnovel` do. Sample the site's
  descriptions first: where they already carry Markdown, keep it unescaped (kkantan on #910,
  MKissa). A `BigScroller` shows its
  entries' descriptions verbatim (`Text(description)` in the app's `HomeBigScrollerView.swift`),
  so escape on the details path only: escaped text there shows every backslash
  (`en.novelarchive`, found by the owner in the app). Walking text nodes needs
  `Element::child_nodes` and `Kind`, which `aidoku verify` dates to app **0.8.3**, the
  `minAppVersion` those three declare (verified on `en.flamecomics`).

### Global mutable state

Sources are single-threaded. Use `RefCell` on the source struct rather than
`static mut`; `multi.cubari` is the reference.

---

## 7. Framework defaults that bite (all verified in aidoku-rs)

- `SortFilter.can_ascend` defaults to **`true`**. If the API has no ascending
  variants you must set `canAscend: false` explicitly — that is not redundant.
- `SortFilter.default` is `Option<SortFilterDefault>` defaulting to `None`, which
  already means index 0 and not ascending. Setting it explicitly *is* redundant.
- `MultiSelectFilter` has `uses_tag_style`, default `false` in Rust. For `filters.json` the app
  decodes `usesTagStyle` as `isGenre` and `canExclude` as `false` when absent (AidokuRunner
  `Models/Filter.swift`, `MultiSelectFilter.init(from:)`), so on a JSON genre filter both are
  redundant. An explicit `"canExclude": false` is valid, including when a review asks for unsupported exclusions to be disabled.
- Every filter has `hide_from_header` (`hideFromHeader` in JSON), unset by default, so a
  filter shows in the header unless told otherwise.
- `Manga` has no field for alternate titles, original language, year or score.
  Don't fetch them.
- `Chapter` has `volume_number`, `locked`, `thumbnail` and `language` — easy to
  leave unset by accident.
- `config.languageSelectType` in `source.json` accepts only `"single"`; the CLI
  schema also lists `"multi"` but the app rejects it and the source fails to
  install. Omit the field for multi-select, which is the default. AidokuRunner's
  `LanguageSelectType` has the cases `single` and `multiple` (`Models/SourceInfo.swift`), and
  an unset value is `.multiple` (`Source.swift`), so `"multi"` decodes to nothing. The
  aidoku-rs book's "defaulting to single" is wrong.
- `parse_date` formats with the **device's locale** unless given one, so English month or
  day names fail on a phone set to another language; pass `"en_US_POSIX"`.
- `QueryParameters::push(name, None)` writes the bare name (`&sort&status`): `None` means a
  key-only parameter (`helpers/uri.rs`), which an API can read as an empty value. Push only when
  there is a value (caught by a `review_` test on `en.novelarchive`).
- Dynamic settings get their defaults registered only when the reader opens the settings
  page, so `defaults_get` returns `None` for them until then; read them with a fallback.
- `defaults_get::<Vec<String>>(key)` returns `None` when unset and `Some(vec![])`
  when the reader cleared it. Keep those distinct — collapsing them means
  unchecking every box silently re-enables everything.
- `aidoku verify` computes the minimum app version from the imports the wasm uses and
  fails if `minAppVersion` is lower (`crates/cli/src/commands/verify.rs`): `webview_eval_async`
  and the cookie calls (`get_cookies`, `delete_cookie`, so `delete_all_cookies`) need **0.9.0**;
  `set_rule_list`, `add_user_script` and `context_eval_async` need **0.8.4**; `child_nodes`,
  `kind`, `get_url` and `set_timeout` need 0.8.3. A plain `WebView::new`, `load_html` and `eval`
  date to no later version (`en.comix` ships them at 0.8.4 for its other imports). Without any
  of these, kkantan has asked for `"0.7.1"` on other PRs.

---

## 8. Comment and naming style

Terse, and only where the code can't say it itself. Explain a non-obvious *why* —
an API quirk, a constraint, an ordering that matters.

- One or two sentences. Prefer a single-line `//` above or trailing the line, or a
  short phrase like `// Fetch genres once per filter sheet`.
- Don't restate what the code already demonstrates, and don't document framework
  behaviour a reader can look up.
- **No decorative dividers** — no `// ---- Section ----` banners, no empty
  docblocks, no editorial asides.
- **Clean naming.** Clear, grammatical, no typos and no abbreviations that only
  make sense to whoever wrote them.
- If a comment stops being true after a change, fix it in the same commit. A stale
  comment is worse than none — one survived here claiming a table fed both the
  filters and the settings long after the setting had moved.

### Constants (verified across the 140 merged sources)

- **URLs:** `BASE_URL` (116 sources), `API_URL` for an API base (20), `CDN_URL` for an image
  host (3). Each is a full literal, even on the same host (`en.stonescape`, `en.comix`); none is
  derived from another. Endpoints aren't constants or one-line helpers: they are formatted
  where they are requested.
- **Sizes:** `PAGE_SIZE` (9), then `<THING>_PAGE_SIZE` or `<THING>_SIZE` for others
  (`CHAPTER_PAGE_SIZE`, `RECENTLY_ADDED_SIZE`). Not `_LIMIT`.
- **Lists:** `SORTS` (5), `SECTIONS`, `GROUPS`, `CONTENT_TYPES`, `CONTENT_RATINGS`,
  `DEFAULT_RATINGS`; site ids named for what they hold (`PORNOGRAPHIC_GENRES`).
- **A value used once is written where it is used**, not given a name.
- `scripts/upstream-usage.py` prints the usual equivalent beside a rare name it recognises.

---

### Dead code

Delete it; never silence it. `-D warnings` already fails unused functions, constants, fields
and imports, even `pub` ones in private modules. What it doesn't fail, and what has shipped:

- **Lints that are off by default.** `extern crate alloc;` sat in `en.atsumaru` unused, since
  everything goes through `aidoku::alloc`, and `unused_extern_crates` is allow-by-default.
  `scripts/source-lints.sh` turns that and the other unused/needless lints on for every source.
- **Silenced warnings.** `#[allow(dead_code)]`, `#[expect(unused…)]` and a test that exists only
  to "use" a type all hide code that should go. `check-sources.py` fails the attributes.
- **Code the compiler can't see is pointless.** A sort that repeats the API's own order (the
  site already lists chapters newest first and pages in order), a field kept only for a
  display that was removed, a fallback for an error shape the site never sends (probe it), a
  one-line wrapper around one call, and a helper called once. Check each against a live
  response or a reference before keeping it.
- **After a removal, follow it through.** Taking out a row subtitle removed its label helper,
  three feed fields and the chapter row they fed. Re-run the lints and re-read the files the
  removal touched.
