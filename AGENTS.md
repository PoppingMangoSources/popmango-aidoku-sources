# Working on these sources

Notes gathered while getting `multi.xcomic` (PR 735), `en.toonkotsu` (PRs 815 and 821) and
the Kagane rework (PR 827) merged into Aidoku-Community sources, plus what kkantan asks of
other contributors. Written down so the next source doesn't have to relearn it.

`AGENTS.md` and `CLAUDE.md` are identical copies of this file. It is the hub: the hard rules,
the upstream rules, the workflow, templates and where to look. Every numbered section keeps its
heading, so "section 6" anywhere in the repository still leads somewhere; a section whose detail
moved says where it went. Read the workflow skill before creating or submitting a source:

- [.claude/skills/aidoku-source/SKILL.md](.claude/skills/aidoku-source/SKILL.md)
- [.agents/skills/aidoku-source/SKILL.md](.agents/skills/aidoku-source/SKILL.md)

| Read | When |
|---|---|
| [`.claude/skills/aidoku-source/references/standards.md`](.claude/skills/aidoku-source/references/standards.md) | before every push of a source: every review rule with its evidence (section 3) |
| [`.claude/skills/aidoku-source/references/conventions.md`](.claude/skills/aidoku-source/references/conventions.md) | before writing code: the API in practice, request cost, merged conventions, framework defaults, comments and naming (sections 2, 4 to 8) |
| [`.claude/skills/aidoku-source/references/testing.md`](.claude/skills/aidoku-source/references/testing.md) | before writing or running tests: coverage and the test runner's differences (section 1) |
| [`.claude/skills/aidoku-source/references/submission.md`](.claude/skills/aidoku-source/references/submission.md) | before a push or an upstream PR: every gate and the submission workflow (section 9) |
| [`.claude/skills/aidoku-source/references/aidoku-host.md`](.claude/skills/aidoku-source/references/aidoku-host.md) | for anything about what the app does with a source |
| [`docs/index.md`](docs/index.md) | before touching a source or template the wiki covers: its traps, and the archives |
| [`.claude/agents/source-auditor.md`](.claude/agents/source-auditor.md) | when an audit is asked for: a read-only agent that checks every line against all of the above |

The two skill trees are physical copies, including their references. After editing either
instruction set, synchronize the copies with `python3 scripts/sync-agent-docs.py` (edit
`AGENTS.md`, `docs/AGENTS.md` and `.claude/skills` first). CI and the pre-push hook fail on drift. This is
intentional duplication so each client can read the full instructions directly. The same
sync covers the docs instruction pair and Claude command wrappers for the `docs-*` skills.
Shared executables, tests and Git hooks stay in `scripts/`, `scripts/tests/` and `.githooks/`.

`docs/` is a wiki of how particular areas work and the traps found there, kept by an LLM as
work touches them. Its format and the `/docs-sync` workflow are in
[`docs/AGENTS.md`](docs/AGENTS.md) / [`docs/CLAUDE.md`](docs/CLAUDE.md); start at [`docs/index.md`](docs/index.md).

Items marked **(verified)** were checked against aidoku-rs, the community sources or
CI config. Items marked **(inferred)** are read from reviewer comments or a single
reference implementation and could be wrong.

---

## 0. Hard rules

The rules missed most often. A configured Claude or Codex hook prints this section again after every compaction; the
rest of this file still applies, so read each section before the work it covers.

- **Commit with `scripts/commit.sh -m "<subject>" -m "<body>"`** (section 9). The configured tool hook
  refuses any other `git commit`, including one inside a script it runs. Conventional subject;
  a one- or two-sentence body, or `- ` bullets for a first `feat: add <id>`. No `#123`, links or
  AI attribution.
- **Every push that changes a published source bumps it** (`scripts/bump.sh <id>`), after
  `scripts/review-source.sh <id>` passes on its current files. Fork-only `src/test.rs` and shared
  `src/source_test.rs` regression changes do not change the published package and need no bump.
- **Before a push or `pr-branch.sh`, read section 3 against the diff row by row.** The hook
  prints it as a checklist at that moment.
- **List what the person asked for before calling it done.** Number each item from their
  messages and name beside it the code, test or output that shows it; an item with nothing
  beside it isn't done. An order or layout they specify gets a `review_` test.
- Read the merged sources built the same way (section 6), and any reference client the person
  names, before writing a feature. `scripts/upstream-usage.py <id>` (or `--feature '<regex>'`)
  names the newest merged sources that already do it, with file and line; match their shape.
- Cards everywhere but `BigScroller` carry key, title and cover, optionally status and rating,
  and never a subtitle (sections 3 and 6).
- Work on `main`; create another branch only when the owner explicitly asks. Requested branches
  are `feat/<id>`, `fix/<id>`, `feat/<template>-template`, `chore/<topic>` or `docs/<topic>`,
  never `claude/`. Pushes to a PR branch use `--force-with-lease=<ref>:<sha>`. Fold updates
  into the requested commit structure by default; a new-source PR stays one commit unless
  the owner explicitly requests separate commits. Force-pushing alone does not squash history.
- Don't open or post on a PR, and don't resolve review threads; draft the text for the person.
- Refer to kkantan, and anyone whose pronouns aren't stated, as they/them: in chat replies,
  summaries and drafts as well as files.
- A mechanical rule that slipped goes into `check-sources.py` with a test, in the same commit
  as its fix (section 9).

---

## 1. Upstream rules

Summarised from `CONTRIBUTING.md` in the Aidoku-Community sources repo and the
official aidoku-rs source development book. Read those before a submission; this is
only a summary.

- **Conventional Commits.** PRs are squashed on merge, so the PR title *is* the
  commit message: `feat: add en.mangago`, `fix(vi.cmanga): update url`.
- **One concern per PR.** Two unrelated sources belong in two PRs. A new template is the
  exception that goes the other way: include the sources that use it, or stack their PRs
  on it, so the template is shown working (#824).
- Compiles with no warnings; `cargo fmt` run; `cargo clippy` clean; every file ends
  with a newline; **JSON indented with tabs**.
- `rustfmt.toml` upstream sets `hard_tabs`, `use_try_shorthand`,
  `use_field_init_shorthand` — match it.

### AI usage — read this before pointing a model at a source

Their CONTRIBUTING explicitly allows AI for assistance or reference, but says it is
**not acceptable when no human thought went into the result** — AI tends to produce
needlessly complex or subtly wrong code, which makes review harder. They'd rather
give review feedback that helps you learn, and point people at `#source-dev` on
their Discord.

So: anything generated needs to be read, understood and cut down before it goes
near a PR. Treat this file as a way to get a draft closer to acceptable, not as a
licence to skip reading the diff.

### Minimum functionality they expect

- `status`, `content_rating` and `viewer` set on details when the site has the data.
  `content_rating` can be derived from tags; `viewer` from manhwa/manhua/webtoon.
- `send_partial_result` in the details path when chapters need another request.
- `DeepLinkHandler` for series and chapter urls where possible.
- Built-in language filtering for multi-language sources — do **not** set the
  language to "multi".
- Every filter the website offers should have a filter in the source.
- Home pages and listings are optional but welcome.

### CI (verified)

Upstream's `pr.yaml` has three jobs, each over what a PR changes (the compatibility
baseline, not a replacement for this fork's additional review gates):

- **Verify** — each changed source: `aidoku package` with `RUSTFLAGS=-Dwarnings`, then
  `aidoku verify`.
- **Lint** — each changed source and template: `cargo fmt --check` and
  `cargo clippy -- -D warnings`; any warning fails.
- **JSON** — tabs only, final newline, for every file under `res/`.

Rust is pinned (`1.99.0`) locally by `rust-toolchain.toml` and in CI; a newer stable formatter
can rewrite unchanged code differently. The CLI comes from `.github/actions/install-aidoku-cli`, which
installs aidoku-rs at its latest commit. This fork keeps packaging/JSON checks in `pr.yaml`, plus a separate `clippy.yaml`
with formatting, warning failures, inline diagnostics and an aggregate tool-version summary.
`check.yaml` adds our source rules, mirror checks, workflow linting and guard regression tests.
Template changes also check their source consumers. Passing these is necessary, but does not
prove the site's behavior or upstream branch isolation.

Locally, from the source directory (the pre-push hook runs exactly these):

```sh
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown -- -D warnings
RUSTFLAGS=-Dwarnings aidoku package && aidoku verify package.aix
```

**Do not add `--all-targets`** (verified) — it pulls in targets with no
`#[panic_handler]` and fails for reasons that have nothing to do with your code.

### Tests (verified)

**Write them for every new source**, in `src/test.rs` with `#[aidoku_test]`, against the live site:
browse paging, every sort and filter, each content rating on its own, a cleared multi-select
setting (shows nothing), a filter overriding its setting, details, a chapters-only update, pages,
deep links and an unknown key, checked against the site's own data. Tests stay on `main`;
`pr-branch.sh` leaves them out of a submission. How to run them, and every place the test runner
differs from the app (web views, `parse_date`, `Element` nodes, `scraper` selectors, Cloudflare),
are in [`.claude/skills/aidoku-source/references/testing.md`](.claude/skills/aidoku-source/references/testing.md).

---

## 2. The aidoku-rs API in practice

Moved whole to [`.claude/skills/aidoku-source/references/conventions.md`](.claude/skills/aidoku-source/references/conventions.md). In short: `Source` and its four functions, `register_source!`, errors, requests, HTML, tag and author
search and the other `source.json` keys.

## 3. Review feedback

Moved whole to [`.claude/skills/aidoku-source/references/standards.md`](.claude/skills/aidoku-source/references/standards.md). In short: Every rule a review comment, an owner's correction or our own miss put in place, each with the
comment quoted and marked **F** (fails `check-sources.py`), **N** (a note) or **R** (read in
review). Read it against the diff, row by row, before every push of a source; the push hook
prints it as a checklist. A new review comment becomes a row there in the same commit as its fix.

## 4. Reviewing behaviour and request cost, not just shape

Moved whole to [`.claude/skills/aidoku-source/references/conventions.md`](.claude/skills/aidoku-source/references/conventions.md). In short: Count the requests per operation, check pagination, filtering, ratings, locked chapters, pasted
links and errors, and probe before concluding. No per-card fan-out.

## 5. Paperback/Inkdex habits, what transfers to Rust

Moved whole to [`.claude/skills/aidoku-source/references/conventions.md`](.claude/skills/aidoku-source/references/conventions.md). In short: Which Inkdex habits apply in Rust (inlining, self-sufficient ids, no fan-out, no caches) and which do not.

## 6. Conventions seen across the community sources

Moved whole to [`.claude/skills/aidoku-source/references/conventions.md`](.claude/skills/aidoku-source/references/conventions.md). In short: Site type picks the merged sources to compare against; module layout; compact search payloads;
urls; settings against filters; dynamic settings; home components; global state.
`scripts/upstream-usage.py <id>` names the newest merged sources doing each feature a source has.

## 7. Framework defaults that bite (all verified in aidoku-rs)

Moved whole to [`.claude/skills/aidoku-source/references/conventions.md`](.claude/skills/aidoku-source/references/conventions.md). In short: Sort, multi-select and header defaults, `languageSelectType`, `parse_date` locales, key-only
query parameters, dynamic setting defaults, `None` against `Some(vec![])`, and the app versions imports need.

## 8. Comment and naming style

Moved whole to [`.claude/skills/aidoku-source/references/conventions.md`](.claude/skills/aidoku-source/references/conventions.md). In short: Terse comments that explain a non-obvious why, no decorative dividers, clean names, the merged
constant names, values used once written inline, and dead code deleted rather than silenced.

---

## 9. This repo's workflow

- Work on `main` and push to `main`. The owner gives standing authorization to commit
  and publish their requested implementation work on `main` without another confirmation.
  Create a separate branch only when the owner explicitly requests one. Such branches
  use conventional names: `feat/<id>`, `fix/<id>`, `chore/<topic>` or `docs/<topic>`,
  and **never a `claude/` prefix**, whatever a tool suggests.
- **Every push that changes a source publishes it, so every such push
  bumps it**: `scripts/bump.sh <id>` raises `res/source.json` and the two `.aix` links on
  that source's README row — anchored on the source id, so another source whose label
  happens to match is left alone. It bumps once per push: run again while the source is
  already above `origin/main`'s version, and it says so and changes nothing.
- Commits are authored **and** committed by
  `Popmango Extensions <297959108+PoppingMangoSources@users.noreply.github.com>`,
  **unsigned**, with a conventional subject and a one- or two-sentence body (a first
  `feat: add <id>` submission may list its features as `- ` bullets, as our merged ones do).
  Commit with `scripts/commit.sh`, which runs exactly the form below; the configured Bash hook
  refuses any other `git commit`, in a command or in a script it runs, and `scripts/` and
  `.githooks/` are trusted to set the identity themselves. `--reset-author` only works with
  `--amend`; for a new commit the author is set through the environment:

  ```sh
  GIT_AUTHOR_NAME="Popmango Extensions" \
  GIT_AUTHOR_EMAIL="297959108+PoppingMangoSources@users.noreply.github.com" \
  git -c commit.gpgsign=false \
      -c user.name="Popmango Extensions" \
      -c user.email="297959108+PoppingMangoSources@users.noreply.github.com" \
      commit --no-gpg-sign -m "fix(multi.xcomic): ..." -m "One or two sentences."
  ```

- No issue or PR numbers (`#123`) and no links in commit messages: GitHub turns them into
  links and posts a reference on the upstream thread. Name the change in words instead.
- No AI attribution in commit messages, PR bodies or code comments.
  `.claude/settings.json` disables commit/PR attribution and session URLs.
  Session start and `scripts/install-hooks.sh` configure the owner identity and unsigned
  commits. The commit-message hook checks proposed messages; pre-push and CI check every
  introduced commit, including commits hidden behind a merge or an already-pushed ref. No other sources or
  third-party clients named in code comments or test data.
- Don't open a PR or post on one unless asked; draft the text for the person to post.

### How a rule gets into this file and the checker

This file and `check-sources.py` are only as good as their least accurate rule. A wrong rule
does damage twice: it fails correct code, and people trust it over the evidence. Every rule here
went in one of these ways, and new ones go in the same way:

1. **A review comment or an owner's correction becomes a section 3 row in the same commit as
   the fix**, quoting what was flagged. A rule from a single PR is marked (inferred) until a
   second review or the implementing code confirms it.
2. **Decide it from the code that implements the behaviour, not the comment's wording.** The
   app decodes `filters.json` in AidokuRunner and the Rust structs live in aidoku-rs; read
   them (both clone anonymously) and cite the file. "Set `can_exclude` to false" on #824 read
   like "the default is true", but the decoder says `false`.
3. **If it can be detected without the network, add it to `check-sources.py` with a test in
   `scripts/tests/test_check_sources.py`**, one case it fails and one legitimate case it
   passes. `scripts/check-rule-tests.py` switches each rule off in turn and fails when no test
   notices; `ci-gates.py` runs it whenever the checker or its tests change.
4. **Calibrate it**: `scripts/calibrate-rules.sh "<rule text>"`. If it fails merged upstream
   code, look at what it fails. Old code kkantan never reviewed is backlog; a recent merge
   that kkantan approved means the rule is too broad, so narrow it (the silenced-warning rule
   allows a commented exception because an untagged serde enum needs its field) or drop it
   (an author-search flag beside an author filter was merged in `multi.nhentai`, so that is
   not a rule). A rule we still want that recent merges break becomes a **note**
   (`note()` in `check-sources.py`): printed as `NOTE`, never failing. Act on each note or say
   why not.
5. **When evidence contradicts a rule, fix the rule in the same commit**, and say where the
   old wording was wrong. Section 2's tag-search paragraph and section 3's tag-style row were
   both corrected that way.

### Gates and upstream PRs

Every repository check (`ci-gates.py`, the hooks, `review-source.sh`, `check-sources.py`,
`review-contracts.py`, `check-workflows.py`, `calibrate-rules.sh`, `upstream-usage.py`,
`check-site-url.py`, `check-host-reference.py`, `check-rule-tests.py`, `source-lints.sh`) and the whole submission
workflow (`pr-branch.sh`, review rounds, updates after a merge, template PRs, scope checks,
rebuilds and leased force-pushes) are in [`.claude/skills/aidoku-source/references/submission.md`](.claude/skills/aidoku-source/references/submission.md). The two
commands to remember: `python3 scripts/ci-gates.py --base origin/main` for the offline gates and
`scripts/review-source.sh <id>` for one source's whole review.

---

## 10. xcomic specifics

Moved to [`docs/subsystems/xcomic.md`](docs/subsystems/xcomic.md). XCOMIC's GraphQL API, editions, feeds and filters; it lives upstream only.

## 11. Toonkotsu specifics

Moved to [`docs/subsystems/toonkotsu.md`](docs/subsystems/toonkotsu.md). Toonkotsu's JSON API, content ratings, the web view reader and read limits; upstream only.

## 12. Templates

Twenty-two of them live in `templates/` upstream (counted 2026-11-20), for sites that share a
common engine or theme:

`madara`, `wpcomics`, `gigaviewer`, `mangathemesia`, `iken`, `mangareader`,
`libgroup`, `mangabox`, `liliana`, `guya`, `mmrcms`, `tukutema`, `ezmanhwa`,
`pizzareader`, `madtheme`, `mangaworld`, `otakuovh`, `vinetheme`, `keyoapp`, `mangak`,
`mangataro`, `zaimanhua`.

`toonz` (Toonz, Toonx and LewdManhwa) is ours alone so far, on `feat/toonz-template`;
`mangataro` merged with #911 and left `main` for `merged-sources` with MangaTaro and Rolia Scan
(section 17).

Most templates track upstream. `madara` and `madtheme` carry additions our sources use
(shared detail parsing, listings and dynamic filters). `mangak` merged with #850 and
is upstream's copy again; its consumers live upstream only. `mangathemesia` adds shared novel
reading and an opt-in `child_theme` module for Violet, Thunder, Madara and Eva; their source
overrides select its home and deep-link handlers. It stays on `main` with its loopback fixtures
for future consumers, though Scythe Scans, its last one, moved to `merged-sources`; `iken` left
with its six consumers and `vinetheme` with its five (section 17). The default home and deep-link handlers remain
upstream's for the other consumers. KingOfShojo and RokariComics merged with the original template
unchanged (section 17). The `madtheme` additions (a home page and a chapter-list fallback)
were for Kaliscan, which now sits on
the `dead-sources` branch with Galaxy Manga, OManga and LikeManga (its site went offline). When syncing, take upstream's files for every other
template and preserve the fork additions to `madara`, `madtheme` and `mangathemesia`.

**Check for one before writing a source from scratch.** Two ways: see whether the
matching keiyoushi extension declares a `themePkg`, or run the site through a
WordPress theme detector.

A template exposes a `Params` struct for per-site configuration and an `Impl` trait
carrying the default behaviour. A consuming source implements `Impl` with `new` and
`params`, overriding only what differs, then registers the template wrapping itself:

```rust
impl Impl for ElfToon {
	fn new() -> Self { Self }
	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			chapter_list_selector: "#chapterlist li:not(:has(.gem-price-icon))".into(),
			..Default::default()
		}
	}
}

register_source!(MangaThemesia<ElfToon>, Home, ImageRequestProvider, DeepLinkHandler);
```

That, plus a path dependency on the template in `Cargo.toml`, is the consumer wiring;
current `en.elftoon` uses VineTheme, not MangaThemesia. If a new source starts looking like an existing template
with different selectors, it should probably be a template consumer, or a new
template if nothing fits.

---

## 13. Where to look

No links here on purpose — search the Aidoku-Community sources repo by these names.

**Reviews worth rereading before submitting.** PRs 726, 531, 624, 637, 646, 690 (two
rounds), 603, 824, 828, 833, 835, and our own 735, 815, 821 and 827. Recurring themes across them: consume owned values instead of
cloning, inline single-use helpers, use the API's own ordering, avoid unnecessary
dynamic settings, keep search results compact, send partial chapter results,
construct queries correctly, and handle only deep links you can actually resolve.
PRs 191, 237, 277 and 309 are the older BatoTo submissions — useful history, but
current sources and current reviews win where they disagree.

**Sources worth reading for structure.** `multi.mangadex` (auth, custom lists, home
with partial results), `ru.senkuro` (GraphQL in its own module, compact search
query, one named query per home section), `zh.komiic` (GraphQL), `multi.kagane`,
`multi.mangadotnet`, `multi.mangafire` (JS deobfuscation), `multi.mangaplus`
(`PageImageProcessor`), `en.comix` (exclusion-style settings, webview),
`en.weebcentral`, `en.chikari` (novels alongside manga), `multi.ehentai` (separate
listing and detail types), `multi.cubari` (`RefCell` state), `zh.zaimanhua`
(legitimate `DynamicSettings`), `zh.noyacg` (the site gates adult tags),
`multi.lanraragi` and `ja.soraraw` (tag search). `en.elftoon` is 28 lines — a template consumer, and
the fastest way to see how thin a templated source can be.

**The reviewer.** kkantan reviews most of these. Reading their own commits and past
review comments is the fastest way to predict what they'll ask for — the patterns
in section 3 came from doing exactly that.


---
## 14. Kagane specifics

Moved to [`docs/subsystems/kagane.md`](docs/subsystems/kagane.md). Kagane's mirrors, chapter index, ratings and search body; upstream only.

## 15. Evidence before submission

Read [references/aidoku-host.md](.claude/skills/aidoku-source/references/aidoku-host.md) for
what the app actually does with a source — how AidokuRunner loads and calls it, every import,
the JSON defaults, and what each home component renders — with each claim tied to the file it
comes from. Read the synchronized skill reference
[references/source-review.md](.claude/skills/aidoku-source/references/source-review.md)
for official code locations, runner limitations, reviewer comment permalinks and the test
matrix. Use the `.agents/skills/aidoku-source/references/source-review.md` copy from Codex.

- Keep **source-code facts**, **live API observations**, **runner results** and **in-app
  results** separate in the review summary. A build is not an end-to-end source test.
- Use the merged `en.toonkotsu`, `multi.xcomic` and Kagane rework as accepted patterns for
  the behavior they actually implement. Their merged status is not a permanent guarantee
  about every API, nor a reason to copy old code without checking the current library.
- Review complete source branches against fresh upstream before pushing; review main
  publication changes against the fork. Report both boundaries explicitly when both change.
- Answer each reviewer thread with a code change or a reasoned reply. Verify the final
  code before calling a comment addressed. Do not mark discussions resolved for the person.
- Run `scripts/verify-source.sh <source-directory> [--test]` for fmt, warning-failing
  Clippy, packaging and schema/WASM verification; `--test` additionally runs aidoku tests.
  Install `aidoku-test-runner` first. Use tests for meaningful parsers, key/url mappings,
  pagination, cleared settings and error behavior. Live tests are opt-in because connectivity
  and site protection are environment-dependent; never silently exclude failures.
- For WebView/cookies/reader rendering, use an actual Aidoku device or simulator. The Rust
  test runner has stubbed WebView imports. The app's `AidokuTests/Support/TestableSource.swift`
  is an app test double, not an executable harness for arbitrary third-party source WASM.
- Caching, `sccache`, artifact retention and compiler versions affect speed/diagnostics;
  none establish correctness. Do not restore an old cache or compiler action as a substitute
  for a missing behavioral or scope check. Preserve diagnostics when a build fails.

Live CI failures retain their raw logs. `scripts/test-sources.py` runs non-review tests after
contracts and reports observed Cloudflare 403/HTTP 522 evidence without suppressing failures.
An unknown-series test accepting any error can pass on a blocked request; that pass does not
establish correct missing-series handling.
## 16. Atsumaru specifics

Moved to [`docs/subsystems/atsumaru.md`](docs/subsystems/atsumaru.md). Atsumaru's Typesense search, feeds, ratings, deduplication and novel keys; upstream only.

## MangaBall

`multi.mangaball` (mangaball.com) is a JSON API behind Cloudflare, which refuses the sandbox:
its live tests have not run, and its models were checked against the owner's captures. The
endpoints, content modes, language codes and every unverified parameter are in
`docs/subsystems/mangaball.md`; read it before changing the source. It is rated 1 because the
site has a Hentai theme. It merged upstream as #902 on 2026-11-14 and then left `main` for
`merged-sources` (section 17), with its tests; it matched the merged source apart from its
version, `src/test.rs` and the tests' `serde_json` dependency. A later change starts from
`upstream/main` and goes up with `pr-branch.sh multi.mangaball --update …`.

## VyManga

`en.vymanga` (mangavyvy.com, mirror mangavyvy.net) is plain server-rendered HTML that the
sandbox reaches, so its live tests run. Its chapter links are encrypted anew on every load and
expire, so chapters are keyed by name and `get_page_list` reads a fresh link from the series
page. The probed search parameters, the ignored `search_po`, the genre lists and the icon's
origin are in `docs/subsystems/vymanga.md`; read it before changing the source. Its catalogue
rating is 2, the owner's explicit request on November 17; per-title ratings still come from the
site's adult flag and genres. A submission
branch, `feat/en.vymanga`, is kept from `upstream/main` while it is prepared for a PR.
## Iken maintenance

Moved to [`docs/subsystems/iken-template.md`](docs/subsystems/iken-template.md). The template, its six consumers and EternalMangas are on `merged-sources`; later work starts from upstream.

## 17. Sources kept off `main` while upstream

A source leaves `main` once it is upstream, or while its upstream branch waits on review; the
`merged-sources` branch keeps its last copy with tests. The procedure (archive first, then remove;
`scripts/check-archives.py`), the table of every archived source with its last copy, and how a
later change starts from upstream's crate are in [`docs/operations/archives.md`](docs/operations/archives.md).
