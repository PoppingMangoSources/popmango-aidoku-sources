---
title: VineTheme template and consumers
type: subsystem
created: 2026-11-13
updated: 2026-11-20
sources:
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/38322e3397255b719c9952de186b3bd0119e1c14/templates/vinetheme/src/lib.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/38322e3397255b719c9952de186b3bd0119e1c14/templates/vinetheme/src/imp.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/38322e3397255b719c9952de186b3bd0119e1c14/templates/vinetheme/src/models.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/38322e3397255b719c9952de186b3bd0119e1c14/templates/vinetheme/src/home.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/38322e3397255b719c9952de186b3bd0119e1c14/templates/vinetheme/src/helpers.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/38322e3397255b719c9952de186b3bd0119e1c14/templates/vinetheme/src/rsc.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/38322e3397255b719c9952de186b3bd0119e1c14/templates/vinetheme/src/markdown.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/38322e3397255b719c9952de186b3bd0119e1c14/templates/vinetheme/src/settings.rs
  - ext: https://github.com/PoppingMangoSources/popmango-aidoku-sources/blob/38322e3397255b719c9952de186b3bd0119e1c14/templates/vinetheme/src/test.rs
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
verified_at: 38322e3
---

> ⚠️ The VineTheme template, Drake Scans, Elf Toon, Valir, Diva and Kayn left main for
> [merged-sources](https://github.com/PoppingMangoSources/popmango-aidoku-sources/tree/merged-sources)
> after #908 merged, from main `38322e3` with their tests. Later maintenance starts from
> upstream's crates and restores only the needed fork tests.

> ⚠️ #908 merged on 2026-11-16 as one squash of our five commits. kkantan left eight
> comments, listed below, and pushed no commit. Our crates matched the merged ones apart
> from tests and versions, checked on 2026-11-17. The rules are in AGENTS.md section 3.

## kkantan's review comments on #908

Read from the review API on 2026-11-17, with what we did (our replies on the PR).

| Where | Comment | What we did |
|---|---|---|
| `templates/vinetheme/src/home.rs` | "I think the "mo" suffix may need to be handled as well, looking at the valir scans home page as an example." | Added `mo`; the sites print m, h and d up to 29 and then mo, so `w` was dropped |
| `templates/vinetheme/src/home.rs` | "this is only called after an html `text` call, so this is unnecessary." (a `trim()`) | Removed |
| `templates/vinetheme/src/imp.rs` | "you can reverse the iterator below before collecting it rather than the vector beforehand." | Reversed the iterator |
| `templates/vinetheme/src/imp.rs` | "you could keep the `push_encoded` calls since they are more performant, and encoding the static values here is not necessary." | Kept `push_encoded` for `limit`, `page` and `contentMode` |
| `templates/vinetheme/src/imp.rs` | "this change isn't necessary. you're unwrapping `query` from the optional, then immediately re-wrapping it." | The `Option` passes straight through |
| `sources/en.elftoon/res/source.json` | "this should not be removed." (`breakingChangeVersion: 3`) | Restored |
| `sources/en.divascans/res/filters.json` | "would it be better to use the "range" type filter, like the atsumaru source does for chapter count filtering?" | One range filter on all five sources, starting at 1 because the API ignores 0 |
| Review summary (changes requested) | "it looks like valirscans may separate authors by `/`. for example, `The Convenient Empress Resigns`." | Authors split on `/` as well |

kkantan approved on 2026-11-16 with no comment.

VineTheme serves Drake, Elf, Valir, Diva and Kayn. The owner requested publication on main
and a clean `fix-vine` template submission, with the new sources in separate commits.
All consumers require app 0.8.3 because the shared novel parser walks text nodes.
The Iken review's test scaffold, isGenre, item spacing, scoped selectors and independent
migration lessons remain in the root rules and [[source-checks]].

## Current site contracts

Tag labels from series names and slug fallbacks are title-cased for display on home,
details and dynamic filters. Filter requests retain the site's original facet slugs.

The public catalogue is `/api/series`; `/api/series/facets` returns current genres and tags.
Details and complete chapters use `/api/ai/series/{canonicalSlug}?chapters=none|all`.
> ⚠️ Every Valir novel has an API slug different from its public `urlSlug` (`…-novel`, `…-2`),
> and a novel's `urlSlug` can equal its comic's, where `/api/ai/series/{urlSlug}` answers the comic.
> The reader (`/series/novel/{slug}/chapter/{n}` with `rsc: 1`) answers only on `urlSlug`; the API
> slug gives a page without reader data ("Missing chapter/isUnlocked/isLocked"). So keys keep
> `urlSlug`, and details that answer a comic for a novel key read the API slug from the novel's
> own page (`/series/novel/{urlSlug}`, whose series object carries both slugs).

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
parameter was ignored on observed endpoints. The owner requested no adult visibility toggle
or local adult filtering because the sites have no such control. Drake, Elf, Valir and Kayn retain
contentRating 1; Diva uses 2 by the owner’s November 14 instruction. Individual titles
keep their ratings. Source settings default Show Locked
Chapters off and Label Novels on where a mixed catalogue currently exists. Commercial sale/purchase filters are omitted.

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
a different order. The shared parser follows document order; Valir, Diva and Kayn then apply
the owner's row order with `home::reorder`, which drops any row it does not list.

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

> ⚠️ An HTML synopsis can be nonempty markup but render to no text. The shared converter
> returns that empty text normally; the series conversion omits it and the reader rejects it.
> Portable contracts cover valid cards/details with empty synopses and empty novel failures.
> Failed pasted-series lookups fall through to the original search query, matching merged MangaBall.

The final review removed the one-use reader request wrapper and its unconditional Flight
body clone, an unused ChapterDto Clone derive, and per-card/per-chapter setting reads. The
existing template/consumer split and terse parser comments match the merged patterns.

Final review rerun: eight template regressions, five Vine public contracts on both main and
submission trees, merged Keyoapp/MangaThemesia calibration, all five package baselines and
151 repository guard tests pass. Drake, Elf, Valir and Kayn full live suites pass. Diva's
three local live checks fail on HTML instead of JSON/Flight; direct requests show the
substituted Site Unavailable response and Firecrawl reaches its welcome page. This rerun
does not establish a new Diva live pass or device rendering. No session cookies were changed.

> ⚠️ Do not restore showAdult or hide adult-tagged cards locally for these five Vine sites.
> The owner requested the site catalogue with rating 1, except Diva at 2. Portable fixtures
> include adult-rated search and home cards to prevent silent filtering from returning.

Valir's source implements its own home order: Trending BigScroller, Popular Today,
Latest Comic Updates (4), Novels, Latest Novel Updates (4), Editor's Pick BigScroller,
and Top 10 in All (3). Three independent requests run concurrently: home HTML,
`/api/series?sort=trending&limit=24&page=1&contentMode=all`, and
`/api/series?type=Web+Novel&limit=24&page=1&contentMode=all`. The novel listing stores
`type=Web%20Novel` so the shared URI decoder passes a space to QueryParameters.
The live homepage's featured stack renders only its active slide as links; its Flight
`series` array supplies every pick, with viewCount and no createdAt/lastChapterAt.
The generic parser handles that shared widget; site-specific ordering stays in consumers.
Drake moves Popular Today immediately after Latest Novel Updates via its parse_home override.
Browser evidence and the Valir runner confirm the current site sections. Deterministic
regressions cover all stack slides, exact consumer ordering, page sizes and compact novels.


Diva uses Featured, Popular Today, Latest Comic Updates (4), Latest Novel Updates (4),
New Series and Top 10 in All (3). Kayn uses Editor's Pick, Popular Today, Latest Comic
Updates (4), New Series, Latest Novel Updates (4) and Top 10 in All (3). Kayn's current
site has only one New Series feed; it is not duplicated. Both discard the untitled popular
hero and reuse the shared reorder function, as does Valir. No per-card requests were added.

Home descriptions may be Flight text references such as `$99`. Resolve them before plain
text conversion, and reuse richer embedded records for partial hero slides. The shared
Flight row iterator honors byte lengths, multibyte content, newlines and adjacent text rows.
Missing optional references omit the synopsis while keeping the valid card. Regressions
cover every one of these shapes, including JSON-string references and complete stack picks.

Kayn's icon now uses an opaque white background. Valir's sharper icon comes from its current
official `/icon-192.png`, normalized to 128px with margin. Final local evidence includes
passing Valir and Kayn homes and description-reference contracts; Diva's local live suite
still receives substituted Site Unavailable HTML, so its live rendering remains unverified.

## Maintenance rules

Empty HTML
synopses remain optional; validate nonempty chapter text at the reader boundary. A failed
pasted-link lookup falls through to ordinary search. Borrow decoded raw Flight text instead
of cloning a whole reader body through a single-use wrapper. Select the actual
reader object rather than chapter metadata; resolve byte-length Flight text rows, preserve
legacy comic keys and typed novel paths, and test home order/deduplication without per-card
requests. Run the portable Vine contracts against clean submission worktrees as well as main.
Keep the Iken review rules in [[iken-template]]: standard test scaffolds, isGenre, separated Rust items,
scoped selectors and independently scoped migrations.

> ⚠️ Vine consumers keep contentRating 1 without an adult visibility switch or local adult filtering,
> matching the sites. Do not invent a Show 18+ Content control when the site has none; retain
> per-title content ratings. The owner explicitly chose this over the general toggle-default policy.
