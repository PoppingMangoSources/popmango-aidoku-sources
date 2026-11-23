# Review standards

Section 3 of the root `AGENTS.md` / `CLAUDE.md`, kept here whole: every rule a review comment,
an owner's correction or our own miss put in place, with the evidence quoted. **F** fails
`scripts/check-sources.py`, **N** prints a note, **R** is read in review. The root file keeps the
hard rules (section 0); read this file against the diff, row by row, before every push of a
source. `scripts/agent-hooks.py checklist` prints its rows as a checklist.

Our own reviews first, then what kkantan asked of other contributors. `scripts/check-sources.py`
catches the mechanical ones; `scripts/review-contracts.py` tests template behavior through public APIs.
The rest are for reading the diff.

**Before every push of a source, read this whole section against the diff, row by row.** A
rule here was broken again after it was written down: `en.atsumaru` went to `main` showing
"Manhwa · ★ 8.3" under every home row, and a pasted-link search returned a series' full
details. Passing CI doesn't mean a rule was followed; the owner found these in the app.

Each of these was a real comment. They generalise.

| What was flagged | The rule |
|---|---|
| Search returned tags + descriptions | Search/listing results carry **key, title, cover** and optionally status and content rating. Everything else is filled in by `get_manga_update`. |
| Short synopses disappeared without an expand control (Keyoapp review) | Read the synopsis content in both short and expandable layouts; expansion markup is optional. Test both layouts and an absent/empty synopsis. |
| Combined credits and duplicate names reached the app (Keyoapp review) | Split the site's verified credit separators, trim names, omit blanks and deduplicate within each role while preserving order. Keep author and artist roles independent. |
| A comment containing "Author" became a title's author (Rage review) | Keep author and artist selectors inside metadata rows. A text match over a container holding comments can mistake prose for metadata. An impossible author returning no results does not establish useful author search; omit that filter when the site has no author metadata. |
| A webtoon labeled "Manga" opened right-to-left (Rage review) | Use a small optional viewer override for a source with mislabeled types. Default it to no override so other template consumers retain their existing mapping. |
| Novel emphasis was stripped to keep an older app minimum (shared-template review) | Preserve inline bold/italic while keeping paragraph and line-break boundaries. Adjacent nested spans must keep every closing marker, not only the last one. If the shared reader imports `child_nodes`, every dependent package needs `minAppVersion` at least 0.8.3, including image-only consumers. Check the current consumer list, bump changed existing submissions and verify every package. |
| Home rows showed "Manhwa · ★ 8.3" (our own miss, `en.atsumaru`) | The same rule covers every card: search, listings, `Scroller`, `MangaList`, `MangaChapterList` and a pasted-link result. Build rows as `Link::from(compact Manga)`, never with a `subtitle`. **Only `BigScroller` shows details.** |
| Big scroller descriptions showed `life\, while` and `\[Domestic` (our own miss, `en.novelarchive`) | `escape_markdown` only on the details path. A `BigScroller` shows its description verbatim, so give it the site's plain text (section 6). |
| WebNex showed rank and readership as a synopsis and dropped streamed titles | Leave the hero description empty when the feed has no synopsis. Compare ordered keys for every row against the rendered or reassembled response, including titles split across fragments. |
| WebNex preset choices were above the numeric input and had an unwanted Any option (owner correction) | Keep presets after their typed input. When no neutral choice is shown, an explicit empty-string select default keeps every real preset selectable; app `SelectFilter.resolvedDefaultValue` otherwise takes the first ID. Cover the resource order/default with a `review_` regression. This is a source UI choice, not a universal ban on Any. |
| WebNex gained an unwanted typed Tags advanced-search field (owner correction) | Omit WebNex's typed Tags control and remove its unused parser arm. Keep the existing selectable genre/format/theme filters and detail tag taps. This is the owner's source-specific choice, not a general ban on tag text filters. |
| MangaBall's bold text became literal markers after loading chapters (owner's screenshots) | Escape plain-text descriptions only. Preserve existing API Markdown on details, as merged MangaDex does; partial and final updates must keep the same description. BigScroller renders plain text and cannot display Markdown emphasis. |
| A chapter escaped whole, one paragraph of 600 hard breaks, crashed the app with a stack overflow (our own miss, `en.novelarchive`, crash logs) | The scroll reader's MarkdownUI joins every inline node of a paragraph into one SwiftUI `Text` with `+`, and the crash logs show the main thread's stack running out at about 650 levels. Make each line its own paragraph and join them with blank lines, as `en.atsumaru` and `zh.wenku8` do. cmark makes every backslash escape a node of its own, so `escape_markdown`, which escapes all punctuation, can still overflow a long line (mirror chapters run to thousands of characters on one line); `en.novelarchive` escapes only Markdown syntax for chapter text. `check-sources.py` fails `PageContent::text(x.escape_markdown())`. |
| A "chapter" invented for `MangaChapterList` from a feed with no chapters | Use `MangaChapterList` only when the feed carries a real chapter; otherwise the section is a `Scroller`. |
| Clearing a filter was tested by comparing two live latest-update snapshots (Nyx retry) | Compare cleared and unset filters with the same stable sort. Keep request failures distinct from order changes between requests; diagnostics name the failing sort/direction. |
| Optional home enrichment touched a ranking row (Iken recheck) | Limit chapter enrichment to named release sections; preserve ranking components and test one series appearing in both featured and release rows. Preserve available HTML tags/rating/viewer when the optional API metadata omits them. |
| Latest releases passed a heading-only test but displayed cover-only rows (owner's Iken screenshots) | Test real chapter keys/numbers/timestamps and pinned titles missing from optional API enrichment. Every latest component remains MangaChapterList when the HTML supplies actual chapters; a missing enrichment match cannot downgrade the row. |
| A legacy API omitted new series and returned escaped descriptions (owner's Eternal screenshots) | Verify the reported positive titles against the current site, including chapter inventory and displayed synopsis. A plausible API response and one old title passing do not establish freshness. Keep an obsolete API only for legacy-key compatibility after a site migration. |
| A pasted link answered with the full series | Look the series up the way a search card is built — one compact request — and fall back to text search when it isn't found. |
| Explicit `"default"` on a sort filter | Don't set what the framework already defaults to. |
| Small favicon assets looked blurred after resizing (Hijala/Sana icons) | Prefer the site’s original high-resolution logo over its favicon. Check natural dimensions before downsampling; do not upscale a small favicon. Verify the final opaque 128×128 icon visually with margin around the artwork. |
| Icon filled the whole canvas | Leave margin; aim for something closer to the site's favicon. 128×128, fully opaque. |
| Madara’s dark artwork sat on white (#884) | Choose an opaque background that fits the site’s artwork/header. Madara uses its verified `#0e0f13` header colour; do not impose that colour on other sources. |
| A translated-languages filter | If the app's own languages setting covers it, the filter is redundant. Don't ship both a filter and a setting for the same axis unless the filter **overrides** the setting for that search. kkantan asked for that with content ratings (#815); `en.atsumaru` does it for content types too, since readers filter novels and comics per search. A filter that only repeats a setting, or is silently ANDed with it, is the redundant kind. |
| Genre filter presentation | Genre multi-selects show as tags. In `filters.json`, `isGenre` already implies `usesTagStyle` (AidokuRunner decodes the default from `isGenre`), so don't set it there; a `MultiSelectFilter` built in Rust defaults to `false` and must set `uses_tag_style: true`. |
| 14 listings | Listings that are just search sorts don't need to exist. Keep the ones home sections link to. |
| A safe API mode still returned adult-tagged titles (MangaBall audit) | Verify flags against tags and apply the same rating classifier in details, featured cards and safe-feed guards. Keep server filtering and pagination, but do not fetch extra pages to fill locally reduced rows. |
| A select default prevented a search from overriding settings (MangaBall audit) | The app omits default select values. Use a neutral source-setting option when all actual values, including Any/Safe Only, must remain selectable overrides. |
| Excluded groups still appeared in latest home chapters (MangaBall audit) | Apply the same group and language rules before choosing a chapter for home and when building the full chapter list. |
| Sorting returned HTTP 200 but the app could not decode the page (MangaBall correction) | Validate every sort direction through the source decoder. Capture legacy scalar/list variants and incomplete identities; preserve valid card order and pagination, reject untitled cards, and keep details failures meaningful. HTTP success or changed result IDs alone do not prove source behavior. |
| Adding a template dependency left consumer lockfiles stale (Scythe review) | Refresh every direct and transitive consumer’s `Cargo.lock`, then verify each with `cargo metadata --locked` for its target. Dependency metadata synchronization alone does not require a peer consumer version bump in an upstream submission; code/configuration or behavior changes still do. |
| A helper sat in `lib.rs` | Key/url helpers belong in `helpers.rs`. |
| `String::new()` / `Vec::new()` everywhere | `#[derive(Default)]` on the struct, then `..Default::default()`. |
| Temporary vectors before assigning to params | Assign straight into the params struct. |
| Settings lookups outside `settings.rs` | If the source has a `settings.rs`, the `defaults_get` calls live there. |
| `page_size: Some(10)` on home components | **5 max**, for all components — 10 means a lot of scrolling on a phone. |
| Latest Comics used `page_size: None` when it was meant to show five rows (owner's correction) | Use `Some(5)` for a five-row paged list. `None` renders every row vertically; it is not a five-row default (`HomeChapterListView.swift`). |
| Web view urls 404'd | Series and chapter urls need to resolve. Have a fallback if the API's path field can be absent. |
| MKissa v32 and v34 failed every chapter with `JsError(MissingResult)` (owner's logs) | Don't use `WebView::eval_async`: the app wraps it as `await (script)` in an async arrow run by `evaluateJavaScript` (AidokuRunner `WebViewHandler.swift`), and it answered nothing even with an expression (inferred: the wrapper's value is a promise WebKit rejects). Start work with `eval` and poll a string state, as merged Comix and Toonkotsu do. v35 did that alongside a content rule list and a reused page, and still failed in the app, so MKissa went back to v31 (owner's call). The runner stubs web views and Chromium passed all of it, so change one web view behaviour per release and have the owner try it in the app before building on it. `check-sources.py` fails statements passed to `eval_async`. |

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
| `canExclude` left on where the API can't exclude | Enable exclusion only when the source handles `excluded` for that filter. It defaults to `false` in JSON and Rust; an explicit `false` is also valid and makes unsupported exclusions clear. |
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

### From #819, #825, #840, #842, #845 and #846 (other contributors, October)

Asked again on these, so they are the most repeated: `bail!` over a hand-built
`AidokuError::Message`, listings with only key, title and cover, no empty-query test, no
explicit sort default, `defaults_get` in `settings.rs` even for one switch, helpers out of
`lib.rs`, no neither-flag guard, no `!included.is_empty()` in a multi-select arm, a check filter
hidden from the header, `has_next_page` from the API rather than the length of the page, and no
url read back to fetch data (#840). Resolved threads on #819 and #825 only show to a signed-in
viewer; their rules were read from the fix commits. New ones, marked as in the owner's table
below (**F** fails `check-sources.py`, **N** prints a note, **R** is read in review):

| What was flagged | The rule | |
|---|---|---|
| `#[serde(rename_all = "camelCase")]` on JSON that isn't ("have you verified this?", #846) | Every rename and field name matches a captured response. A wrong rename leaves `#[serde(default)]` fields at their defaults without an error. | R |
| `sort` and `order` sent to an endpoint that ignores them ("this doesn't seem to do anything", #846) | Send only parameters the endpoint honours; probe each with a value that should change the result. | R |
| The sort filter built last, titled "Sort" (#846) | The sort filter goes first, untitled unless the site labels it. 47 merged sources put it later, en.hentai2read among them, so it is a note. | N |
| `hideNsfw` defaulting to `false` with `contentRating` 1 (#845) | A source rated 1 hides adult titles by default; to show them by default, rate it 2. Keep the Rust fallback the same as the JSON default. | F |
| Discover sections in a different order from the site (#845) | Order the home sections as the site's home page does. | R |
| `.unwrap_or("")` repeated down a chain (zh.suduguu, November: "keep this an optional result. it doesn't make sense for callers to check if the result is an empty string rather than `None`") | Keep a missing value as `None` through the chain and test it with `is_some_and`/`is_none_or`; never default it to `""` for a later check. (inferred) | R |
| A page limit ending a loop with `break` (zh.suduguu, November: "it would be better to return an error here") | When a guard stops work the reader asked for, `bail!` with why, rather than return a shortened result. (inferred) | R |
| Genre exclusion offered on Genz Toons, whose site and keiyoushi only include genres ("are genres even excludable in genz site?", owner's correction) | A filter offers what the site's own control does. Local filtering could exclude, but a control the site lacks is one more thing to review; state `"canExclude": false`. (inferred) | R |
| MKissa's genre and tag exclusion was removed to match the site's include-only panel, then restored (owner's call) | The owner may keep an exclusion the site's panel lacks when the API's own exclusion is probed to work; record that choice and the probe in the source's docs page, and test it against results. | R |
| Eva Scans rated 17+ because its genre list has Adult, Smut and Mature (our own miss, owner's correction) | A source's `contentRating` follows what most of the site carries, not every genre it lists; per-title ratings come from tags. Eva is safe, as keiyoushi has it, while Raven is 2 since adult titles fill every list. A Hentai genre is the exception (the next row): Eva, Thunder and Violet list Adult or Smut and stay 0 (owner's call). | R |
| Scans.GG rated 0 though its genres include Hentai (kkantan: "given that there is a 'Hentai' genre and such, this should be marked as containing nsfw content", 0 to 1) | A source offering a Hentai genre is rated at least 1, however little of the site it covers; Madara Scans is 1 for that reason. | F |
| Comics and novels behind a source setting (#842, also dyphire) | When a setting would switch between two catalogues, make two sources over a template, or make it a search filter. A setting makes readers give one catalogue up. | R |
| `page_url_hd` dropped in a rework (#842) | A rework keeps every field the old code preferred, such as a higher-quality image, unless a response shows it is gone; fall back to the plain one. | R |
| `requiresFalse: "contentType == novel"` on a setting (#842) | A setting can depend on another one's value. Below app 0.9 the dependent toggle isn't disabled, so the code must still check both. (inferred, the author's test) | R |
| A slug looked up by searching the whole series list each time (#840) | Keys hold only what never changes: a slug carrying a date goes stale, and a stale key duplicates the series in the library. Map id to slug in a `HashMap` filled when the list is fetched. kkantan first asked for `id\|slug`, then withdrew it once the slugs turned out to change. | R |
| `ImageScroller` with `width: None` (#819) | Set `width` and `height` when the banners have one size; it smooths preloading. | N |
| A banner linking a series page with `LinkValue::Url` (#819) | Link a series as `LinkValue::Manga` with only the key; keep `Url` for pages that aren't series. | R |
| The two newest chapters of a long series didn't load (#819) | Reviewers read in the app. Before submitting, open the newest chapters of a long series as well as the first. | R |
| A chapter that failed to load returned "(empty chapter)" text (#819, fix commit) | Fail the chapter with `bail!` rather than return placeholder text, and don't skip a fragment that failed. | R |

### From #865, #869 and #872 (November)

| What was flagged | The rule | |
|---|---|---|
| Missing titles became empty strings (#865) | Drop a card without a nonempty title; keep missing values optional rather than manufacturing empty strings. | R |
| Rechecking the host in a deep-link handler (#869) | The app dispatches only matching source URLs. Parse their paths without a redundant host guard. Pasted search URLs are user input and still need an exact base/path boundary; substring hostname checks are insufficient (#865). | R |
| Chapter ids with suffixes were truncated (#872) | Preserve the site's complete chapter key, including hyphens or page suffixes. Chapter-number parsing must not rewrite identity. (inferred) | R |
| Extra helpers and long explanations in `lib.rs` (#872) | Put free helpers in `helpers.rs` or inline a single-use operation. Comments explain a non-obvious constraint, not the function body. | R |
| A contributor with both writing and drawing credits lost one role (#865) | Assign author and artist roles independently, so one person can appear in both lists. (inferred) | R |

### From #878, #894 and #899 (other contributors, November)

Read on 2026-11-13 from the PR pages; quotes are kkantan's.

| What was flagged | The rule | |
|---|---|---|
| A listing that repeated the search code ("you could just be calling `get_search_manga_list` instead of duplicating the logic", #878) | A listing that is a search sort calls the search path with that sort. `multi.mangaball` builds both from one `SearchParams`. | R |
| `unwrap_or("[]")` on a parse that failed ("it would be better to propagate the errors", "instead of returning an empty chapter list", #894) | A failed parse is an error, never an empty list or a default. | R |
| `return Ok(MangaPageResult { entries: Vec::new(), has_next_page: false })` (#894) | `MangaPageResult::default()`. | R |
| `.text()` then `.trim()` ("`text()` already trims the result", #894) | Don't trim what `text()` returns. | R |
| `serde_json = "1.0.105"` ("exclude the patch version for dependencies", #878) | Dependencies name major and minor only. | R |
| A doc comment on the page-list function ("this is considered common knowledge", #878) | No comment for what the trait already documents. | R |
| `let mut manga = manga;` (#878) | Mark the parameter `mut`. | R |
| A hand-written percent encoder (#878) | `encode_uri_component`. | R |
| A select filter used for sorting (#878) | Use the `sort` filter type, first in the list. | R |
| A genre select without `isGenre` (#878, #894) | Every genre select or multi-select sets `"isGenre": true`. | R |
| Filter `ids` that repeat `options` (#899) | Leave `ids` out when the site's ids are the option names. | R |
| A metadata selector that also matched chapter badges (#899) | Scope selectors to the metadata area. | R |
| An adult switch off by default on a source rated 2 (#899) | Rated 2 may show adult titles by default; rated 1 hides them (the #845 row). | R |

### From the owner's checks of `multi.mangaball`

| What was flagged | The rule | |
|---|---|---|
| A Relevance sort the site doesn't have, copied from a reference client that omits `sort_by` (owner's screenshot of the site's Sort By list) | Sorts and filters come from the site's own controls, in the site's order and wording. A reference client shows which parameters work, never which options to offer. When the sandbox can't load the search page, ask for a screenshot of its panel before writing `filters.json`. | R |
| Extra fix commits remained after the owner asked to force-push (MangaBall correction) | A new-source submission stays one `feat: add <id>` commit, including after the PR opens, unless the owner explicitly requests separate commits. Fold fixes into the requested history, preserve a backup, verify tree equality and use an explicit-SHA force-with-lease. `--fix` adds a commit, so use it only when a separate commit was requested. | R |
| Shared reader changes landed in the WebtoonClub commit (Toonz owner's correction) | A new template submission puts every shared template change in its first template commit, followed by one source-only commit per consumer. Rebuild the requested history, verify final tree equality and use an explicit-SHA force-with-lease. Existing upstream template updates can still have separate review-fix commits. | F |
| Toonz viewer override copied an unmerged fork branch's inline `mut strip` flag in `details()` instead of matching merged sources' `viewer()` helper (owner's correction) | Before writing any feature or fix — even when adapting behavior from an unmerged fork branch — run `scripts/upstream-usage.py --feature '<regex>'` and read the newest merged upstream sources built the same way (e.g., `en.mkissa/src/helpers.rs::viewer`, `ja.soraraw/src/helpers.rs::viewer`), then match their helper shape rather than scattering flags inline. | R |

### From #900 (Iken and EternalMangas)

Merged on 2026-11-15 (#900) and 2026-11-16 (#905); kkantan pushed one commit to each branch,
the version revert and the optional rewrap above. The Iken template, Hijala and Sana on
`merged-sources` matched the merged crates; EternalMangas differed by the rewrap, now copied
into the archive, and by the tag title-casing that never went upstream; the archive now matches the merged crate and a comment in `helpers.rs` records the dropped helper.

| What was flagged | The rule | |
|---|---|---|
| Eternal's independent rewrite was bundled with the Iken template update | A rewrite that no longer uses a template belongs in its own PR. Keep only necessary API/minimum-version compatibility changes with the shared template, and test the remaining legacy consumer until its migration merges. | R |
| `aidoku`/`aidoku-test` development dependencies were removed because tests were excluded | Retain the standard test scaffold even without test files. Removing fork-only tests does not justify deleting these dependencies. Submission normalization may remove genuinely unused test-only serde_json. Scope checks reject removing an upstream scaffold. | F |
| Genre resource rewrite dropped `isGenre` | Preserve genre capabilities while changing options, IDs or select types. Eternal's genre select must keep `isGenre: true`. | F |
| Structs, impls, registration and methods ran together | Separate Rust items and methods with blank lines; rustfmt does not insert them. Reviewed Iken/Eternal gaps fail the checker; existing merged counterexamples are advisory elsewhere. | F/N |
| Metadata label lookup could use `:has`/`:contains` | A scoped selector can simplify the walk. Eternal uses `.col-lg-3 li:has(strong)` and exact label-prefix matching. `:contains` was an optional suggestion, not a general requirement; test host/runner support before using it. | R |
| EternalMangas' version raised in the Iken PR ("this pr leaves eternalmangas broken, so the version change should be saved for the other pr"; kkantan pushed `chore(es.eternalmangas): revert version change`) | A consumer a template PR leaves broken keeps its upstream version in that PR, and its own PR carries the bump. | R |
| Hijala, Sana and Hive wiring (kkantan wrote the exact layout as suggestions on #900) | Declare `struct X;`, then `impl Impl for X` with `new` and `params`, then `register_source!`, each item separated by a blank line, and copy the signature the trait gives. (inferred) | R |

### From the Vine reader reports and template audit

| What was flagged | The rule | |
|---|---|---|
| A Next.js chapter metadata object was accepted as an empty reader | Match the reader payload by its access fields and chapter content/pages, not the first object named `chapter`. Empty/missing pages fail; never silently drop one malformed image. Test comics and novels from keys only. | R |
| Inactive image material flags rejected readable pages | Check the site's active protection configuration. Tile/strip metadata can remain when protection is disabled and the site renders the normal imageUrl. Preserve that fallback and test it separately from active protection, redacted pages and missing URLs. | R |
| The first latest-update card appeared twice | Cover and title anchors belong to one card. Preserve the first unique series per row, real chapter numbers/dates, and the site's section order. A section query can include its root; test nested wrappers and sibling rows. | R |
| Hero descriptions showed Flight references or missed embedded metadata | Resolve home descriptions through Flight text rows before converting to plain text. Reuse richer records already on the page; never fetch details per hero card. Shared row ordering and page-size handling stay in the template, with source-specific row declarations. | R |
| An empty HTML synopsis rejected a valid series | Optional descriptions may render to empty text. Omit them without rejecting the series; reject empty content at the reader boundary instead. | R |
| Vine sources invented a Show 18+ Content setting (owner correction) | Do not add adult visibility controls the sites do not offer. Vine sources use contentRating 1, except Diva at 2 by the owner’s request, with no adult toggle or local adult filtering; per-title ratings still describe the returned content. | R |
| A failed pasted-series lookup aborted search | A missing/stale pasted URL falls back to the original ordinary search query, as merged MangaBall does. | R |
| Long novel HTML was a Flight reference | Decode the JSON push strings and resolve byte-length text rows. Raw text may contain newlines and may end immediately before the next row. Do not unescape Flight with chained string replacements. | R |
| Vine home and detail tags appeared in lowercase | Format display-only names from API genres, tags and slug fallbacks as readable title case, consistently across home, details and dynamic filters. Keep filter ids/slugs untouched and test mixed-case duplicates and content ratings. | R |
| Displayed genre names were sent as API slugs | Resolve dynamic tag taps through the fetched genre/tag facets once. A successful response with an ignored or wrong parameter is not a filter test. Static sorts have no ascending arrow when the site has one order. | R |
| A Valir novel card opened its comic edition, then keying novels by the API slug broke the reader (owner's screenshots and logs) | A novel's public slug can equal its comic's, and the reader route answers only on the public slug. Keep public-slug keys; when details for a novel key answer a comic, read the API slug from the novel's own page, as the Paperback source reads its details there. Test a shared-title novel from details through a key-only page list. | R |
| Locked chapters still showed on a library series with Show Locked off (owner's screenshot) | The app shows a library series' stored chapters until a refresh replaces them (`MangaView+ViewModel.swift`, `CoreDataManager+Chapter.swift`). Prove the source's filter with a key-only live test before changing it. | R |

The owner requested `fix-vine` as an explicit template branch-name exception. Its template,
Drake/Elf updates, Valir consumer and new Diva/Kayn consumers are submitted from canonical
upstream. Main publishes every affected source with tests and the normal version bump.
Vine's regression contracts intentionally reproduce the open upstream reader/home defects;
pre-existing Keyoapp/MangaThemesia contracts remain the merged calibration baseline.
See `docs/subsystems/vinetheme-template.md` for endpoint, rendering and network evidence. It merged as #908 on 2026-11-16, and the template and its five consumers are on `merged-sources`.

### From #860 (`en.flamecomics`)

| What was flagged | The rule | |
|---|---|---|
| `impl From<Entry> for Manga` with `key(..).unwrap_or_default()` ("we don't want to return manga objects with empty keys") | A conversion that can fail is a `TryFrom` with an error, never a default key. Lists drop or report the entry that fails; the details path returns the error. | R |
| A "Show Locked Chapters" switch defaulting on | Default it off when the source has no login, since chapters the reader can't open are confusing. The Rust fallback and the JSON default stay equal. | R |

### From #873 (`en.novelarchive`)

| What was flagged | The rule | |
|---|---|---|
| A home row and listing titled "New in the Archive" for the library's default sort ("does this title come from somewhere?") | Name a row after what its feed orders by, checked against the data: compare `created_at` and `updated_at` down the first page. "New" reads as brand-new series, so a sort by date added is "Recently Added" (`en.atsumaru` names it that too), and one by new chapters is "Recently Updated". A title the site doesn't use needs that evidence ready for review. (inferred) | R |

### From the owner's checks of the keyoapp template

| What was flagged | The rule | |
|---|---|---|
| Genz Toon's cover slider showed as cropped banners in the app, while the runner and its `review_` test read it as covers (owner's screenshots) | Don't quote brackets inside an attribute selector (`[class*='aspect-[2/1]']`): the app's selector engine read it differently from the runner's. Select the elements, then compare the attribute in Rust. (inferred, one case) | N |
| V2 Trending Now was overridden with Month despite the site's Today default (owner's correction) | Use Today cards directly for Trending. Fetch Month for Most Reviewed, the owner's explicit choice; do not apply that override to Trending. | R |
| Newly Added fell back to a document-wide grid (owner's review) | Read its own cards or its immediately adjacent grid, then reuse the normal card conversion. An empty section must not borrow another row's cards. | R |
| The Keyoapp V1 section ancestor bound was widened from three to five without evidence (owner's correction) | Keep the three-level walk supported by Mist's headings, and test that a heading cannot capture a card only present at a fourth ancestor. (verified on the live Mist homepage, 2026-11-09) | R |
| Blocked requests looked like missing catalogues or invalid JSON (owner’s CI log) | On fork main, live-tested Keyoapp and MangaThemesia requests use checked fetch helpers before parsing. Image request providers still return unsent requests to the host. Report non-2xx status and log URL, content type and at most 240 body characters. `scripts/test-http.py` checks MangaThemesia's failures with a loopback fixture; Keyoapp's and Iken's fixtures live on `merged-sources`. `check-sources.py` rejects direct requests that bypass the helper. | F |

### From #900 and #905 (iken template, `es.eternalmangas`)

| What was flagged | The rule | |
|---|---|---|
| `.text()` followed by `.filter(\|t\| !t.trim().is_empty())` ("`text()` trims already") | `Element::text()` returns normalized, trimmed text; trim only attributes, `own_text`, `data` and other raw strings. | N |
| `"isGenre": true` after a genre filter's `options` and `ids` | Put `isGenre` before the options and ids. | N |
| `mod settings;` straight above `const URL`, and structs with no line between them ("top level items should have newlines separating them") | One blank line between top-level items of different kinds and between structs; rustfmt doesn't add it. | F |
| `chrono::DateTime::parse_from_rfc3339` ("the built-in `parse_date` should be favored") | `parse_date(date, "yyyy-MM-dd'T'HH:mm:ssXXX")`. The runner drops quoted literals, so check the date in the app rather than with an assertion (section 1). Upstream's own Iken still uses chrono for API dates. | N |
| An `if let` chain indented with spaces ("what's happening with the indentation here?") | rustfmt silently leaves an expression it can't fit in 100 columns, here a long `matches!` guard, along with everything around it. Split the long line until `cargo fmt` reformats the block, and read the diff for indentation. | R |
| Banners sized 1920 by 1080 ("ridiculously large… no higher than a standard phone width, around 400px"), and Hive's 400 by 150 "slightly too wide" | An image scroller is at most about 400 wide (merged sources use 300 to 400; Iken's are 340), with the height from the banners' measured ratio. Ask an image host that resizes (`?w=`) for twice the width. | F |
| Sana's stitched pages and Nyx, Vortex and Magus series typed "manga" opened in the paged reader | Open a site's "manga" series and long chapters before mapping types to viewers. Every Iken and VineTheme site's comics are long strips (VineTheme's "MANGA" chapters measured 800 to 900 wide and up to 16,000 tall), so they all open as Webtoon and only novels are Unknown. | R |
| The EternalMangas rewrite rode along in the Iken template PR ("completely separate… better put in a separate pr") | A source that left a template goes up in its own PR (section 1, one concern per PR). | R |
| Vortex's banner row showed three slides, one a cropped cover (owner's screenshots) | The slider mixes cover slides with banner art and renders most slides lazily, without an image or a title. Keep a slide only when the page's series data gives it a `banner`, take the image from that and the title from the link's label, and keep requesting twice the row width. Hive's 2048px script banners request 680px the same way. | R |
| Nyx home tags showed in lowercase (owner's screenshot) | Some sites capitalise tag names only with CSS (`span.capitalize`). Title-case a name that arrives all lowercase, leave cased ones alone, and deduplicate case-insensitively. Merged EternalMangas (#905) does not: it sorts and deduplicates tags as the site writes them. Our `tags()` helper was dropped from the archived copy, which keeps a two-line comment saying it never merged. | R |

### From #902 (`multi.mangaball`)

Merged on 2026-11-14. kkantan left one comment, on the listing names in `source.json` ("we
should match the titles from the site"), and pushed the change themselves as `chore: update
listing titles`, the only place `main` then differed from what merged. An earlier reading of
this PR, made from the commit alone, recorded "no review comments" and a rule about short
common names; the comment, read from the review API on 2026-11-17, says otherwise.

| What was changed | The rule | |
|---|---|---|
| The `source.json` listings "Latest Chapter Updates" and "Recently Added Titles" became "Latest Updates" and "Recently Added"; the home rows in `home.rs` kept the site's headings | A listing in `source.json` is named as the site titles that list. MangaBall's page titles are "Latest Updates" and "Recently Added"; VyManga's are "Latest Update" and "New Release", so those are its listings. Don't substitute a name that is merely common upstream. | N |
| The reviewer's commit sat on the PR branch and `main` kept the old names | After a merge, diff `upstream/main` against `main` for the source (ignoring the version, `test.rs` and `serde_json`) and copy back whatever the reviewer changed, as the #821 note says. | R |

The merged source is the standard for this kind of site. Its diff against the first version
published here (v6, `4c4a08a`) is 246 lines added and 82 removed. The rows above under "A safe
API mode", "A select default", "Excluded groups", "Sorting returned HTTP 200" and "MangaBall's
bold text" already record five of its changes; these are the rest, each read from that diff.

| What v6 did | What merged, and the rule | |
|---|---|---|
| A cleared language selection fell back to English (`.filter(\|l\| !l.is_empty())`), following XCOMIC and Kagane | Only an unset selection is English. A cleared one browses nothing: search and listings return `MangaPageResult::default()` and home returns an empty layout, before any request. This is the section 7 `None` against `Some(vec![])` rule; the two older sources are not the precedent. | R |
| A chapter name was dropped when it merely contained the number (`name.contains(&number.to_string())`), so "12 Days Later" lost its title on chapter 12 | Strip the site's own label prefixes ("Vol.N ", "Chapter ", "Ch.") and drop the name only when what remains parses to exactly the chapter number. Test numbered prose. | R |
| Home swallowed every section's error and reported "MangaBall didn't answer" | Keep the last real error and return it when no section loads, so the reader sees the status or the decode failure. | R |
| `From<Title> for Manga` with a required `name` | The API returns titles with a null name. Make the name optional, convert with `TryFrom`, drop the failing card from lists and home rows, and return the error on the details and pasted-link paths. | R |
| The page list failed only when it was empty | Fail it as well when any page url is blank, rather than hand the reader a hole. | R |
| Authors and tags were passed through as the API listed them | Trim author names, drop blanks and repeats, leave `authors` as `None` when none remain, and `dedup` the sorted tags. | R |
| The excluded-scanlator test sat in `lib.rs`, so the home chapter row ignored it | Put the group and language tests inside `into_chapter`, which both the chapter list and the home row call. | R |
| Seven comments, such as "The site's three content modes." and "Pairs each site language code with the app language that selects it." | They were cut; four short ones stayed, each naming an API or runner quirk. A comment that says what a well-named function does goes. | R |
| `pub` on `Title.id`, `Title.is18plus`, `ChapterItem.group` and `Group` | A field or type is `pub` only while another module reads it; narrow it again when the reader moves into `models.rs`. | R |
| `.iter().map(\|name\| name.trim().to_lowercase())` over an owned `Vec` from `defaults_get` | `.into_iter()` on a value that is consumed anyway. | R |
| No way to collapse several groups' uploads of one chapter | A Deduplicate Chapters switch, off by default: per volume, number and language keep the group with the most likes, then views, then the newest upload, and keep every part from that group. Unnumbered chapters always stay. | R |

### From #903 and #909 (other contributors, November)

Read on 2026-11-17 from the review comments API (`/repos/Aidoku-Community/sources/pulls/comments`,
newest first), which also shows comments the PR pages hide once resolved.

| What was flagged | The rule | |
|---|---|---|
| Filters read on a path that ignores them ("filters aren't used when searching", #903; "it doesn't look like the status filter is supported on this path", #909) | Probe each filter together with a query. When the site ignores filters while searching, set `hidesFiltersWhileSearching`; when one filter is ignored on one path, say so in a note filter. | R |
| `let query = query.trim();` ("the site doesn't appear to trim queries so I don't think the source needs to", #903) | Send the query as typed. | R |
| `attr(..).unwrap_or_default()` then used ("does it make sense to unwrap to an empty string here? it seems to me like it should be in a let chain", #903) | Skip the item in a let chain when an attribute it needs is missing. | R |
| An icon with rounded corners (#903) | Fully opaque, square, no corner radii of its own. | R |
| A status the site lists mapped to nothing, or two site statuses mapped to Completed (#903, #909) | Map every status the site's filter lists, each to its own `MangaStatus` where one fits. | R |

### From #908 (vinetheme template, Valir, Diva and Kayn)

Merged on 2026-11-16 with eight comments from kkantan (seven inline and the review's authors comment) and no commit of theirs. The template
and the five consumers on `main` matched the merged crates apart from tests and versions
(diffed on 2026-11-17), so nothing was copied back. They are on `merged-sources` (section 17).

| What was flagged | The rule | |
|---|---|---|
| Relative dates without the "mo" suffix ("looking at the valir scans home page as an example") | Collect every suffix the home pages actually print before writing the parser. The VineTheme sites print `m`, `h`, `d` up to 29 and then `mo`, never `w`; an unknown suffix leaves the date unset rather than guessing. | R |
| `value.trim()` in a helper only ever fed `text()` ("this is only called after an html `text` call") | The `text()` rule covers helpers too: don't trim inside a function whose every caller passes `text()` output. | N |
| `chapters.reverse()` before `into_iter()` ("reverse the iterator below before collecting it") | Reverse with `.rev()` on the iterator that is already being collected, not the vector beforehand. | R |
| `push` replacing `push_encoded` for `limit`, `page` and `contentMode` ("encoding the static values here is not necessary") | Keep `push_encoded` for values that never need escaping; `push`/`set` are for user input. | R |
| `if let Some(query) = query { qs.set("q", Some(query)) }` ("unwrapping `query` from the optional, then immediately re-wrapping it", #908; again on #905, where kkantan pushed the fix themselves as `refactor(es.eternalmangas): avoid optional rewrap`) | Pass the `Option` straight through when the callee takes one: `if query.is_some() { qs.push("search", query.as_deref()); }`. Eight merged sources still unwrap and re-wrap, so `check-sources.py` notes it. | N |
| A template update dropped Elf Toon's `"breakingChangeVersion": 3` ("this should not be removed") | Rewriting a consumer's `source.json` keeps every existing `config` key; readers below that version still need the migration prompt. | F |
| Chapter count as two text filters, "Chapters From" and "Chapters To" ("would it be better to use the range type filter, like the atsumaru source does") | A numeric bound pair is one `range` filter. Probe what the API does with 0: VineTheme answers `ch_max=0` with everything, so its range starts at 1. | R |
| Valir authors such as The Convenient Empress Resigns joined by `/` | Split credits on every separator the sites use, `/` included, and test a real example from the reviewer. | R |
| MangaTaro's comma split turned "Oda, Eiichiro" into two authors (our own miss, found checking #908's rule) | Read a page of real credits before choosing separators. Where the site writes names surname first with commas ("Kaneshiro, Muneyuki, Nomura, Yuusuke" is two people), a comma separates nothing reliably, so keep the credit whole as the site prints it. | R |

### From #910 (`en.mkissa`)

kkantan requested changes on 2026-11-18, read from the PR page, then approved the fix commit and
merged it on 2026-11-19 (`f0e2e49`) with one more comment, the last row.

| What was flagged | The rule | |
|---|---|---|
| `"urls": ["https://mkissa.to"]` beside the same `url` ("this is not necessary when `url` is provided") | List `urls` only for mirrors. | F |
| A local offset from parsing one date with `parse_date` and `parse_local_date` ("is this the same result as `get_utc_offset` from aidoku-rs? if you're already using chrono I'm a little confused why `parse_date` is being used") | Use `get_utc_offset()`: seconds of UTC minus local time, in AidokuRunner (`-secondsFromGMT`) and the test runner alike. Where chrono is already a dependency, build a timestamp from date parts with chrono rather than formatting a string for `parse_date`. | R |
| `escape_markdown` on details descriptions ("why are we escaping markdown? don't we want to display markdown in descriptions?") | Sample real descriptions before escaping. MKissa's carry Markdown (`**Links:**`, `[Official Twitter](…)`, `___`) and HTML `<b>`/`<i>`; escaping showed every marker. Keep the Markdown, turn bold and italic tags into it and keep line breaks hard; the big scroller still gets plain text. | R |
| "this source seems rather unreliable, with chapters taking quite a bit of time to load and failing a multiple times" | A web-view reader is judged in the app, where the reviewer reads it. Time and retry several chapters of several series on a device before submitting; the test runner stubs web views. | R |
| `apply_image_quality` appended `?w=` to a CDN path ("this function assumes the url doesn't already have query parameters, which is likely fine for the existing urls, but would be good to either handle for correctness or to make a note of", approval) | A helper that adds a parameter to a url it did not build joins with `&` when the url already has a `?`, or a comment says why it never does. MKissa now picks the separator and a `review_` test passes a url with a query. `check-sources.py` notes a `?` written straight after a url variable in `format!`; 10 merged sources do that, so it is a note. | N |

### From #911 (mangataro template)

kkantan approved and merged it on 2026-11-19 (`eb4f68c`) with one comment, read from the PR page.

| What was flagged | The rule | |
|---|---|---|
| `tags.sort()` before `tags.dedup()` ("stable sorting usually isn't necessary and is more expensive, for future reference", suggesting `tags.sort_unstable()`) | Sort with `sort_unstable` (`_by`, `_by_key`) unless equal elements must keep their order; then keep the stable sort and say why in a comment above it, as the toonz template does for equal ratings. Names that compare equal are identical, so a stable sort of strings buys nothing. `check-sources.py` notes a stable sort without such a comment; 27 merged sources still use one, so it is a note. | N |

### From #913 (`en.vymanga`) and #916 (`multi.yaoime`)

kkantan requested changes on both on 2026-11-20, with inline comments only.

| What was flagged | The rule | |
|---|---|---|
| Listing cards marked Completed from the catalogue badge ("this seems incorrect sometimes, for example with `Hyakujuu Kingdom` ... we should probably trust the website's series detail page") | Set a card's status only from data that agrees with the series page; when a listing badge can disagree, leave the card's status unset and let details fill it. | R |
| `excluded: Vec<String>` on a function that only reads it ("generally you should prefer not using owned values in function parameters ... it's just a best practice recommendation") | Take `&[T]` or `&str` for a parameter the function only reads; chain borrowed iterators rather than taking ownership. | R |
| `url` on a home `MangaChapterList` chapter ("I don't think there's any reason to set this, as it doesn't seem like the app uses it") | Leave `Chapter::url` unset on home chapter rows; the app opens them by key. | R |
| A `for` loop pushing to a mutable `Vec` with `let ... else { continue }` ("cleaner to collect ... and use a `filter_map` so that you can use the `?` operator") | Build a list with `filter_map` and `?` rather than a loop of `continue`s pushing to a `Vec`; dedup in the same pass with a set's `insert`, as `multi.mangadex` and `multi.mangaplus` do. | R |
| `parse_date` on an ISO timestamp ending in `Z` ("this parses in the user's time zone ... more correct to use `parse_date_with_options` with the time zone set to UTC") | A timestamp the site gives in UTC is parsed with `parse_date_with_options(date, "yyyy-MM-dd'T'HH:mm:ss.SSSXXX", "en_US_POSIX", "UTC")`, as `ja.rawdevart` and `ja.soraraw` do: `XXX` reads the `Z` as the zone rather than a literal. | R |
| A settings footer explaining "Official chapters only" at length, and deduplication not saying it drops extras ("a little long winded and confusing") | Keep setting footers to one short sentence per option, naming what the option removes. | R |

### From #923 (mangataro tag sort update)

kkantan approved and merged it on 2026-11-21 (`4878f1a`) with one comment, read from the PR page.

| What was flagged | The rule | |
|---|---|---|
| Version bumps on an update that only swapped `sort` for `sort_unstable` ("there shouldn't be any difference for users apart from a very tiny performance boost that is probably negligible, so incrementing the versions in cases like this isn't strictly necessary") | An upstream update with no user-visible change may keep its version; bump upstream when users would see a difference. Fork `main` still bumps every publishing push (section 0), since a push there is a release. | R |

### From #926 (`en.webnex`)

Merged on 2026-11-23 (`9171c4a`) with no reviewer commits pushed to the branch; archived to
`merged-sources` afterwards.

| What was flagged | The rule | |
|---|---|---|
| Popular This Week as a `BigScroller` with only cover, title and genres ("I'm not sure if I agree with making the popular section a big scroller without information like author and description, since it looks empty and doesn't match the website layout anyways") | Use a `BigScroller` only when the feed already gives each card a description or authors, or the site itself shows a hero. A row the site shows as plain cards stays a `Scroller`, and no extra request is made only to dress up a hero. | R |
| `Listing { id, name: title.clone().unwrap_or_default(), .. }` ("usually we don't want to `unwrap_or_default` strings in the data models, but this won't matter unless the site is updated and the source would need to be updated anyways", approval review) | Never `.unwrap_or_default()` required string fields (`Manga.key`/`title`, `Chapter.key`, `Listing.id`/`name`, `Link.title`) when constructing Aidoku data models; require them with `zip`/`?` or omit/fail the item instead. `check-sources.py` fails `.unwrap_or_default()` on those fields. | F |
| Same-group multiple uploads under `one_per_chapter` ("if the same group uploads multiple of the same chapter, wouldn't this keep all of them even with `one_per_chapter` enabled?") | Deduplicate per group per chapter row before ranking so one group's re-upload can't survive `one_per_chapter` twice, while still keeping distinct same-number parts from the winning group; verify against real series data. | R |

### From WebNex (`en.webnex`), checked line by line against merged sources

The owner asked that anything with a merged equivalent be done the merged way. Each row names
the merged source it follows, alongside the site lessons learnt while building it.

| What was flagged | The rule | |
|---|---|---|
| A home hero whose feed has no genres or synopsis, first replaced wholesale by a second list | When a feed omits featured metadata, make one lookup for the featured keys and fill each feed card from it by key, keeping the feed's own cards and order, as `en.atsumaru` does ("Feeds omit featured metadata"). If the lookup fails, keep the plain cards. Never one request per card, which `vi.cuutruyen` does. | R |
| A deep-link handler splitting on `"://"` so any host passed | Strip `BASE_URL` and return `None` otherwise, as 84 of 85 merged handlers do; the app only hands a source its own links. `check-sources.py` notes the `"://"` split (`es.olympusscanlation` still has it). | N |
| A pasted chapter link resolved through the deep-link handler, one extra request for the reader page | Resolve pasted series links locally and make one request, as `multi.mangaball` does; anything else is searched as text. | R |
| Chapter pages fetched with `step_by` and `min` arithmetic | Batch numbered pages with `.chunks(n)` over a list of page numbers, as `multi.xcomic` does. | R |
| Ratings that skip one collapsed Discover to one "Filtered Catalogue" row | When home only takes a rating ceiling, rebuild the rows the catalogue endpoint can reproduce with every chosen rating, one parallel request each, as `multi.mangadex` sends its ratings with every row's request; leave out rows it cannot reproduce. A row whose own type filter replaces the Types setting is skipped for a hidden type. | R |
| A site row shown as "Readers Favourites" while it held a personalised "Your taste" list | Find a row by its heading when the site reuses an id, and title a personal row as personal, beside the shared one, as `multi.mangadotnet` keeps Most Tracked beside For You. | R |
| A comment cut as obvious ("The series url has no query of its own") that answered a `check-sources.py` note | Before cutting a comment, check whether the checker asks for it; the note returns without it. | R |
| Empty React/Next.js streamed `S:` chunks broke placeholder expansion (`Element::html()` returns `None` for empty elements) | When splicing Next.js streaming chunks (`div[id^='S:']` into `template[id^='P:']`), treat `Element::html() == None` as an empty string rather than failing or dropping the shelf. | R |
| Inline JS reader cookie gate initially used a `WebView` + user script (`minAppVersion` 0.8.4) | Before reaching for a `WebView` on an inline script cookie gate, probe a plain request with `Accept: text/html` (or `Sec-Fetch-Mode: navigate` + `Sec-Fetch-Dest: document`): if the HTML returns the cookie script, parse the cookie directly and avoid `WebView` completely (lowering `minAppVersion` and avoiding iOS WebView flakiness). | R |
| Creator links combining native and romanized names with `/` (`오솟 / Osot`) | Sample real creator anchors before splitting on `/` or `,`. When each link's `href` searches the whole string and `/` joins native and romanized forms of one creator, keep credits whole and only deduplicate. | R |
| Long-strip Korean/Chinese series labeled `"Manga"` opened right-to-left | Sample chapter page dimensions on `"Manga"` entries; where a site labels long strips `"Manga"`, let strip tags (`Long Strip`, `Webtoon`) and Korean/Chinese origin override `Viewer::RightToLeft` to `Viewer::Webtoon` inside a dedicated `viewer()` helper. | R |
| Home "View all" rows linked to `/browse?...` pages | Carry the site's browse link as the `Listing` id (as VineTheme does) and parse its query parameters back through the browse filter path, failing on unknown parameters rather than widening the catalogue so the reader's settings still apply. | R |
| A home `Sections` multi-select setting | An unset `Sections` setting enables every section; a cleared selection returns an empty layout before issuing any network request. | R |

### From #878, #894, #909 and #912 (other contributors, November 18-19)

Read on 2026-11-20 from the review comments API; every review body was empty, so the inline
comments are the whole of it. Rows already in this section (status lowercasing, compact listing
conversions, `into_iter` over `clone`, `text()` trimming, `isGenre`, passing an `Option` through)
were asked for again on these PRs.

| What was flagged | The rule | |
|---|---|---|
| A hand-written `parse_timestamp` ("you could just use the aidoku-rs-provided `parse_date` or `parse_date_with_options` functions instead of reimplementing the behavior", #909) | Parse every date with `parse_date` or `parse_date_with_options` (locale, timezone); reshape the input string first if needed, never reimplement the parse. | R |
| `let key = manga.key.clone();` before overwriting it ("I think you could use `core::mem::take` instead of cloning", #909) | Move a field out with `core::mem::take` (or `Option::take`) when the original is about to be replaced; clone only what must stay. | R |
| `title.split_once('화')` to find a chapter number ("titles can contain the `화` character regularly, right? I think you probably want to split from the last occurrence", #894) | When the part you want sits at the end and the separator can appear in the text before it, split with `rsplit_once`. Scans.GG's "Name • Group" now does. | R |
| HTML scraping beside a JSON API ("why not use the api that appears to exist instead of scraping the html? I believe it is faster and doesn't require cloudflare verification, which I think is unreliable for page loading", #878) | Look for the site's own API before writing selectors, in its network requests and in `/api/` paths; use it for listings, details and pages when it exists. | R |
| `"url": "https://okutoon.com"` ("it looks like this redirects to https://okutoon.net now", #878) | `source.json`'s `url` is the site's current address: follow its redirects before submitting, and move the source to the new domain (with `breakingChangeVersion` only if keys change). | R |

### From #843 (`en.atsumaru`)

- **Filter exclusions:** every multi-select with `canExclude: true` must process both
  `included` and `excluded`. Genre and demographic exclusions are supported; Content Type,
  Status and Content Rating explicitly disable them. Test each enabled exclusion against the
  site, and check the filter declarations as well as the match arms.
- **Self-sufficient keys:** only manga and chapter keys can be relied on in later callbacks.
  Reader routing must not require URLs, titles, viewer hints or cached details. Encode a
  required discriminator in a key, use the same encoding in chapter lists and deep links,
  and test reading with fresh `Manga` and `Chapter` values containing only keys. Atsumaru
  prefixes novel series keys with `novel/`; chapter keys remain the site ids.
- **Rating defaults (inferred from this review; also used by merged MangaDex):** with
  source content rating `1`, Atsumaru defaults to Safe and Suggestive, leaving Erotica and
  Pornographic opt-in. Keep the Rust fallback and settings JSON defaults identical; do not
  overwrite the reader's explicit selection.
- **Submission dependencies:** removing fork-only tests also removes their unused
  `serde_json` dev dependency and root lockfile entry, along with every lockfile package only
  it needed (`en.vymanga` has no `json` feature, so `memchr` and `zmij` were left behind and
  cargo rewrote the lock during the push gates). Keep it on main while tests use it.
  `check-sources.py` notes unused declarations; preparation normalizes the same files when
  checking preserved snapshots and final source equality.
- **Tag order:** sort featured tags alphabetically, using the same order in details, so
  opening a featured card does not animate a needless tag reorder. Test shuffled input and
  preserve the detail-only tag weighting rules.
- `review_` tests cover the filter capability contract and chapter/deep-link key agreement
  without live requests. The Source tests workflow runs them in a separate step before the
  full live suite so a site outage does not obscure these regressions.

### From #850 (`en.mangak`)

- **Exclusions follow the code that sends them (kkantan, on #850 and again on #853):** every
  multi-select whose excluded values the code doesn't send states `"canExclude": false`, even
  though false is the default. The mangak template sends `excluded` for genres only; XCOMIC sends
  genres and formats (as `excGenres`) only. Probe the API's exclusion parameters before enabling
  one. `check-sources.py` notes a non-genre multi-select that leaves `canExclude` unstated.
- **Template first (inferred, kkantan):** a feature a template consumer adds goes in the
  template when it also applies to the template's other consumers. Per-site differences become
  `Params` fields, as merged templates do (`iken`'s `hide_home_lists`, `mangareader`'s
  `home_path`). A consumer whose behaviour changes (its sorts, its home) is bumped in the same
  PR; a model change that doesn't affect it needs no bump. A home page that can't be built
  generically goes in each source, with the template's left empty.
- Check the other consumer's site before moving anything: ToonTop and MangaK run the same API
  and search page, so every filter and the sorts were shared, while `demographic` and `format`
  match nothing on ToonTop and stayed MangaK's alone. Both homes share their rows except the
  hero (ToonTop's ranking's Top This Week, MangaK's Most Views) and the latest row's name, so
  only those are `Params`. MangaK's search lists fewer genres than its details, so its hero's
  tags can grow on open; the owner chose that over the trending feed's slug lookup. Read the owner's screenshots of both sites before settling a
  layout; the server-rendered data alone suggested a hero ToonTop doesn't show.

### From our own reviews of `en.atsumaru` (owner's corrections)

Each of these went to `main` and was put right in a later commit. **F** fails `check-sources.py`
or `check-workflows.py`; **N** prints a note (merged sources disagree, so it is a judgement
call: change it or say why not); **—** needs the site or the app, so it is read in review.

| What was flagged | The rule | |
|---|---|---|
| `SEARCH_URL`, `BASE_URL` plus a path and only ever followed by `?` | No merged source names one endpoint: format it inline, `format!("{BASE_URL}/search?{qs}")` (11 sources). `API_URL` for an API base is fine (20 sources). | F |
| `manga_page_url`, `chapters_url`, `read_url` over an `api_url` helper, each used once | API endpoints are formatted where they are requested, `format!("{API_URL}/chapter/{key}")` (`en.comix`, `multi.mangadex`, `en.toonkotsu`). Helpers are for site urls used in several places (`manga_url` 4 sources, `cover_url` 3, `chapter_url` 2). The used-once note counts calls, not imports. | N |
| Five Typesense parameters as constants, each sent once | Write a value used once where it is sent, `query.push("num_typos", Some("4,3,2,1,0"))`. | F |
| `FEED_LIMIT`, `SEARCH_LIMIT` for page sizes | `PAGE_SIZE` (9 sources), or `<THING>_PAGE_SIZE` for a second one; `RATINGS` is `CONTENT_RATINGS` (`multi.xcomic`). | N |
| `fn is_novel(&self) { is_novel(self.medium…) }` | A method that only passes its own fields to a free function is a wrapper; call the function. `ja.soraraw` keeps such accessors, so it is a note. | N |
| `aidoku::imports::std::current_date()` in an expression | Import it. kkantan's own commits write full paths too, so it is a note. | N |
| Deduplication discarded same-number parts from one scanlator | Select a winning group, keep all its entries for that number and every unnumbered chapter, and break equal rankings by chapter id. Test reversed inputs. | — |
| The novels row mapped search hits to cards a second way | Reuse the existing conversion (`into_page`) rather than restating it. | — |
| A comment on the fallback sort described the feeds | When code moves, re-read each comment beside it; a comment true of different code is stale. | — |
| Full `mediums`/`types` lists sent to the feeds | Probe what a missing parameter means, then send nothing for a full selection, except ratings where missing means safe-only (section 6). | — |
| Ranking periods defaulted to a week and applied to the home page | A setting's default is the site's own default for that control (Explore's All), and it applies where the site applies it. Read the site's code for the default. | — |
| `actions/cache@v4` (and `checkout@v4`, `upload-artifact@v4`, a pages action) on Node 20 | `check-workflows.py` reads each action's runtime at its pinned ref; move to a release on a supported Node. **Action runtimes** checks weekly. | F |
| `aidoku-test-runner` install failed on GitHub's runner | It links fontconfig and freetype; install `libfontconfig-dev libfreetype-dev` first. `check-workflows.py` fails a workflow that doesn't. | F |
| WebNex's `src/reader.js` user script ("no merged source has a javascript file") | No merged source ships a script file: `en.toonkotsu`, `en.comix` and `en.mkissa` pass short inline JavaScript to `eval`. First try plain requests: a gate that writes its cookie from a script often does so for any page request (`Accept: text/html`), which WebNex's reader does, and needs no web view at all. | F |

## Product rules

Commercial price, sale, discount and purchasing controls are omitted from source forms,
filters and settings. Chapter visibility controls remain useful: Show Locked Chapters
uses `showLocked`, defaults off and only appears where chapter lock metadata exists.
Mixed comic/novel catalogues use `labelNovels`, default on, to append `(Novel)` without
changing keys or reader routing. Eternal has novels but no locked chapters; its novel
catalogue form uses `type=novel`.
