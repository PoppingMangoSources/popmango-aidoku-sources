---
title: Kagane (multi.kagane)
type: subsystem
created: 2026-11-20
updated: 2026-11-20
sources:
  - ext: https://github.com/Aidoku-Community/sources/tree/main/sources/multi.kagane
verified_at: 3ac574e
---

`multi.kagane` lives upstream only. The rework merged as #827; a later change is a new
branch from `upstream/main` (it isn't on our `main`, so `scripts/pr-branch.sh` can't copy it
until it is restored from its last copy, section 17).
Confirmed by console probes on kagane.si unless marked.

- Mirrors: kagane.cx (default), kagane.si, kagane.moe, kagane.net serve the same catalogue
  (kagane.si probed). kagane.org now opens a page that picks one of those four, so it is
  listed last. API under `{base}/api/v2`; the selected url is read as `defaults_get("url")`.
- **Chapters** come from `GET /series/{id}/books/index?fields=List` — rows of values in
  the order `fields` names, groups as indices into `groups`; order by `sort_no`, since
  chapter numbers restart. The series endpoint no longer lists books. (inferred from
  issue #816 and the reference clients)
- **Ratings** are `age_rating` (`Safe`, `FifteenPlus`, `EighteenPlus`) plus
  `is_pornographic`. `is_pornographic: false` gives 18+ without pornographic titles,
  `true` only pornographic ones, absent both. The source keeps the old four names
  (Safe, Suggestive, Erotica, Pornographic) and maps them, as keiyoushi does.
- **Search** `POST /search/series?page=<0-based>&size=&sort=<field>,asc|desc`, answering
  `{content, last}`. Sorts `rating`, `popular_now` and ascending order work. Bodies take
  `current_books`/`start_year` as `{min, max}`, `original_language` codes (an unknown one
  matches nothing), and `publication_status` from `Ongoing`, `Hiatus`, `Completed`,
  `Abandoned` (anything else is a 400 naming the variants).
- More than 50 ids in one list answers `400 {"error":"too_many_values"}`, so the source
  stops with its own message first. `/sources/list` has 315 sources, so it bites.
- Lookups: `POST /search/staff?size=8 {name}` and `GET /groups/search?q=&limit=8` both
  answer; the search body takes the resolved id as `staff`/`groups` `{values: [id]}`.
- `GET /series/trending` answers ten entries, but the section was dropped on request.
- Search results carry `upload_status`, `age_rating`, `is_pornographic`, `format`, and
  `genre_list`/`tag_list` (`{id, name}`) and `staff_list` (`{id, name, role}`); the bare
  `genres`/`tags`/`staff` are ids only. No description, so the big scroller shows everything
  else from the search response.
- **Upstream history** (`git log upstream/main -- sources/multi.kagane`; PR heads fetch as
  `refs/pull/<n>/head`): #603 added it, and its review moved the request helpers into
  `helpers.rs`, replaced a languages setting with the app's languages, cut clones, and
  dropped `supportsTagSearch` because the static genre filter already makes tags
  tappable. #632, #811 moved the domain (`kagane.to` → `kagane.org`); #728 added the
  sort list and the relevance fallback to `total_views` for browsing.
