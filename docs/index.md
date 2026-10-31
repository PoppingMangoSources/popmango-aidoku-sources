# popmango-aidoku-sources -- documentation

Aidoku sources published from this fork, and the work behind their upstream submissions.

This directory is a wiki maintained by an LLM: linked markdown pages, nothing outside the
repository. The rules are in [`CLAUDE.md`](CLAUDE.md). The rules for writing sources live in
the root `CLAUDE.md` / `AGENTS.md`; this wiki records how particular areas work and the
traps found in them. It grows as work touches new areas.

## Start here

- Working on MangaK or ToonTop: [[mangak-template]]

---

## concepts

_(none yet)_

## architecture

_(none yet)_

## subsystems

- [[mangak-template]] -- the template MangaK and ToonTop share: search, home, ratings

## decisions

_(none yet)_

## reference

_(none yet)_

## operations

_(none yet)_

## history

_(none yet)_

---

## Using this wiki

```
/docs-sync     right after a deploy or release -- reads only what changed since verified_at
/docs-lint     integrity check (orphans, broken links, broken sources, stale, unverified)
/docs-query    ask the docs. If it is not there, the answer says so
/docs-status   commits not yet reflected, stale pages, recent work
```

Every page carries `verified_at`, the commit it was checked against. Change code, run
`/docs-sync`.
