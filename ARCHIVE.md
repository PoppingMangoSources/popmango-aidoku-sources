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
