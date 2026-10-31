# /docs-query -- ask the docs wiki

Question: $ARGUMENTS

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
