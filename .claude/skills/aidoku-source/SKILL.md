---
name: aidoku-source
description: Create, fix or submit an Aidoku source in this repo. Use when adding a site as a source, changing an existing one, preparing an upstream Aidoku-Community PR, or answering review comments on one.
---

# Building Aidoku sources

`AGENTS.md` (identical to `CLAUDE.md`) holds the rules and why they exist. This skill is the
order to apply them in. Work in phases; each depends on facts the one before established.

## Phase 1 — Recon

Never write a request or selector from memory or from another client's code. Confirm each
against the live site first.

1. **Look for a template.** If the matching keiyoushi extension declares a `themePkg`, or
   the site is a known WordPress theme, the source is a template consumer (AGENTS.md §12).
2. **Read the reference clients** for endpoints and request shapes — keiyoushi first. Treat
   them as leads, not evidence: a reference may send parameters the endpoint ignores, and
   its comments may be copies of ours.
3. **Probe the site's own console.** When the API is undocumented, have the person run
   snippets from `references/console-probes.md` in the site's DevTools console and paste
   the output. That is how every Toonkotsu question was settled — status values, sort ids,
   which parameters each endpoint honours, which value matches nothing.
4. **Write down before coding:** the URL for home sections, search, details, chapter list
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

Follow AGENTS.md §2 and §6 for layout. Checklist, each item a real review comment or a bug
that shipped:

- [ ] Search and listing cards carry key, title, cover (and status/rating) only
- [ ] `status`, `content_rating`, `viewer` set on details when the site has them
- [ ] `send_partial_result` only when chapters need a second request
- [ ] `DeepLinkHandler` for series **and** chapter urls; series and chapter urls resolve
- [ ] Every site filter present; dynamic genres → `supportsTagSearch` plus a
      `FilterValue::Select { id: "genre" }` arm; an author filter → `supportsAuthorSearch`
- [ ] No `language` on chapters of a single-language source
- [ ] Every home feed honours the same settings as browse — check what the feed endpoint
      accepts before filtering locally
- [ ] Home components `page_size` ≤ 5; one request per section; no per-card fan-out
- [ ] Settings read only in `settings.rs`; a cleared multi-select means "nothing", not "all"
- [ ] Moving an axis between settings and filters renames the setting key
- [ ] `bail!` over `Err(error!())`; consume owned values instead of cloning
- [ ] Comments: terse, why only, no other sources or third parties named, no dividers

## Phase 4 — Verify

From the source directory:

```sh
cargo fmt --all
cargo clippy --release --target wasm32-unknown-unknown   # never --all-targets
aidoku package && aidoku verify package.aix
```

Then from the repo root: `scripts/check-sources.py <id>`.

`git config core.hooksPath .githooks` once per clone makes `git push` run all of this for
every source the push changes.

This sandbox usually cannot reach the sites. Say so rather than calling a change verified,
and ask for a console probe or an in-app check when behaviour matters.

## Phase 5 — Release on main

Every push that changes a source publishes it, so every such push bumps it:

```sh
scripts/bump.sh <id>
```

Commit as Popmango Extensions, unsigned, conventional subject, a one- or two-sentence body,
no AI attribution (AGENTS.md §9).

## Phase 6 — Upstream PR

```sh
scripts/pr-branch.sh <id>            # feat/<id> from upstream/main, one "feat: add <id>" commit
scripts/pr-branch.sh <id> --push     # same, then push with a lease
```

The script copies the source from `main`, sets `version` to 1, runs the gates, and refuses
if the branch touches anything outside `sources/<id>` or differs from main beyond the
version. The person opens the PR; do not open it unless asked.

Before opening it, reread the reviews AGENTS.md §13 lists and predict kkantan's comments.

## Phase 7 — Review rounds

For each comment: fix it on `main` (with a bump), then add one commit to the PR branch:

```sh
scripts/pr-branch.sh <id> --fix "fix(<id>): address review feedback" "<one or two sentences>" --push
```

Draft a one- or two-sentence reply per comment for the person to post; do not post on the
PR yourself.

After the PR is merged, later changes go up from the same branch name as an update:

```sh
scripts/pr-branch.sh <id> --update "feat(<id>): <what>" "<one or two sentences>" --push
```

It restarts the branch from `upstream/main` and sets the version to upstream's plus one. When a comment asks "does X not support Y", answer it with a console probe, not
from a reference client.
