---
title: README art and the install page
type: operation
created: 2026-11-07
updated: 2026-11-07
sources:
  - code: README.md
  - code: media/banner.svg
  - code: media/badge-sources.svg
  - code: public/index.html
  - code: public/main.js
  - code: public/styles.css
verified_at: f082439
---

# README art and the install page

The README header (banner, four badges, add button) is hand-written SVG in `media/`, in the
red "doki doki" heart style of the owner's profile art: ink `#33162a`, heart red `#ff4d63`,
blush gradient `#ff5a76` to `#ffe9f2`. The install page in `public/` uses the same palette as a
manhwa system crossed with a dating sim: Mango-chan (the heart mascot) talks in a visual novel
dialogue box that ends in choices (add, copy, browse), a STATUS window carries the level and an
affection meter that choices and downloads fill, SYSTEM toasts report each action, a quest
window holds the repository url, and the sources are "routes" on a Route Select grid.

- `public/main.js` sets the level (`LV.`), reward and bar from `index.min.json`: the level is the
  catalogue size, the bar fills with what the search and filters show, and an empty result shows
  a "bad ending" with a reset. Nothing in the page hardcodes a source count.
- The Add choice is an `aidoku://` link; off Apple devices it shows a toast with the manual steps
  instead of following the link.
- The README lists sources as two-column Route select card tables (`media/route-button.svg`,
  `media/heat-<contentRating>.svg`). Each card keeps the two `.aix` links and the `.aix (vN)` label
  that `scripts/bump.sh` rewrites and `scripts/check-sources.py` reads, and `scripts/new-source.sh`
  prints a card in the same shape.
- `media/badge-sources.svg` and the README's "**N routes**" line do hardcode the count, so
  they change whenever a source joins or leaves `main` (a merge upstream removes it).

> ⚠️ The README loads its header images by relative path (`media/banner.svg`), not through
> jsDelivr. jsDelivr caches `@main` files for days and ignores a query string, and GitHub's
> image proxy then caches whatever it was served under that url, so the sources badge kept
> showing 25 after the file said 16. A relative path is served from the commit itself. The
> source icons further down still use jsDelivr; an icon rarely changes.

> ⚠️ `bump.sh` stops unless a source has exactly two `.aix` links in the README. A source shown in
> both lists (ValirScans carries comics and novels) links only from its first card; the second
> card says it is the same source.

> ⚠️ The install page only reaches GitHub Pages when **Build source list** (`build.yaml`) runs,
> since it deploys `public/` to `gh-pages`. Its push paths must keep `public/**`: without it, two
> redesigns sat on `main` while the live site still served the original page.

> ⚠️ The token-optimizer read guard blocks reading `.svg` and `.png` files whole. Edit SVGs by
> writing a copy and `cp`, and view a screenshot with `Read` plus `limit: 1`.
