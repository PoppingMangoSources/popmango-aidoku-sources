---
name: docs-status
description: Report this repository documentation wiki status, including code commits not reflected in docs, stale pages and recent work. Use when checking documentation freshness or whether a sync is needed.
---

Read `docs/AGENTS.md` / `docs/CLAUDE.md` before following this workflow.
Run commands from the repository root. Claude invokes this as `/docs-status`; Codex can invoke it as `$docs-status`.

# /docs-status -- docs wiki status

## 1. Commits not yet reflected (first)

Against the oldest `verified_at` across all pages. **Count code and docs separately** --
a docs-only commit is not something `/docs-sync` acts on.

```bash
EXCL=". :(exclude)docs :(exclude).claude :(exclude)CLAUDE.md :(exclude)AGENTS.md :(exclude).agents"
git log <baseline>..HEAD --oneline -- $EXCL | wc -l          # code
git log <baseline>..HEAD --oneline -- docs .claude .agents AGENTS.md CLAUDE.md | wc -l   # docs
```

```
<baseline>..HEAD   code 0 commits - docs 1 commit    -> /docs-sync not needed
<baseline>..HEAD   code 4 commits - docs 0 commits   -> /docs-sync recommended
```

**If code is 0, do not recommend `/docs-sync`.** Recommending work that does not exist is
how people stop trusting `/docs-status`.

## 2-5

Stale and unverified pages, unprocessed `raw/` material, unfinished `- [ ]` items, the
last 3 entries of `docs/log.md`.

```bash
python3 scripts/docs-lint.py --root . --quiet
```

## 6. Size

Page count per category.

## Reporting

Fit it on one screen. If something needs doing, recommend it in one line at the end.
