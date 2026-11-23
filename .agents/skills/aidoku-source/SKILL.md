---
name: aidoku-source
description: Create or change an Aidoku source or template in this repo. Use when adding a site as a source, adding a filter, home row, parser fix or template consumer, probing a site's API, or verifying source code before any push.
model: inherit
---

# Building Aidoku sources

`AGENTS.md` (identical to `CLAUDE.md`) is the hub: hard rules, upstream rules, workflow and
templates. The detail lives beside this skill. This skill is the order to apply it in. Work in
phases; each depends on facts the one before established.
The physical `.agents/skills/aidoku-source` copy must match this tree; run
`python3 scripts/sync-agent-docs.py` after editing and `--check` before committing.

| Read | When |
|---|---|
| [standards.md](references/standards.md) | before every push: every review rule with its evidence (AGENTS.md / CLAUDE.md section 3) |
| [conventions.md](references/conventions.md) | before writing code: API in practice, request cost, merged conventions, framework defaults, comments and naming (sections 2, 4 to 8) |
| [aidoku-host.md](references/aidoku-host.md) | for anything about what the app does with a source |
| [console-probes.md](references/console-probes.md) | when the sandbox cannot reach a site |
| [testing.md](references/testing.md) | before writing or running tests |
| [source-review.md](references/source-review.md) | official references, reviewer permalinks and the test matrix |
| [submission.md](references/submission.md) | every gate, and how a source goes upstream (section 9) |

Before coding, declare the destination: fork publication (`main`) or upstream submission
(`feat/<id>`/`fix/<id>` from canonical upstream). Never create a submission by branching
from the fork's main. Keep the tooling checkout available while working in clean worktrees.

## Phase 1 — Recon

Never write a request or selector from memory or from another client's code. Confirm each
against the live site first.

1. **Look for a template.** If the matching keiyoushi extension declares a `themePkg`, or
   the site is a known WordPress theme, the source is a template consumer (AGENTS.md / CLAUDE.md §12).
   Otherwise find out how the site is built (server HTML, Next.js, a JSON API behind a
   client-rendered app, GraphQL) and read the merged sources built the same way (AGENTS.md / CLAUDE.md §6).
2. **Read the reference clients** for endpoints and request shapes — keiyoushi first. Treat
   them as leads, not evidence: a reference may send parameters the endpoint ignores, and
   its comments may be copies of ours.
3. **Probe the site's own console.** When the API is undocumented, have the person run
   snippets from `references/console-probes.md` in the site's DevTools console and paste
   the output. That is how every Toonkotsu question was settled — status values, sort ids,
   which parameters each endpoint honours, which value matches nothing.
4. **Know what the app will do with it.** `references/aidoku-host.md` records how the host
   loads and calls a source, its JSON defaults and what each home component shows, citing
   AidokuRunner, the Aidoku app and aidoku-rs. Settle "is that the default?" there.
5. **Write down before coding:** the URL for home sections, search, details, chapter list
   and pages; every filter the site offers and the parameter it sends; the pagination
   signal; what a cleared selection sends.

Port everything the site offers: every filter, every sort, every home row it shows.

## Phase 2 — Scaffold

```sh
scripts/new-source.sh <id> <Name> <https://site> [safe|contains-nsfw|primarily-nsfw] [languages]
```

It runs `aidoku init`, removes the nested repository it makes, and prints the README row.
Replace `res/icon.png` (128×128, opaque, margin around the mark).

For completion and checking while editing `res/*.json`, point the editor at the CLI's schemas,
`crates/cli/src/supporting/schema/{source,filters,settings}.schema.json` in aidoku-rs (the
aidoku-rs book, "Resource Files"). `aidoku verify` checks the same schemas on the package.

## Phase 3 — Implement

**Before writing any feature or fix — even when adapting behavior from an unmerged fork branch
— check how the newest merged upstream sources built the same way do it:**

```sh
python3 scripts/upstream-usage.py <id>                  # each feature this source has, and who upstream does it
python3 scripts/upstream-usage.py --feature '<regex>'   # who upstream does what you are about to add
```

Both list merged sources newest-changed first, with the file and line. Read the newest one or
two built the same way (Next.js, HTML, JSON API, or template; recent rewrites are what review
currently accepts) and match their shape: module placement, naming, fetch helper,
viewer/status/rating helpers, deduplication, date parsing, and home rows. Never copy an unmerged
fork branch's inline layout without checking merged upstream first. Diverge only for a reason
the site forces, and say it in the PR. The same report ends `scripts/review-source.sh`.

Follow AGENTS.md / CLAUDE.md §2 and §6 for layout. Checklist, each item a real review comment or a bug
that shipped:

- [ ] Every card carries a nonempty key and title, cover (and status/rating) only: search results,
      listings, a pasted-link result, and the `Scroller`, `MangaList` and `MangaChapterList`
      rows, which are `Link::from(compact Manga)` with no subtitle. Only `BigScroller` shows
      details (AGENTS.md / CLAUDE.md §3)
- [ ] `MangaChapterList` only when the feed names a real chapter
- [ ] A filter on the same axis as a setting overrides it for that search, never repeats it.
      The default select value is omitted by the app; keep actual override values selectable.
- [ ] No dead code: nothing silenced with `allow`/`expect`, no sort the API already does, no
      field or helper left over from removed behaviour (AGENTS.md / CLAUDE.md §8)
- [ ] `src/test.rs` stays on `main`; `pr-branch.sh` keeps it and its unused `serde_json`
      dev dependency out of the submission, including the root lockfile entry. Keep standard
      aidoku/aidoku-test development dependencies even when no test files remain
- [ ] `status`, `content_rating`, `viewer` set on details when the site has them
- [ ] Synopsis parsing covers short and expanded layouts plus missing/empty text. Credit lists
      split verified separators, trim/omit blanks and deduplicate within each role in site order.
- [ ] Author/artist selectors cannot match comments or descriptions; author search needs useful
      author data, not only an empty result for an impossible name. A site with mislabeled types
      may opt into an optional viewer override while other consumers retain their mapping.
- [ ] Novel emphasis stays balanced across adjacent nested spans, `<br>` and paragraph boundaries; escape Markdown
      syntax rather than ordinary prose punctuation. Shared `child_nodes` imports require
      minimum app 0.8.3 for every dependent source, not only sites currently listing novels.
- [ ] `send_partial_result` only when chapters need a second request
- [ ] `DeepLinkHandler` for series **and** chapter urls; series and chapter urls resolve
- [ ] Reader routing works with only manga/chapter keys; encode required type flags in keys,
      and ensure search, Discover, details, chapter-list and deep-link keys match, including URL slash/query variants; strip source
      prefixes before sending ids to the site
- [ ] Every enabled multi-select exclusion handles `excluded`; state `canExclude: false` on
      every other multi-select (kkantan asked twice, though false is the default), including
      a template consumer's filters the template doesn't exclude; test supported exclusions
      against the site
- [ ] Every site filter present; dynamic genres → `supportsTagSearch` plus a
      `FilterValue::Select { id: "genre" }` arm; use `supportsAuthorSearch` only without
      an explicit author text filter (likewise for artists)
- [ ] No `language` on chapters of a single-language source
- [ ] Toggles and other secondary filters set `"hideFromHeader": true`
- [ ] The sort filter comes first, untitled unless the site labels it
- [ ] Every serde rename and every query parameter checked against a captured response; a
      parameter the endpoint ignores is removed. Decode responses for every sort direction through
      the source, covering observed scalar/list variants and incomplete listing records.
- [ ] With `contentRating` 1, an adult-hiding switch defaults to hidden (or rate the source 2)
- [ ] Home sections in the site's order; banners as `LinkValue::Manga` with sizes set when fixed
- [ ] Every home feed honours the same settings as browse — check what the feed endpoint
      accepts before filtering locally
- [ ] Home components `page_size` ≤ 5; one request per section; no per-card fan-out
- [ ] Settings read only in `settings.rs`; a cleared multi-select means "nothing", not "all"
- [ ] Chapter deduplication removes excluded groups first, honors ordered group preferences,
      and preserves the winning group's same-number parts and unnumbered chapters; equal
      scores have a stable tie-breaker, tested with reversed input
- [ ] Moving an axis between settings and filters renames the setting key
- [ ] `bail!` over `Err(error!())`; consume owned values instead of cloning
- [ ] JSON types and their `From` impls in `models.rs`, free functions in `helpers.rs`
- [ ] `.json_owned()` rather than `.string()` plus `serde_json::from_str`; `parse_date`
      rather than a hand-written parser; no `JsContext` where a Rust crate does the job
- [ ] An Option goes straight to a callee that takes one (`if query.is_some() { qs.push("q", query.as_deref()) }`); no `if let Some(q) = q { push(.., Some(&q)) }`
- [ ] No empty-query test, no neither-flag guard, no `!included.is_empty()` in a
      multi-select arm, no trait registered just to return `Unimplemented`
- [ ] A per-page failure fails the page list or leaves the page unprocessed, never drops it
- [ ] Regexes compiled once in a `static`; fixed sets as arrays
- [ ] `sort_unstable` (`_by`, `_by_key`) unless ties must keep their order, and then a comment
      above the stable sort says so (kkantan on #911)
- [ ] A helper that adds a query parameter to a url it did not build joins with `&` when the url
      already has a `?`, or a comment says why it never does (kkantan on #910)
- [ ] A rebranded site keeps its id and sets `breakingChangeVersion` in `config`
- [ ] Separate Rust items and methods with blank lines; rustfmt alone does not insert them
- [ ] Preserve isGenre when rewriting genre options, IDs or select types
- [ ] Comments: terse, why only, no other sources or third parties named, no dividers
- [ ] Constants as merged sources name them: `BASE_URL`/`API_URL`/`CDN_URL`, `PAGE_SIZE`; no
      endpoint constants and no constant for a value used once (AGENTS.md / CLAUDE.md §8)
- [ ] No method that only passes its own fields on, no full-path calls, no second copy of a
      conversion that already exists
- [ ] Each setting's default is the site's own default for that control, read from its code,
      unless review requires a narrower source-rating policy; keep Rust/JSON defaults aligned
- [ ] Featured and detail tags use the same alphabetical order; test shuffled API input
- [ ] A full selection sends nothing, unless a missing value means less (ratings, §6) — probe it
- [ ] An unset language selection takes the default; a cleared one browses nothing, with no
      request (search, listings and home)
- [ ] A chapter name is dropped only when, without the site's label prefix, it parses to the
      chapter number; numbered prose keeps its title
- [ ] A name the API can leave null is optional and converted with `TryFrom`; lists and home
      rows drop that card, details and pasted links return the error
- [ ] Never `.unwrap_or_default()` required string fields (`Manga.key`/`title`, `Chapter.key`,
      `Listing.id`/`name`, `Link.title`) in Aidoku data models; require them with `zip`/`?` or
      omit/fail the item (kkantan on #926)
- [ ] Before using a `WebView` for an inline JS cookie gate, probe whether a plain request with
      `Accept: text/html` returns the script; when splicing Next.js `S:` streaming chunks, treat
      `Element::html() == None` as empty string rather than failing
- [ ] Home returns the last real error when no section loads, not a generic message
- [ ] Group and language rules live in the chapter conversion, so home rows and chapter lists
      apply the same ones
- [ ] Listing names in `source.json` are the site's own titles for those lists
- [ ] After a merge, diff `upstream/main` against `main` and copy back the reviewer's own commits
- [ ] `src/test.rs` with `#[aidoku_test]` cases checked against the site's data (AGENTS.md / CLAUDE.md §1)

## Phase 4 — Verify

First number each thing the person asked for in this task, from their messages, and name beside
it the code, test or output that shows it (AGENTS.md / CLAUDE.md section 0). An item with nothing
beside it isn't done; an order or layout they specified gets a `review_` test.

Run `scripts/review-source.sh <id>` before calling any source work done. It runs every step
below, prints the section 3 checklist to answer row by row, and stamps the result. The configured
Claude Code and Codex Stop hooks refuse to finish until a passing stamp matches the source's current files.
Codex local hooks require a trusted project and supported runtime; run the gates yourself
where command hooks are unavailable, including cloud orchestration. The individual commands follow.

From the source directory:

```sh
cargo fmt --all
cargo clippy --target wasm32-unknown-unknown -- -D warnings   # never --all-targets
RUSTFLAGS=-Dwarnings aidoku package && aidoku verify package.aix
```

`scripts/verify-source.sh <dir>` runs these with the extra dead-code lints from
`scripts/source-lints.sh`, and lints the test module too; use it rather than the bare commands.

These cover the Rust/package baseline (AGENTS.md / CLAUDE.md §1). Also run the independent scope
check and behavioral tests; compilation alone cannot establish upstream readiness.

Then from the repo root: `scripts/check-sources.py <id>`, and `scripts/upstream-usage.py <id>`
to list what the source uses or names that merged sources don't; replace each with the usual
equivalent it prints unless the site needs it (AGENTS.md / CLAUDE.md §9). Act on every `NOTE`
from `check-sources.py`, or say in the summary why it stays. After a workflow change, run
`scripts/check-workflows.py`.

Then read AGENTS.md / CLAUDE.md §3 top to bottom against the diff, one row at a time, and say which rows
you checked. Passing the scripts isn't the same as following the rules: rows showing
"Manhwa · ★ 8.3" went to `main` with every gate green.

Run `python3 /path/to/main/scripts/review-contracts.py --repo . --crate sources/<id>`
for Keyoapp/MangaThemesia/VineTheme consumers, or `--base <upstream-base>` for a template change.
These public-API contracts run even when the submission excludes fork tests. Never skip a
failure because the template was archived; the candidate is what must pass. Check the
coverage and manual-evidence boundaries in `docs/operations/source-checks.md` on main.

Run deterministic `review_` regressions separately (`cargo test review_`), including key-only
reader inputs and filter capabilities, then the full suite.

Run the tests in the source directory (`cargo test`, after installing `aidoku-test-runner`
once) and lint them with `cargo clippy --tests --target wasm32-unknown-unknown -- -D
warnings`. After a push to `main`, **Source tests** (`test.yaml`) runs them again.

`scripts/install-hooks.sh` once from the main tooling checkout keeps the hook available
in upstream worktrees. It validates pushed submission refs against upstream before running
the source gates. Do not bypass it because the destination tree lacks fork scripts.

This sandbox usually cannot reach the sites. Say so rather than calling a change verified,
and ask for a console probe or an in-app check when behaviour matters.

## Next

Publishing, upstream PRs and review rounds: the aidoku-submit skill. Moving a merged
source off main: the aidoku-archive skill.
