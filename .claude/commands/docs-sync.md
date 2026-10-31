# /docs-sync -- reflect code changes into the docs wiki

Follow **Workflow > /docs-sync** in `docs/CLAUDE.md`.

## Procedure

> ⚠️ One syncer at a time, and pull first. Two sessions syncing at once both append
> `docs/log.md` and edit `docs/index.md`, and the second one conflicts. In a multi-session
> setup, let one session sync and the others wait.

### 1. Baseline

The **oldest** `verified_at` sha across `docs/**/*.md` frontmatter.

### 2. First check whether any code changed

**Do not count docs commits.** The docs live in the same repository, so editing only
docs still moves HEAD. Without this filter you keep seeing "N commits behind" when
there is nothing to do.

```bash
git log <baseline>..HEAD --oneline -- . ':(exclude)docs' ':(exclude).claude' ':(exclude)CLAUDE.md' ':(exclude)AGENTS.md' ':(exclude).agents'
```

**If it is empty, stop: "no code changes, /docs-sync not needed".**
Leave `verified_at` alone -- the code has not moved, so it is still valid.

### 2-1. Collect changed files

```bash
git log <baseline>..HEAD --name-only --pretty=format: -- . ':(exclude)docs' ':(exclude).claude' ':(exclude)CLAUDE.md' ':(exclude)AGENTS.md' ':(exclude).agents' \
  | sort -u | grep -v '^$'
```

> Git quotes non-ASCII paths by default (`core.quotepath`), so a path with
> non-Latin characters comes back escaped and a naive `grep '^docs/'` misses it.
> Use `-z`, or `git -c core.quotepath=false`, when a filename might not be ASCII.

### 3. Select affected pages

Match changed file paths against each page's `sources[].code` by **prefix**.
Report the selection first: N changed files -> M affected pages.

### 4. Update

For each page, **read the actual diff, not the commit message**
(`git diff <baseline>..HEAD -- <path>`).

- Prose still correct -> move `verified_at` only
- Prose wrong -> fix the body, and move both `updated` and `verified_at`
- A cited `file:line` that shifted -> new line number

> ⚠️ `verified_at` is written *before* you commit, so it names the commit before yours.
> A docs-only commit does not make anything stale: staleness counts only commits that
> touch a page's `code:` sources. If one commit changes both code and the page that
> describes it, that page shows "stale by 1" -- commit code and docs separately.

### 5. Unclassified changes

List files that matched no page and **propose new pages**. Do not create them without
approval.

### 6. Finish

- Update `docs/index.md` (if pages were added)
- Append to `docs/log.md`
- Verify with `/docs-lint`

## Principles

- **Only read what changed.** Do not re-read the whole codebase every time
- State lives in `verified_at` and nowhere else. Do not add a status file
