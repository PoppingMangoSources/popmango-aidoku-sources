import http.server
from urllib.parse import parse_qs, urlsplit


class Responses(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        path = urlsplit(self.path).path.rstrip("/")
        kind = path.rsplit("/", 1)[-1]
        status = 200
        body = ""
        if path.startswith(("/keyoapp/", "/keyoapp-v2/")):
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
