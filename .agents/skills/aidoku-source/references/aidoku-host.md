# What the app does with a source

Read with `AGENTS.md` (`CLAUDE.md` is the same file) and the skill's `SKILL.md`. This records how
the host treats a source: how AidokuRunner loads and calls it, what each import does, and what
the Aidoku app renders. Every claim names the file it comes from. When this file and a rule in
`AGENTS.md` / `CLAUDE.md` disagree, the code decides: re-read the cited file, then fix
whichever is wrong.

Paths are relative to each repository. `AidokuRunner` means `Sources/AidokuRunner/`; the app's
home views are under `Aidoku/Features/Source/`. All three repositories clone anonymously:

```sh
git clone --depth 1 https://github.com/Aidoku/AidokuRunner
git clone --depth 1 --filter=blob:none --sparse https://github.com/Aidoku/Aidoku   # then: sparse-checkout set Aidoku/Features
git clone --depth 1 https://github.com/Aidoku/aidoku-rs
```

`scripts/check-host-reference.py` compares the cited files with each repository's current
`main` and lists the ones that changed. `.github/workflows/host-reference.yaml` runs it weekly.
When it reports a change, re-read that file, correct this page, then run it with `--update`.

## Loading (`Source.swift`, `Models/SourceInfo.swift`)

- `source.json` → `info`: `id`, `name`, `altNames`, `version`, `url`, `urls`, `contentRating`
  (`0` safe, `1` contains NSFW, `2` primarily NSFW; absent means safe), `languages`.
- `listings` entries may be a string (used as both id and name) or `{id, name, kind}`; `name`
  defaults to `id`, `kind` to default.
- `config` has exactly eight keys: `languageSelectType` (`single`/`multiple`),
  `supportsArtistSearch`, `supportsAuthorSearch`, `supportsTagSearch`, `allowsBaseUrlSelect`,
  `breakingChangeVersion`, `hidesFiltersWhileSearching`, `maximumParallelRequests`. Anything
  else is ignored.
- `filters.json` and `settings.json` are optional; a file that fails to decode fails the
  source's load.
- **Features come from the wasm's exports**, not from `register_source!` directly
  (`Interpreter.swift`, `init`): `get_manga_list` (listings), `get_home`, `get_filters`,
  `get_settings`, `get_listings`, `process_page_image`, `process_cover_image`,
  `get_image_request`, `get_page_description`, `get_alternate_covers`, `get_base_url`,
  `handle_notification`, `handle_deep_link`, `handle_basic_login`, `handle_web_login`,
  `handle_key_migration`. Registering a trait exports its function, so a trait registered only
  to return `Unimplemented` still switches the feature on.
- `onlySearch`: with no home and no listings, the source opens on search.
- `start` runs once after loading and builds the source with `new()` (aidoku-rs
  `crates/lib/src/macros/mod.rs`).

## Settings the app adds and how values are stored

- **Languages**: with more than one language the app adds a group with key `languages`
  (multi-select, the default) or `language` (`languageSelectType: "single"`). Default: the
  device's preferred languages that the source has. `refreshes: content`.
- **Base url**: with `allowsBaseUrlSelect` and more than one url, a `url` select over
  `info.url` then `info.urls` (a `get_base_url` result is put first). Default: the first.
- Every value lives in `UserDefaults` under `<source id>.<key>` (`Imports/Defaults.swift`).
  `defaults_set` with null removes the key.
- **Defaults are registered** (`Source.swift`, `loadSettingsDefaults`): select and picker
  default to their `default` or else the first value; switch, stepper, segment, text,
  editable list and multi-select register only a `default` that is written (a switch decodes
  a missing default as `false`, so it is always registered). A multi-select with no
  `default` reads as `None` until the reader changes it.
- **Dynamic settings' defaults are registered only when `get_settings` runs**, that is when
  the reader opens the settings page. Until then `defaults_get` returns `None` for them, so
  the Rust side needs its own fallback for every dynamic setting.
- Setting JSON keys (`Models/Setting.swift`): `type` (`group`, `select`, `multi-select`,
  `switch`, `stepper`, `segment`, `text`, `button`, `link`, `login`, `page`, `editable-list`,
  `picker`), `key`, `title`, `notification`, `requires`, `requiresFalse`, `refreshes`, and
  `default` for the value. Page settings take an `icon` (`system` with `name`, `color`,
  `inset` 5 by default, or `url`).

## Filters (`Models/Filter.swift`, `Models/FilterValue.swift`)

| Key | Default when absent |
| --- | --- |
| `id` | `title`, else the `type` |
| sort `canAscend` | **true** |
| sort `default` | none (index 0, descending); `ascending` inside it defaults to false |
| check `canExclude` | false |
| multi-select `canExclude` | false |
| multi-select / select `usesTagStyle` | **the filter's `isGenre`** |
| `isGenre` | false |
| range `decimal` | false |

Types: `text`, `sort`, `check`, `select`, `multi-select`, `note` (a `text` line), `range`. A
check sends `1` when ticked and `2` when excluded (`canExclude`); an unticked check isn't sent
(app `Filter/CheckFilterView.swift`). Values reach the source as
`FilterValue` with the filter's id; select and multi-select send the `ids` entry for an option
when the filter has `ids`, else the option text (app `Filter/SelectFilterView.swift`,
`MultiSelectFilterView.swift`).

**Tag taps** (`Source.matchingGenreFilter`): with `supportsTagSearch`, every tag sends
`select(id: "genre", value: <tag name>)`. Without it, only a tag equal to an option of a
**static** `isGenre` multi-select or select is tappable, sent as that filter's value (its id
when it has `ids`); dynamic filters are never consulted.

**Author/artist search** (`Source.supportsAuthorSearch`): `config.supportsAuthorSearch`, or
else whether a **static** `text` filter has id `author` (`artist` likewise). A dynamic author
filter doesn't count.

## Calls (`Source.swift`, `Interpreter.swift`)

- **Search**: the query is passed as `""` when there is none, and aidoku-rs turns `""` into
  `None` (`crates/lib/src/imports/std.rs`, `read_string`). A whitespace-only query arrives as
  `Some`. With `hidesFiltersWhileSearching` and a non-empty query, the app sends no filters.
- **Manga update**: on a **single-language source the app sets every chapter's language to
  the source's own** when the source leaves it unset, so setting it is redundant. Partial
  manga from `send_partial_result` update the open series page while chapters load (app
  `Features/Manga/MangaView+ViewModel.swift`).
- **Page list**: only manga and chapter keys should be required for reader routing. URLs and
  other metadata are not guaranteed on every call path (kkantan, #843); encode required
  type information in the keys and test with fresh key-only values. The manga arrives with
  `chapters` removed. Pages whose content can't be
  rebuilt (an image the source stored and lost) are dropped without an error.
- **Listings** and **home** throw `unimplemented` unless the export exists.
- **Errors**: a negative result maps to unimplemented (-2), network (-3), html (-4), js (-5),
  canvas (-6), utf8 (-7), json parse (-8), deserialize (-9); `bail!`/`error!` text is shown to
  the reader as the message.
- `restart()` reloads the wasm and runs `start` again; state kept in the source struct is lost.

## Home (`Interpreter.getHome`, `Models/Home.swift`, the app's `SourceHomeContentView.swift`)

- Components: `imageScroller`, `bigScroller`, `scroller`, `mangaList`, `mangaChapterList`,
  `filters`, `links`.
- A `layout` partial replaces the page. A `component` partial **replaces the layout entry
  with the same title**, or is appended when no title matches, so titles must be unique and
  must match the layout exactly.
- When `get_home` returns a home with components, the app shows that. When it returns an
  empty one, the app keeps the last partial state, **including the empty placeholder of any
  section whose component never arrived**.
- While loading, an empty component shows a shimmering placeholder; after loading, an empty
  scroller or list shows its title over nothing (`HomeScrollerView`, `HomeListView`,
  `HomeBigScrollerView`: `if partial && entries.isEmpty`). To drop failed sections, return the
  components that did load as `get_home`'s result.
- What each component shows (app `Home/HomeComponents/`):
  - `BigScroller`: cover, title, **authors, description, tags**, and a content rating badge
    for suggestive and NSFW. This is the component for details.
  - `Scroller`: cover, title and the link's `subtitle`.
  - `MangaList`: cover, title, `subtitle` and **the manga's tags** when the link's manga has
    any. A row is only compact when the manga carries no tags.
  - `MangaChapterList`: the manga plus the chapter's formatted title and a relative
    `dateUploaded`.
- `Link` from a manga (`Manga.intoLink`, and aidoku-rs `impl From<Manga> for Link`): subtitle
  is the authors joined by ", ", else the description. A compact manga gives no subtitle.
- The app remembers each component's type and size under `<source id>.homeComponents` to draw
  the next skeleton; changing the layout shows one stale skeleton once.

## Models (`Models/Manga.swift`, `Models/Chapter.swift`, `Models/DeepLink.swift`)

- `Manga`: `key`, `title`, `cover`, `artists`, `authors`, `description`, `url`, `tags`,
  `status` (unknown, ongoing, completed, cancelled, hiatus), `contentRating` (unknown, safe,
  suggestive, nsfw), `viewer` (unknown, LTR, RTL, vertical, webtoon), `updateStrategy`
  (always, never), `nextUpdateTime`, `chapters`. No alternate titles, year or score.
- The series page merges an update with `Manga.copy(from:)` (app
  `Features/Manga/MangaView+ViewModel.swift`): optional fields keep the old value when
  the new one is `None`, but **`status`, `contentRating` and `viewer` always take the new
  value**, so returning a default there clears what details had set.
- `Chapter`: `key`, `title`, `chapterNumber`, `volumeNumber`, `dateUploaded`, `scanlators`,
  `url`, `language`, `thumbnail`, `locked`.
- `DeepLinkResult`: a manga key, a chapter key (with its manga), or a listing.

## Network (`Imports/Net.swift`, `Utilities/NetRequest.swift`, `Utilities/RateLimit.swift`)

- A url `URL(string:)` rejects (unencoded spaces and some characters) fails the request
  before it is sent; build queries with `QueryParameters`.
- A non-2xx status is **not** an error: the response comes back and the source must check
  `status_code`.
- Requests use `URLSession.shared`, so plain requests share one cookie store with the app.
- `set_rate_limit(permits, period, unit)` counts every request, including each one inside
  `send_all`, against a fixed window; over the limit, a request waits for the next window.
- `send_all` runs the requests concurrently and reports a failure per request.
- `html()` decodes the body as UTF-8, falling back to ASCII, and parses it with the response
  url as the base, which `abs:` attributes resolve against.
- Methods: GET, POST, PUT, HEAD, DELETE, PATCH, OPTIONS, CONNECT, TRACE. `set_timeout`
  overrides the default timeout.

## Std, env and defaults (`Imports/Std.swift`, `Imports/Env.swift`)

- `current_date` is seconds since 1970 as a float; `utc_offset` is the device's offset.
- **`parse_date` uses `DateFormatter` with the device's locale and time zone unless given
  one.** English month or day names fail on a device set to another language: pass
  `en_US_POSIX` as the locale. `current` selects the device's.
- `print` goes to the log handler (`aidoku logcat` in the app's Log Server setting).
- `sleep(seconds)` blocks the source.
- `send_partial_result` delivers a partial home or manga while the call is still running.

## Web views and JavaScript (`Imports/JavaScript.swift`, `Utilities/WebViewHandler.swift`)

- Imports: `context_create`, `context_eval`, `context_eval_async`, `context_get`,
  `webview_create`, `webview_set_rule_list`, `webview_load`, `webview_load_html`,
  `webview_wait_for_load`, `webview_eval`, `webview_eval_async`, `webview_add_user_script`,
  `webview_get_cookies`, `webview_delete_cookie`.
- **A web view has its own data store per source** (`WKWebsiteDataStore.forSource`):
  persistent on iOS 17 and later, non-persistent before. Its cookies are not the
  `URLSession.shared` cookies plain requests carry, so a check passed in the web view does not
  unlock plain requests (`AGENTS.md` / `CLAUDE.md`, Toonkotsu specifics).
- `webview_wait_for_load` waits on a semaphore with **no timeout**: a page that never
  finishes loading blocks the call.
- `webview_eval_async` wraps the script in an async function and resolves with its value.
- `aidoku verify` requires `minAppVersion` 0.9.0 for the web view imports.

## HTML and canvas

- HTML (SwiftSoup): `parse`, `parse_fragment`, `escape`, `unescape`, `select`,
  `select_first`, `attr`, `text`, `untrimmed_text`, `own_text`, `html`, `outer_html`, `data`,
  `id`, `tag_name`, `class_name`, `has_class`, `has_attr`, `base_uri`, `children`,
  `child_nodes`, `parent`, `siblings`, `next`, `previous`, `first`, `last`, `get`, `size`,
  and the mutators `set_attr`, `remove_attr`, `set_text`, `set_html`, `prepend`, `append`,
  `add_class`, `remove_class`, `remove`.
- Canvas (UIKit only): `new_context`, `set_transform`, `draw_image`, `copy_image`, `fill`,
  `stroke`, `draw_text`, `get_image`, `new_font`, `system_font`, `load_font`, `new_image`,
  `get_image_data`, `get_image_width`, `get_image_height`. A processor's image goes through
  the app's normal decoder, so converting formats in one gains nothing.

## Pinned files

```pins
repo https://github.com/Aidoku/AidokuRunner 2ed3f0d395be0c3ee9ebe238e2ffcead0094613f
f5aeea6626989b83495b71cf201b14a772d120d1 Sources/AidokuRunner/Source.swift
5214dd451403c31d85ab4b0eba2d548db47c2eca Sources/AidokuRunner/Interpreter.swift
0d9fa6e81f9fa943c32976c5f959096899a9df93 Sources/AidokuRunner/Models/Filter.swift
85a4703c0d135ff06c28e99abc31de9504662ba2 Sources/AidokuRunner/Models/FilterValue.swift
79bf9fb752b3450e64cdd408253dc4222d1e94ed Sources/AidokuRunner/Models/Setting.swift
c0c0b231b9d1ad53681e111acef3613bdba28bec Sources/AidokuRunner/Models/Home.swift
669ed3f7153b20884caa8f0a843a90837c4a8ee8 Sources/AidokuRunner/Models/Manga.swift
a9b5a27a57d1ac0df7fb9534d7a3c24c3fffdcef Sources/AidokuRunner/Models/Chapter.swift
297b1a8f194e9de707077a632687d778d77a09d4 Sources/AidokuRunner/Models/Page.swift
7b4f2edd5c6daf88e0664b9aee3ddc9bfca74786 Sources/AidokuRunner/Models/Listing.swift
d8716be5cf55920b59cb71ee2de3911fdc3154dd Sources/AidokuRunner/Models/DeepLink.swift
b75c0f3dffe7dd3801fd1d33362f6698210116d4 Sources/AidokuRunner/Models/SourceError.swift
5f2a57f0ae2e83d14d425e60e512414b5af4567f Sources/AidokuRunner/Models/SourceInfo.swift
f7cf8f1549b730e0443d77ba7558efa1eeeb51d5 Sources/AidokuRunner/Models/SourceFeatures.swift
3c8b1e65c59fdd02ff15ad28209a790fb7c97c20 Sources/AidokuRunner/Imports/Net.swift
4e21c49712a0497af304b0f2a9c51252104d8cd7 Sources/AidokuRunner/Imports/Std.swift
bd2108b979bd5407b5bec9639bb136790799f74a Sources/AidokuRunner/Imports/Defaults.swift
c0241929be2345969614c27970a4e7b7416e2b76 Sources/AidokuRunner/Imports/Env.swift
5d0d005fb5a70ee967221a7172561885a75ba8b2 Sources/AidokuRunner/Imports/JavaScript.swift
d9456ed9d5341567021c7625c93c2c08ff36f9cb Sources/AidokuRunner/SettingsStore.swift
35cb3b4ba21de3b96bf86d6bd942b1a400c78cc0 Sources/AidokuRunner/Utilities/RateLimit.swift
a7f13d1bf76bcef7fd4b21ff06151757dab38a1b Sources/AidokuRunner/Utilities/NetRequest.swift
7fcdbfcee44d8104d01568a97066896fc55f501b Sources/AidokuRunner/Utilities/WebViewHandler.swift
910439e66800b775462dade4469b93db22473c25 Sources/AidokuRunner/Extensions/WKWebsiteDataStore.swift
repo https://github.com/Aidoku/Aidoku ceee58b29d4f48106cf63042ca0cb075a8ac22b4
456b537a372cd9101fc9b7866da4e4bc6ca5ae14 Aidoku/Features/Source/SourceHomeContentView.swift
43eddb85cc3a0f0c1f50f0a5e9b8a8f6011a0c3b Aidoku/Features/Source/Home/HomeComponents/HomeBigScrollerView.swift
cb4a00225a980b7ffa21e0fa16b7e877784ecce8 Aidoku/Features/Source/Home/HomeComponents/HomeScrollerView.swift
62beb2cf461ae8db0d0f5890b10d54e7cc5ec6a2 Aidoku/Features/Source/Home/HomeComponents/HomeListView.swift
e24fec29263bfd01716e8120ee4727a0e6bcc817 Aidoku/Features/Source/Home/HomeComponents/HomeChapterListView.swift
e160171316736af6ad3c6be5f383230db3a05f6b Aidoku/Features/Source/Filter/CheckFilterView.swift
7ce4dd8a31e06f9937a45d4f7e7337e7762170d2 Aidoku/Features/Source/Filter/SelectFilterView.swift
8bb91994eeb7619b6b65194951e7e40ad1425292 Aidoku/Features/Source/Filter/MultiSelectFilterView.swift
a3b36574aad3d06b73c00d285746aaf038f168a6 Aidoku/Features/Manga/MangaView+ViewModel.swift
repo https://github.com/Aidoku/aidoku-rs ae6485e3c14ecd1fd5497d170eae884c68d04a87
b3c58d9fb5bc71481e0e77b935438b6621a74fd9 crates/lib/src/macros/mod.rs
dd864db00a2da689d35fb3af362bcf295741cb36 crates/lib/src/imports/std.rs
cf2e1cae5c4ae0964193a420db0fc776f91a4af8 crates/lib/src/structs/home.rs
42c9725dddacf7224795d38749f3db88f2465364 crates/lib/src/structs/filter.rs
```
