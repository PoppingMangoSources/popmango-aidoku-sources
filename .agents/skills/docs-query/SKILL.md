---
name: docs-query
description: Answer questions strictly from this repository documentation wiki. Use when the user asks what the wiki documents, with citations and explicit gaps rather than guesses from code.
---

Read `docs/AGENTS.md` / `docs/CLAUDE.md` before following this workflow.
Run commands from the repository root. Claude invokes this as `/docs-query`; Codex can invoke it as `$docs-query`.

# /docs-query -- ask the docs wiki

Use the question supplied by the user or command invocation.

## Rules

1. **Answer from `docs/` only.** Get your bearings from `docs/index.md`, then read the
   relevant pages
2. **Attribute every claim with `[[page-name]]`**
3. **If the wiki does not have it, say so.**
   Do not dig through the code and improvise an answer to paper over the gap. That hides
   the fact that the wiki is empty there, and the next person hits the same wall:
   > Not in the wiki. It looks like it lives near `<path>` --
   > shall I check and add it to [[that-page]]?
4. If the answer produced a useful new synthesis, save it as a page **after confirming**

## Reading order

- "why is it like this" -> exploratory (`concepts/`, `decisions/`)
- "how does it work" -> exploratory (`architecture/`, `subsystems/`)
- "what was the value" -> lookup (`reference/`)
- "how do I do it" -> lookup (`operations/`)
