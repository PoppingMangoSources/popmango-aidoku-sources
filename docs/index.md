# popmango-aidoku-sources -- documentation

Aidoku sources published from this fork, and the work behind their upstream submissions.

This directory is a wiki maintained by an LLM: linked markdown pages, nothing outside the
repository. The rules are in [`AGENTS.md`](AGENTS.md) / [`CLAUDE.md`](CLAUDE.md). The rules for writing sources live in
the root `CLAUDE.md` / `AGENTS.md`; this wiki records how particular areas work and the
traps found in them. It grows as work touches new areas.

## Start here

- Working on MangaK or ToonTop: [[mangak-template]]
- Adding or changing a source check: [[source-checks]]
- Working on Flame Comics: [[flamecomics]]
- Working on Novel Archive: [[novelarchive]]
- Working on KingOfShojo or RokariComics: [[mangathemesia-consumers]]
- Working on Genz Toon or another Keyoapp site: [[keyoapp-template]]
- Working on MangaBall: [[mangaball]]

---

## concepts

_(none yet)_

## architecture

_(none yet)_

## subsystems

- [[mangak-template]] -- the template MangaK and ToonTop share: search, home, ratings
- [[flamecomics]] -- Flame's embedded page data, path keys, the novel lock rule
- [[novelarchive]] -- Novel Archive's API, adult exclusions, dynamic genres, home feeds
- [[mangathemesia-consumers]] -- KingOfShojo and Rokari on upstream's template unchanged
- [[keyoapp-template]] -- the Keyoapp template: whole-catalogue search, generic home, which sites fit
- [[mangaball]] -- MangaBall's API, content modes, language codes, home feeds, what is unverified

## decisions

_(none yet)_

## reference

_(none yet)_

## operations

- [[source-checks]] -- what `scripts/check-sources.py` checks, where it runs, fail vs note
- [[readme-and-site]] -- the README art, the install page, and why images load from the repo

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

Codex can invoke the same workflows as `$docs-sync`, `$docs-lint`, `$docs-query` and
`$docs-status`. The shared linter is `python3 scripts/docs-lint.py --root .`.

Every page carries `verified_at`, the commit it was checked against. Change code, run
`/docs-sync`.

- [[iken-template]]: shared API, home layouts, reader keys and verification.
