# Work log

Append-only. Newest entries at the bottom.

---

## [2026-10-31] setup | docs/ wiki scaffolded

- Categories: concepts - architecture - subsystems - decisions - reference - operations - history
- Baseline commit: `07f5fdc`
- Created: `docs/{CLAUDE,index,log}.md`, `raw/`, 4 `.claude/commands/docs-*`,
  and `.claude/scripts/docs-lint.py`

- Started in grow mode: one page, [[mangak-template]], written from `templates/mangak` and
  `sources/en.mangak` after the #850 review round. `AGENTS.md` and `.agents/` were added to
  the code exclude list, since they mirror `CLAUDE.md` and `.claude/skills/`.

## [2026-10-31] add | operations/source-checks

- New page [[source-checks]] for `scripts/check-sources.py`, which the commit hook
  named as uncovered. Verified against `e9c39f5`.

## [2026-11-01] add | subsystems/flamecomics

- New page [[flamecomics]] after rewriting the source on the site's embedded data.
- [[source-checks]] re-verified at `a99bd93` (the new rules are described there).

## [2026-11-02] update | flamecomics, mangak-template

- `en.flamecomics` and `en.mangak` left `main` (last copies at `171e770`). Both pages now point
  at that commit and upstream; the review rules stay in the root `CLAUDE.md`.

## [2026-11-02] update | source-checks

- [[source-checks]] updated for owner-only unsigned commit metadata, automatic hook setup,
  repository checks before packaging and Stop-hook execution failures. Verified at `90f9d77`.

## [2026-11-06] add | subsystems/novelarchive

- New page [[novelarchive]] after reworking the source for the moved site and its library
  script: adult exclusions sent like the site, dynamic genres, the site's three home rows.

## [2026-11-09] update | shared agent docs workflows

- Added matching docs instruction files and four docs skills for both clients.
- Shared the docs linter under `scripts/`, retained Claude command wrappers, and extended sync drift checks.
- Added Codex hook configuration using the existing handlers, including patch input support.
- Updated [[source-checks]] with setup boundaries and regression coverage.

## [2026-11-09] review | Keyoapp template and consumers

- Compared all eleven template/consumer commits with main's rules and merged upstream implementations.
- Fixed series identity, pasted links, malformed catalogue handling, unavailable chapters and novel Markdown.
- Moved home parsing, reused SDK links and shortened comments; added template regression selection to CI and pre-push.
- Recorded the #872/#869/#865/#824/#666 review cross-checks and current live-site boundaries in [[keyoapp-template]].
- Updated [[source-checks]] and synchronized both clients' review instructions and references.
