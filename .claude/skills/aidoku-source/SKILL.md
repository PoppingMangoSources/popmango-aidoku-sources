---
name: aidoku-source
description: Create, fix or submit an Aidoku source in this repo. Use when adding a site as a source, changing an existing one, preparing an upstream Aidoku-Community PR, or answering review comments on one.
---

# Building Aidoku sources

`AGENTS.md` and the identical full `CLAUDE.md` holds the rules and why they exist. This skill is the
order to apply them in. Work in phases; each depends on facts the one before established.
The physical `.agents/skills/aidoku-source` copy must match this tree; run
`python3 scripts/sync-agent-docs.py` after editing and `--check` before committing.
Read [source-review.md](references/source-review.md) for official references and the test matrix.

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

## Phase 3 — Implement

Follow AGENTS.md / CLAUDE.md §2 and §6 for layout. Checklist, each item a real review comment or a bug
that shipped:

- [ ] Every card carries key, title, cover (and status/rating) only: search results,
      listings, a pasted-link result, and the `Scroller`, `MangaList` and `MangaChapterList`
      rows, which are `Link::from(compact Manga)` with no subtitle. Only `BigScroller` shows
      details (AGENTS.md / CLAUDE.md §3)
- [ ] `MangaChapterList` only when the feed names a real chapter
- [ ] A filter on the same axis as a setting overrides it for that search, never repeats it
- [ ] No dead code: nothing silenced with `allow`/`expect`, no sort the API already does, no
      field or helper left over from removed behaviour (AGENTS.md / CLAUDE.md §8)
- [ ] `src/test.rs` stays on `main`; `pr-branch.sh` keeps it out of the submission
- [ ] `status`, `content_rating`, `viewer` set on details when the site has them
- [ ] `send_partial_result` only when chapters need a second request
- [ ] `DeepLinkHandler` for series **and** chapter urls; series and chapter urls resolve
- [ ] Reader routing works with only manga/chapter keys; encode required type flags in keys,
      and ensure search, Discover, details, chapter-list and deep-link keys match; strip source
      prefixes before sending ids to the site
- [ ] Every enabled multi-select exclusion handles `excluded`; disable unsupported ones
      with `canExclude: false` and test supported exclusions against the site
- [ ] Every site filter present; dynamic genres → `supportsTagSearch` plus a
      `FilterValue::Select { id: "genre" }` arm; use `supportsAuthorSearch` only without
      an explicit author text filter (likewise for artists)
- [ ] No `language` on chapters of a single-language source
- [ ] Toggles and other secondary filters set `"hideFromHeader": true`
- [ ] Every home feed honours the same settings as browse — check what the feed endpoint
      accepts before filtering locally
- [ ] Home components `page_size` ≤ 5; one request per section; no per-card fan-out
- [ ] Settings read only in `settings.rs`; a cleared multi-select means "nothing", not "all"
- [ ] Chapter deduplication preserves the winning group's same-number parts and unnumbered
      chapters; equal scores have a stable tie-breaker, tested with reversed input
- [ ] Moving an axis between settings and filters renames the setting key
- [ ] `bail!` over `Err(error!())`; consume owned values instead of cloning
- [ ] JSON types and their `From` impls in `models.rs`, free functions in `helpers.rs`
- [ ] `.json_owned()` rather than `.string()` plus `serde_json::from_str`; `parse_date`
      rather than a hand-written parser; no `JsContext` where a Rust crate does the job
- [ ] No empty-query test, no neither-flag guard, no `!included.is_empty()` in a
      multi-select arm, no trait registered just to return `Unimplemented`
- [ ] A per-page failure fails the page list or leaves the page unprocessed, never drops it
- [ ] Regexes compiled once in a `static`; fixed sets as arrays
- [ ] A rebranded site keeps its id and sets `breakingChangeVersion` in `config`
- [ ] Comments: terse, why only, no other sources or third parties named, no dividers
- [ ] Constants as merged sources name them: `BASE_URL`/`API_URL`/`CDN_URL`, `PAGE_SIZE`; no
      endpoint constants and no constant for a value used once (AGENTS.md / CLAUDE.md §8)
- [ ] No method that only passes its own fields on, no full-path calls, no second copy of a
      conversion that already exists
- [ ] Each setting's default is the site's own default for that control, read from its code
- [ ] A full selection sends nothing, unless a missing value means less (ratings, §6) — probe it
- [ ] `src/test.rs` with `#[aidoku_test]` cases checked against the site's data (AGENTS.md / CLAUDE.md §1)

## Phase 4 — Verify

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

## Phase 5 — Release on main

Every push that changes a source publishes it, so every such push bumps it:

```sh
scripts/bump.sh <id>
```

Commit as Popmango Extensions, unsigned, conventional subject, a one- or two-sentence body,
no AI attribution (AGENTS.md / CLAUDE.md §9).

## Phase 6 — Upstream PR

```sh
scripts/pr-branch.sh <id>            # feat/<id> from upstream/main, one "feat: add <id>" commit
scripts/pr-branch.sh <id> --push     # same, then push with a lease
# Use --from <committed-ref> when the source snapshot is on another branch.
```

The script copies the source from the selected committed snapshot (`HEAD` by default), sets `version` to 1, runs the gates, and refuses
if the branch touches anything outside `sources/<id>` or differs from the snapshot beyond the
version. It validates both the final diff and the commit history against upstream.
Before a manual push, fetch canonical upstream and run:

```sh
python3 /path/to/tooling/scripts/check-upstream-scope.py --repo . --source <id>
```

If a branch is contaminated, preserve it and use `--rebuild --from <saved-ref>`. Never
cherry-pick a commit that includes README, workflow or unrelated source changes. Keep
the backup until the reconstructed source has been tested and the person no longer needs it. The person opens the PR; do not open it unless asked.

Before opening it, reread the reviews AGENTS.md / CLAUDE.md §13 lists and predict kkantan's comments.

## Phase 7 — Review rounds

For each comment: fix it on `main` (with a bump), then add one commit to the PR branch:

```sh
scripts/pr-branch.sh <id> --fix "fix(<id>): address review feedback" "<one or two sentences>" --push
```

Draft a one- or two-sentence reply per comment for the person to post; do not post on the
PR yourself.

Every comment, and every correction the person makes, also becomes a rule: a row in AGENTS.md / CLAUDE.md
§3, decided from the implementing code, plus a `check-sources.py` rule with a test when it can
be detected, calibrated with `scripts/calibrate-rules.sh` (AGENTS.md / CLAUDE.md §9, "How a rule gets
into this file"). Fix the same pattern everywhere in the source before pushing, not just the
line the reviewer pointed at.

After the PR is merged, later changes go up from the same branch name as an update:

```sh
scripts/pr-branch.sh <id> --update "feat(<id>): <what>" "<one or two sentences>" --push
```

It restarts the branch from `upstream/main` and sets the version to upstream's plus one.
Select the intended committed source with `--from <ref>` when it is not on `main`. Sources that live only upstream (`multi.kagane`,
`en.toonkotsu`) get a branch cut straight from `upstream/main` instead, with upstream's
version plus one and the same gates. When a comment asks "does X not support Y", answer it with a console probe, not
from a reference client.


## Phase 8 — Review evidence

Record the upstream base SHA, source snapshot SHA, prepared head SHA, changed-path scope,
fmt/Clippy/package/verify results, runner test results and remaining in-app checks. Check
pagination with a second page, filters with a value that should match nothing, each content
rating independently, missing/locked chapters, pasted URLs, deep links and page-list errors.
Count requests per operation and identify any per-item fan-out. Consult the reference's
reviewer evidence, then reread the final diff and every prior review thread before resubmission.
Do not claim a live test passed when network access or WebView support prevented execution.
