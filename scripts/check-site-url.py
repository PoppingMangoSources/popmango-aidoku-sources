#!/usr/bin/env python3
"""Fails when a source's `source.json` url now redirects to another host.

kkantan on #878: "it looks like this redirects to https://okutoon.net now". A site that refuses
the request (Cloudflare, an outage) is reported, not failed, since that proves nothing.

Usage: scripts/check-site-url.py <id>...
"""

import json
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def host(url: str) -> str:
    return urllib.parse.urlparse(url).netloc.lower().removeprefix("www.")


def final_url(url: str) -> str:
    request = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    try:
        with urllib.request.urlopen(request, timeout=40) as response:
            return response.geturl()
    except urllib.error.HTTPError as error:
        return error.geturl() or url


def main() -> int:
    failed = False
    for source in sys.argv[1:]:
        info = json.loads((ROOT / "sources" / source / "res" / "source.json").read_text())["info"]
        url = info.get("url") or info["urls"][0]
        try:
            final = final_url(url)
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            print(f"{source}: could not reach {url} ({error}); check the address by hand")
            continue
        if host(final) != host(url):
            print(f"FAIL {source}: {url} now redirects to {final}; move the source to the new address")
            failed = True
        else:
            print(f"{source}: {url} answers at its own address")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
