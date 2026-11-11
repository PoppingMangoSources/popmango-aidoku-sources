---
title: Source checks (check-sources.py)
type: operation
created: 2026-10-31
updated: 2026-11-11
sources:
  - code: scripts/check-sources.py
  - code: scripts/test-http.py
  - code: scripts/review-contracts.py
  - code: scripts/review-contracts/keyoapp.rs
  - code: scripts/review-contracts/mangathemesia.rs
  - code: scripts/review-contracts/fixtures.py
  - code: scripts/tests/test_review_rules.py
  - code: scripts/tests/test_review_contracts.py
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/81e42e5312cda0aed39d174c3dcf3b2797f858c0/templates/keyoapp/src/test.rs
  - code: templates/mangathemesia/src/test.rs
  - code: scripts/tests/test_check_sources.py
  - code: scripts/calibrate-rules.sh
  - code: .github/workflows/check.yaml
  - code: .githooks/pre-push
  - code: scripts/pr-branch.sh
  - code: scripts/review-source.sh
  - code: scripts/agent-hooks.py
  - code: .codex/hooks.json
  - code: scripts/docs-lint.py
  - code: scripts/sync-agent-docs.py
  - code: scripts/tests/test_agent_docs.py
  - code: scripts/tests/test_agent_hooks.py
  - code: .claude/settings.json
  - code: scripts/check-commits.py
  - code: scripts/ci-gates.py
  - code: scripts/tests/test_commit_enforcement.py
  - code: .github/workflows/build.yaml
  - code: .github/workflows/test.yaml
  - code: scripts/tests/test_template_tests.py
  - code: scripts/commit.sh
  - code: scripts/check-archives.py
  - code: scripts/test-sources.py
  - code: scripts/tests/test_archives.py
  - code: scripts/tests/test_live_test_report.py
  - code: scripts/tests/test_archive_gates.py
verified_at: 2184823
---

`scripts/check-sources.py` checks the source rules from the root `CLAUDE.md` / `AGENTS.md`
that need no network and no toolchain. It is the third gate after `cargo fmt`/`clippy` and
`aidoku package`/`verify`; the rules themselves live in the root file, not here.

## Automatic review contracts

`python3 scripts/review-contracts.py --repo <candidate> --crate sources/<id>` selects the
candidate's template through Cargo path dependencies. `--template keyoapp`,
`--template mangathemesia` and `--base <ref>` support explicit/template reviews. Without
selection arguments, it tests every supported template present in the candidate checkout.
It builds a temporary harness using the candidate's pinned SDK and public APIs, serves
synthetic pages on loopback and preserves nonzero compiler/runner/assertion exits.
`scripts/tests/test_review_contracts.py` verifies selection, error propagation and that
candidate source files remain untouched. The installed push hook, review command and
Source tests workflow invoke it. Contract edits also run against fresh merged upstream in
CI, so Keyoapp coverage remains active after its fork sources were archived.

| Lesson | Automatic enforcement |
|---|---|
| Short descriptions need no expand control | Runtime: short, expanded, empty and missing synopsis. Static: reject the known expanded-only assignment. |
| Credits need separate names and independent roles | Runtime: comma/ampersand split, trim, omit blanks, preserve order, deduplicate per role, V2 byline/block fallback. Static: reject the old comma-space-only helper. |
| Required title/key cannot be blank | Runtime: V1 title and V2 title/slug errors; home cards reject blank identity. Existing static conversion checks remain. |
| Newly Added must use its own grid | Runtime: own/adjacent grid accepted, unrelated grid rejected, V1 three-ancestor boundary retained. |
| Trending Today differs from Most Reviewed Month | Runtime: distinct default periods and monthly fallback request. |
| Rage metadata must exclude comment prose | Source-specific static guard: both selectors stay in metadata rows, author search stays disabled. |
| Mislabeled Rage webtoons need a local override | Static guard for its opt-in; runtime verifies optional override and unchanged template defaults. |
| Reviewed filters only include values | Static guard rejects enabled exclusions for Keyoapp and the reviewed MangaThemesia consumers. Unrelated sites retain their own capabilities. |
| Novel paragraphs/emphasis and key-only reading | Runtime: real page-list APIs with no cached manga data, balanced adjacent/nested emphasis and line breaks; V2 text/image order. |
| Compact internal banners and real Latest chapters | Runtime: internal banner key, dimensions, compact data, locked-chapter omission and five-row pagination. |
| Deep-link keys must be canonical | Runtime: tracking parameters/fragments removed and catalogue-only breadcrumbs skipped. |
| Shared imports affect every package minimum | Changed templates select every consumer for existing package/SDK verification and fork version-bump checks. |
| Madara artwork needs a suitable opaque background | Known rejected white asset fails by hash; reviewed asset passes; replacement artwork gets a visual-review note. Existing SDK package verification checks opacity. |
| Hentai catalogue rating differs from individual title rating | Existing static Hentai-genre minimum and hidden-adult-default checks remain. No blanket rating increase for adult-related words. |

> ⚠️ These contracts establish behavior on captured layout shapes, not current site support
> or app rendering. New separators, exclusions, ratings, branding and changed markup need
> fresh evidence and updated tests. Loopback tests do not establish WebView/cookie parity.
> Minimum app versions are checked on built packages; do not infer linked imports solely
> from Rust text when feature gating could remove them.

## Archive and live-test evidence

`scripts/check-archives.py --base <before-removal>` finds deleted source manifests and
requires an exact tracked snapshot of each source, its tests, icon and recursive Cargo path
dependencies on `origin/merged-sources` or `origin/dead-sources`. It compares blob hashes and
file modes, including template tests. Both `ci-gates.py` and the installed push hook run it.
The archive must be published before main removes the source. Full-history Actions checkouts
include the archive branch; a missing archive ref fails preservation checks for a removal.
`scripts/tests/test_archives.py` exercises complete and incomplete snapshots and stale templates.

> ⚠️ Archive refs are historical snapshots, not publication branches. Their CI still runs
> commit, documentation, workflow, script and changed-source checks, but source checks use
> `--upstream` without a publication base: no install-card or release-bump requirement.
> Main and submission refs keep their normal checks. `test_archive_gates.py` executes both
> paths and verifies that the publication checks are retained outside the two archive refs.

> ⚠️ CI's live tests stay red when any suite fails. `scripts/test-sources.py` runs after
> the offline contracts, skips only the already-run `review_` cases, retains every raw suite
> log and writes a per-crate Actions summary. The workflow uploads those logs even on failure.
> HTTP evidence labels are observations, not automatic skips or proof that every assertion
> failure is environmental. Unknown/parser failures remain failures without an HTTP label.

On 2026-11-11 the Keyoapp CI run passed its template regressions but Grim returned 522;
Kaizen, Luna and Mist returned 403 HTML titled "Just a moment...". Fresh direct requests
reproduced those statuses. Firecrawl also received Grim's origin-timeout page, while the
other three returned real catalogues through Firecrawl. This distinguishes an origin timeout
observed from these clients from a runner/IP-dependent challenge. It does not establish global
availability or device behaviour. The root guide and [[keyoapp-template]] retain that boundary.

## Running it

| Invocation | Checks |
|---|---|
| `scripts/check-sources.py` | every source, the whole backlog |
| `scripts/check-sources.py en.foo` | just those ids |
| `scripts/check-sources.py --base origin/main` | sources changed since the ref, plus the version-bump rule |
| `--repo <dir> --upstream <ids>` | source rules only, against another checkout: no README row, no agent-doc sync |

Arguments are parsed in `scripts/check-sources.py:430`. Output is one `NOTE` or `FAIL` line
per finding, then a count; the exit code is non-zero only for failures
(`scripts/check-sources.py:451`).

Where it runs:

- CI's Checks workflow with `--base` on every push (`.github/workflows/check.yaml:41`)
- the pre-push hook, both ways: `--upstream` per submission source, `--base` for `main`
  (`.githooks/pre-push:29`, `.githooks/pre-push:51`)
- the submission builder on its prepared worktree (`scripts/pr-branch.sh:131`)
- `scripts/calibrate-rules.sh` over every merged upstream source

## What a run covers

`check_source` (`scripts/check-sources.py:390`) walks one source: Cargo manifest and release
profile, tests that assert nothing, unused test dependencies, the source, filter and settings
JSON, the icon, the Rust rules, constant naming, then the README row and the version bump. Outside `--upstream` it also runs `sync-agent-docs.py --check`
(`scripts/check-sources.py:440`), so a drifted `AGENTS.md` fails here too.

> ⚠️ The generic Rust checker still walks source crates. Template-only edits now select
> their consumers, and known Keyoapp review patterns are checked through path dependencies.
> Portable contracts cover the two reviewed templates; other template internals still need
> their own tests and a manual review.

> ⚠️ A directory without a Cargo manifest is skipped silently (`scripts/check-sources.py:392`),
> so a misspelled id on the command line reports "checked 1 source(s): 0 failure(s)".

## Fail or note

`fail()` and `note()` (`scripts/check-sources.py:30`) are the two outcomes. A failure is a
rule every recent merge follows. A note is a judgement call merged sources disagree on:
printed, never failing.

> ⚠️ A note is not optional. The skill's verify phase asks for every `NOTE` to be acted on
> or explained in the summary. A green exit with notes printed is not a clean run.

> ⚠️ Decide fail or note by calibration, not by how sure the review sounded. Run
> `scripts/calibrate-rules.sh "<message text>"` before a new rule ships: if a recent merge
> kkantan approved fails it, the rule is too broad, so narrow it, drop it, or make it a note.
> The procedure is in the root `CLAUDE.md` section 9, "How a rule gets into this file".

> ⚠️ Every rule needs failing and legitimate passing cases in `scripts/tests/`: one case it
> catches and one legitimate case it lets through. The pass case is what stops the rule
> from widening later. The tests share module-level `failures` and `notes` lists; a test
> asserting on notes must clear them first, as `test_unstated_exclusion_is_noted` does.

## Filter rules

`check_filters_json` (`scripts/check-sources.py:91`) fails a sort default the framework
already uses, `usesTagStyle` beside `isGenre`, and a check filter shown in the header. It
notes a multi-select that is not a genre filter and leaves `canExclude` unstated
(`scripts/check-sources.py:106`), and a sort filter that is not first (`scripts/check-sources.py:109`).

`check_settings_json` (`scripts/check-sources.py:119`) fails a `hide…`/`show…` adult switch whose
default shows adult titles on a source rated 1. `check_cards` notes an image scroller left
without a width.

> ⚠️ The sort-first rule is a note, not a failure: 47 merged sources put the sort later,
> including en.hentai2read, which kkantan approved this month. Don't promote it without a
> fresh calibration.

> ⚠️ The `canExclude` note asks for an explicit `false` although false is the app's default.
> kkantan asked for it on two PRs, and upstream's backlog still notes. Don't "fix" the note
> by deleting a stated `false`; that brings the note back on our sources.

## Version bump

> ⚠️ The bump rule runs only with `--base` (`scripts/check-sources.py:337`), comparing
> the source's version with its version at that ref. A plain run never reports a missing bump.
> A source new since the ref has no file to compare and passes.

## Review command and agent hooks

`scripts/review-source.sh <id>` runs the gates with live tests, this checker, `upstream-usage.py`,
the upstream scope of the source's branch, and prints the section 3 rules as a checklist, then
stamps the result with a hash of the source's files (`scripts/agent-hooks.py:212`). Both
`.claude/settings.json` and `.codex/hooks.json` call `agent-hooks.py`: `pre-edit` points the first
edit of each source or template per session at the workflow, and `stop` blocks the end of a turn
while a changed source fails this checker or has no passing stamp for its current files
(`scripts/agent-hooks.py:186`). The pre-push hook runs each changed source or template's `review_` tests, including a template's test-module clippy gate. `pre-bash` refuses a `git commit` outside `scripts/commit.sh`, prints section 3 before a push
or `pr-branch.sh`, and `digest` prints section 0 after a compaction.

> ⚠️ A commit inside a helper script is invisible to a check on the command line alone: a
> scratchpad script's `git commit` once skipped the documented form. `pre-bash` reads every
> `.sh` a command runs outside `scripts/` and `.githooks/` (`scripts/agent-hooks.py:121`), so
> keep tooling that commits on purpose (`pr-branch.sh`) inside those folders.

> ⚠️ A rebuilt submission branch sits on a newer `upstream/main`, so a push's `before..after`
> holds other contributors' upstream commits, and the identity check failed on them. The pre-push
> check leaves out `refs/remotes/upstream/*` (`scripts/check-commits.py:44`); our own commits in the
> range are still checked.

> ⚠️ Hook output over about 10 KB is saved to a file and only a 2 KB preview reaches the
> agent, so the full section 3 rows (13 KB) arrived cut off on the first push. The push hook
> prints one line per rule (`checklist --compact`, about 6 KB) and only when the push changes a
> source or template; keep anything a hook adds below that size.

> ⚠️ The stamp covers every file in the source except `target/` and `package.aix`, so `cargo fmt`
> or a bump after the review invalidates it. Run the review last.

> ⚠️ A Stop hook that blocks again on its own retry would loop, so with `stop_hook_active` set it
> only prints the problems. A live-site failure gets reported to the person instead of fixed.

> ⚠️ `.claude/settings.json` is shared and committed. A plugin that writes its own settings must
> use `.claude/settings.local.json`, which `.gitignore` covers; one that rewrites `settings.json`
> can drop these hooks. The repository regression suite now checks the required hook entries
> and attribution settings, so settings changes also need a passing gate run.

## Shared documentation workflows

The `docs-sync`, `docs-lint`, `docs-query` and `docs-status` workflows live in
`.claude/skills/` and are physically mirrored into `.agents/skills/`.
`scripts/sync-agent-docs.py:10` lists them and regenerates the Claude command wrappers;
Codex discovers their named `SKILL.md` files directly. The sync also copies root and
`docs/` instruction pairs (`scripts/sync-agent-docs.py:9`), and `--check` detects drift
without writing. Edit the primary `AGENTS.md` / `CLAUDE.md` pair through `AGENTS.md`,
and the docs pair through `docs/AGENTS.md` / `docs/CLAUDE.md`'s first file.

The shared linter is `python3 scripts/docs-lint.py --root .`. It excludes both docs
instruction files from wiki page validation (`scripts/docs-lint.py:28`), while still
checking ordinary pages for frontmatter, source paths and links. The wiki itself,
`scripts/tests/` and `.githooks/` remain shared. The old `.claude/scripts/docs-lint.py`
entry point forwards to the shared implementation for existing callers.

> ⚠️ Codex's patch hook receives the patch in `tool_input.command`, not a `file_path`.
> `scripts/agent-hooks.py:74` extracts added, updated, moved and deleted paths, resolves
> them against the event's working directory, and reminds for each affected crate.
> `scripts/tests/test_agent_docs.py` executes the configured hook commands with Codex
> payloads; `scripts/tests/test_agent_hooks.py` covers multi-crate and nested-directory patches.

> ⚠️ Local Codex command hooks require a trusted project and a runtime that loads them.
> Cloud orchestration does not run local command hooks. In those environments, install
> the Git hooks explicitly and run the documented review gates yourself. Merely copying
> Claude settings into `.agents/` does not configure Codex hooks.

## Repository and commit gates

`python3 scripts/ci-gates.py --base origin/main` runs commit metadata checks, agent-doc sync,
Python regression tests, offline workflow checks and changed-source rules. The checking and
packaging workflows use its event mode to validate the actual GitHub update range. The pre-push
hook also runs repository checks for tooling-only changes, while retaining source compilation,
verification, review tests and upstream submission scope checks. Packaging checks changed
Rust crates before building. Live-site and device checks remain separate.

`scripts/install-hooks.sh` installs executable commit-message and pre-push hooks, configures
the owner's identity and disables signing. Both clients' configured SessionStart hooks run it automatically;
the shared settings disable commit and PR attribution and session URLs. Pending messages and
raw author, committer, signature and message metadata are checked, including merge parents
and already-pushed commits in the update range. See the root guide for the canonical policy.

Unexpected Stop-hook execution errors block completion. Regression tests cover the actual
installed commit hook, an invalid merged commit that is already on the remote, broken hook
input and the settings and workflow wiring. These checks supplement the existing source rules
and review stamps; they do not replace live tests or manual review.

> ⚠️ Git exports `GIT_DIR` to hooks, and in a `pr-branch.sh` worktree it points at the repository
> being pushed. The script tests build throwaway repositories with plain `git` calls, so run from
> the hook as they were they wrote into it: `core.bare=true`, a temporary `core.hooksPath`, a
> "Guard tests" identity and a test branch (2026-11-06). The hook now runs them from the tooling
> checkout with `git rev-parse --local-env-vars` unset (`.githooks/pre-push:12`), and
> `test_settings_and_publication_gate_cannot_drift` checks that line. After such an accident,
> `git config core.bare false` and `scripts/install-hooks.sh` restore the checkout.

> ⚠️ A lockfile-only change still makes a source "changed", so the pre-push hook and CI run its
> full rules and the fork's extra lints. Moving the 18 old aidoku-rs locks at once surfaced 30
> unrelated failures (2026-11-06), so `check_lock` instead fails each stale lock when its source is
> next edited (`scripts/check-sources.py:365`).


> ⚠️ The Source tests workflow selects changed templates and their consumers, and manual
> runs accept `templates/<name>` alongside source IDs. Selecting only dependent sources
> leaves a template's own parser regressions unexecuted. `scripts/tests/test_template_tests.py`
> runs the workflow's actual selection step against a temporary repository to cover both modes.

> ⚠️ After publication, review-source compares against the same origin/main snapshot. An
> unchanged source and its path dependencies need no second bump. Changed source or template
> files still require one; the version checker tests both cases.

> ⚠️ Template submissions use the tooling checkout's hook but must pass `--repo` for the
> submission worktree and `--upstream` to the source checker. Otherwise the hook checks the
> fork's unrelated history and reports unrelated consumers as failures.

> ⚠️ An HTTP 522 from a source request does not establish a global site outage. Compare
> direct requests, a fresh browser session and the device report. Keep the origin-timeout
> response distinct from a Cloudflare 403 challenge; changing parsers does not establish
> that either server-side failure is fixed. HTTP regressions cover 522 alongside other errors.
