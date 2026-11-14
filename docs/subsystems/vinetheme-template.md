---
title: VineTheme template and consumers
type: subsystem
created: 2026-11-13
updated: 2026-11-13
sources:
  - code: templates/vinetheme/src/lib.rs
  - code: templates/vinetheme/src/imp.rs
  - code: templates/vinetheme/src/models.rs
  - code: templates/vinetheme/src/home.rs
  - code: templates/vinetheme/src/helpers.rs
  - code: templates/vinetheme/src/rsc.rs
  - code: templates/vinetheme/src/markdown.rs
  - code: templates/vinetheme/src/settings.rs
  - code: templates/vinetheme/src/test.rs
  - code: scripts/review-contracts/vinetheme.rs
  - code: scripts/review-contracts/fixtures.py
  - code: scripts/check-upstream-scope.py
  - ext: https://github.com/Aidoku-Community/sources/issues/886
  - ext: https://github.com/Aidoku-Community/sources/issues/887
  - ext: https://github.com/Aidoku-Community/sources/issues/537
  - ext: https://github.com/Aidoku-Community/sources/issues/610
  - ext: https://github.com/Aidoku-Community/sources/pull/888
  - ext: https://github.com/Aidoku-Community/sources/pull/884
  - ext: https://github.com/Aidoku-Community/sources/pull/882
  - ext: https://github.com/Aidoku-Community/sources/pull/860
  - ext: https://github.com/Aidoku-Community/sources/pull/900
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/actions/runs/43550746633
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/lib-multisrc/vinetheme
  - ext: https://drakecomic.net/
  - ext: https://elftoon.net/
  - ext: https://valirscans.org/
  - ext: https://divascans.org/
  - ext: https://kaynscans.com/
verified_at: 5b086ac
---

VineTheme serves Drake, Elf, Valir, Diva and Kayn. The owner requested publication on main
and a clean `fix-vine` template submission, with the new sources in separate commits.
All consumers require app 0.8.3 because the shared novel parser walks text nodes.
The Iken review's test scaffold, isGenre, item spacing, scoped selectors and independent
migration lessons remain in the root rules and [[source-checks]].

## Current site contracts

The public catalogue is `/api/series`; `/api/series/facets` returns current genres and tags.
Details and complete chapters use `/api/ai/series/{canonicalSlug}?chapters=none|all`.
Readers use `/series/{comic|novel}/{urlSlug}/chapter/{number}` with `rsc: 1`.
The public routes use `urlSlug` when it differs from the database slug. Bare legacy comic
keys remain valid; novel keys include `novel/`, and old Valir `comic/` keys remain readable.
No cached details or WebView metadata is required to read a chapter.

The catalogue accepts page/limit, q, the seven site sorts, Title Case type names, status,
origin, genre/exgenre, tag/extag, ch_min/ch_max and images. Elf's current catalogue has no
novels or tag options. Novel-only responses are empty there, not an error or invented row.
Static controls are resources; dynamic facets are fetched once per invocation. Tag taps
resolve displayed names to their actual genre/tag slugs. No sort has an ascending option.

> ⚠️ `genre=Action` returns zero while `genre=action` narrows correctly. Type `NOVEL` also
> returns zero; the current novel type is `Web Novel`/`WEB_NOVEL`, not the old generic value.

Probes checked page 2, positive/impossible genres, genre exclusions, status, novel mode,
an impossible chapter minimum and query, and Illustrated narrowing. The `nsfw` query
parameter was ignored on observed endpoints, so adult visibility is applied locally to each
returned page without extra fetch loops. Source rating 1 keeps adult titles hidden by default.
Source settings default Show Locked Chapters off, Show 18+ Content off and Label Novels on
where a mixed catalogue currently exists. Commercial sale/purchase filters are omitted.

## Reader and descriptions

> ⚠️ The first object named `chapter` can contain only analytics/reader metadata. The actual
> reader object includes chapter plus isUnlocked/isLocked. Match that object; otherwise every
> image chapter can look empty despite HTTP 200. Never filter missing page URLs out of a chapter.

Next.js Flight may be raw rows or JSON-escaped push strings inside HTML. Long novel HTML is
a `$<row>` reference to a `T<hex-byte-length>,` row, not literal text to display. Byte-length
rows can contain newlines and end immediately before another row. The decoder consumes their
full length. No JavaScript interpreter, build ID lookup, or regex JSON unescaping is needed.

The Markdown parser follows [[flamecomics]], [[novelarchive]] and [[iken-template]]:
preserve paragraphs and balanced emphasis, escape syntax rather than ordinary punctuation.
Featured descriptions are plain text. Details retain formatting. Split credit separators,
trim and deduplicate per role. Do not replace authors with the site's translation team.

> ⚠️ Material flags do not establish that protection is active. Diva retains encrypted tiles
> and strips while protectionConfig.isProtected is false; the site then uses imageUrl directly.
> Follow that disabled-protection fallback and keep missing-page/redacted/active-protection
> errors explicit. Device rendering and uncommon protected formats remain separate app checks.

## Home

One HTML request supplies row order, headings, links and embedded metadata. Featured slides
use that available metadata without per-card requests. Portrait-cover heroes use BigScroller.
Ordinary rows contain compact Manga links; latest rows use a real accessible chapter number
and timestamp. Ranked rows retain ranking. Duplicate cover/title links collapse within a row.
Empty rows and commercial rows are omitted. Collections, user-specific continuation and
random/recommendation interfaces are not invented as manga feeds.

> ⚠️ A section selector can include its own root. Test nested section wrappers rather than
> assuming `select_first("section")` means a descendant; that mistake removes every row.

Elf currently orders Popular Today, Latest Comic Updates, Top 10 Comics after its hero.
Drake has Latest Novel Updates, Latest Comic Updates, Popular Today, Top 10 in All.
Valir and Kayn include New Series and Editor's Pick before those update/ranking rows.
Diva's current DOM puts its Featured band before New Series; older browser snapshots showed
a different order. Follow document order and current data attributes, not a hard-coded list.

## Evidence and gates

The attached Drake Pages document was decompressed and compared with fresh site data.
Named Keiyoushi, Inkdex, Mana, Kitty and CorruptBytes clients were read as leads; the older
MangaStream Drake implementation and former Kayn API/filter assumptions are stale.
The merged review references above were reread; #900 is still an open comparison.

Portable public-API contracts run on main and clean submission worktrees. They cover reader
metadata collisions, complete pages/errors, novel text references, chapter completeness/locked
settings, request parameters, compact cards, home order, duplicate links and actual timestamps.
The new Vine reader/home cases intentionally reproduce the open upstream reports; existing
Keyoapp/MangaThemesia contracts remain the merged calibration baseline until Vine is merged.
Fork unit and live tests stay on main; the standard aidoku/aidoku-test scaffold stays in submissions.

Direct native requests reached Elf, Drake, Valir and Kayn. Diva returned an environment
"Site Unavailable" page while public browser requests returned working JSON and readers.
The browser also observed its non-auth welcome preference `nh_welcome_seen2`; do not overwrite
the user's cookies or import another browser's authenticated session to work around this.
Build/package/runner evidence does not establish device rendering or cookie parity.


Current local evidence: Drake, Elf, Valir and Kayn live suites passed, including their
available key-only novel readers. Diva's deterministic checks pass, but its three native
live tests fail on the environment's substituted HTML response; do not report them as passing.
The official Diva 320px WebP was downloaded through Firecrawl and reduced to an opaque
128px icon with 8px margin; Kayn uses the verified current 512px site icon with the same margin.


The public AI details endpoint was verified on Elf, Drake, Valir and Kayn. Its `chapters=all`
response returned all 500 Drake novel chapters, oldest first, with chapter_count 500.
The source reverses that known order for the app and requires the received count to match;
missing or duplicate chapters are errors. Details-only uses chapters=none. Combined updates
need one request, so there is no artificial partial result or extra chapter fan-out.
The reader still requires Flight because image URLs and full novel text are not in this API.

Post-publication CI built and deployed all five packages, and passed source rules, lint,
portable contracts and upstream calibration. The scope report passed fix-vine; its aggregate
failure comes from older Raven/Scythe submission refs. Live tests passed Drake, Elf, Valir
and Kayn. Diva home and novel reading passed in CI, but its comic-reader test encountered
the explicit unsupported-image-format error. This is a reader limitation, distinct from
the substituted HTML seen by local native requests, and must not be reported as a passing
Diva comic check. Browser inspection then confirmed the same chapter has protection disabled
and a valid full-image URL despite its material flags. The shared reader now honors that
configuration; the portable regression covers the exact inactive-protection shape.
Reference: https://divascans.org/series/comic/my-twin-sister-stole-my-place-as-a-wife/chapter/24
and the loaded reader bundle /_next/static/chunks/50999.7f63d2af911a0e87.js.

After the correction, the full CI source-test run passed for all five consumers, including
Diva comic pages and key-only novel reading. Seven template regressions and five portable
contracts also passed. Local Rust is now pinned to CI's 1.99.0 so a newer stable formatter
cannot silently produce a different template layout. Device rendering remains unverified.
