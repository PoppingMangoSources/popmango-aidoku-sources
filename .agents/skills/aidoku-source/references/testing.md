# Tests

The `### Tests (verified)` part of section 1 of the root `AGENTS.md` / `CLAUDE.md`, kept here
whole: what the live tests cover, how to run them, and what the test runner does differently
from the app.

`aidoku-test` plus aidoku's `test` feature (which disables the panic handler) are
dev-dependencies on ~129 sources — the scaffold adds them. Tests go in `src/test.rs`,
declared in `lib.rs` as `#[cfg(test)] mod test;`, and are marked `#[aidoku_test]`;
`en.mangakakalot`, `ja.senmanga`, `ja.soraraw`, `ja.spoilerplus` and `en.chikari` (inline in
`lib.rs`) have real ones upstream; the sources in section 17 had them here, kept in `main`'s
history. They hit live endpoints and assert
on actual results — pinned keys, whether a url resolves, whether a listing comes back sane,
whether each setting and filter actually narrows what the site returns.

**Write them for every new source.** Most of what went wrong in #735 would have shown up in
one. Cover browse paging, every sort and filter, each content rating on its own, a cleared
multi-select setting (must show nothing), a filter overriding its setting, details, a
chapters-only update, pages, deep links and a key the site doesn't know. Check results against
the site's own data rather than only for being non-empty. Name no other sources, clients or
groups in test data. **Tests stay on `main`:** `scripts/pr-branch.sh` leaves `src/test.rs` and
its `#[cfg(test)] mod test;` line out of the submission and fails if either reaches it. Upstream
doesn't require tests (`en.toonkotsu` went up without them); the dev-dependencies stay, as on
most upstream sources.

Running them:

- Install the runner once: `cargo install --git https://github.com/Aidoku/aidoku-rs
  aidoku-test-runner`. It links the system fontconfig and freetype, so install their headers
  first (`libfontconfig-dev libfreetype-dev` on Ubuntu). `.cargo/config.toml` already names
  the runner, so `cargo test` in the source directory runs every `#[aidoku_test]`; `scripts/verify-source.sh <dir> --test` runs the
  gates and then the tests. `cargo clippy --tests --target wasm32-unknown-unknown -- -D
  warnings` lints the test module, which the plain clippy gate skips.
- `verify-source.sh` also lints the test module when a source has one (`cargo clippy --tests`),
  with the same dead-code lints as the source (section 8).
- `.github/workflows/test.yaml` (**Source tests**) runs them for every source and template a push to
  `main` changes, and on demand from the Actions tab for named ids, or every crate with
  tests when none are named. It is a report, not a gate: the tests call the live site, so a
  red run can be the site or the network. Read the failing assertion before blaming either.
- The runner simulates most of the app, not all of it: WebView calls return `-1`, and the
  layout of a home page or how a component renders can only be seen in the app.
- The runner's `parse_date` converts the format to chrono's and drops quoted literals and
  fractional seconds, so an ISO format such as `"yyyy-MM-dd'T'HH:mm:ss.SSS'Z'"` (the app parses
  it, `en.novelbuddy` ships it) returns `None` there (verified on `en.novelarchive`). Check such
  dates in the app, not with an assertion. It also needs a time in the format: a date-only one
  such as `"MMM d, yyyy"` returns `None` there for every input, while `"MMM d, yyyy HH:mm"`
  parses (verified on `en.grimscans`), so assert only relative dates ("6 hours ago").
- The runner stores an `Element` made with `Element::try_from(node)` as a plain node, so its
  `tag_name()`, `html()` and other element calls return `None` there (`store.rs`,
  `as_html_element`), and a `child_nodes` walk sees only text. Take text from `child_nodes` and
  elements from `children()`, pairing each element node with the next child, as `en.novelbuddy`,
  `en.chikari` and `en.novelarchive` do; those elements work in the runner and the app alike. A
  document's own `child_nodes` is empty too, so walk from a selected element.
- The runner keeps a default that is set to `DefaultValue::Null` (its `set` fails to read the
  empty value and returns before storing), so a test that resets settings that way inherits
  the last test's values. Write each default out, `Bool(false)` or an empty `StringArray`
  (verified on `multi.mangaball`).
- The runner parses selectors with `scraper`, which rejects jsoup's `:contains(…)`, so a whole
  selector list containing one returns nothing there (the mangathemesia template's status, type,
  author and genre selectors; verified on `en.kingofshojo`). The app's SwiftSoup supports it, so
  check those fields in the app and test such filters by how they change the results.
- Sites behind Cloudflare can refuse the sandbox outright (403 "Attention Required" for curl
  and headless Chromium alike, `en.novelarchive`). Their live tests then fail on the network, not
  the code; read the site through Firecrawl and say the live suite could not run. Firecrawl strips
  the tags from HTML inside a JSON response's strings, so read such markup from the page that
  renders it (`en.novelarchive`'s mirror chapters looked like plain text that way).

For behaviour only the app shows, `aidoku logcat` starts a log server; enter its url in the
app's Advanced settings under Log Server and every `println!` from a device on the same
network streams to the terminal.

Treat live HTTP diagnostics as evidence, not a reason to skip a failure. Source tests keeps
offline `review_` contracts separate from live suites, preserves logs and reports observed
Cloudflare challenges/origin timeouts while retaining nonzero exits. Compare direct requests
with fresh page evidence before changing a parser or declaring a site globally offline.
Do not claim a live test passed when network access or WebView support prevented execution.
