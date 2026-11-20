# Merged source snapshots

This branch preserves merged sources and their fork tests. It does not publish source packages.
For new maintenance, start from the current canonical upstream source and restore only the
needed tests from this branch.

The six MangaThemesia sources below and their complete shared template were copied from
main at `ce4c29829e74a69ca10d9061551ef968a0fe6c1f` on 2026-11-11:

- `en.violetscans`
- `en.thunderscans`
- `en.madarascans`
- `en.evascans`
- `en.ragescans`
- `en.lagoonscans`

The template includes the child-theme home/deep-link parsers, novel formatting, viewer
override and deterministic regressions. `scripts/test-http.py templates/mangathemesia` runs
the loopback regressions with `aidoku-test-runner` installed. Existing archived sources remain.
The earlier KingOfShojo/Rokari metadata minimums are raised to 0.8.3 because the shared
reader now imports `child_nodes`; archive versions are unchanged.

## Keyoapp

PR 888 merged into canonical upstream at `289b5613bb47f62c6eb91de3d8b2282643586533`.
The complete fork snapshot is main `81e42e5312cda0aed39d174c3dcf3b2797f858c0`, including
both template generations, all source/template tests and `scripts/test-http.py`.

- `en.asmotoon`
- `en.erisscans`
- `en.genztoons`
- `en.grimscans`
- `en.kaizenscan`
- `en.kewnscans`
- `en.lunatoons`
- `en.mistscans`
- `en.sirenscans`
- `en.timelesstoons`

Run the template contracts with `cargo test review_` from `templates/keyoapp` and its
loopback regressions with `python3 scripts/test-http.py templates/keyoapp` from this root.
Source versions are preserved. Later changes start from current canonical upstream.

## Iken, MangaThemesia and EternalMangas

PR 900 (Iken template, Hive, Nyx, Vortex, Magus, Hijala, Sana) and PR 897 (Scythe Scans) merged
into canonical upstream. EternalMangas' own rewrite, PR 905, was approved and still open when it
was archived at the owner's request. The complete fork snapshot is main `56d5306e971cb47db1cc0ea9d3a08767d637fa64`,
including both templates, every source and template test and `scripts/test-http.py`.

- `en.hivescans`
- `en.nyxscans`
- `en.vortexscans`
- `en.magusmanga`
- `en.hijalascans`
- `en.sanascans`
- `es.eternalmangas`
- `en.scythescans`

`templates/iken` and `templates/mangathemesia` replace the earlier copies here; Scythe Scans
was the last MangaThemesia consumer on main. Run the loopback regressions with
`python3 scripts/test-http.py templates/iken` (or `templates/mangathemesia`, `templates/keyoapp`).

## MangaTaro template, MangaTaro, Rolia Scan and MKissa

PR 911 (mangataro template with MangaTaro and Rolia Scan) and PR 910 (MKissa) merged into
canonical upstream on 2026-11-19. The complete fork snapshot is main `e4213542887d6a3744f188d8b72917a07b21608a`, including the
template and its fixtures, every source test and MKissa's README icon. It also carries the fixes
for the last comment on each PR (unstable tag sorts, and the image quality parameter joined with
`&` when a url has a query), which went up again as updates on `feat/mangataro-template` and
`feat/en.mkissa`.

- `en.mangataro`
- `en.roliascan`
- `en.mkissa`
