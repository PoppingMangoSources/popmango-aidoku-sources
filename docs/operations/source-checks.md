---
title: Source checks (check-sources.py)
type: operation
created: 2026-10-31
updated: 2026-11-06
sources:
  - code: scripts/check-sources.py
  - code: scripts/tests/test_check_sources.py
  - code: scripts/calibrate-rules.sh
  - code: .github/workflows/check.yaml
  - code: .githooks/pre-push
  - code: scripts/pr-branch.sh
  - code: scripts/review-source.sh
  - code: scripts/agent-hooks.py
  - code: scripts/tests/test_agent_hooks.py
  - code: .claude/settings.json
  - code: scripts/check-commits.py
  - code: scripts/ci-gates.py
  - code: scripts/tests/test_commit_enforcement.py
  - code: .github/workflows/build.yaml
verified_at: 149a7b7
---

`scripts/check-sources.py` checks the source rules from the root `CLAUDE.md` / `AGENTS.md`
that need no network and no toolchain. It is the third gate after `cargo fmt`/`clippy` and
`aidoku package`/`verify`; the rules themselves live in the root file, not here.

## Running it

| Invocation | Checks |
|---|---|
| `scripts/check-sources.py` | every source, the whole backlog |
| `scripts/check-sources.py en.foo` | just those ids |
| `scripts/check-sources.py --base origin/main` | sources changed since the ref, plus the version-bump rule |
| `--repo <dir> --upstream <ids>` | source rules only, against another checkout: no README row, no agent-doc sync |

Arguments are parsed in `scripts/check-sources.py:364`. Output is one `NOTE` or `FAIL` line
per finding, then a count; the exit code is non-zero only for failures
(`scripts/check-sources.py:387`).

Where it runs:

- CI's Checks workflow with `--base` on every push (`.github/workflows/check.yaml:41`)
- the pre-push hook, both ways: `--upstream` per submission source, `--base` for `main`
  (`.githooks/pre-push:29`, `.githooks/pre-push:51`)
- the submission builder on its prepared worktree (`scripts/pr-branch.sh:131`)
- `scripts/calibrate-rules.sh` over every merged upstream source

## What a run covers

`check_source` (`scripts/check-sources.py:327`) walks one source: Cargo manifest and release
profile, tests that assert nothing, unused test dependencies, the source, filter and settings
JSON, the icon, the Rust rules, constant naming, then the README row and the version bump. Outside `--upstream` it also runs `sync-agent-docs.py --check`
(`scripts/check-sources.py:375`), so a drifted `AGENTS.md` fails here too.

> ⚠️ Only `sources/` is walked (`scripts/check-sources.py:373`). Templates under `templates/`
> are never checked, so a template's filters or Rust passes whatever it holds. Read a
> template change against the rules by hand.

> ⚠️ A directory without a Cargo manifest is skipped silently (`scripts/check-sources.py:329`),
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

> ⚠️ Every rule needs two tests in `scripts/tests/test_check_sources.py`: one case it
> catches and one legitimate case it lets through. The pass case is what stops the rule
> from widening later. The tests share module-level `failures` and `notes` lists; a test
> asserting on notes must clear them first, as `test_unstated_exclusion_is_noted` does.

## Filter rules

`check_filters_json` (`scripts/check-sources.py:91`) fails a sort default the framework
already uses, `usesTagStyle` beside `isGenre`, and a check filter shown in the header. It
notes a multi-select that is not a genre filter and leaves `canExclude` unstated
(`scripts/check-sources.py:106`), and a sort filter that is not first (`scripts/check-sources.py:110`).

`check_settings_json` (`scripts/check-sources.py:116`) fails a `hide…`/`show…` adult switch whose
default shows adult titles on a source rated 1. `check_cards` notes an image scroller left
without a width.

> ⚠️ The sort-first rule is a note, not a failure: 47 merged sources put the sort later,
> including en.hentai2read, which kkantan approved this month. Don't promote it without a
> fresh calibration.

> ⚠️ The `canExclude` note asks for an explicit `false` although false is the app's default.
> kkantan asked for it on two PRs, and upstream's backlog still notes. Don't "fix" the note
> by deleting a stated `false`; that brings the note back on our sources.

## Version bump

> ⚠️ The bump rule runs only with `--base` (`scripts/check-sources.py:287`), comparing
> the source's version with its version at that ref. A plain run never reports a missing bump.
> A source new since the ref has no file to compare and passes.

## Review command and agent hooks

`scripts/review-source.sh <id>` runs the gates with live tests, this checker, `upstream-usage.py`,
the upstream scope of the source's branch, and prints the section 3 rules as a checklist, then
stamps the result with a hash of the source's files (`scripts/agent-hooks.py:118`). The shared
Claude Code hooks in `.claude/settings.json` call `agent-hooks.py`: `pre-edit` points the first
edit of each source or template per session at the workflow, and `stop` blocks the end of a turn
while a changed source fails this checker or has no passing stamp for its current files
(`scripts/agent-hooks.py:92`). The pre-push hook runs each changed source's `review_` tests.

> ⚠️ The stamp covers every file in the source except `target/` and `package.aix`, so `cargo fmt`
> or a bump after the review invalidates it. Run the review last.

> ⚠️ A Stop hook that blocks again on its own retry would loop, so with `stop_hook_active` set it
> only prints the problems. A live-site failure gets reported to the person instead of fixed.

> ⚠️ `.claude/settings.json` is shared and committed. A plugin that writes its own settings must
> use `.claude/settings.local.json`, which `.gitignore` covers; one that rewrites `settings.json`
> can drop these hooks. The repository regression suite now checks the required hook entries
> and attribution settings, so settings changes also need a passing gate run.

## Repository and commit gates

`python3 scripts/ci-gates.py --base origin/main` runs commit metadata checks, agent-doc sync,
Python regression tests, offline workflow checks and changed-source rules. The checking and
packaging workflows use its event mode to validate the actual GitHub update range. The pre-push
hook also runs repository checks for tooling-only changes, while retaining source compilation,
verification, review tests and upstream submission scope checks. Packaging checks changed
Rust crates before building. Live-site and device checks remain separate.

`scripts/install-hooks.sh` installs executable commit-message and pre-push hooks, configures
the owner's identity and disables signing. Claude's SessionStart hook runs it automatically;
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
> next edited (`scripts/check-sources.py:315`).

