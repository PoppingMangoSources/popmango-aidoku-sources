#!/usr/bin/env python3
"""Run template HTTP regressions against a loopback server, without contacting a site."""

import http.server
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
from urllib.parse import urlsplit, parse_qs


class Responses(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        parts = urlsplit(self.path).path.split("/")
        status = int(parts[2]) if parts[1] == "status" else 200
        content_type = "text/html; charset=utf-8"
        body = "<h1>Unavailable</h1>" + "界" * 300 + "PREFIX_MUST_STOP_BEFORE_THIS"
        if self.path == "/json":
            content_type = "application/json"
            body = '{"html":"","has_more":false}'
        if self.path.startswith("/metadata/"):
            kind = parts[2]
            body = f'<div class="postbody"><h1 class="entry-title">Title</h1><div class="tsinfo"><span class="type">{kind}</span></div><div class="comments"><span>Author discussed this story.</span><span>Artist discussed this story.</span></div></div>'
        if self.path == "/novel":
            body = '<div id="readerarea"><p><strong>First<br>Second</strong></p><p><em>Third</em></p></div><script>ts_reader.run({"is_novel":true,"images":[]})</script>'
        query = parse_qs(urlsplit(self.path).query)
        if urlsplit(self.path).path.startswith("/api/"):
            content_type = "application/json"
            chapter = {"id": 11, "slug": "chapter-1", "number": "1", "content": "<p><strong>Text</strong></p>"}
            if parts[2] == "chapter":
                if set(query) != {"chapterId"}:
                    status = 400
                chapter_id = query.get("chapterId", [""])[0]
                if chapter_id == "12": chapter.update(content=None, images=[{"url": " "}])
                if chapter_id == "13": chapter.update(isLocked=False, isTimeLocked=True)
                body = json.dumps({"chapter": chapter})
            elif parts[2] == "chapters":
                skip = int(query.get("skip", ["0"])[0])
                post_id = query.get("postId", [""])[0]
                ids = [11, 12] if skip == 0 or post_id == "998" else ([13] if post_id != "999" else [])
                body = json.dumps({"post": {"chapters": [dict(chapter, id=i) for i in ids]}, "totalChapterCount": 3})
        if parts[1:3] == ["captured", "home"]:
            evidence = os.environ.get("AIDOKU_IKEN_EVIDENCE_DIR")
            if evidence and parts[3] in {"hive", "nyx", "vortex", "magus", "eternal"}:
                body = (Path(evidence) / (parts[3] + "-home.html")).read_text()
            else:
                status = 404
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.end_headers()
        self.wfile.write(body.encode())

    def log_message(self, *_args):
        pass


def run(crate):
    root = Path(__file__).resolve().parents[1]
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), Responses) as server:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        base = f"http://127.0.0.1:{server.server_port}"
        try:
            result = subprocess.run(
                ["cargo", "test", "review_http_", "--", "--ignored", "--nocapture"],
                cwd=root / crate,
                env={**os.environ, "AIDOKU_HTTP_TEST_URL": base, "NO_PROXY": "127.0.0.1"},
                capture_output=True, text=True, timeout=180,
            )
        finally:
            server.shutdown()
            thread.join()
    print(result.stdout, end="")
    print(result.stderr, end="")
    if result.returncode:
        return result.returncode
    lines = result.stdout.splitlines()
    for status in (403, 404, 429, 503):
        if not any(line.startswith(f"URL: {base}/status/{status}") and
                   f'; HTTP {status}; Content-Type: Some("text/html; charset=utf-8"); body prefix: Some("<h1>Unavailable</h1>' in line
                   for line in lines):
            raise AssertionError(f"Missing HTTP {status} diagnostics for {crate}")
    if crate == "templates/keyoapp":
        for path in ("/invalid-json", "/series/"):
            if not any(line.startswith(f"URL: {base}{path}; HTTP 200;") for line in lines):
                raise AssertionError(f"Missing parser diagnostics for {path}")
    if "PREFIX_MUST_STOP_BEFORE_THIS" in result.stdout:
        raise AssertionError("Response prefix was not bounded")
    for line in result.stdout.splitlines():
        if "; body prefix: Some(" in line:
            prefix = json.loads(line.split("; body prefix: Some(", 1)[1][:-1])
            if len(prefix) != 240:
                raise AssertionError("Expected exactly 240 Unicode characters in the preview")
    return 0


if __name__ == "__main__":
    crates = sys.argv[1:] or ["templates/keyoapp", "templates/mangathemesia", "templates/iken"]
    if any(crate not in {"templates/keyoapp", "templates/mangathemesia", "templates/iken"} for crate in crates):
        raise SystemExit("Expected templates/keyoapp, templates/mangathemesia or templates/iken")
    results = [run(crate) for crate in crates]
    raise SystemExit(int(any(results)))
