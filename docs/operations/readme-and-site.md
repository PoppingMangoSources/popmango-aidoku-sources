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
- The Add choice and the quest's "Accept quest" button (both `.add-repo`) are `aidoku://` links;
  off Apple devices they show a toast with the manual steps instead. Affection is kept in
  `localStorage`, wrapped in try/catch, so it survives a reload when storage allows.

> ⚠️ Dark mode is `:root[data-theme="dark"]`, not a `prefers-color-scheme` query. A script in
> `index.html`'s head sets `data-theme` before the first paint (light, unless the reader chose dark
> with the toggle; the owner wants light as the default whatever the device setting) and the
> moon/sun toggle in `main.js` flips and saves it. A theme rule on a bare
> element must sit in `:where(...)`: `[data-theme="dark"] a` out-ranked `.brand` and the button
> links and turned them pink.

> ⚠️ `index.html` loads `styles.css?v=dev` and `main.js?v=dev`, and `build.yaml` replaces `dev`
> with the commit before deploying. Without it Safari kept the previous deploy's script and
> stylesheet under the new markup: the old script crashed on a renamed button before loading the
> sources (LV. ?, an empty ticker, no routes) and the old CSS left the new buttons unstyled.

> ⚠️ Hover effects sit inside `@media (hover: hover)`: iOS keeps `:hover` on whatever was last
> tapped, which left cards highlighted and choices shifted on a phone.

> ⚠️ The ticker's clipping strip must not be rotated. iOS Safari drew the moving row as an empty
> red band while `.ticker` itself carried `rotate` (with a `100vw` negative-margin bleed and
> `will-change`); shrinking the row to icons did not cure it. The Mana site's straight ticker
> never blanks, so the strip now matches it: `.ticker` sits outside `.wrap` at full width and holds
> only the row (two `.ticker__set`s sliding by `translateX(-50%)`), and the tilt is a still
> `::before` band on the `.ticker-band` around it. With the band inside the clip the row still
> froze now and then, so keep the clip holding the row alone.

> ⚠️ Keep the page's animation light. The ticker froze on iOS while eight floating hearts animated
> a `clip-path` and a ray layer 1.8 times the scene's size spun behind Mango-chan. The hearts are
> now `♥` glyphs and the rays are still.

> ⚠️ Spacing, type sizes, radii and the fixed colours are named in `:root` (`--sp-N` in 4px steps
> by multiple, `--text-*`, `--r-*`, `--ink-fixed`, `--white`, `--safe-*`). Write a new value as
> one of these; the dialogue's 13rem room for Mango-chan is the one deliberate literal.
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
