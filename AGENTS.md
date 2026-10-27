# Working on these sources

Notes gathered while getting `multi.xcomic` (PR 735), `en.toonkotsu` (PRs 815 and 821) and
the Kagane rework (PR 827) merged into Aidoku-Community sources, plus what kkantan asks of
other contributors. Written down so the next source doesn't have to relearn it.

`AGENTS.md` and `CLAUDE.md` are identical, complete copies of these rules. Both agents
must read the source workflow before creating or submitting a source:

- [.claude/skills/aidoku-source/SKILL.md](.claude/skills/aidoku-source/SKILL.md)
- [.agents/skills/aidoku-source/SKILL.md](.agents/skills/aidoku-source/SKILL.md)

The two skill trees are physical copies, including their references. After editing either
instruction set, synchronize the copies with `python3 scripts/sync-agent-docs.py` (edit
`AGENTS.md` and `.claude/skills` first). CI and the pre-push hook fail on drift. This is
intentional duplication so each client can read the full instructions directly.

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
- **One concern per PR.** Two unrelated sources belong in two PRs. A new template is the
  exception that goes the other way: include the sources that use it, or stack their PRs
  on it, so the template is shown working (#824).
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

Upstream's `pr.yaml` has three jobs, each over what a PR changes (the compatibility
baseline, not a replacement for this fork's additional review gates):

- **Verify** — each changed source: `aidoku package` with `RUSTFLAGS=-Dwarnings`, then
  `aidoku verify`.
- **Lint** — each changed source and template: `cargo fmt --check` and
  `cargo clippy -- -D warnings`; any warning fails.
- **JSON** — tabs only, final newline, for every file under `res/`.

Rust is pinned (`1.99.0`) and the CLI comes from `.github/actions/install-aidoku-cli`, which
installs aidoku-rs at its latest commit. This fork keeps packaging/JSON checks in `pr.yaml`, plus a separate `clippy.yaml`
with formatting, warning failures, inline diagnostics and an aggregate tool-version summary.
`check.yaml` adds our source rules, mirror checks, workflow linting and guard regression tests.
Template changes also check their source consumers. Passing these is necessary, but does not
prove the site's behavior or upstream branch isolation.

Locally, from the source directory (the pre-push hook runs exactly these):

```sh
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown -- -D warnings
RUSTFLAGS=-Dwarnings aidoku package && aidoku verify package.aix
```

**Do not add `--all-targets`** (verified) — it pulls in targets with no
`#[panic_handler]` and fails for reasons that have nothing to do with your code.

### Tests (verified)

`aidoku-test` plus aidoku's `test` feature (which disables the panic handler) are
dev-dependencies on ~129 sources — the scaffold adds them. Tests go in `src/test.rs`,
declared in `lib.rs` as `#[cfg(test)] mod test;`, and are marked `#[aidoku_test]`;
`en.mangakakalot`, `ja.senmanga`, `ja.soraraw`, `ja.spoilerplus` and `en.chikari` (inline in
`lib.rs`) have real ones upstream, and `en.atsumaru` here. They hit live endpoints and assert
on actual results — pinned keys, whether a url resolves, whether a listing comes back sane,
whether each setting and filter actually narrows what the site returns.

**Write them for every new source.** Most of what went wrong in #735 would have shown up in
one. Cover browse paging, every sort and filter, each content rating on its own, a cleared
multi-select setting (must show nothing), a filter overriding its setting, details, a
chapters-only update, pages, deep links and a key the site doesn't know. Check results against
the site's own data rather than only for being non-empty. Name no other sources, clients or
groups in test data. **Tests stay on `main`:** `scripts/pr-branch.sh` leaves `src/test.rs` and
its `#[cfg(test)] mod test;` line out of the submission and fails if either reaches it. Upstream
doesn't require tests (`en.toonkotsu` went up without them); the dev-dependencies stay, as on
most upstream sources.

Running them:

- Install the runner once: `cargo install --git https://github.com/Aidoku/aidoku-rs
  aidoku-test-runner`. It links the system fontconfig and freetype, so install their headers
  first (`libfontconfig-dev libfreetype-dev` on Ubuntu). `.cargo/config.toml` already names
  the runner, so `cargo test` in the source directory runs every `#[aidoku_test]`; `scripts/verify-source.sh <dir> --test` runs the
  gates and then the tests. `cargo clippy --tests --target wasm32-unknown-unknown -- -D
  warnings` lints the test module, which the plain clippy gate skips.
- `verify-source.sh` also lints the test module when a source has one (`cargo clippy --tests`),
  with the same dead-code lints as the source (section 8).
- `.github/workflows/test.yaml` (**Source tests**) runs them for every source a push to
  `main` changes, and on demand from the Actions tab for named ids, or every source with
  tests when none are named. It is a report, not a gate: the tests call the live site, so a
  red run can be the site or the network. Read the failing assertion before blaming either.
- The runner simulates most of the app, not all of it: WebView calls return `-1`, and the
  layout of a home page or how a component renders can only be seen in the app.

For behaviour only the app shows, `aidoku logcat` starts a log server; enter its url in the
app's Advanced settings under Log Server and every `println!` from a device on the same
network streams to the terminal.

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

---

## 3. Review feedback

Our own reviews first, then what kkantan asked of other contributors. `scripts/check-sources.py`
catches the mechanical ones; the rest are for reading the diff.

**Before every push of a source, read this whole section against the diff, row by row.** A
rule here was broken again after it was written down: `en.atsumaru` went to `main` showing
"Manhwa · ★ 8.3" under every home row, and a pasted-link search returned a series' full
details. Passing CI doesn't mean a rule was followed; the owner found these in the app.

Each of these was a real comment. They generalise.

| What was flagged | The rule |
|---|---|
| Search returned tags + descriptions | Search/listing results carry **key, title, cover** and optionally status and content rating. Everything else is filled in by `get_manga_update`. |
| Home rows showed "Manhwa · ★ 8.3" (our own miss, `en.atsumaru`) | The same rule covers every card: search, listings, `Scroller`, `MangaList`, `MangaChapterList` and a pasted-link result. Build rows as `Link::from(compact Manga)`, never with a `subtitle`. **Only `BigScroller` shows details.** |
| A "chapter" invented for `MangaChapterList` from a feed with no chapters | Use `MangaChapterList` only when the feed carries a real chapter; otherwise the section is a `Scroller`. |
| A pasted link answered with the full series | Look the series up the way a search card is built — one compact request — and fall back to text search when it isn't found. |
| Explicit `"default"` on a sort filter | Don't set what the framework already defaults to. |
| Icon filled the whole canvas | Leave margin; aim for something closer to the site's favicon. 128×128, fully opaque. |
| A translated-languages filter | If the app's own languages setting covers it, the filter is redundant. Don't ship both a filter and a setting for the same axis unless the filter **overrides** the setting for that search. kkantan asked for that with content ratings (#815); `en.atsumaru` does it for content types too, since readers filter novels and comics per search. A filter that only repeats a setting, or is silently ANDed with it, is the redundant kind. |
| Genre filter presentation | Genre multi-selects show as tags. In `filters.json`, `isGenre` already implies `usesTagStyle` (AidokuRunner decodes the default from `isGenre`), so don't set it there; a `MultiSelectFilter` built in Rust defaults to `false` and must set `uses_tag_style: true`. |
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

### From #815 (`en.toonkotsu`)

| What was flagged | The rule |
|---|---|
| `language: Some("en")` on every chapter | Don't set a chapter's language on a single-language source. |
| "Does this not support the filtered types setting?" on the latest feed | Every home feed honours the same settings as browse. Find out what the feed endpoint accepts (section 4) before deciding between sending the parameters and filtering locally. |
| Dynamic genres without `supportsTagSearch` | Opt in and handle the `"genre"` select (section 2). |

The fixes went up as a separate `fix(<id>): address review feedback` commit with a
one- or two-sentence body, and each comment got a one- or two-sentence reply.

On approval kkantan suggested, for later, a **content rating filter in search that
overrides the setting** (as `multi.mangadex` has) and a **"Has chapters" toggle**, both
matching the website. Expect the same on any source whose site offers them.

### From #821 (`en.toonkotsu` filters)

| What was flagged | The rule |
|---|---|
| The "Has Chapters" check sat in the filters header | Set `"hideFromHeader": true` on toggles and other secondary options, so the header keeps to the main filters. |

kkantan pushed that fix to our branch themselves (`chore: hide has chapters filter from header`)
and merged it, so the change reached `main` afterwards as a sync rather than as our commit.
Fetch the PR branch after a review: a reviewer's commit there is the fix to copy back.

### From #827 (`multi.kagane` rework)

| What was flagged | The rule |
|---|---|
| `query.filter(\|q\| !q.is_empty())` | The app passes `None` for an empty search; never test for an empty string. |
| A big scroller with only titles and covers | Fill whatever the listing response already carries — authors, status, rating, tags. Fetch descriptions only if the site gives them without a request per card. |

Merged as `fix(multi.kagane): update for new api and add filters`: one title naming the fix
and the features, lowercase, like other merged multi-change titles.

### From other contributors' PRs

What kkantan asked for on #624, #637, #824, #828, #833 and #835. None of it is site-specific.

| What was flagged | The rule |
|---|---|
| Everything in `lib.rs` | JSON types in `models.rs`, free functions in `helpers.rs`; `lib.rs` keeps the trait impls. |
| `fn into_manga(self)` / hand-built `Chapter` in a loop | `impl From<Item> for Manga` (and `Chapter`) in `models.rs`, then `.map(Into::into)`. |
| `AidokuError::message(..)`, `.map_err(\|_\| ..)` on `Request::post` | `error!`/`bail!`. Don't remap a request error: `Request::get`/`post` only fails on a bad url, and `send` already gives a network error. |
| `.string()?` then `serde_json::from_str` | `.json_owned()` or `.send()?.json()`. |
| A hand-written ISO date parser | `parse_date` with a format string. |
| HMAC signing in `JsContext` | Do it in Rust with a `no_std` crate. `JsContext` is discouraged, and its async eval needs an app version most readers don't have. |
| `Home` registered returning `Unimplemented` | Leave the trait out of `register_source!`. |
| `if !needs_details && !needs_chapters` | Never happens; delete it. |
| `query.as_deref().map(\|q\| !q.is_empty())` | `query.is_some()`. |
| `if !included.is_empty()` in a multi-select arm | A multi-select with nothing chosen isn't sent at all. |
| `url.starts_with(base)` then slicing | `strip_prefix`. |
| Matching status strings as-is | Lowercase first; the site's casing varies. |
| A name→id map in Rust for a filter | Put the ids in `filters.json` (`ids`). |
| `canExclude` left on where the API can't exclude | Allow exclusion only where the site takes it. It defaults to `false` both in `filters.json` and in Rust, so omit it rather than writing `false`. |
| A description that can be `""` | `.filter(\|d\| !d.is_empty())` so it's `None`. |
| Two code paths for query and no query | Find the one endpoint that takes both; a site's own search API usually does. |
| Listings that are really search options (Uncensored, Raw) | Make them search filters. |
| Scraping `__NEXT_DATA__` when an API exists | Use the API; check what keiyoushi's source calls. |
| A source on a known theme written from scratch | Make it a template (section 12). |
| A rebranded site added under a new id | Keep the old id and set `breakingChangeVersion` in `config`. |
| `Vec<Regex>` built per call | Compile once in a `static` with `spin::Lazy`; a fixed set is an array (`[Regex; 3]`), not a `Vec`. |
| Pages silently dropped when a per-page key fails | Fail the page list with the error, or return the page unprocessed so the reader sees which one broke. |
| A per-item cache on the source struct | Drop it unless the site makes it necessary; the reference client didn't need it. |
| `"HQ-now"`, `struct Theblank` | The site's own name and spelling, in `source.json` and the struct name. |
| No `minAppVersion` | `"0.7.1"`, unless verify asks for more. |
| Comments marked resolved without a change | Every comment gets a change or a reply; reviewers check. |

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

### Site type picks the sources to compare against

Find out how the site is built before writing anything, then read merged sources built the
same way. View the page source and watch the Network tab:

- **Server-rendered HTML** (the data is in the page): scrape it with selectors —
  `en.weebcentral`. A WordPress theme (`wp-content` in the source) is a template (section 12).
- **Next.js** (`/_next/`, `__NEXT_DATA__`): look for the API the pages call and use that
  instead of the embedded data (section 3).
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
  redundant. Upstream writes `"canExclude": false` 73 times; it is harmless but unnecessary.
- Every filter has `hide_from_header` (`hideFromHeader` in JSON), unset by default, so a
  filter shows in the header unless told otherwise.
- `Manga` has no field for alternate titles, original language, year or score.
  Don't fetch them.
- `Chapter` has `volume_number`, `locked`, `thumbnail` and `language` — easy to
  leave unset by accident.
- `config.languageSelectType` in `source.json` accepts only `"single"`; the CLI
  schema also lists `"multi"` but the app rejects it and the source fails to
  install. Omit the field for multi-select, which is the default.
- `parse_date` formats with the **device's locale** unless given one, so English month or
  day names fail on a phone set to another language; pass `"en_US_POSIX"`.
- Dynamic settings get their defaults registered only when the reader opens the settings
  page, so `defaults_get` returns `None` for them until then; read them with a fallback.
- `defaults_get::<Vec<String>>(key)` returns `None` when unset and `Some(vec![])`
  when the reader cleared it. Keep those distinct — collapsing them means
  unchecking every box silently re-enables everything.
- `aidoku verify` computes the minimum app version from the imports the wasm uses and
  fails if `minAppVersion` is lower. The WebView calls (`delete_all_cookies`,
  `set_rule_list`, `add_user_script`) need **0.9.0**; without them, kkantan has asked
  for `"0.7.1"` on other PRs.

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

## 9. This repo's workflow

- Work on `main` and push to `main`. Any other branch gets a conventional name —
  `feat/<id>`, `fix/<id>`, `chore/<topic>`, `docs/<topic>` — and **never a `claude/` prefix**,
  whatever a tool suggests.
- **Every push that changes a source publishes it, so every such push
  bumps it**: `scripts/bump.sh <id>` raises `res/source.json` and the two `.aix` links on
  that source's README row — anchored on the source id, so another source whose label
  happens to match is left alone. It bumps once per push: run again while the source is
  already above `origin/main`'s version, and it says so and changes nothing.
- Commits are authored **and** committed by
  `Popmango Extensions <297959108+PoppingMangoSources@users.noreply.github.com>`,
  **unsigned**, with a conventional subject and a one- or two-sentence body.
  `--reset-author` only works with `--amend`; for a new commit set the author through
  the environment:

  ```sh
  GIT_AUTHOR_NAME="Popmango Extensions" \
  GIT_AUTHOR_EMAIL="297959108+PoppingMangoSources@users.noreply.github.com" \
  git -c commit.gpgsign=false \
      -c user.name="Popmango Extensions" \
      -c user.email="297959108+PoppingMangoSources@users.noreply.github.com" \
      commit --no-gpg-sign -m "fix(multi.xcomic): ..." -m "One or two sentences."
  ```

- No issue or PR numbers (`#123`) and no links in commit messages: GitHub turns them into
  links and posts a reference on the upstream thread. Name the change in words instead.
- No AI attribution in commit messages, PR bodies or code comments. No other sources or
  third-party clients named in code comments or test data.
- Don't open a PR or post on one unless asked; draft the text for the person to post.

### How a rule gets into this file and the checker

This file and `check-sources.py` are only as good as their least accurate rule. A wrong rule
does damage twice: it fails correct code, and people trust it over the evidence. Every rule here
went in one of these ways, and new ones go in the same way:

1. **A review comment or an owner's correction becomes a section 3 row in the same commit as
   the fix**, quoting what was flagged. A rule from a single PR is marked (inferred) until a
   second review or the implementing code confirms it.
2. **Decide it from the code that implements the behaviour, not the comment's wording.** The
   app decodes `filters.json` in AidokuRunner and the Rust structs live in aidoku-rs; read
   them (both clone anonymously) and cite the file. "Set `can_exclude` to false" on #824 read
   like "the default is true", but the decoder says `false`.
3. **If it can be detected without the network, add it to `check-sources.py` with a test in
   `scripts/tests/test_check_sources.py`** — one case it fails and one legitimate case it
   passes.
4. **Calibrate it**: `scripts/calibrate-rules.sh "<rule text>"`. If it fails merged upstream
   code, look at what it fails. Old code kkantan never reviewed is backlog; a recent merge
   that kkantan approved means the rule is too broad, so narrow it (the silenced-warning rule
   allows a commented exception because an untagged serde enum needs its field) or drop it
   (an author-search flag beside an author filter was merged in `multi.nhentai`, so that is
   not a rule).
5. **When evidence contradicts a rule, fix the rule in the same commit**, and say where the
   old wording was wrong. Section 2's tag-search paragraph and section 3's tag-style row were
   both corrected that way.

### Gates

- `scripts/install-hooks.sh` once per clone from the tooling checkout on `main`. It installs
  an absolute hooks path so the same hook works in clean upstream worktrees; keep that
  checkout available. Scope validation runs before source/toolchain checks. The pre-push hook runs CI's lint and
  verify commands (section 1) for every source and template the push changes, then
  `scripts/check-sources.py`.
- `scripts/check-sources.py [--base <ref>] [ids…]` checks the rules that need no network:
  tab-indented JSON, final newlines, no `"multi"` language, `minAppVersion` present,
  `breakingChangeVersion` in `config`, a 128×128 icon, redundant filter defaults, check
  filters left in the header, decorative dividers, `page_size` above 5, chapter `language`
  on single-language sources, `defaults_get` outside `settings.rs`, the review-comment
  patterns section 3 lists (an empty-query test, a neither-flag guard, a hand-built
  `AidokuError`, `Unimplemented`), a home-row `Link` given a subtitle, a series' full details
  fetched inside search, an `allow`/`expect` that silences an unused-code warning without a
  comment above it, `usesTagStyle` on a JSON genre filter, a `Cargo.toml` without upstream's
  release profile, an `#[aidoku_test]` that asserts nothing, README row in step with `source.json`, a version bump
  since `<ref>`, and byte-identical `AGENTS.md`/`CLAUDE.md` plus matching physical skill trees. CI (`check.yaml`) runs it on the
  sources each push changes. Run with no arguments it reports the backlog in older sources —
  fix those when you're in the source anyway.
- `scripts/calibrate-rules.sh [rule text]` runs `check-sources.py` over every merged upstream
  source and counts failures per rule (or lists the sources one rule fails). CI's
  `check.yaml` prints it in the job summary on every push.
- `scripts/upstream-usage.py <id>` lists every aidoku item the source uses and every name it
  defines that few or no merged sources share. For each, either the site needs it or there is
  a usual equivalent among sources built the same way (section 6): `API_URL` for the API base
  (20 sources), a fetch function for a filter list (`en.toonkotsu`'s `listed_facets`),
  `SearchParams`/`BrowseParams` for search parameters, and `ErrorResponse` for an API error.
  An equivalent isn't always better: `en.atsumaru` keeps a `PageSetting` for its Hidden Tags
  (no merged source uses one) and "Excluded Scanlators" rather than MangaDex's "Blocked
  Groups", because the owner found them clearer in the app. Run it before a submission, and
  put each swap to the person.
- `scripts/check-host-reference.py` compares the files `references/aidoku-host.md` cites in
  AidokuRunner, the Aidoku app and aidoku-rs with each repository's `main`, and lists the ones
  that changed. **Host reference** (`host-reference.yaml`) runs it every Monday and when the
  reference changes; when it fails, re-read the changed file, correct the reference, then run
  the script with `--update`.
- `scripts/source-lints.sh` holds the dead-code lints (section 8) that `verify-source.sh` and
  CI's `lint-crates.sh` add to every source's clippy run, templates excepted.
- Before pushing a source with tests, run `cargo test` in it; the hook doesn't, since the
  tests need the live site. **Source tests** (`test.yaml`) runs them after the push (section 1).

### Upstream PRs

- `scripts/new-source.sh <id> <Name> <url>` scaffolds through `aidoku init`, removes the
  nested repository it creates, and prints the README row.
- `scripts/pr-branch.sh <id> [--push]` builds `feat/<id>` from `upstream/main` in a
  separate worktree: the source copied from a committed snapshot (`HEAD` by default;
  `--from <ref>` explicitly selects another) (`.cargo`, `Cargo.toml`,
  `Cargo.lock`, `res`, `src` — no `package.aix`, no README), `version` set to **1**, one
  `feat: add <id>` commit, gates run. It refuses if the branch touches anything outside
  the source or differs from the selected snapshot beyond the version. Our `main` keeps its own
  incrementing version, since our users have it installed and a version going
  backwards means they never get an update.
- Review rounds: fix on `main` (with a bump), then
  `scripts/pr-branch.sh <id> --fix "fix(<id>): address review feedback" "<body>" --push`
  adds one commit on top of what origin holds; it keeps whatever version the branch has.
- After a merge, a later change to the source is a new PR from the same branch name:
  `scripts/pr-branch.sh <id> --update "feat(<id>): …" "<body>" --push` restarts
  `feat/<id>` from `upstream/main` (it only does so once origin's branch is merged) and
  sets `version` to upstream's plus one. PRs are squashed on merge, so the extra
  commits only make the round easier to review.
- Before any submission push, independently run
  `python3 scripts/check-upstream-scope.py --source <id> --head <ref>` after fetching
  canonical upstream. It checks the merge-base diff **and every submission commit**, not
  `origin/main`. README/website/workflow edits, other sources, generated files, merge commits,
  wrong ids and wrong submission versions fail. A clean final diff cannot conceal unrelated
  commits that were later reverted.
- Submission branches are `feat/<id>` or `fix/<id>`; the existing
  `fix/kagane-chapter-fetch` maps to `multi.kagane`. Use `chore/<topic>` for tooling and
  preserved snapshots. Do not put custom fork checks or docs onto an upstream source branch.
- `upstream-scope.yaml` runs from `main` on main pushes, on demand, and every half hour.
  It reads all submission refs without executing their code and attaches an
  **Upstream source scope** status to each head. This independently catches API/browser
  pushes that bypass local hooks. Scheduled checks can be delayed by GitHub; they report
  failures, not a repository protection rule. Require that status in a ruleset if immediate
  merge blocking is wanted. No protection settings are changed by these scripts.
- To reconstruct a contaminated branch, use
  `scripts/pr-branch.sh <id> --from <saved-ref> --rebuild [--push]`. It preserves the old
  branch as `chore/<id>-before-rebuild-<sha>` before replacing it. Ordinary runs refuse
  to replace an unmerged branch. No source behavior is changed during reconstruction.
- **A rebuild refuses a branch that holds work `main` doesn't have** — source files matching
  no commit of the source on `main`, ignoring the version, `test.rs` and its module line.
  Another session pushed straight to `feat/en.atsumaru` once and the next rebuild replaced
  it; the work survived only in the `chore/…-before-rebuild-…` copy. Bring such work into
  `main` with a three-way merge (base: the commit it was built on, then `git apply -3`),
  then rebuild. `--discard-unmerged` replaces it anyway, for work that is truly unwanted.
- Review rounds preserve the submission version; updates of sources already upstream use
  exactly upstream's version plus one. Fork publication versions advance independently.
- Pushes to the PR branch always use `--force-with-lease=<ref>:<expected-sha>`, never a
  bare `--force`. The person may push to it too (they did on #815), so fetch before
  building on it.

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

## 11. Toonkotsu specifics

`en.toonkotsu` lives upstream only; it was removed from `main` once merged there, and a later
change is a new branch from `upstream/main`, as for Kagane.
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

### Reference implementations

- keiyoushi `src/en/toonkotsu` — Kotlin. Its Latest is browse with `sort=latest`; its
  Type filter is built from the facets.
- `PoppingMangoSources/popmango-extensions-mana`, `src/Toonkotsu` — our own Mana source;
  where the unsigned reader path came from.

## 12. Templates

Eighteen of them live in `templates/` upstream, for sites that share a common
engine or theme:

`madara`, `wpcomics`, `gigaviewer`, `mangathemesia`, `iken`, `mangareader`,
`libgroup`, `mangabox`, `liliana`, `guya`, `mmrcms`, `tukutema`, `ezmanhwa`,
`pizzareader`, `madtheme`, `mangaworld`, `otakuovh`, `vinetheme`.

Our `templates/` is upstream's copy, except `madara`, `madtheme` and `mangathemesia`, which
carry additions our sources use (shared detail parsing, listings and dynamic filters). The
`madtheme` ones (a home page and a chapter-list fallback) were for Kaliscan, which now sits on
the `dead-sources` branch with Galaxy Manga and OManga. When syncing, take upstream's files for every other
template and bring only upstream's own changes into those three.

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

## 13. Where to look

No links here on purpose — search the Aidoku-Community sources repo by these names.

**Reviews worth rereading before submitting.** PRs 726, 531, 624, 637, 646, 690 (two
rounds), 603, 824, 828, 833, 835, and our own 735, 815, 821 and 827. Recurring themes across them: consume owned values instead of
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
(legitimate `DynamicSettings`), `zh.noyacg` (the site gates adult tags),
`multi.lanraragi` and `ja.soraraw` (tag search). `en.elftoon` is 28 lines — a template consumer, and
the fastest way to see how thin a templated source can be.

**The reviewer.** kkantan reviews most of these. Reading their own commits and past
review comments is the fastest way to predict what they'll ask for — the patterns
in section 3 came from doing exactly that.


---

## 14. Kagane specifics

`multi.kagane` lives upstream only. The rework merged as #827; a later change is a new
branch from `upstream/main` (it isn't on our `main`, so `scripts/pr-branch.sh` can't copy it).
Confirmed by console probes on kagane.si unless marked.

- Mirrors: kagane.cx (default), kagane.si, kagane.moe, kagane.net serve the same catalogue
  (kagane.si probed). kagane.org now opens a page that picks one of those four, so it is
  listed last. API under `{base}/api/v2`; the selected url is read as `defaults_get("url")`.
- **Chapters** come from `GET /series/{id}/books/index?fields=List` — rows of values in
  the order `fields` names, groups as indices into `groups`; order by `sort_no`, since
  chapter numbers restart. The series endpoint no longer lists books. (inferred from
  issue #816 and the reference clients)
- **Ratings** are `age_rating` (`Safe`, `FifteenPlus`, `EighteenPlus`) plus
  `is_pornographic`. `is_pornographic: false` gives 18+ without pornographic titles,
  `true` only pornographic ones, absent both. The source keeps the old four names
  (Safe, Suggestive, Erotica, Pornographic) and maps them, as keiyoushi does.
- **Search** `POST /search/series?page=<0-based>&size=&sort=<field>,asc|desc`, answering
  `{content, last}`. Sorts `rating`, `popular_now` and ascending order work. Bodies take
  `current_books`/`start_year` as `{min, max}`, `original_language` codes (an unknown one
  matches nothing), and `publication_status` from `Ongoing`, `Hiatus`, `Completed`,
  `Abandoned` (anything else is a 400 naming the variants).
- More than 50 ids in one list answers `400 {"error":"too_many_values"}`, so the source
  stops with its own message first. `/sources/list` has 315 sources, so it bites.
- Lookups: `POST /search/staff?size=8 {name}` and `GET /groups/search?q=&limit=8` both
  answer; the search body takes the resolved id as `staff`/`groups` `{values: [id]}`.
- `GET /series/trending` answers ten entries, but the section was dropped on request.
- Search results carry `upload_status`, `age_rating`, `is_pornographic`, `format`, and
  `genre_list`/`tag_list` (`{id, name}`) and `staff_list` (`{id, name, role}`); the bare
  `genres`/`tags`/`staff` are ids only. No description, so the big scroller shows everything
  else from the search response.
- **Upstream history** (`git log upstream/main -- sources/multi.kagane`; PR heads fetch as
  `refs/pull/<n>/head`): #603 added it, and its review moved the request helpers into
  `helpers.rs`, replaced a languages setting with the app's languages, cut clones, and
  dropped `supportsTagSearch` because the static genre filter already makes tags
  tappable. #632, #811 moved the domain (`kagane.to` → `kagane.org`); #728 added the
  sort list and the relevance fallback to `total_views` for browsing.


## 15. Evidence before submission

Read [references/aidoku-host.md](.claude/skills/aidoku-source/references/aidoku-host.md) for
what the app actually does with a source — how AidokuRunner loads and calls it, every import,
the JSON defaults, and what each home component renders — with each claim tied to the file it
comes from. Read the synchronized skill reference
[references/source-review.md](.claude/skills/aidoku-source/references/source-review.md)
for official code locations, runner limitations, reviewer comment permalinks and the test
matrix. Use the `.agents/skills/aidoku-source/references/source-review.md` copy from Codex.

- Keep **source-code facts**, **live API observations**, **runner results** and **in-app
  results** separate in the review summary. A build is not an end-to-end source test.
- Use the merged `en.toonkotsu`, `multi.xcomic` and Kagane rework as accepted patterns for
  the behavior they actually implement. Their merged status is not a permanent guarantee
  about every API, nor a reason to copy old code without checking the current library.
- Review complete source branches against fresh upstream before pushing; review main
  publication changes against the fork. Report both boundaries explicitly when both change.
- Answer each reviewer thread with a code change or a reasoned reply. Verify the final
  code before calling a comment addressed. Do not mark discussions resolved for the person.
- Run `scripts/verify-source.sh <source-directory> [--test]` for fmt, warning-failing
  Clippy, packaging and schema/WASM verification; `--test` additionally runs aidoku tests.
  Install `aidoku-test-runner` first. Use tests for meaningful parsers, key/url mappings,
  pagination, cleared settings and error behavior. Live tests are opt-in because connectivity
  and site protection are environment-dependent; never silently exclude failures.
- For WebView/cookies/reader rendering, use an actual Aidoku device or simulator. The Rust
  test runner has stubbed WebView imports. The app's `AidokuTests/Support/TestableSource.swift`
  is an app test double, not an executable harness for arbitrary third-party source WASM.
- Caching, `sccache`, artifact retention and compiler versions affect speed/diagnostics;
  none establish correctness. Do not restore an old cache or compiler action as a substitute
  for a missing behavioral or scope check. Preserve diagnostics when a build fails.

## 16. Atsumaru specifics

`en.atsumaru` (atsu.moe): comics and novels. Confirmed by live requests from the sandbox, which
reaches the site, unless marked.

- **Site type.** A client-rendered React app: the page is an empty `#root` and a Vite bundle
  (`/assets/index-<hash>.js`), with no server-rendered data to scrape. Everything comes from a
  JSON API under `/api/` and a Typesense search at `/collections/manga/documents/search`. So
  the sources to compare against are the JSON-API ones — `en.toonkotsu`, `multi.kagane`,
  `en.chikari` (also novels), `en.comix`, `multi.mangadex` — not the HTML or GraphQL ones.
- **What it shows.** Details only in the big scroller (Popular This Week), filled from one
  search lookup. Every other row, listing and search card is key, title, cover, status and
  rating. Recently Updated is a plain row, since the feed names no chapter.
- **Filters vs settings.** Content Types and Content Ratings are settings, and both are also
  search filters that override the setting for one search (the owner's call: readers pick
  novels or one comic type from Discover). The site's tag groups are **not**
  search filters (the owner's call): a tapped tag still searches by its id, and each group
  has a Hidden Tags list in settings. The site has 6,140 tags, too many for one list, so the
  19 group lists sit on a Hidden Tags page under Exclusions, which keeps the settings short
  (the owner's choice over listing them inline). Excluded Scanlators leaves chapters from
  named groups out.
- **Home** sends the layout as a partial, then returns the sections that loaded, so a failed
  one drops out rather than leaving an empty row; with every section failing it reports an
  error.
- **Sorts.** Popularity, Title, Trending, Recently Added, Release Date, Top Rated, then the nine
  ranked feeds (Hot Updates to Hidden Gems), then Random. As both reference clients do, the
  sort applies to text searches too, and the arrow maps straight through: off is `desc`, on is
  `asc`, for every indexed sort including Title (Typesense takes `field:asc` on each, probed).
  The feeds and Random have one order and ignore the arrow.
- **Rankings.** Only `popular`, `mostBookmarked` and `mostTalkedAbout` take `timeframe`
  (`daily`, `weekly`, `monthly`, `all`; anything else is a 400); the other feeds accept and
  ignore it (probed). The site's Explore offers a period on the Popularity, Most Bookmarked and
  Most Talked About sorts, All by default; with All, Popularity is the index's `views` sort, and
  otherwise the `popular` feed. Each has a setting, `<feed>Timeframe`, defaulting to all, that
  applies only to those search sorts; the home sections and their listings rank by week, as the
  site's home does. A feed sort reads the feed with the settings, then
  narrows it with one search lookup over its ids, so every search filter applies and the
  feed's order is kept. Random sorts on the index's `shuffleA`/`shuffleB` fields from a seed
  the source picks at launch, so paging stays in one order until the app restarts.
- **Search** is Typesense: `q` (`*` for none), `filter_by`, `sort_by`, `page`, `per_page`,
  `include_fields`. `filter_by` takes `&&`, `||` and parentheses; values are backtick-quoted
  (escape backticks and backslashes). `tags` holds genre names, `genreIds`/`tagIds` the ids;
  tags only match by id, so a tapped tag name is looked up in
  `/api/explore/availableFilters`. `id:=[…]` fetches several series in one request. Its typo
  tolerance can match a nonsense query; that's the site, not a bug.
- **Feeds** `/api/home2/{id}` with `offset`, `limit`, `mediums`, `types`, `adult=1` (adult
  only), `contentRatings` and `excludedTags` (genre and tag ids together; the two never share
  an id). `types` and `mediums` are ANDed, so with novels on the types are filtered locally.
  `contentRatings` only filters rated titles: unrated ones pass, and official translations
  can't be filtered at all. A missing `mediums` or `types` means every one, so a full selection
  is left out; a missing `contentRatings` hides adult titles, so it is always sent (probed).
  No total, so a full page is the only next-page signal.
- **Ratings.** `mbContentRating` (`Safe`, `Suggestive`, `Erotica`, `Pornographic`) plus
  `isAdult`; most titles are unrated (579 of 2,669 novels have one). Unrated titles are rated
  from genres: Hentai (`10`) as Pornographic, Adult (`46`) and Smut (`41`) as Erotica.
- **Series** `/api/manga/page?id=` (`mangaPage: null` for an unknown id, with a 200) carries the
  first chapters and `hasMoreChapters`; `/api/manga/allChapters?mangaId=` has the rest. Both
  list chapters newest first, so only deduplicating sorts. One chapter can appear once per
  group; `scanlators` carry a `score`. The page has no rating, so details also look the
  series up in search.
- **Reader** `/api/read/chapter` (pages in order, `/static/…` paths on `cdn.atsu.moe`) and
  `/api/read/novelChapter` (`paragraphs`). Covers are AVIF with a WebP copy beside each.
- **Errors.** Search answers `{"message": …}`; the other endpoints answer plain text
  (`Chapter not found`) or a validation object, so only search's message is shown.
