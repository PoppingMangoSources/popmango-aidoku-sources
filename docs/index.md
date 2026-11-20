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
- Working on MangaBall: [[mangaball]] (archived)
- Working on VyManga: [[vymanga]]
- Working on VineTheme, Drake, Elf, Valir, Diva or Kayn: [[vinetheme-template]] (archived)
- Working on yaoi.me: [[yaoime]]
- Working on Toonz, Toonx or LewdManhwa (the toonz template): [[toonz]]
- Working on Swarm: [[swarm]]

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
- [[mangaball]] -- MangaBall's API, content modes, language codes, home feeds, what is unverified (archived)
- [[vymanga]] -- VyManga's pages, probed search parameters, expiring chapter links and name keys
- [[mkissa]] -- MKissa's GraphQL feeds, viewer and tag mapping, and the web view reader (archived)

- [[vinetheme-template]] -- current Vine sites, Flight reader data, home rows and novel routing (archived)
- [[yaoime]] -- yaoi.me's settings cookie, browse parameters, chapter variants and the signed reader
- [[toonz]] -- the toonz template: its content cookie, streamed pages, home rows per site and the challenged Toonz pages
- [[swarm]] -- Swarm's API, local exclusions, encrypted page request, languages and uploads
- [[xcomic]] -- XCOMIC's GraphQL API, editions, feeds and filters (upstream only)
- [[toonkotsu]] -- Toonkotsu's JSON API, ratings, web view reader and read limits (upstream only)
- [[kagane]] -- Kagane's mirrors, chapter index, ratings and search body (upstream only)
- [[atsumaru]] -- Atsumaru's Typesense search, feeds, ratings, deduplication and novel keys (upstream only)

## decisions

- [[skill-layout]]: why the root file is a short hub and the work is split into the
  aidoku-source, aidoku-submit and aidoku-archive skills

## reference

_(none yet)_

## operations

- [[source-checks]] -- what `scripts/check-sources.py` checks, where it runs, fail vs note
- [[readme-and-site]] -- the README art, the install page, and why images load from the repo
- [[archives]] -- sources kept off main: the archive procedure and every archived source's last copy

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
