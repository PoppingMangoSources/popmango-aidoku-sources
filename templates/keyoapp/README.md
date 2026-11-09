# Keyoapp Template

For sites on Keyoapp's first generation, whose `/series/` page lists the whole catalogue as
`#searched_series_page > button` cards. Sites on the newer `?_ajax=search` pages need their own
implementation.

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

Leave `novel` out of the types: the template has no novel reader and hides novels everywhere.
Every filter is include-only, as on the site, so each states `"canExclude": false`.

## Home

The home follows the site's headings. A row that repeats another one on a particular site can be
left out with `Params::hidden_rows`, as Genz Toon does with its Pinned Series.
