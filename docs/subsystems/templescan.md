---
title: Temple Scan (en.templescan)
type: subsystem
created: 2026-11-28
updated: 2026-11-28
sources:
  - code: sources/en.templescan/src/helpers.rs
  - code: sources/en.templescan/src/lib.rs
  - code: sources/en.templescan/src/test.rs
  - code: sources/en.templescan/src/settings.rs
  - code: sources/en.templescan/res/settings.json
  - ext: https://templetoons.com/comics
verified_at: 27f8122
---

Temple Scan (templetoons.com) is a Next.js app: the catalogue, details and reader data come
from the page's RSC Flight payload, with obfuscated keys (`rsc.rs`) and scrambled reader
tokens (`tokens.rs`). This sandbox can reach the site, so `live_pages_pass_the_site_check` in
`src/test.rs` runs here.

> ⚠️ Every page request without the headers a browser sends when it opens a page gets a 307
> to `/challenge?redirect=…`, the site's own Turnstile page, served with a 200. Probed on
> 2026-11-28: `Accept`, `Sec-Fetch-Dest: document`, `Sec-Fetch-Mode: navigate` and
> `Sec-Fetch-Site: none` are each required, while `Accept-Language`,
> `Upgrade-Insecure-Requests` and the user agent made no difference. `build_request`
> (`sources/en.templescan/src/helpers.rs:202`) sends the four. The live test fails with the
> verification error once `Accept` is removed.

- **No Verify setting.** Up to v13 a web login item opened the check page and sent the cookies
  it collected. It never helped: the redirect comes from the missing headers, so the app kept
  showing the verification error after a pass. v14 removed it.
> ⚠️ Every catalogue and chapter date is `yyyy-MM-ddTHH:mm:ss.SSSZ` (1217 of 1217 on
> 2026-11-28), parsed with one format in UTC. The test runner drops the format's quoted `'T'`,
> so the dates are `None` there and the date sorts are checked in the app, not asserted.

> ⚠️ Descriptions are HTML whose apostrophes arrive as `&#x27;`. `strip_html` parses the
> fragment and takes its text, as merged SoraRaw does. The old hand loop decoded `&#39;` but not
> `&#x27;`, and cut every description at its first `#`, so "Don't" ended the text. No live
> description carried a hashtag (16 of 16 on 2026-11-28), so nothing is cut now.

> ⚠️ Only a query starting with `BASE_URL` is treated as a pasted link. It answers with a card
> of key, title and cover, and a link whose page fails is searched as text.

- **The check page is detected by its script** (`/app/challenge/page-<hash>.js`), not by its
  title: a series title containing "Verify" or "Challenge" would otherwise fail its details.
- **Images take only a Referer.** Covers and pages on `media.templetoons.com` load with it.
  Adding browser image headers (`Sec-Fetch-Dest: image`, `Sec-Fetch-Site: same-site`) got a
  403 in the same probe, so the image request provider leaves them out.
- **Script chunks** (`/_next/static/chunks/*.js`, read for the reader seed) answer without the
  headers, and `build_request` sends them anyway.

The source's own web view keeps its cookies apart from plain requests
([aidoku-host.md](../../.claude/skills/aidoku-source/references/aidoku-host.md)), so passing
the check page in one would not unlock the other. If the check ever comes back with the
headers sent, that is where a fix has to start.
