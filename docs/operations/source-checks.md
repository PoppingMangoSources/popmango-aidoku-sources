---
title: Source checks (check-sources.py)
type: operation
created: 2026-10-31
updated: 2026-10-31
sources:
  - code: scripts/check-sources.py
  - code: scripts/tests/test_check_sources.py
  - code: scripts/calibrate-rules.sh
  - code: .github/workflows/check.yaml
  - code: .githooks/pre-push
  - code: scripts/pr-branch.sh
verified_at: a99bd93
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
  (`.githooks/pre-push:21`, `.githooks/pre-push:36`)
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
