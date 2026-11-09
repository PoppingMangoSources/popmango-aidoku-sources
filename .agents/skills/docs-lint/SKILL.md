---
name: docs-lint
description: Check this repository documentation wiki for broken links, orphan pages, invalid sources, stale pages and unverifiable claims. Use when validating or reviewing wiki integrity.
---

Read `docs/AGENTS.md` / `docs/CLAUDE.md` before following this workflow.
Run commands from the repository root. Claude invokes this as `/docs-lint`; Codex can invoke it as `$docs-lint`.

# /docs-lint -- integrity check for the docs wiki

Run the script for anything a script can decide.

```bash
python3 scripts/docs-lint.py --root .
```

Catches: broken `[[links]]`, orphan pages, broken sources (`code:` paths),
missing frontmatter, citations past end of file, unfinished checklists,
**stale**, **unverified**.

### stale and unverified are different

| | Meaning | What to do |
|---|---|---|
| **stale** | the code this page points at **changed** | `/docs-sync` |
| **unverified** | freshness **cannot be decided** | a human has to read it |

Ways a page becomes unverified:

1. **No `code:` sources** -- nothing to compare against, so staleness is not computable.
   Legitimate for concept and history pages, which is why this never fails the run
2. **All `code:` sources have disappeared**
3. **The sha moved past N code commits with no body change** -- allowed by the schema,
   but `verified_at` is a declaration rather than proof, and this is the one place where
   "did you actually read it?" shows up. The larger N is, the more suspect

4. **The `verified_at` sha is not in this checkout**, for example after a squash merge,
   rebase or shallow clone. Re-check the page before moving its sha to a known commit.

## What the script cannot catch -- read and judge these yourself

1. **Contradictions between pages** -- the same fact stated differently
2. **Missing cross-references** -- two pages covering one concept without linking
3. **Duplicated prose** -- a lookup page and an exploratory page each carrying the same
   table. That breaks the single-source rule
4. **Prose that went stale without the code moving** -- `verified_at` is current but the
   text no longer matches reality. A script cannot see this

## Reporting

Summarise the findings and **ask the user before fixing.** Do not auto-fix.
If it is clean, say so and stop.
