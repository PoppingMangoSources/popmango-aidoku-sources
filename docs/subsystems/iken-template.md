---
title: Iken template and consumers
type: subsystem
created: 2026-11-11
updated: 2026-11-12
sources:
  - code: templates/iken/src/lib.rs
  - code: templates/iken/src/imp.rs
  - code: templates/iken/src/models.rs
  - code: templates/iken/src/helpers.rs
  - code: templates/iken/src/home.rs
  - code: templates/iken/src/markdown.rs
  - code: templates/iken/src/test.rs
  - code: templates/iken/src/source_test.rs
  - code: sources/en.hivescans/src/helpers.rs
  - code: sources/es.eternalmangas/src/helpers.rs
  - code: sources/es.eternalmangas/src/home.rs
  - code: sources/es.eternalmangas/src/test.rs
  - code: sources/es.eternalmangas/res/source.json
  - code: sources/en.hijalascans/src/lib.rs
  - code: sources/en.sanascans/src/lib.rs
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/lib-multisrc/iken
  - ext: https://github.com/LucifersCircle/pirate.vodka-extensions/tree/0.9/stable/src/Hijala
  - ext: https://github.com/kittycatgit/kitty-extensions-0.9/tree/main/src/SanaScans
  - ext: https://github.com/Aidoku-Community/sources/commit/a5b9802a25780ba5db5dbe2590793e88bfd3d355
  - ext: https://github.com/Aidoku-Community/sources/commit/d88a384f99bcccff7e2e4340b5b19bd6ea66210f
  - ext: https://github.com/Aidoku-Community/sources/commit/289b5613bb47f62c6eb91de3d8b2282643586533
  - ext: https://github.com/Aidoku-Community/sources/commit/79f1619cc0d14e853619b4e13d48b5331cd099b1
verified_at: 67021b2
---


Iken serves Hive, Nyx, Vortex, Magus, Hijala and Sana through the Params/Impl arrangement
used by [[mangak-template]]. Eternal now has an independent current-site HTML implementation.
The production-only `fix-iken` submission starts from canonical `289b5613`; its five commits
separate shared fixes, Eternal migration, novels, Hijala and Sana. Fork tests and tooling stay on main.

## API, keys and current controls

The shared API uses `/api/query`, `/api/post`, `/api/chapters` and `/api/chapter`.
Numeric library keys survive; home paths resolve by slug, including complete URL-decoded chapter
suffixes. Numeric chapter-only updates avoid a details request. Reader calls need no cached metadata
(`templates/iken/src/imp.rs:130`). Checked responses retain bounded status/body diagnostics.

> ⚠️ Post details can omit chapters despite a nonzero total. Collect skip/take chapter pages until
> totalChapterCount is reached; empty or overlapping pages must error. Independent lock flags stay
> independent; accessible overrides only the price fallback (`templates/iken/src/helpers.rs:60`).

Hijala and Sana use the public API directly; no JS state decoder or cache is necessary.
Current site controls include sort, tri-state genres, single status, multiple types, availability,
chapter count and creation-year ranges. Firecrawl browser requests verified saleFilter=on_sale /
not_on_sale, comma-separated seriesType and single seriesStatus. Positive and impossible API probes
established actual narrowing and genre exclusion. Existing four consumers retain include-only controls.
Sana sets page_size=30 and filename page sorting, matching its current Keiyoushi override;
Hijala uses 18 results. Both use official site icons with an opaque 128-pixel canvas and margin.

> ⚠️ A full-catalogue first response is terminal even when it exceeds perPage. Compare response
> length with totalCount as well as the requested offset (`templates/iken/src/imp.rs:75`).

## Home and novels

Home concurrently requests HTML, createdAt New Releases, NOVEL results and one metadata batch.
HTML determines the existing section order and real chapter rows. Every Latest Releases and
Latest Updates component uses MangaChapterList with page_size=5 when chapters are present.
Pinned entries absent from the first metadata page survive with readable path keys, chapter
numbers and HTML timestamps. Relative footer dates are supported, and multiple chapter updates
for the same series remain separate (`templates/iken/src/home.rs:170`).
Optional batch enrichment may replace a matching chapter with numeric IDs; a missing match must
never downgrade a chapter row to covers. There is no per-card request fan-out. Hive alone keeps its
custom script banners. Popular rows use three entries per page. Ordinary cards stay compact.

> ⚠️ Bound each heading's ancestor traversal before another section. A blank row must never borrow
> the next row's cards; the deterministic home tests pin these boundaries.

> ⚠️ Novel paragraphs and inline emphasis remain separate and balanced. Escape Markdown syntax,
> not ordinary punctuation. All consumers declare minimum app 0.8.3 for child_nodes imports.
> Actual reader rendering remains a device check; see [[flamecomics]].

## Eternal migration

The former JSON catalogue is stale: the reported miracle title is absent, while the current site
has chapters. The reported divorce title has more chapters on the site than its old API record.
The new implementation reads current HTML catalogue, details, chapters and reader content.
Old numeric keys use the API only to recover the slug, then load current HTML. They retain their
original library key (`sources/es.eternalmangas/src/helpers.rs:40`). Current descriptions use the
site's displayed synopsis, avoiding literal escaped quotes and carriage-return strings.
The original EM logo replaces the old icon. Catalogue contentRating is **2**, explicitly requested
on November 12; this supersedes earlier rating-1 instructions. Individual title ratings remain separate.

> ⚠️ Search results being plausible does not establish API freshness. Keep positive regression
> fixtures for both reported titles and compare chapter inventories with current HTML.

> ⚠️ The .org site can redirect through sistemasoperativos.info. Keep canonical series paths in
> keys and .org web links. Reader images use original data-src before src, never placeholder pages.

The current HTML archive supports query/type/status/one genre, with no visible sort control.
Removed old API sort options and genre IDs must not be restored without fresh evidence.

## Review evidence and verification

The earlier audit against merged Keyoapp, Flame, MangaK and MangaThemesia remains recorded in
history. Applied patterns include fallible identities, per-role unique credit names, short/HTML
synopses, compact Link::from cards, local Params differences and a separate Markdown reader.
See root section 3 for the individual review rules and [[source-checks]] for automatic gates.
Named Hijala and Sana clients plus Keiyoushi were read, then their endpoints and controls checked
against current HTML, API pages and browser network requests. Their caches and state parsers are
not needed for the verified public endpoints.

Deterministic template tests cover chapter rows, pinned metadata misses, repeated-series updates,
relative timestamps, credit splitting, synopses, identities, locks and novel formatting. Loopback
HTTP tests cover errors, complete decoded chapter paths, key-only reading, filename ordering and
pagination failures. Optional captured-home tests require AIDOKU_IKEN_EVIDENCE_DIR; a missing
directory does not count as captured-site evidence. Live home suites check every latest component.

Full live suites passed for Eternal, Hive, Vortex, Magus, Hijala and Sana. Nyx passed four tests,
but the sort/pagination suite timed out on a network request, including a targeted retry.
This does not establish a global outage and does not justify removing the assertion or hiding
its failure. Deterministic checks and publication gates remain separate from live availability.
No runner result establishes app selector parity, cookies, WebView sessions or device rendering.

| Consumer | Fork version | Submission version | Catalogue rating |
| --- | --- | --- | --- |
| Hive | 11 | 3 | 0 |
| Nyx | 10 | 2 | 0 |
| Vortex | 12 | 4 | 0 |
| Magus | 11 | 3 | 0 |
| Eternal | 12 | 3 | 2 |
| Hijala | 1 | 1 | 0 |
| Sana | 1 | 1 | 0 |
