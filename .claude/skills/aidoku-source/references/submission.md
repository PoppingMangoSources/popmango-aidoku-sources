# Gates and upstream submissions

The `### Gates` and `### Upstream PRs` parts of section 9 of the root `AGENTS.md` / `CLAUDE.md`,
kept here whole: every repository check and what it catches, and how a source or template goes
upstream, is reviewed and is updated after a merge.

## Gates

- Run `python3 scripts/ci-gates.py --base origin/main` for the offline repository gates.
  CI and packaging invoke the same checks. Pre-push also runs instruction-copy checks,
  script regressions and workflow checks even when no source changed. Rust source gates
  and honest reporting of blocked live/device checks still apply.
  A Stop-hook execution error blocks completion rather than silently reporting success.


- `scripts/install-hooks.sh` once per clone from the tooling checkout on `main`. It installs
  an absolute hooks path so the same hook works in clean upstream worktrees; keep that
  checkout available. Scope validation runs before source/toolchain checks. The pre-push hook runs CI's lint and
  verify commands (section 1) for every source and template the push changes, then
  `scripts/check-sources.py`.
- `scripts/review-source.sh <id>` is the one review command for a source: the gates plus live
  tests (`verify-source.sh --test`), `check-sources.py --base origin/main`, `check-site-url.py` (the address must not redirect to another host), `upstream-usage.py`, the
  upstream scope of its `feat/`/`fix/` branch, and the section 3 rules printed as a checklist to
  read against the diff. It records a stamp of the source's files under `.git/aidoku-review/`.
- `.claude/settings.json` and `.codex/hooks.json` call the same shared handlers
  (`scripts/agent-hooks.py`). Before the first edit to a source or template in a session, the agent is pointed at the skill, section 6
  and `review-source.sh`. When it tries to finish, the Stop hook blocks while a changed source
  fails `check-sources.py` or has no passing review of its current files, so any edit after a
  review needs another one. A second stop in the same turn only reports, so a live-site failure
  can be explained rather than loop. The Bash hook refuses a `git commit` outside
  `scripts/commit.sh` (it also reads the scripts a command runs, except `scripts/` and
  `.githooks/`), prints the section 9 commit rules beside each `commit.sh`, and prints section 3
  as a checklist, one line per rule, before a `pr-branch.sh` or a `git push` that changes a source
  or template (the full rows overflow a hook's inline output). An edit that writes a home component
  gets the section 6 home rules once per source, and after a compaction the SessionStart hook
  prints section 0, since a summary keeps only a pointer to this file. Personal settings, such as a plugin's permission lists,
  go in `.claude/settings.local.json`, which is ignored. Codex hooks require a trusted project
  and a runtime that loads local command hooks; cloud orchestration does not run them. Codex
  patch edits use `tool_input.command`, which the shared handler reads for every affected file.
  Run `scripts/install-hooks.sh` once and the documented review gates explicitly wherever
  session/tool hooks are unavailable. Git hooks and CI remain shared enforcement. Tests are
  in `scripts/tests/test_agent_hooks.py` and `scripts/tests/test_agent_docs.py`.
- The pre-push hook also runs each changed source or template's `review_` tests (`cargo test review_`), which
  make no requests, and fails when `aidoku-test-runner` isn't installed. The live suite stays out
  of the hook, since a site outage would block every push.
- `scripts/check-sources.py [--base <ref>] [ids…]` checks the rules that need no network:
  tab-indented JSON, final newlines, no `"multi"` language, `minAppVersion` present,
  `breakingChangeVersion` in `config`, a 128×128 icon, redundant filter defaults, check
  filters left in the header, decorative dividers, `page_size` above 5, chapter `language`
  on single-language sources, `defaults_get` outside `settings.rs`, the review-comment
  patterns section 3 lists (an empty-query test, a neither-flag guard, a hand-built
  `AidokuError`, `Unimplemented`), a home-row `Link` given a subtitle, a series' full details
  fetched inside search, an `allow`/`expect` that silences an unused-code warning without a
  comment above it, `usesTagStyle` on a JSON genre filter, a `Cargo.toml` without upstream's
  release profile, an `#[aidoku_test]` that asserts nothing, README row in step with `source.json`, a version bump
  since `<ref>`, and byte-identical `AGENTS.md`/`CLAUDE.md` plus matching physical skill trees. CI (`check.yaml`) runs it on the
  sources each push changes. Run with no arguments it reports the backlog in older sources —
  fix those when you're in the source anyway. It also fails an endpoint constant used only with
  a query, a constant sent once as a query value, an adult-hiding switch that shows adult
  titles by default on a source rated 1, a Hentai genre on a source rated 0, and a `Cargo.lock` pinning an aidoku-rs from before
  `ae6485e`, whose `bail!` fails Rust 1.99 under `-Dwarnings` as a match arm. Upstream moved every
  lock in #807; ours move when a source is next edited (`cargo update -p aidoku -p aidoku-test`),
  since a sweep would first need each old source's backlog fixed. It notes the section 3 **N** rows (page-size names,
  own-field wrappers, single-call helpers used once, full-path calls, a sort filter that isn't
  first, an unsized image scroller, an Option unwrapped and wrapped again, a stable sort without a comment saying why, a `?` appended
  to a url variable) and a non-genre multi-select that leaves `canExclude` unstated.
  From sections 3 and 7 it also fails a `push(name, None)`, a `From` into `Manga` or `Chapter`
  that defaults its key, a `parse_date` of month or day names without a locale, a response body
  parsed with `serde_json` rather than `json_owned`, a Rust genre multi-select without
  `uses_tag_style: true`, a remapped `Request` error, a "Show Locked" switch on by default
  without a login, and a chapter escaped whole into one `PageContent::text`. With `--base`, a template
  the push changes gets the same per-file Rust rules as its sources. It notes `!included.is_empty()` and slicing off a prefix by its length, both
  in recent merges (`multi.xcomic`, `multi.sumidoku`, `multi.mangadex`).
- `scripts/review-contracts.py --repo <candidate> [--crate sources/<id> | --base <ref>]`
  runs portable Keyoapp/MangaThemesia contracts against the candidate's actual template and
  pinned SDK. The fixture server is loopback only; source files stay untouched. Installed
  push hooks and `review-source.sh` invoke it, including clean submission worktrees without
  fork tests. Source tests CI runs it for selected consumers/templates and calibrates edited
  contracts against fresh canonical upstream, retaining Keyoapp coverage after archiving.
  Missing toolchains, compilation errors and assertion failures fail the gate. No live-site
  fallback or ignored cases. See `docs/operations/source-checks.md` for the coverage matrix.
- Template-only edits select every current consumer for source checks and package verification;
  a consumer does not need its own edited file to require a release bump on fork main.
- `scripts/check-workflows.py [--offline]` reads every `uses:` action's runtime from its
  `action.yml` at the pinned ref and fails a deprecated Node, and fails a workflow that installs
  `aidoku-test-runner` without the font headers, or that lints (`lint-crates.sh`, `cargo clippy`,
  `cargo fmt`) without `components: clippy, rustfmt` on its pinned toolchain: `build.yaml`'s lint
  step once failed on that. `check.yaml` runs it on every push and
  **Action runtimes** (`action-runtimes.yaml`) every Monday, since deprecations arrive without a
  push. Its tests are in `scripts/tests/test_check_workflows.py`.
- `scripts/check-rule-tests.py` disables each `fail()` and `note()` in `check-sources.py` in turn
  and runs its tests; a rule whose tests still pass has no case that can fail, and the script
  fails naming it. Rules for single reviewed sources archived off main are listed in its
  `ARCHIVED`. `ci-gates.py` runs it when the checker or `test_check_sources.py` changes (about
  20 seconds); its own test is `scripts/tests/test_check_rule_tests.py`.
- `scripts/calibrate-rules.sh [rule text]` runs `check-sources.py` over every merged upstream
  source and counts failures per rule (or lists the sources one rule fails). CI's
  `check.yaml` prints it in the job summary on every push.
- `scripts/upstream-usage.py <id>` first lists each feature the source (and its template) has,
  from deduplication and partial results to date parsing, status mapping and web views, with the
  merged sources that do the same, newest-changed first and with file and line; `--feature
  '<regex>'` does the same for something about to be added (the design of the Paperback repo's
  `upstream-usage.mjs`). Then it lists every aidoku item the source uses and every name it
  defines that few or no merged sources share. For each, either the site needs it or there is
  a usual equivalent among sources built the same way (section 6): `API_URL` for the API base
  (20 sources), a fetch function for a filter list (`en.toonkotsu`'s `listed_facets`),
  `SearchParams`/`BrowseParams` for search parameters, and `ErrorResponse` for an API error.
  An equivalent isn't always better: `en.atsumaru` keeps a `PageSetting` for its Hidden Tags
  (no merged source uses one) and "Excluded Scanlators" rather than MangaDex's "Blocked
  Groups", because the owner found them clearer in the app. Run it before a submission, and
  put each swap to the person.
- `scripts/check-host-reference.py` compares the files `references/aidoku-host.md` cites in
  AidokuRunner, the Aidoku app and aidoku-rs with each repository's `main`, and lists the ones
  that changed. **Host reference** (`host-reference.yaml`) runs it every Monday and when the
  reference changes; when it fails, re-read the changed file, correct the reference, then run
  the script with `--update`.
- `scripts/source-lints.sh` holds the dead-code lints (section 8) that `verify-source.sh` and
  CI's `lint-crates.sh` add to every source's clippy run, templates excepted.
- Before pushing a source with tests, run `cargo test` in it; the hook doesn't, since the
  tests need the live site. **Source tests** (`test.yaml`) runs them after the push (section 1).

## Upstream PRs

- `scripts/new-source.sh <id> <Name> <url>` scaffolds through `aidoku init`, removes the
  nested repository it creates, and prints the README row.
- `scripts/pr-branch.sh <id> [--push]` builds `feat/<id>` from `upstream/main` in a
  separate worktree: the source copied from a committed snapshot (`HEAD` by default;
  `--from <ref>` explicitly selects another) (`.cargo`, `Cargo.toml`,
  `Cargo.lock`, `res`, `src` — no `package.aix`, no README), `version` set to **1**, one
  `feat: add <id>` commit, gates run. It refuses if the branch touches anything outside
  the source or differs from the selected snapshot beyond the version. Our `main` keeps its own
  incrementing version, since our users have it installed and a version going
  backwards means they never get an update.
- A new template goes up as `feat/<name>-template` with the sources that show it working:
  `scripts/pr-branch.sh --template <name> <id>... --body "<template>" --body "<id>"... [--push]`
  (`scripts/template-branch.sh`) makes one `feat: add <name> template` commit, then one
  `feat: add <id>` commit per source in the order given, each without fork tests, formatted and
  at version 1. It runs the template's fmt and Clippy, every source's scope, rules and package
  checks, refuses a crate that drifted from the snapshot, preserves an existing branch as
  `chore/<name>-template-before-rebuild-<sha>` and pushes with a lease. Owner's call for
  Toonz on 2026-11-22: separate commits rather than one. Owner's correction on 2026-11-23:
  fold shared template changes into the first template commit, then keep one commit per
  consumer. A new template's later consumer commits must not modify the template.
- An upstream worktree has no `rust-toolchain.toml`, so cargo there falls back to the machine's
  default Rust. `scripts/toolchain.sh` exports the pinned version, and `verify-source.sh`,
  `template-branch.sh` and the pre-push hook source it, so a worktree formats as upstream CI
  does. #927 failed CI's `cargo fmt` because its template carried a line formatted unlike
  `main`'s (which is clean under both 1.97 and 1.99): rebuild from `main` rather than editing a
  submission branch by hand.
- Review rounds: fix on `main` (with a bump), then fold the changes into the PR's requested
  commit structure. New-source PRs stay one `feat: add <id>` commit by default, even after
  opening. Preserve the prior head, use the commit wrapper and verify the final tree before
  force-pushing with its expected remote SHA. `pr-branch.sh --fix` appends a commit; use it
  only when the owner explicitly requests a separate review-fix commit. Keep the submission version.
- After a merge, a later change to the source is a new PR from the same branch name:
  `scripts/pr-branch.sh <id> --update "feat(<id>): …" "<body>" --push` restarts
  `feat/<id>` from `upstream/main` (it only does so once origin's branch is merged) and
  sets `version` to upstream's plus one. PRs are squashed on merge, so the extra
  commits only make the round easier to review.
- Before any submission push, independently run
  `python3 scripts/check-upstream-scope.py --source <id> --head <ref>` after fetching
  canonical upstream. It checks the merge-base diff **and every submission commit**, not
  `origin/main`. README/website/workflow edits, other sources, generated files, merge commits,
  wrong ids and wrong submission versions fail. A clean final diff cannot conceal unrelated
  commits that were later reverted.
- An independent rewrite that leaves a template belongs in its own PR. A template PR may keep
  only necessary compatibility changes to a consumer still using it.
- A submission may also change `templates/<name>/` when its source depends on that template,
  and other sources depending on it, each at exactly its upstream version plus one. A source not
  yet upstream may ride along at version 1 only when it uses the changed template, so a template
  change ships with the new sources it serves (#824; Violet, Thunder and Madara share the
  mangathemesia child theme module). Other templates and sources still fail. `pr-branch.sh` copies one source only, so such a round is
  built in a worktree from the PR branch: copy the template and the source from `main`, drop
  `src/test.rs` and its module line, keep version 1, and edit the other consumers there.
- Submission branches are `feat/<id>` or `fix/<id>`; the existing
  `fix/kagane-chapter-fetch` maps to `multi.kagane`. A new template goes up as
  `feat/<template>-template` with the sources that show it working, as upstream's own template
  PRs do (branches `madara`, `mangathemesia`); the scope check validates it through one of its
  consumers and the pre-push hook verifies every crate it changes. Use `chore/<topic>` for tooling and
  preserved snapshots. Do not put custom fork checks or docs onto an upstream source branch.
- `upstream-scope.yaml` runs from `main` on main pushes, on demand, and every half hour.
  It reads all submission refs without executing their code and attaches an
  **Upstream source scope** status to each head. This independently catches API/browser
  pushes that bypass local hooks. Scheduled checks can be delayed by GitHub; they report
  failures, not a repository protection rule. Require that status in a ruleset if immediate
  merge blocking is wanted. No protection settings are changed by these scripts.
- To reconstruct a contaminated branch, use
  `scripts/pr-branch.sh <id> --from <saved-ref> --rebuild [--push]`. It preserves the old
  branch as `chore/<id>-before-rebuild-<sha>` before replacing it. Ordinary runs refuse
  to replace an unmerged branch. No source behavior is changed during reconstruction.
- **A rebuild refuses a branch that holds work `main` doesn't have** — source files matching
  no commit of the source on `main`, ignoring the version, `test.rs` and its module line.
  Another session pushed straight to `feat/en.atsumaru` once and the next rebuild replaced
  it; the work survived only in the `chore/…-before-rebuild-…` copy. Bring such work into
  `main` with a three-way merge (base: the commit it was built on, then `git apply -3`),
  then rebuild. `--discard-unmerged` replaces it anyway, for work that is truly unwanted.
- Review rounds preserve the submission version; updates of sources already upstream use
  exactly upstream's version plus one. Fork publication versions advance independently.
- Pushes to the PR branch always use `--force-with-lease=<ref>:<expected-sha>`, never a
  bare `--force`. The person may push to it too (they did on #815), so fetch before
  building on it.
- Check the PR is still open before each rebuild push (the PR page, or ask the person). #924
  was closed between two WebNex rebuilds; three more pushes followed, and GitHub then greyed out
  Reopen, because a closed PR only reopens while its branch still points at the commit it
  closed on. Recovering means a leased push back to that commit, which the HarnessKit
  plugin's force-push hook blocks in this environment, so the person opened #926 from the
  same branch instead. A contributor cannot delete a PR; a closed duplicate stays closed.
- A PR body drifts as review rounds change the source: re-read it against the final
  behaviour before each push and hand the person the corrected text (WebNex's body still
  described a removed single-request Discover).
