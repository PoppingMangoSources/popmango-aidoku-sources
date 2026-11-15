# MangaTaro and Rolia Scan evidence

Both sites use the same MangaTaro engine, confirmed against their live browse pages,
captured JavaScript, Keiyoushi's mangataro shared engine and both Inkdex sources.
A single `feat/mangataro-template` submission includes the template and both consumers.
It is based on canonical Aidoku-Community main; fork publication stays on main.

## Requested behavior

| Request | Implementation / evidence |
| --- | --- |
| Rich featured hero, no per-card requests | home.rs parses `.carousel-slide`, including available plain description, tags, rating and viewer. |
| New follows, one month, ranking size 3 | `/auth/most-followed-new-manga?period=1m&limit=15`; `MangaList`. `month` and `30d` fall back to seven days. |
| Row order per source | Each source lists its rows in `Params::sections`. MangaTaro puts Latest Updates and then Popular Manga (This Week) right under the hero, and Latest Novels where Popular was (owner's order, not the site's); Rolia keeps the site's order. Both novel rows drop out when Novel is an excluded type. |
| Recently Added (MangaTaro only) | Actual `#manga-grid` cards from the same home document, in site order. Rolia leaves the row out, since its grid repeats Latest Updates (owner's call). |
| Latest Novels (MangaTaro only) | The browse search with type Novel in its Latest Updates sort, one POST to `/load`, shown as cards because it names no chapter. |
| Popular chapters this week | `/auth/popular-chapters?period=week&limit=15`, compact unique-series scroller. |
| Latest Updates, size 4, chapter/time | `/wp-json/manga/v1/latest-chapters?page=1`, real `permalink`, `chapter`, `time`. Browse `/load` lacks these chapter fields. |
| Weekly follows / popular | `/auth/most-followed-manga?period=week&limit=15` and `/wp-json/manga/v1/popular?period=week&number=15`. |
| Rolia recently added novels, size 4 | Actual chapter links/numbers/timestamps in `#novels-slider-container` from home HTML. |
| High Score Manga | `/wp-json/manga/v1/highscore?type=all&number=15`. |
| Tag and type settings, overridden by search | Both sites call the one taxonomy "Tags" on /browse, so the filter is Tags and the setting Excluded Tags. Empty exclusion lists exclude nothing. As in MangaBall, search exclusions add to the settings, and a tag or type the search includes lifts its setting exclusion. |
| Content rating | Both sources are rated 1 (owner's call); per-title ratings come from tags. |
| All browse controls | Four bidirectional sorts (untitled, as on the site), multi-select Type and Tags, single-choice Status and Year with the site's "All" option (its `app.js` keeps one value for each), and Match (Any/All) after Tags, read from live `/browse`. Selecting every status was not the same as none on Rolia, which a single choice avoids. Type stays a multi-select with exclusion although the site's Type dropdown takes one value (owner's call, after checking it in the app); excluded types are filtered locally and the live suites test excluding Novel. |
| Key-only reading, deep links, novels | Series keys are public slugs; chapter keys are the site's slug/chapter path. Comic pages and paragraph-preserving novel Markdown use chapter-content. |

## API boundaries

Advanced browsing is POST `/wp-json/manga/v1/load`, 24 raw rows per page. Its list
parameters are JSON-encoded strings. Both pages and impossible genre values were
probed. Types omitted includes Webtoon, which the site's four type buttons omit;
therefore an explicit four-type selection must not be treated as an omitted filter.
The site's sort buttons initially select descending, including the button labeled
Title A-Z; the source retains that label and provides the direction control.

Negative genres and guessed exclusion parameters do not work. Excluded genres/tags
use WordPress `manga?slug=...&tags=...&_fields=slug` in batches of at most 100 candidates.
Matching and impossible tag IDs returned different results on both sites. No per-card
fetches or compensating page-fill loops are used. Home normally makes seven parallel
requests; taxonomy exclusions add only the necessary bulk lookup(s). Recently Added
cards do not expose type; types from other already-loaded home feeds are reused when
available, and unclassified cards retain their eligibility.

Rolia's Popular ascending page returned 24 chapter-only records with blank series
URLs, even with every known type explicitly selected. The source reports a meaningful
error for a page containing only invalid identities rather than inventing series links
or silently presenting an empty successful page. The live sort test retains this
known site failure as an expected error; it is not a working sort claim.

Chapter lists paginate by the raw count and `has_more`; no artificial cap. MangaTaro
requires the short-lived MD5 timestamp token. A 403 can retry once using the server's
Date header to correct clock skew. Novel and comic reader responses are validated;
missing pages or empty text fail rather than disappear or become placeholder prose.

## Review and validation

Compared current review feedback including MangaBall, Keyoapp, Iken/Eternal, TeamX,
and recent kkantan inline comments. Retained compact cards, isGenre, standard test
scaffolds, QueryParameters, fallible identity conversion, separate models/helpers,
plain hero text, sorted tags and terse comments. Independent standards/spec review
caught and corrected duplicate response models and the Recently Added feed mismatch.

Deterministic template regressions cover requested row order/kinds/sizes, actual
update chapter identity, real novel rows, settings overrides/clearing, metadata,
text/emphasis, content ratings and reader errors. Live consumer suites cover page 2,
all eight sort/direction paths (with Rolia's documented error), impossible queries,
filters, details, chapters-only updates and key-only comic/novel reading. Formatting,
warning-failing Clippy, package schema/minimum-version/icon verification and locked
Cargo metadata were checked. Device layout and text-reader rendering still require
an in-app check.

Sources: https://mangataro.org/browse ; https://roliascan.com/browse ;
https://github.com/keiyoushi/extensions-source/tree/main/lib-multisrc/mangataro ;
https://github.com/inkdex/general-extensions/tree/0.9/stable/src/MangaTaro ;
https://github.com/inkdex/general-extensions/tree/0.9/stable/src/Roliascan
