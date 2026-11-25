---
name: aidoku-submit
description: Publish or submit finished Aidoku source work from this repo. Use for scripts/bump.sh and scripts/commit.sh on main, scripts/pr-branch.sh and an upstream Aidoku-Community PR, folding review fixes into a PR branch, drafting replies to kkantan's review comments, or updating a source after its PR merged.
model: inherit
---

# Publishing and submitting Aidoku sources

Start only when the aidoku-source skill's Verify phase passed: `scripts/review-source.sh <id>`
is green on the current files. The gates and scripts named here are described whole in
[submission.md](../aidoku-source/references/submission.md); the review rules in [standards.md](../aidoku-source/references/standards.md).

Done when the push hook reports `pre-push: all gates green` and, for a PR, the drafted title,
body and replies are handed to the person, who opens and posts them.

## 1. Release on main

Every push that changes a source publishes it, so every such push bumps it:

```sh
scripts/bump.sh <id>
```

Commit as Popmango Extensions, unsigned, conventional subject, a one- or two-sentence body,
no AI attribution (AGENTS.md / CLAUDE.md §9).

## 2. Upstream PR

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
An independent source rewrite that leaves a template gets its own PR, even if it began in
the template update. Keep only compilation/minimum-version compatibility with the shared
template until the standalone migration merges. Inspect every consumer and preserve standard
aidoku/aidoku-test development dependencies when excluding fork-only tests.

## 3. Review rounds

For each comment, fix it on `main` with a bump, then fold the changes into the PR's
requested commit structure. Any later push of that source to `main` does the same: an unmerged
`feat/<id>` or `fix/<id>` on origin is rebuilt from `main` in the same task
(`scripts/pr-branch.sh <id> --rebuild --push`), and the Stop hook blocks until it is. A new-source submission stays one `feat: add <id>` commit by
default, including after it opens. Fetch first, preserve the old head, commit through the
wrapper and verify the resulting source tree and commit count. Force-push with the exact
expected remote SHA. A force-push alone does not fold an added commit into the original.

Only when the owner explicitly requests a separate review-fix commit:

```sh
scripts/pr-branch.sh <id> --fix "fix(<id>): address review feedback" "<one or two sentences>" --push
```

Draft a one- or two-sentence reply per comment for the person to post; do not post on the
PR yourself.

Every comment, and every correction the person makes, also becomes a rule: a row in
[standards.md](../aidoku-source/references/standards.md), decided from the implementing code, plus a `check-sources.py` rule with a test when it can
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

## 4. Review evidence

Record the upstream base SHA, source snapshot SHA, prepared head SHA, changed-path scope,
fmt/Clippy/package/verify results, runner test results and remaining in-app checks. Check
pagination with a second page, filters with a value that should match nothing, each content
rating independently, missing/locked chapters, pasted URLs, deep links and page-list errors.
Count requests per operation and identify any per-item fan-out. Consult the reference's
reviewer evidence, then reread the final diff and every prior review thread before resubmission.
