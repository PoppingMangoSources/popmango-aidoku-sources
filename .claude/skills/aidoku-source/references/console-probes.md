# Console probes

When an API is undocumented and the sandbox cannot reach the site, the person runs these in
the site's own DevTools console (Safari on iOS works) and pastes the output. Same-origin
`fetch` carries the site's cookies, so the answers match what a reader's session sees.

Rules that each cost a round trip once:

- **Send `Accept: application/json`.** Without it, Toonkotsu's filtered `/cdn/v1/latest`
  requests hung for 8+ seconds and looked like "the feed takes no filters". With it they
  answered in 200 ms, filtered.
- **Bound every request** with `AbortSignal.timeout(…)` and catch per request, or one hang
  silently stops the loop and the person pastes a single line.
- **Print status and a body prefix**, not just parsed fields, so an error page is visible.
- **Include a value that should match nothing** (`zzz`) and one that should narrow
  (`manhwa`). Same result for both means the parameter is ignored.
- **Print a total or a count**, so sums can be compared against the unfiltered total.
- **Keep the printed label out of the request.** A Kagane probe appended its label to the
  url (`size=5 books 1-3`), and every case answered "invalid digit" — a probe bug that read
  as the API refusing the parameter.

## Does an endpoint honour a parameter?

```js
(async () => {
  const base = '/cdn/v1/browse?limit=5&sort=latest';
  for (const p of ['', '&type=zzz', '&type=manhwa', '&exclude=Romance']) {
    const s = performance.now();
    try {
      const r = await fetch(base + p, { headers: { Accept: 'application/json' }, signal: AbortSignal.timeout(30000) });
      const t = await r.text();
      let line; try { const j = JSON.parse(t); line = `total ${j.total} | ` + (j.items || []).map(i => (i.series || i).title + ' [' + (i.series || i).type + ']').join(' | '); } catch { line = t.slice(0, 200); }
      console.log(p || '(none)', r.status, Math.round(performance.now() - s) + 'ms', line);
    } catch (e) { console.log(p, 'FAILED', Math.round(performance.now() - s) + 'ms', e.name); }
  }
})();
```

## Which values does a parameter take?

Count each candidate and compare the sum with the unfiltered total; a gap means values are
missing, an overshoot means aliases (Toonkotsu answers `zh-hk` and `zh-ro` as `zh`).

```js
(async () => {
  const q = async p => (await (await fetch('/cdn/v1/browse?limit=1&' + p, { headers: { Accept: 'application/json' } })).json()).total;
  const all = await q('');
  let sum = 0;
  for (const v of ['ja', 'ko', 'zh', 'en', 'other']) { const n = await q('originalLanguage=' + v); sum += n; console.log(v, n); }
  console.log('sum', sum, 'of', all);
})();
```

## What does a facets or config endpoint carry?

```js
(async () => {
  const f = await (await fetch('/cdn/v1/facets?sfw=0', { headers: { Accept: 'application/json' } })).json();
  for (const [k, v] of Object.entries(f)) console.log(k, Array.isArray(v) ? v.length : '', JSON.stringify(v).slice(0, 600));
})();
```

## What does the site's own UI send?

Open the Network tab, use the site's filter or sort, and read the request URL. That is how
Toonkotsu's sort ids (`latest`, `new`, `popular`, `score`…) and `status=Complete` were found;
guessing lowercase status values shipped broken once.
