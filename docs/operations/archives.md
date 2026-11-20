---
title: Sources kept off main (merged-sources and dead-sources)
type: operation
created: 2026-11-20
updated: 2026-11-20
sources:
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/tree/merged-sources
verified_at: 3ac574e
---

A source leaves `main` once it is upstream, or while its upstream branch waits on review. Its
crate and live tests stay in `main`'s history; what it taught stays in this file and `docs/`.
The `merged-sources` branch keeps the last copy of each merged one, tests included (Atsumaru,
MangaK, XCOMIC, Toonkotsu, KingOfShojo, RokariComics, Novel Archive and the seven MangaThemesia
sources merged in the November batch, plus both Keyoapp generations and their ten consumers, VineTheme with its five consumers, MangaBall,
and the MangaTaro template with MangaTaro, Rolia Scan and MKissa),
so they can be read and
tested in one place.

Confirm the canonical PR is merged before archiving. Copy the complete tracked crate, tests,
icons and its current path dependencies to `merged-sources`, verify the snapshot and publish
that branch first. Remove the crate, README entry and unused source icon from main afterwards;
removal does not publish a new source version. Keep shared templates and their regressions on
main while other sources use them. Move an unused template and its source-specific fixtures
with its last consumers; preserve review guidance and working history links on main. `scripts/check-archives.py --base <before>` compares every
removed crate and dependency against `origin/merged-sources` or `origin/dead-sources`; repository
and push gates reject incomplete preservation. Archive branches do not publish packages.

| Source | State | Last copy on `main` | Branch | What it taught |
|---|---|---|---|---|
| `multi.kagane` | merged (#827) | `99c3658`, never had tests | from `upstream/main` | sections 3 and 14 |
| `en.toonkotsu` | merged (#815, #821) | `15762be`; tests last at `ff9e199` | from `upstream/main` | sections 3 and 11 |
| `en.atsumaru` | merged (#843) | `43255ad` | from `upstream/main` | sections 3 and 16 |
| `multi.xcomic` | merged (#735, #738, #762, #771, #853) | `b86969e` | from `upstream/main` | sections 3 and 10 |
| `en.mangak` | merged (#850) | `171e770` | from `upstream/main` | section 3, `docs/subsystems/mangak-template.md` |
| `en.flamecomics` | fix open (#860) | `171e770` | `fix/en.flamecomics` | section 3, `docs/subsystems/flamecomics.md` |
| `en.kingofshojo` | merged (#875) | `c667b2d` | from `upstream/main` | `docs/subsystems/mangathemesia-consumers.md` |
| `en.rokaricomics` | merged (#874) | `c667b2d` | from `upstream/main` | `docs/subsystems/mangathemesia-consumers.md` |
| `en.novelarchive` | merged (#873) | `56e8bb7` | from `upstream/main` | section 3, `docs/subsystems/novelarchive.md` |
| `en.violetscans` | merged (#884) | `ce4c298` | `merged-sources`; maintenance from upstream | child-theme layouts and keys; MangaThemesia docs |
| `en.thunderscans` | merged (#884) | `ce4c298` | `merged-sources`; maintenance from upstream | novels and shared template reuse; MangaThemesia docs |
| `en.madarascans` | merged (#884) | `ce4c298` | `merged-sources`; maintenance from upstream | icon background and catalogue rating; MangaThemesia docs |
| `en.evascans` | merged (#884) | `ce4c298` | `merged-sources`; maintenance from upstream | shared reader and minimum app version; MangaThemesia docs |
| `en.ragescans` | merged (#883) | `ce4c298` | `merged-sources`; maintenance from upstream | metadata scope, author filter and viewer override; MangaThemesia docs |
| `en.lagoonscans` | merged (#881) | `ce4c298` | `merged-sources`; maintenance from upstream | static filters and template reuse; MangaThemesia docs |
| `en.ravenscans` | merged (#882) | `5882af8` | `merged-sources`; maintenance from upstream | catalogue rating and standard template wiring; MangaThemesia docs |
| `en.asmotoon` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.erisscans` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.genztoons` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.grimscans` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.kaizenscan` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.kewnscans` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.lunatoons` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.mistscans` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.sirenscans` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.timelesstoons` | merged (#888) | `81e42e5` | `merged-sources`; maintenance from upstream | `docs/subsystems/keyoapp-template.md` |
| `en.hivescans` | merged (#900) | `56d5306` | `merged-sources`; maintenance from upstream | `docs/subsystems/iken-template.md` |
| `en.nyxscans` | merged (#900) | `56d5306` | `merged-sources`; maintenance from upstream | `docs/subsystems/iken-template.md` |
| `en.vortexscans` | merged (#900) | `56d5306` | `merged-sources`; maintenance from upstream | `docs/subsystems/iken-template.md` |
| `en.magusmanga` | merged (#900) | `56d5306` | `merged-sources`; maintenance from upstream | `docs/subsystems/iken-template.md` |
| `en.hijalascans` | merged (#900) | `56d5306` | `merged-sources`; maintenance from upstream | `docs/subsystems/iken-template.md` |
| `en.sanascans` | merged (#900) | `56d5306` | `merged-sources`; maintenance from upstream | `docs/subsystems/iken-template.md` |
| `es.eternalmangas` | rewrite approved, #905 open when archived (owner's call) | `56d5306` | `merged-sources`; `fix-eternal` for #905 | `docs/subsystems/iken-template.md` |
| `en.scythescans` | merged (#897) | `56d5306` | `merged-sources`; maintenance from upstream | `docs/subsystems/mangathemesia-consumers.md` |
| `en.drakescans` | merged (#908) | `38322e3` | `merged-sources`; maintenance from upstream | `docs/subsystems/vinetheme-template.md` |
| `en.elftoon` | merged (#908) | `38322e3` | `merged-sources`; maintenance from upstream | `docs/subsystems/vinetheme-template.md` |
| `en.valirscans` | merged (#908) | `38322e3` | `merged-sources`; maintenance from upstream | `docs/subsystems/vinetheme-template.md` |
| `en.divascans` | merged (#908) | `38322e3` | `merged-sources`; maintenance from upstream | `docs/subsystems/vinetheme-template.md` |
| `en.kaynscans` | merged (#908) | `38322e3` | `merged-sources`; maintenance from upstream | `docs/subsystems/vinetheme-template.md` |
| `multi.mangaball` | merged (#902) | `38322e3` | `merged-sources`; maintenance from upstream | section 3, `docs/subsystems/mangaball.md` |
| `en.mangataro` | merged (#911); sort fix on `feat/mangataro-template` | `e421354` | `merged-sources`; maintenance from upstream | section 3, `docs/raw/2026-11-15_mangataro.md` |
| `en.roliascan` | merged (#911); sort fix on `feat/mangataro-template` | `e421354` | `merged-sources`; maintenance from upstream | section 3, `docs/raw/2026-11-15_mangataro.md` |
| `en.mkissa` | merged (#910); query fix on `feat/en.mkissa` | `e421354` | `merged-sources`; maintenance from upstream | section 3, `docs/subsystems/mkissa.md` |

A review round on an open branch restores the crate on `main` from its last copy
(`git checkout <sha> -- sources/<id>` plus its README row), fixes and tests it there with a
bump, then folds the update into the PR's requested commit structure with a leased force-push.
Use `pr-branch.sh --fix` only for an explicitly requested separate commit; removing the crate
again afterwards keeps `main` to the fork's own sources.

A later change to a merged source starts from **upstream's** crate, not the last copy here:
`git checkout upstream/main -- sources/<id>`, then take only `src/test.rs` from the last copy and
add `#[cfg(test)] mod test;` back to `lib.rs`. Upstream is often ahead: XCOMIC's `b86969e` lacks
the `canExclude: false` lines kkantan asked for on #853 and #807's warning fixes landed upstream
only. Set the version above both upstream's and the fork's last one, then
`pr-branch.sh <id> --update …`.
