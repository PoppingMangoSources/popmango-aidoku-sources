import http.server
import json
from urllib.parse import parse_qs, urlsplit


class Responses(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        path = urlsplit(self.path).path.rstrip("/")
        kind = path.rsplit("/", 1)[-1]
        status = 200
        body = ""
        if path.startswith("/vine"):
            query = parse_qs(urlsplit(self.path).query)
            series = {"slug": "title", "title": "Title", "coverImage": "/cover.webp", "type": "MANHWA", "status": "ONGOING", "description": "<p>Plain description.</p>", "author": "First & Second,First"}
            def chapters(numbers):
                return [{"number": n, "isLocked": n == 3, "hasAccess": n != 3, "publishedAt": "2026-11-13T12:00:00.000Z"} for n in numbers]
            if "/chapter/" in path:
                if "/novel/" in path:
                    text = "<p><strong>First<br><em>Second</em></strong></p><p>Third 界.</p>"
                    reader = {"chapter": {"content": "$a"}, "isUnlocked": True, "isLocked": False, "kind": "novel"}
                    body = "1:" + json.dumps(reader) + "\na:T" + format(len(text.encode()), "x") + "," + text
                else:
                    pages = [{"imageUrl": "/1.webp"}, {"imageUrl": "/2.webp"}]
                    if kind == "missing": pages[1]["imageUrl"] = " "
                    if kind == "empty": pages = []
                    if kind == "fragments": pages[0]["hasFragments"] = True
                    if kind in {"inactive-protection", "active-protection"}:
                        for page in pages: page.update(isEncrypted=True, hasStrips=True)
                    if kind == "redacted": pages[0]["isRedacted"] = True
                    reader = {"chapter": {"content": "", "pages": pages}, "isUnlocked": kind != "locked", "isLocked": kind == "locked"}
                    if kind in {"inactive-protection", "active-protection", "redacted"}: reader["protectionConfig"] = {"isProtected": kind == "active-protection"}
                    body = '1:{"chapter":{"id":"metadata","number":2}}\n2:' + json.dumps(reader)
            elif "/api/ai/series/" in path:
                data = dict(series, chapter_count=3)
                if query.get("chapters") == ["all"]: data["chapters"] = chapters([1, 2, 3])
                if kind == "incomplete": data["chapter_count"] = 4
                if kind == "duplicate": data["chapters"] = chapters([1, 2, 2])
                body = json.dumps(data)
            elif path.endswith("/api/series/facets"):
                body = json.dumps({"genres": [{"name": "Action", "slug": "action"}], "tags": []})
            elif path.endswith("/api/series"):
                if query.get("q") and (query.get("q") != ["word & café"] or query.get("sort") != ["newest"] or query.get("genre") != ["action"] or query.get("exgenre") != ["romance"] or query.get("ch_min") != ["20"] or "order" in query): status = 400
                if "genre" in query and query["genre"] != ["action"]: status = 400
                page_two = query.get("page") == ["2"]
                entry = dict(series, slug="title-2" if page_two else "title")
                body = json.dumps({"data": [entry], "meta": {"hasMore": not page_two}})
            elif path == "/vine":
                def section(title, marker, novel=False):
                    route = "novel/title" if novel else "comic/title"
                    return f"<section><h2>{title}</h2><div><a href='/series/{route}?ref={marker}'><img src='/cover.webp' alt='Title'></a><a href='/series/{route}?ref={marker}'><h3>Title</h3></a><a href='/series/{route}/chapter/2'><span data-chapter-row-time>1d</span></a></div></section>"
                comic = dict(series, chapters=chapters([3, 2]))
                novel = dict(comic, type="WEB_NOVEL")
                flight = "1:" + json.dumps({"initialSlides": [series], "all": [comic, novel]})
                body = "<html><body>" + section("New Series", "homepage_new") + section("On Sale Now", "homepage_sale") + section("Latest Novel Updates", "homepage_latest", True) + section("Latest Comic Updates", "homepage_latest") + section("Popular Today", "homepage_popular") + section("Top 10 in All", "homepage_most_popular") + "<script>self.__next_f.push(" + json.dumps([1, flight]) + ")</script></body></html>"
            else:
                status = 404
        elif path.startswith(("/keyoapp/", "/keyoapp-v2/")):
            title = " " if kind == "blank-title" else "Title"
            slug = " " if kind == "blank-slug" else "title"
            body = f'<div class="grid" data-series-uid="1" data-series-slug="{slug}"><h1>{title}</h1></div>'
            if path.startswith("/keyoapp-v2/"):
                if kind != "blocks":
                    body += '<p class="l2-by">By ' + (" " if kind == "empty-by" else "First &amp;Second,First") + '</p>'
                for label, value in [("Author", "First ,Second &amp; First, ,"), ("Artist", "Third &amp;First,Third")]:
                    body += f'<div class="bg-surface rounded-lg"><div class="text-xs">{label}</div><span class="text-sm">{value}</span></div>'
            else:
                for label, value in [("Author", "First ,Second &amp;First, ,"), ("Artist", "Third &amp;First,Third")]:
                    body += f'<div class="grid"><div><span>{label}</span></div><div>{value}</div></div>'
                if kind != "missing":
                    container = 'id="expand_content"' if kind == "expanded" else 'class="overflow-hidden"'
                    text = " " if kind == "empty" else "Full synopsis"
                    body += f'<div class="grid gap-2"><div>Synopsis</div><div class="grid"><div {container}><p style="white-space: pre-wrap">{text}</p></div></div></div>'
        elif path.startswith("/mangathemesia/"):
            body = f'<div class="postbody"><h1 class="entry-title">Title</h1><span class="type">{kind}</span><div class="comments"><span>Author discussed this story.</span></div></div>'
        elif path == "/keyoapp-home":
            monthly = parse_qs(urlsplit(self.path).query).get("range") == ["monthly"]
            if urlsplit(self.path).query:
                body = f'<a data-slide-card href="/reviewed" title="{"Monthly" if monthly else "Wrong period"}"><img data-src="https://example.com/cover.jpg"></a>'
            else:
                body = '<section><h2>Trending Now</h2><a data-slide-card href="/today" title="Today"><img data-src="https://example.com/cover.jpg"></a></section><section><h2>Most Reviewed Series</h2></section>'
        else:
            status = 404
        if kind.startswith("novel"):
            text = '<p><strong>First<br>Second</strong></p><p><em>Third</em></p>'
            if kind == "novel-adjacent":
                text = '<p><b><i>First</i></b><b><i>Second</i></b></p>'
            if path.startswith("/keyoapp-v2/"):
                body = f'<div id="strip-reader"><div class="reader-page"><div class="prose-note">{text}</div></div><img class="reader-page" src="https://example.com/page.jpg"></div>'
            elif path.startswith("/keyoapp/"):
                body = f'<div class="tiptap novel-reader">{text}</div>'
            elif path.startswith("/mangathemesia/"):
                body = f'<div id="readerarea">{text}</div><script>ts_reader.run({{"is_novel":true,"images":[]}})</script>'
        self.send_response(status)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        self.wfile.write(body.encode())

    def log_message(self, *_args):
        pass
