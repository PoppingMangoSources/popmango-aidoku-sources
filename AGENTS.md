# Working on these sources

Notes gathered while getting `multi.xcomic` through review on Aidoku-Community
sources PR 735. Written down so the next source doesn't have to relearn it.

Items marked **(verified)** were checked against aidoku-rs, the community sources or
CI config. Items marked **(inferred)** are read from reviewer comments or a single
reference implementation and could be wrong.

---

## 1. Upstream rules

Summarised from `CONTRIBUTING.md` in the Aidoku-Community sources repo and the
official aidoku-rs source development book. Read those before a submission; this is
only a summary.

- **Conventional Commits.** PRs are squashed on merge, so the PR title *is* the
  commit message: `feat: add en.mangago`, `fix(vi.cmanga): update url`.
- **One concern per PR.** Two unrelated sources belong in two PRs.
- Compiles with no warnings; `cargo fmt` run; `cargo clippy` clean; every file ends
  with a newline; **JSON indented with tabs**.
- `rustfmt.toml` upstream sets `hard_tabs`, `use_try_shorthand`,
  `use_field_init_shorthand` — match it.

### AI usage — read this before pointing a model at a source

Their CONTRIBUTING explicitly allows AI for assistance or reference, but says it is
**not acceptable when no human thought went into the result** — AI tends to produce
needlessly complex or subtly wrong code, which makes review harder. They'd rather
give review feedback that helps you learn, and point people at `#source-dev` on
their Discord.

So: anything generated needs to be read, understood and cut down before it goes
near a PR. Treat this file as a way to get a draft closer to acceptable, not as a
licence to skip reading the diff.

### Minimum functionality they expect

- `status`, `content_rating` and `viewer` set on details when the site has the data.
  `content_rating` can be derived from tags; `viewer` from manhwa/manhua/webtoon.
- `send_partial_result` in the details path when chapters need another request.
- `DeepLinkHandler` for series and chapter urls where possible.
- Built-in language filtering for multi-language sources — do **not** set the
  language to "multi".
- Every filter the website offers should have a filter in the source.
- Home pages and listings are optional but welcome.

### CI (verified)

`pr.yaml` runs `aidoku package` then `aidoku verify` per changed source.
`clippy.yaml` runs `cargo clippy` and turns **any** annotation into a failure.

Locally, from the source directory:

```sh
cargo fmt --all
cargo clippy --release --target wasm32-unknown-unknown
aidoku package && aidoku verify package.aix
```

**Do not add `--all-targets`** (verified) — it pulls in targets with no
`#[panic_handler]` and fails for reasons that have nothing to do with your code.

### Tests (verified)

There **is** a harness, and we aren't using it. `aidoku-test` plus aidoku's `test`
feature (which disables the panic handler) are dev-dependencies on ~129 sources,
ours included — the scaffold adds them. Tests go in `src/test.rs` and are marked
`#[aidoku_test]`; `en.mangakakalot`, `ja.senmanga`, `ja.soraraw` and
`ja.spoilerplus` have real ones. They hit live endpoints and assert on actual
results — pinned keys, whether a url resolves, whether a listing comes back sane.

CI does not run them (only package, verify and clippy), so they're a local tool
rather than a gate. Worth adding for anything whose behaviour is hard to eyeball,
which is most of what went wrong in #735.

---

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

---

## 3. Review feedback from #735

Each of these was a real comment. They generalise.

| What was flagged | The rule |
|---|---|
| Search returned tags + descriptions | Search/listing results carry **key, title, cover** and optionally status and content rating. Everything else is filled in by `get_manga_update`. |
| Explicit `"default"` on a sort filter | Don't set what the framework already defaults to. |
| Icon filled the whole canvas | Leave margin; aim for something closer to the site's favicon. 128×128, fully opaque. |
| A translated-languages filter | If the app's own languages setting covers it, the filter is redundant. Don't ship both a filter and a setting for the same axis. |
| Genre filter presentation | Genre multi-selects should set `uses_tag_style`. |
| 14 listings | Listings that are just search sorts don't need to exist. Keep the ones home sections link to. |
| A helper sat in `lib.rs` | Key/url helpers belong in `helpers.rs`. |
| `String::new()` / `Vec::new()` everywhere | `#[derive(Default)]` on the struct, then `..Default::default()`. |
| Temporary vectors before assigning to params | Assign straight into the params struct. |
| Settings lookups outside `settings.rs` | If the source has a `settings.rs`, the `defaults_get` calls live there. |
| `page_size: Some(10)` on home components | **5 max**, for all components — 10 means a lot of scrolling on a phone. |
| Web view urls 404'd | Series and chapter urls need to resolve. Have a fallback if the API's path field can be absent. |

Also from the wider repo: **excessive `clone` is the single most common review
comment.** Take `&str` parameters, iterate by reference, and remember `format!`
doesn't need an owned input.

---

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

Two failure modes that cost the most:

- **N+1 fan-out.** Never issue a detail request per card when the listing response
  already carries what the card shows. It delays the whole carousel and multiplies
  rate-limit and Cloudflare failures.
- **Hidden request multipliers.** Filtering locally after the fact, then fetching
  more to compensate, is worse than asking the server correctly once.

---

## 5. Paperback/Inkdex habits — what transfers to Rust

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
mostly newly added entries.

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
- `Scroller` and `MangaList` take `Link`s and show neither.
- `MangaChapterList` renders a relative timestamp from the chapter's date.
- `Link::from(Manga)` derives its subtitle from authors, falling back to
  description — with a compact payload it's always `None`.
- `HomePartialResult` + `send_partial_result` let one failed section not kill the
  whole page. Worth doing when there are several independent sections.

### Global mutable state

Sources are single-threaded. Use `RefCell` on the source struct rather than
`static mut`; `multi.cubari` is the reference.

---

## 7. Framework defaults that bite (all verified in aidoku-rs)

- `SortFilter.can_ascend` defaults to **`true`**. If the API has no ascending
  variants you must set `canAscend: false` explicitly — that is not redundant.
- `SortFilter.default` is `Option<SortFilterDefault>` defaulting to `None`, which
  already means index 0 and not ascending. Setting it explicitly *is* redundant.
- `MultiSelectFilter` has `uses_tag_style`, default `false`.
- `Manga` has no field for alternate titles, original language, year or score.
  Don't fetch them.
- `Chapter` has `volume_number`, `locked`, `thumbnail` and `language` — easy to
  leave unset by accident.
- `config.languageSelectType` in `source.json` accepts only `"single"`; the CLI
  schema also lists `"multi"` but the app rejects it and the source fails to
  install. Omit the field for multi-select, which is the default.
- `defaults_get::<Vec<String>>(key)` returns `None` when unset and `Some(vec![])`
  when the reader cleared it. Keep those distinct — collapsing them means
  unchecking every box silently re-enables everything.

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

---

## 9. This repo's workflow

- Work on `main`. Version-bump `res/source.json` and update the two `.aix` links in
  `README.md` for the source you touched — edit **only that source's line**, a
  loose `sed` will catch another source whose label happens to match.
- Commits are authored **and** committed by
  `Popmango Extensions <297959108+PoppingMangoSources@users.noreply.github.com>`,
  and **unsigned**:

  ```sh
  git -c commit.gpgsign=false \
      -c user.name="Popmango Extensions" \
      -c user.email="297959108+PoppingMangoSources@users.noreply.github.com" \
      commit --no-gpg-sign --reset-author -m "fix(multi.xcomic): ..."
  ```

- No AI attribution in commit messages, PR bodies or code comments.
- For an upstream PR branch, the source's `version` stays **1** — it's a new source
  there. Our `main` keeps its own incrementing version, since our users have it
  installed and a version going backwards means they never get an update.
- Keep `src/` byte-identical between `main` and the PR branch; check with
  `git diff <main-sha> -- sources/<id>/src`.
- Force-push the PR branch with `--force-with-lease=<ref>:<expected-sha>`, never a
  bare `--force`.

## 10. xcomic specifics

Kept here because the API is undocumented and this took a while to work out.

- GraphQL, `POST {base}/query/`. Bato-family schema.
- **Two id namespaces.** `/title/{id}` is the series; `/comic/{id}-{lang}-{slug}` is
  one language edition. Only edition ids work as manga keys. A `/title/` page lists
  its editions in a "Sources" section, one `/comic/` link each.
- The site's own url rewriter maps `/series/{id}` → `/comic/{id}` and
  `/chapter/{id}` → `/comic/_/{id}`, so `_` is its placeholder for an unnamed comic.
- Feeds vs. browse: `get_comic_recentlyAdded` and `get_comic_latestUploads` are
  their own endpoints and take **no filters** — filter those locally. Browse
  (`get_comic_browse_items`) applies everything server-side, so don't re-filter it.
- `ignoreGlobalGenres` / `ignoreGlobalULangs` / `ignoreGlobalBlocks` should stay
  **off**. Turning off the blocklist admits uploads the site hasn't approved.
- Chapter lists come in two forms: `get_comic_chapterList_fullList` (every
  scanlator's upload, paged ~100) and `get_comic_chapterList_uniqList`
  (deduplicated, paged ~1000). Alias both to the same field name so one response
  model reads either.
- Every chapter's `urlPath` embeds its language, so a chapters-only refresh doesn't
  need to fetch the comic first.
- The `/search` page exposes genres as `details.group` blocks with an id per option
  in a `:` attribute — scrapeable. **Languages are not**: that group is one wrapping
  div with the names as plain text, no per-option ids. (verified)
- `xcomic.net` and `comik.to` resolve alongside `xcomic.me`; `xcomic.cr` and
  `xcomic.cs` appear in the site's own domain list but don't resolve. `mpark.top` is
  a sibling site, not a mirror — different ids. **`comik.to` has not been confirmed
  to serve the same ids** (inferred from DNS and the PWA manifest naming the app
  "ComiK.TO"); if urls 404, check which base url is selected.

### Reference implementations

Two other clients for the same site, useful for diffing behaviour:

- keiyoushi `src/all/xcomic` — Kotlin, closest to the API surface.
- The Paperback source in `PoppingMangoSources/general-extensions-mangago`
  (`0.9/test`, `src/XCOMIC`).

When a section returns the wrong entries, diff the **request payload** against
theirs field by field before changing anything. Both times something was wrong it
was a parameter we sent and they didn't, not the endpoint or the sort key.
---

## 11. Templates

Sixteen of them live in `templates/` upstream, for sites that share a common
engine or theme:

`madara`, `wpcomics`, `gigaviewer`, `mangathemesia`, `iken`, `mangareader`,
`libgroup`, `mangabox`, `liliana`, `guya`, `mmrcms`, `tukutema`, `ezmanhwa`,
`pizzareader`, `madtheme`, `mangaworld`.

**Check for one before writing a source from scratch.** Two ways: see whether the
matching keiyoushi extension declares a `themePkg`, or run the site through a
WordPress theme detector.

A template exposes a `Params` struct for per-site configuration and an `Impl` trait
carrying the default behaviour. A consuming source implements `Impl` with `new` and
`params`, overriding only what differs, then registers the template wrapping itself:

```rust
impl Impl for ElfToon {
	fn new() -> Self { Self }
	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			chapter_list_selector: "#chapterlist li:not(:has(.gem-price-icon))".into(),
			..Default::default()
		}
	}
}

register_source!(MangaThemesia<ElfToon>, Home, ImageRequestProvider, DeepLinkHandler);
```

That, plus a path dependency on the template in `Cargo.toml`, is the whole of
`en.elftoon` — 28 lines. If a new source starts looking like an existing template
with different selectors, it should probably be a template consumer, or a new
template if nothing fits.

---

## 12. Where to look

No links here on purpose — search the Aidoku-Community sources repo by these names.

**Reviews worth rereading before submitting.** PRs 726, 531, 624, 646, 690 (two
rounds) and 603. Recurring themes across them: consume owned values instead of
cloning, inline single-use helpers, use the API's own ordering, avoid unnecessary
dynamic settings, keep search results compact, send partial chapter results,
construct queries correctly, and handle only deep links you can actually resolve.
PRs 191, 237, 277 and 309 are the older BatoTo submissions — useful history, but
current sources and current reviews win where they disagree.

**Sources worth reading for structure.** `multi.mangadex` (auth, custom lists, home
with partial results), `ru.senkuro` (GraphQL in its own module, compact search
query, one named query per home section), `zh.komiic` (GraphQL), `multi.kagane`,
`multi.mangadotnet`, `multi.mangafire` (JS deobfuscation), `multi.mangaplus`
(`PageImageProcessor`), `en.comix` (exclusion-style settings, webview),
`en.weebcentral`, `en.chikari` (novels alongside manga), `multi.ehentai` (separate
listing and detail types), `multi.cubari` (`RefCell` state), `zh.zaimanhua`
(legitimate `DynamicSettings`). `en.elftoon` is 28 lines — a template consumer, and
the fastest way to see how thin a templated source can be.

**The reviewer.** kkantan reviews most of these. Reading their own commits and past
review comments is the fastest way to predict what they'll ask for — the patterns
in section 3 came from doing exactly that.

