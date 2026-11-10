---
title: Keyoapp template
type: subsystem
created: 2026-11-08
updated: 2026-11-10
sources:
  - code: templates/keyoapp/src/lib.rs
  - code: templates/keyoapp/src/imp.rs
  - code: templates/keyoapp/src/helpers.rs
  - code: templates/keyoapp/src/home.rs
  - code: templates/keyoapp/src/test.rs
  - code: .github/workflows/test.yaml
  - code: scripts/tests/test_template_tests.py
  - code: scripts/test-http.py
  - code: templates/keyoapp/src/v2.rs
  - code: templates/keyoapp/src/models.rs
  - code: sources/en.asmotoon/src/test.rs
  - code: sources/en.genztoons/src/lib.rs
  - code: sources/en.genztoons/src/test.rs
  - code: sources/en.genztoons/res/filters.json
  - code: sources/en.lunatoons/res/filters.json
  - code: sources/en.kaizenscan/res/filters.json
  - code: sources/en.sirenscans/res/filters.json
  - code: sources/en.asmotoon/res/filters.json
  - code: sources/en.timelesstoons/res/filters.json
  - code: sources/en.kewnscans/res/filters.json
  - code: sources/en.erisscans/res/filters.json
  - code: sources/en.mistscans/res/filters.json
  - code: sources/en.grimscans/res/filters.json
  - code: templates/keyoapp/README.md
  - code: scripts/check-upstream-scope.py
  - ext: https://github.com/keiyoushi/extensions-source/tree/main/lib-multisrc/keyoapp
  - ext: https://github.com/Aidoku-Community/sources/tree/a6ec13ae4127331c8edee6bad469144245b1db8a/templates
  - ext: https://github.com/Aidoku-Community/sources/pull/872
  - ext: https://github.com/Aidoku-Community/sources/pull/869
  - ext: https://github.com/Aidoku-Community/sources/pull/865
  - ext: https://github.com/Aidoku-Community/sources/pull/824
  - ext: https://github.com/Aidoku-Community/sources/pull/666
verified_at: 17c5faf
---

The Keyoapp template ports keiyoushi's first-generation `Keyoapp` library: a site lists its whole
catalogue on `/series/`, a series page carries its chapters, and a chapter page its images. Genz
Toons is the first consumer; the same live checks (catalogue, details, chapters, pages, home,
chapter deep links) passed against Grim, Mist, Eris, Kewn and Timelesstoons on 2026-11-08.

> ⚠️ The second generation is `KeyoappV2` in `templates/keyoapp/src/v2.rs`, with Asmodeus,
> Siren, Kaizen and Luna Toons as consumers (keiyoushi still files Kaizen under the first).
> A plain sandbox HTTP client can receive 403 while Firecrawl or the Aidoku runner succeeds;
> report runner results separately from browser observations. Art Lapsa, Kitsune Dawn and Rithar are
> custom in keiyoushi (`/series/` is a 404). MeiToon answers "the website has been stopped".

> ⚠️ V2 search is `/?_ajax=search` with `q`, `sort`, `type`, `status`, `tag[]` and
> `offset` (20 a page), answering `{html, has_more}`; the site filters, so `has_more` is the next
> page. A–Z sorts the raw titles, so ones with a leading space come first; that is the site's
> order. A tapped tag sends its name lowercased as `tag[]`, which is how the browse ids read.

> ⚠️ V2 sites pick one of two series layouts: Asmodeus's banner one (`h1.l2-title`,
> `.l2-meta span` "Mangatoon · Action · Ongoing", `p.l2-by`, `.l2-tagline a`) and Siren's,
> Kaizen's and Luna's labelled blocks (`div.bg-surface.rounded-lg` holding a `.text-xs` label
> and a `span.text-sm` value, tags as `a[href*='tag%5B%5D']`, cover from `og:image`). Details
> read the first and fall back to the second. An unknown slug answers 200 with no
> `[data-series-uid]`, so details fail with "Series not found".

> ⚠️ V2 chapters come from `/?_chapters_html=1&series_slug=` as `{rows_html}`; coin rows carry
> `data-ch-locked="1"` and are skipped. A new series can have no chapters at all, so the live
> tests pick the first series that has one. Dates are short ("4h ago", "6d ago").

> ⚠️ The V2 home is one row per `section` under its first `h2`. Featured cards put the title in
> `title` (Asmodeus) or an `h3` (Kaizen's image slider); Recently Updated `article`s become a
> chapter list of the newest free chapter. Novels stay, with the site's own titles ("[ Novel ] …"
> on some Asmodeus cards). Editor's Choice repeats for mobile and its `data-ec-clone` copies have
> no title, so both drop out. Its cards are covers and titles only, so it is a card row, not a big
> scroller. Search cards reuse the same card conversion.

> ⚠️ Newly Added cards can sit inside `#home-newly-added` or in its immediately adjacent
> `#series-grid`. Scope the fallback with `#home-newly-added + #series-grid` and use the normal
> card conversion; a document-wide lookup can attach an unrelated grid to an empty row.

> ⚠️ Trending uses Today from the initial homepage. Most Reviewed uses Month by the owner's
> explicit choice, with one extra request when that row exists. Keep these defaults separate.

> ⚠️ A V2 series and chapter share the path shape (`/slug`, `/slug-chapter-4`), so a deep link
> fetches the page: `[data-series-uid]` makes it a series, and `#sidebar-ch-list` names a
> chapter's series in `data-series-slug`.

> ⚠️ The site ignores `q`, `genre`, `type` and `status` on `/series/` (every request returns all
> 263 Genz cards), and filters in the browser from each card's `title`, `tags`, `data-type` and
> `data-status`. So search reads the one page and filters it the same way, with no next page,
> as keiyoushi does. A card's `title` attribute also carries the alternative titles, so a query
> matches those too.

> ⚠️ Genres, types and statuses are each site's own lists from the `initializeDropdownMenu`
> script, written to the consumer's static `filters.json` (the template's README has the
> console snippet), so tags are tappable without `supportsTagSearch`. Every filter is
> include-only, as on the site and in keiyoushi: a series needs every chosen genre and any one
> chosen type or status, and none can be excluded (the owner's call; the local filtering could
> exclude, but the site offers no such control).

> ⚠️ A card's `tags` is a JSON list (`["Drama","Sports"]`, apostrophes written `___`), and a
> name can hold a comma ("Host Club, Unrequited Love" on Kaizen), so it splits between quotes.
> Some Genz names carry stray spaces and tabs (`" \taction"`, `"shounen  "`), which the site's
> own filter then misses; the template trims them.

> ⚠️ A pasted series link (`<base>/series/<slug>/`) as the query finds that card by its path,
> with or without the trailing slash, query or fragment, from the same catalogue page. Normalize
> absolute card hrefs before comparing: Genz serves absolute links. Missing catalogue markup
> is an error, while a present but empty catalogue is a valid empty result.

> ⚠️ An unknown series redirects (302) to `/not-found/`, which has no `div.grid > h1`, so the
> details and chapter paths fail with "Series not found" rather than return an empty series.

> ⚠️ A series page links only its first genres above the title (Mechanic: Slayer shows Action and
> Adventure of six), so details tags are those, as the site shows and keiyoushi reads. Its
> `meta[name=keywords]` starts with every genre, then a per-site tail of brand and type words
> (Timelesstoons' tail even lists Action and Shounen), so it can't give the tags, but the
> content rating reads it too: an Eris series linking Romance and Drama lists Mature and Smut
> there, and rated Safe before.

> ⚠️ Novels appear among comics everywhere, unlike in keiyoushi, which drops them. First
> generation novel chapters are text in `div.tiptap.novel-reader` (`p` with `em`, `strong`,
> `span`, `br` on Genz, Mist and Timelesstoons), read as one Markdown paragraph per `p`, since the
> reader's Markdown overflows its stack on one long paragraph; text is escaped, so a `<Status>`
> window reads as `\<Status\>`. Escape only Markdown syntax, since escaping ordinary punctuation
> adds inline nodes too. Emphasis closes before each paragraph break, including a `<br>` inside
> `strong` or `em`, and reopens in the next paragraph. Second generation novel chapters (Asmodeus, Siren) mix image pages
> (`img.reader-page`) and written ones (`div.reader-page` holding `.prose-note` paragraphs), so
> each reader page is read as one or the other. The chapter page itself says which, so reading
> needs only the keys; merged sources put a novel flag in the key only because their APIs read
> novels from another endpoint. Novels get `Viewer::Unknown`, as in the merged novel sources.
> Walking text nodes needs `child_nodes`, which raises every consumer's `minAppVersion` to 0.8.3.

> ⚠️ A latest poster carries a 5px blurred copy of its cover as a backdrop before the series link
> (Kewn), so a card's cover is looked up inside the link first.

> ⚠️ Details labels ("Status", "Author", "Artist", "Type") are read through the children of a
> `div.grid` whose first child names the label in a `span` and whose second child holds the
> value. The test runner cannot walk siblings, and the page's search menu uses the same words
> with no value beside them. Avoid reintroducing `+` selectors for this layout (Grim and Mist
> previously showed the wrong reader in the app).

> ⚠️ About half the series on Grim, Mist and Timelesstoons have no synopsis anywhere (no
> `#expand_content`, no meta description), so a missing description is `None`, not an error.

> ⚠️ Coin chapters carry `img[alt=Coin]` and need a login, so chapter lists and Latest Updates
> skip them; "Upcoming" rows are links with a future date and an `Upcoming` badge, also skipped.
> Dates are "6 hours ago" or "Aug 3, 2026".

> ⚠️ Pages are `#pages > img[uid]`, and the reader builds each url from a `realUrl` template in a
> script (`https://cdn.meowing.org/uploads/${uid}` on every site checked), which needs no referer.
> An image without a `uid` fails the page list rather than dropping that page.

> ⚠️ The home is generic: every `h2` over cards becomes a row in the site's order, with the
> untitled featured slider first. Latest Updates is a chapter list of five per page; Pinned
> Series uses the same `.latest-poster` markup, so the heading tells them apart and Pinned is a
> card row like the rest. Some rows repeat for the mobile layout ("Editor's Choice"), so titles
> are kept once; headings with no cards ("Sync Local Library") and Bulk Discounted Series (coin
> chapters for sale) are skipped (the owner's call). Genz's Pinned Series lists the same series
> as its Latest Updates, so Genz hides it with `Params::hidden_rows`; Mist, Timelesstoons and Eris
> keep theirs.

> ⚠️ A row's cards are found by climbing at most three parents from its heading, and the climb
> stops at a block holding another row's heading: otherwise a heading with no cards near the
> top of the page reaches `<body>` and takes every card on it (caught by the offline fixture).
> `parse_home` is apart from the request so `review_` tests can check the layout against a
> cut-down page with no network.

> ⚠️ The featured slider is covers on Genz (`aspect-[0.74/1]`) and 2:1 banners on Mist, Kewn
> and Timelesstoons (`aspect-[2/1]`, 1000px wide), so a banner slider is an image scroller sized
> 400 by 200 and the rest a card row. The class of each `a.splide__slide > div` is compared in
> Rust: the selector `[class*='aspect-[2/1]']` matched Genz's covers in the app (showing them as
> cropped banners) though the runner read it correctly. The banner slides carry only a title and the image: Mist's
> descriptions and genres belong to its Collections carousel (themed groups, not series), so a
> big scroller there would cost a details request per slide. Chromium shows `--clone` slides
> that the script adds; the served page has none.

> ⚠️ Editor's Choice is a big scroller. Its picks are `.editor-pick-button` divs with no link:
> the site's script opens `/series/<data-id>`, loads the cover from `data-img` on the upload
> host and decodes `data-description` from base64. The key keeps the trailing slash so it
> matches the search cards, or the same series would be added to the library twice.

> ⚠️ A chapter path holds the series id, not its slug, so a chapter deep link fetches the chapter
> page for its series link, skipping the menu's bare `/series/` link (Kewn's chapter pages put it
> first).

> ⚠️ The submission branch is `feat/keyoapp-template`. `check-upstream-scope.py` validates a
> `feat/<template>-template` branch through one of its changed consumers, and the pre-push hook
> verifies every crate it changes.

## Review against merged sources

The 2026-11-09 audit covered the eleven commits from `fc3d3cf` through `ae92053`, every
Rust line in the template and ten consumers, their manifests, locks, resources and main-only
tests. The main rules, skill references, workflows, hooks and checking scripts were read
alongside the changes. Upstream comparisons used `a6ec13a`, including MangaK, MangaThemesia,
MangaReader, Madara, VineTheme and Zaimanhua templates, and NovelArchive, NovelBuddy and
Wenku8 for text chapters. The reviewer comments on #872, #869, #865, #824 and #666 were
checked individually; reusable new lessons live in the root guide and source-review reference.

| File | Responsibility and review decision |
|---|---|
| `templates/keyoapp/src/lib.rs:8` | Generic source/trait delegates, as in merged MangaK and VineTheme; keep this wiring explicit. |
| `templates/keyoapp/src/imp.rs:21` | V1 requests and full-catalogue filtering; a separate parser permits offline error and pasted-link checks. |
| `templates/keyoapp/src/v2.rs:26` | V2 search and chapter endpoints; preserve the site's pagination and separate protocol. |
| `templates/keyoapp/src/home.rs:14` | Both home parsers and their private row helpers; keeps layout logic out of request methods. |
| `templates/keyoapp/src/helpers.rs:13` | Shared conversions, dates and Markdown; `manga_from_link`, `manga_from_card`, `manga_from_page` and `chapter_from_link` name their inputs. |
| `templates/keyoapp/src/models.rs:1` | Two response objects matching captured endpoint fields; no shared response abstraction needed. |

> ⚠️ Search, home, pasted links and deep links must return the same series key. V1 keeps
> `/series/<slug>/`; V2 takes the canonical slug from the page. Strip query/fragment variants
> before classifying a deep link. The app already chooses the source by host, but arbitrary
> pasted search URLs still need an exact base/path boundary. Keep chapter identity separate
> from the displayed chapter number, including hyphens in the key.

> ⚠️ `chapter_from_link` owns V1 paid/upcoming filtering, so latest rows and chapter lists
> cannot disagree. Preserve both image and written V2 novel pages: Asmo currently serves
> each format, and the previous README's image-only claim was wrong.

The current filter IDs and order match every site's full definition, including all genre
options: Genz 105, Grim 39, Mist 66, Eris 79, Kewn 36, Timeless 192, Asmo 99, Siren 163,
Kaizen 245 and Luna 617. V1 has six type and four status values; V2 has seven sorts,
six type choices and three status choices. Browser-visible lists can be shorter than the
full DOM definition, so a visible-option count alone does not prove missing filters.

| Review | Checked against Keyoapp |
|---|---|
| #872 | V1 catalogue is one page, V2 uses `has_more`; chapters have no observed extra-page mechanism. Preserve complete keys, fail missing catalogue markup, keep absent values optional, avoid repeated allocations, keep helpers out of wiring and retain title/cover links. Icons retain each site's branding and pass 128px/opacity verification. No page-limit truncation exists. |
| #869 | No redundant included-values or higher-page guard; host dispatch is left to the app; listing conversions omit detail fields. |
| #865 | Untitled cards are skipped; no redundant chapter language or login-state logic. Search covers comics and novels through the same catalogue/API. Pasted URLs use a base/path boundary. Viewer/description belong to details; author and artist labels are read independently. Response fields match captured schemas. |
| #824 | Chapter requests need only keys; compact cards, SDK date parsing, ten consumers, sort-first V2 filters, static tappable genres, empty descriptions as `None`, observed coin/upcoming rules and explicit disabled exclusions. No speculative next-update timestamp is added. |
| #666 | No unused-variable suppression workaround; browse and query share one endpoint per generation. V1's local filtering remains necessary because the server ignores filter/query parameters. |

Normal request counts remain: V1 search/home/pages/details plus chapters each use one
request, direct series deep links use none and chapter deep links use one. V2 search,
pages, details, chapter lists and deep links each use one; home uses one plus the monthly review
request when that row exists. Details plus chapters use
two with a partial details result. A valid pasted V2 series uses one, while an unrecognized
pasted page can fall through to search for a second. Neither generation fetches each card's
details to build rows.

> ⚠️ Main owns nine template `review_` regressions for canonical links, absolute hrefs,
> malformed catalogues, missing titles, unavailable chapters, novel paragraphs (including literal
> block syntax after breaks), and newly added row bounds. The ten consumers add offline
> layout/filter regressions. CI and pre-push now include
> changed templates as well as dependent sources; the workflow selection regression executes
> the actual shell step. Strip the template test module and test file on submission, as for
> source tests.

> ⚠️ On 2026-11-09 the Aidoku runner's live suites passed for Genz, Grim, Mist, Eris, Kewn,
> Timeless, Asmo and Siren. Kaizen and Luna's search requests returned HTTP 403 with a
> Cloudflare "Just a moment..." page instead of JSON, confirmed using the runner's own request
> implementation. Their offline fixtures, resource checks, compilation and package verification
> passed; browser/Firecrawl observations do not replace their blocked runner or device checks.
> The native relative/absolute date behavior and long novel rendering still need an app check.

> ⚠️ The fork checks HTTP status before parsing every template response. A non-2xx result
> returns `HTTP <status>` and logs the request URL, content type and a Unicode-safe preview
> capped at 240 characters. Malformed JSON and missing V1 catalogue markup also log the
> response without replacing the original parser error (`templates/keyoapp/src/helpers.rs:15`,
> `templates/keyoapp/src/imp.rs:31`). The optional monthly home row keeps its existing fallback.

`scripts/test-http.py:30` runs the ignored HTTP regression against a loopback fixture.
It checks 403/404/429/503 before decoding, successful 2xx responses, malformed JSON,
missing catalogue markup and bounded diagnostic output. Source tests runs it separately
when the changed crates include the template. The fixture needs no external website.

Validation on 2026-11-10: all ten consumers passed formatting, warning-failing Clippy,
packaging and `aidoku verify`; the template's nine ordinary regressions, the separate
HTTP fixture regression and 97 repository script tests passed. Eight live source suites
passed. Kaizen and Luna remained blocked by HTTP 403 Cloudflare challenge pages, now
identified directly in failure logs. Asmodeus initially compared different slugs from two
live responses; the filter case and full 13-test suite passed on retry. The existing upstream
submission was inspected without modification: scope and upstream CI checks passed.
