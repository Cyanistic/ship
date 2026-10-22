"""Serve a built site the way a static host would: directory indexes, /404.html for anything missing,
markdown and llms.txt as text. For local checks only.
usage: python3 tools/serve.py <dir> <port>"""
import http.server, os, sys

root, port = os.path.abspath(sys.argv[1]), int(sys.argv[2])


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map,
                      ".md": "text/markdown; charset=utf-8", ".txt": "text/plain; charset=utf-8"}

    def __init__(self, *a, **k):
        super().__init__(*a, directory=root, **k)

    def send_error(self, code, message=None, explain=None):
        page = os.path.join(root, "404.html")
        if code != 404 or not os.path.exists(page):
            return super().send_error(code, message, explain)
        body = open(page, "rb").read()
        self.send_response(404)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def log_message(self, *a):
        pass


class Server(http.server.ThreadingHTTPServer):
    # the default backlog of 5 drops connections when a page and Pagefind's worker fetch in parallel,
    # which surfaces as a flaky "Failed to fetch" in search
    request_queue_size = 128


Server(("127.0.0.1", port), Handler).serve_forever()
