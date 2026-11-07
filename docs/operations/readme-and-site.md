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
blush gradient `#ff5a76` to `#ffe9f2`. The install page in `public/` uses the same palette and
opens with a manhwa-style "System" quest window.

- `public/main.js` sets the window's level (`LV.`), reward and bar from `index.min.json`: the
  level is the catalogue size, the bar fills with what the search and filters show, and an empty
  result prints a line in the same voice. Nothing in the page hardcodes a source count.
- `media/badge-sources.svg` and the README's "**N sources**" line do hardcode the count, so
  they change whenever a source joins or leaves `main` (a merge upstream removes it).

> ⚠️ The README loads its header images by relative path (`media/banner.svg`), not through
> jsDelivr. jsDelivr caches `@main` files for days and ignores a query string, and GitHub's
> image proxy then caches whatever it was served under that url, so the sources badge kept
> showing 25 after the file said 16. A relative path is served from the commit itself. The
> source icons further down still use jsDelivr; an icon rarely changes.

> ⚠️ The token-optimizer read guard blocks reading `.svg` and `.png` files whole. Edit SVGs by
> writing a copy and `cp`, and view a screenshot with `Read` plus `limit: 1`.
