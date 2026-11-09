# Keyoapp Template

Two generations of Keyoapp sites:

- `Keyoapp<T: Impl>` for the first, whose `/series/` page lists the whole catalogue as
  `#searched_series_page > button` cards.
- `KeyoappV2<T: ImplV2>` for the second, with series at the site root, `?browse=1` for the
  catalogue and `?_ajax=search` behind it. Its consumers set `"supportsTagSearch": true`, since
  a series page's tags are cased unlike the browse list.

```rust
register_source!(KeyoappV2<Site>, Home, DeepLinkHandler);
```

## Updating Filters

Each consumer's `res/filters.json` holds the site's own type, genre and status values. On the
`/series/` page, paste into the console:

```js
(() => {
	const script = [...document.scripts].map(s => s.textContent).find(t => t.includes("initializeDropdownMenu"));
	for (const kind of ["type", "genre", "status"]) {
		const part = script.slice(script.indexOf(`type: "${kind}"`));
		const items = part.slice(part.indexOf("items:"), part.indexOf("]"));
		const options = [...items.matchAll(/displayName:\s*"([^"]+)"/g)].map(m => m[1]);
		const ids = [...items.matchAll(/value:\s*"([^"]+)"/g)].map(m => m[1]);
		console.log(kind, "options:", JSON.stringify(options));
		console.log(kind, "ids:", JSON.stringify(ids));
	}
})();
```

Keep `novel` among the types: first generation novels read as text and second generation ones as
images. Every filter is include-only, as on the site, so each states `"canExclude": false`.

For the second generation, the sort, type and status selects and the genre list come from the
`?browse=1` page:

```js
(() => {
	for (const [id, key] of [["fsel-sort", "sort"], ["fsel-type", "type"], ["fsel-status", "status"], ["fsel-genres", "tag[]"]]) {
		const options = [...document.querySelectorAll(`#${id} [role=option]`)];
		console.log(id, "options:", JSON.stringify(options.map(o => o.textContent.trim())));
		console.log(id, "ids:", JSON.stringify(options.map(o => new URL(o.href).searchParams.get(key) ?? "")));
	}
})();
```

Sorts are the template's fixed list; genre ids are the lowercase `tag[]` values.

## Home

The home follows the site's headings. A row that repeats another one on a particular site can be
left out with `Params::hidden_rows`, as Genz Toon does with its Pinned Series.
