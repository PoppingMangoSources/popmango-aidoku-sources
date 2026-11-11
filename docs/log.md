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

## [2026-11-09] fix | MangaThemesia child-theme consumers

- Added Madara's Novel filter, five-row Latest Comics paging and opt-in chapter deep links for all four consumers.
- Rejected blank card/banner titles and reused SDK banner links, with offline fixtures and live key-only reader checks.
- Updated [[mangathemesia-consumers]] and both clients' rules for the paging default and shared handlers, verified at `d49266c`.

## [2026-11-09] fix | Kaizen Scan content rating

- Set Kaizen Scan's source rating to 2 at the owner's request and updated its publication badge.
- Main publishes version 5; the upstream submission retains version 1 with the change folded into its source commit.

## [2026-11-09] fix | Keyoapp newly added row scope

- Reused the normal card conversion for internal and adjacent newly added grids, with a regression that rejects unrelated grids.
- Recorded Today for Trending and Month for Most Reviewed as separate owner-selected defaults in [[keyoapp-template]] and both instruction copies.
- Folded all submission template changes into its first commit while preserving the ten source commits and version 1 metadata.
- Code verified at `c1e1cbf`; Mist remains on its existing domain and parser.

## [2026-11-10] fix | Keyoapp HTTP diagnostics

- Added checked template requests and bounded failure diagnostics on main, verified at `2dd1cdc`.
- Added a separate loopback HTTP regression to Source tests and a calibrated parser guard.
- Verified all ten packages; eight live suites passed and Kaizen/Luna returned Cloudflare HTTP 403.
- Recorded the remaining device checks in [[keyoapp-template]]; inspected the open upstream submission without changing it.

## [2026-11-10] fix | HTTP checks for all live-tested sources

- Added MangaThemesia status checks for standard and child-theme request paths, preserving a missing deep link as no match.
- Shared the HTTP fixture runner between both live-tested templates and added a calibrated guard against unchecked response parsing.
- Bumped the eight affected consumers once; PR branches remain unchanged.
- Updated [[mangathemesia-consumers]] and [[keyoapp-template]], verified at `17c5faf`.

## 2026-11-10: Rage Scans review

Removed unavailable author search, scoped Rage author and artist parsing to metadata rows,
and added an optional viewer override for its mislabeled webtoons. Recorded why impossible
author queries did not justify the old filter and added loopback viewer regression coverage.

## 2026-11-10: Shared novel emphasis

Preserved bold and italic across novel paragraphs and line breaks using the merged reader
references, without an opt-in feature. Raised the app minimum for every template consumer,
updated dependent source versions, and changed Madara's icon background to its header color.

## 2026-11-10: MangaThemesia follow-up audit

Fixed adjacent nested emphasis markers and empty home keys, aligned Editor Picks title and
description handling, and shortened comments. Added deterministic coverage without changing
the shared default home or introducing dependencies.

## 2026-11-11

Resumed [[iken-template]] maintenance for the five upstream consumers. Updated pagination, keys, lock handling, home parsing and novel emphasis; retained fork-only regression coverage.

Added New Releases and Novels API rows to [[iken-template]], with compact cards, concurrent requests and deterministic partial-failure/order coverage. The submission keeps novel sections and formatted reading in a separate commit.

Synchronized checked-request helpers into the KeyoApp and Rage submission trees. Extended the HTTP regressions to include 522 and clarified route-specific site availability in [[source-checks]].

Set [[iken-template]] latest-release lists to five rows per page and popular ranked lists to three, with English/Spanish layout regressions preserving all cards.

Moved Mist Scans to its working `.org` V2 site and replaced V1 filters/tests in [[keyoapp-template]].

Restored real chapter numbers and release dates to the Iken latest row with one metadata batch, preserving pinned HTML order and existing page sizes. Checked Hive against the linked reference client and current API; the lowercase title comes from the site. See [[iken-template]].
