#!/usr/bin/env python3
"""Record and replay model traffic for the VHS demo tapes.

The demo's agents (claude and pi) send their model requests to this proxy on
127.0.0.1 instead of to the providers. Requests under /backend-api/ go to the
ChatGPT backend that pi's openai-codex provider uses; everything else goes to
api.anthropic.com.

  model-proxy.py record --dir DIR   forward each request to the real provider,
                                    stream the reply back unchanged, and save
                                    each successful reply under DIR. Request
                                    headers, and so credentials, are forwarded
                                    and never written anywhere.
  model-proxy.py replay --dir DIR   serve the saved replies from DIR with no
                                    outbound connections at all. A request
                                    with no saved reply gets a 500.

Matching. A request's conversation key hashes its method, path, model, tool
names and first user message (skipping <system-reminder> blocks). Requests
for one key are served in the order they were recorded, so a side request
with its own system prompt or tools never takes a turn's reply.

ID rewrite. Ship IDs (`pane:` plus 32 hex digits) differ on every run. Each
request carries the IDs from all tool results so far, in order. Replay pairs
the recorded list with the live list by position and rewrites the recorded
reply to use the live IDs. Streamed strings are held back a few characters so
an ID split across deltas is still matched whole.

Pacing. Replay sleeps for the recorded gap between SSE events, capped at
--max-gap seconds and scaled by --speed.

Needs Python 3.14 or later for compression.zstd, which decodes pi's
zstd-compressed request bodies.
"""

import argparse
import copy
import gzip
import hashlib
import http.client
import http.server
import json
import os
import re
import signal
import ssl
import sys
import threading
import time
import urllib.parse
from collections import deque

PORTS = range(43332, 43338)
ANTHROPIC = "https://api.anthropic.com"
CODEX = "https://chatgpt.com"
HOLD = 40  # longest ID we rewrite is 36 characters
ID_RE = re.compile(
    r"(?<![0-9a-f])(?:[0-9a-f]{32}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?![0-9a-f])"
)
FINAL = {"message_stop", "response.completed", "response.incomplete", "response.failed", "error"}
HOP = {
    "connection", "keep-alive", "proxy-connection", "te", "trailer", "transfer-encoding",
    "upgrade", "host", "content-length", "accept-encoding",
}


def say(msg):
    print(f"model-proxy: {msg}", file=sys.stderr, flush=True)


def norm(text):
    return text.replace("-", "")


def hyphenate(hex32):
    return f"{hex32[:8]}-{hex32[8:12]}-{hex32[12:16]}-{hex32[16:20]}-{hex32[20:]}"


# Request parsing ----------------------------------------------------------

def decode_body(body, encoding):
    if not body:
        return b""
    if encoding == "zstd":
        from compression import zstd  # Python 3.14+

        return zstd.decompress(body)
    if encoding == "gzip":
        return gzip.decompress(body)
    return body


def items_of(obj):
    """Conversation items: Anthropic `messages`, or Responses `input`."""
    for field in ("messages", "input"):
        if isinstance(obj.get(field), list):
            return obj[field]
    return []


def text_of(content):
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "".join(b.get("text", "") for b in content if isinstance(b, dict) and isinstance(b.get("text"), str))
    return ""


def strings_in(obj):
    if isinstance(obj, str):
        yield obj
    elif isinstance(obj, list):
        for value in obj:
            yield from strings_in(value)
    elif isinstance(obj, dict):
        for value in obj.values():
            yield from strings_in(value)


def conversation_key(method, path, obj):
    parts = [method, urllib.parse.urlsplit(path).path]
    if obj is not None:
        parts.append(str(obj.get("model", "")))
        names = [t.get("name", "") for t in obj.get("tools") or [] if isinstance(t, dict)]
        parts.append(json.dumps(sorted(names)))
        first = ""
        for item in items_of(obj):
            if isinstance(item, dict) and item.get("role") == "user":
                text = text_of(item.get("content"))
                if text and not text.startswith("<system-reminder>"):
                    first = text
                    break
        parts.append(first)
    return hashlib.sha256("\n".join(parts).encode()).hexdigest()[:16]


def tool_result_ids(obj):
    """Normalised IDs found in tool results, in conversation order."""
    texts = []
    for item in items_of(obj):
        if not isinstance(item, dict):
            continue
        if item.get("type") == "function_call_output":
            texts.extend(strings_in(item.get("output")))
        content = item.get("content")
        if isinstance(content, list):
            for block in content:
                if isinstance(block, dict) and block.get("type") == "tool_result":
                    texts.extend(strings_in(block.get("content")))
    return [norm(m.group()) for text in texts for m in ID_RE.finditer(text)]


def describe(method, path, encoding, body):
    """What a request is matched and paired on. Nothing from it is stored raw."""
    try:
        decoded = decode_body(body, encoding)
        obj = json.loads(decoded) if decoded else None
    except Exception as exc:  # noqa: BLE001 - reported, not fatal
        return {"key": conversation_key(method, path, None), "model": None, "msgs": 0, "ids": [],
                "unreadable": type(exc).__name__}
    if not isinstance(obj, dict):
        obj = None
    return {
        "key": conversation_key(method, path, obj),
        "model": obj.get("model") if obj else None,
        "msgs": len(items_of(obj)) if obj else 0,
        "ids": tool_result_ids(obj) if obj else [],
    }


# SSE rewriting ------------------------------------------------------------

def get_at(obj, path):
    for key in path:
        obj = obj[key]
    return obj


def set_at(obj, path, value):
    for key in path[:-1]:
        obj = obj[key]
    obj[path[-1]] = value


def walk(obj, fn):
    if isinstance(obj, dict):
        for key, value in obj.items():
            if isinstance(value, str):
                obj[key] = fn(value)
            else:
                walk(value, fn)
    elif isinstance(obj, list):
        for i, value in enumerate(obj):
            if isinstance(value, str):
                obj[i] = fn(value)
            else:
                walk(value, fn)


def stream_slot(ev):
    """(stream id, path to the streamed string) for a delta event, else None."""
    kind = ev.get("type")
    if kind == "content_block_delta" and isinstance(ev.get("delta"), dict):
        for field in ("text", "partial_json", "thinking"):
            if isinstance(ev["delta"].get(field), str):
                return ("cb", ev.get("index")), ("delta", field)
        return None
    if isinstance(kind, str) and kind.endswith(".delta") and isinstance(ev.get("delta"), str):
        return ("rs", kind, ev.get("item_id"), ev.get("output_index"), ev.get("content_index")), ("delta",)
    return None


def stream_ends(sid, ev):
    kind = ev.get("type") or ""
    if sid[0] == "cb":
        return kind == "content_block_stop" and ev.get("index") == sid[1]
    if not (kind.endswith(".done") or kind == "response.output_item.done"):
        return False
    if "output_index" in ev:
        return ev["output_index"] == sid[3]
    return "item_id" in ev and ev["item_id"] == sid[2]


def frame(ev):
    lines = [f"event: {ev['type']}"] if "type" in ev else []
    lines.append("data: " + json.dumps(ev, ensure_ascii=False, separators=(",", ":")))
    return "\n".join(lines) + "\n\n"


def split_sse(raw):
    name, data = None, []
    for line in raw.split("\n"):
        if line.startswith("event:"):
            name = line[6:].strip()
        elif line.startswith("data:"):
            data.append(line[5:].lstrip())
    return name, "\n".join(data)


class Rewriter:
    """Swaps recorded IDs for live ones, event by event.

    A streamed string is held back HOLD characters so an ID split across
    deltas is whole when matched. The held tail is flushed in a synthetic
    delta just before the event that ends its block.
    """

    def __init__(self, mapping):
        self.mapping = mapping
        self.open = {}  # stream id -> [template event, path, held tail]
        self.swapped = 0

    def _repl(self, m):
        text = m.group(0)
        live = self.mapping.get(norm(text))
        if live is None:
            return text
        self.swapped += 1
        return hyphenate(live) if "-" in text else live

    def sub(self, text):
        return ID_RE.sub(self._repl, text)

    def _split(self, buf):
        cut = len(buf) - HOLD
        if cut <= 0:
            return "", buf
        found = list(ID_RE.finditer(buf))
        for m in found:
            if m.start() < cut < m.end():
                cut = m.start()
        out, pos = [], 0
        for m in found:
            if m.end() > cut:
                break
            out.append(buf[pos:m.start()])
            out.append(self._repl(m))
            pos = m.end()
        out.append(buf[pos:cut])
        return "".join(out), buf[cut:]

    def _close(self, sid):
        tpl, path, tail = self.open.pop(sid)
        if not tail:
            return []
        flushed = copy.deepcopy(tpl)
        set_at(flushed, path, self.sub(tail))
        return [flushed]

    def event(self, ev):
        out = []
        for sid in [s for s in self.open if stream_ends(s, ev)]:
            out += self._close(sid)
        found = stream_slot(ev)
        if found is not None:
            sid, path = found
            if sid not in self.open:
                tpl = copy.deepcopy(ev)
                set_at(tpl, path, "")
                self.open[sid] = [tpl, path, ""]
            entry = self.open[sid]
            head, entry[2] = self._split(entry[2] + get_at(ev, path))
            set_at(ev, path, head)
            return out + [ev]
        if ev.get("type") in FINAL:
            for sid in list(self.open):
                out += self._close(sid)
        walk(ev, self.sub)
        return out + [ev]

    def finish(self):
        out = []
        for sid in list(self.open):
            out += self._close(sid)
        return out

    def raw_event(self, raw):
        """Rewrite one SSE event given as text; returns the text to send."""
        _, data = split_sse(raw)
        try:
            ev = json.loads(data) if data else None
        except ValueError:
            ev = None
        if not isinstance(ev, dict):
            return [raw]
        return [frame(e) for e in self.event(ev)]


def build_mapping(recorded, live):
    mapping = {}
    for old, new in zip(recorded, live):
        if old not in mapping and old != new:
            mapping[old] = new
    return mapping


# Shared HTTP plumbing -----------------------------------------------------

def upstream_for(path):
    return CODEX if path.startswith("/backend-api/") else ANTHROPIC


def open_upstream(base, method, path, body, headers):
    url = urllib.parse.urlsplit(base)
    if url.scheme == "https":
        conn = http.client.HTTPSConnection(url.hostname, url.port, timeout=600, context=ssl.create_default_context())
    else:
        conn = http.client.HTTPConnection(url.hostname, url.port, timeout=600)
    conn.request(method, path, body=body, headers=headers)
    return conn, conn.getresponse()


def has_body(method, status):
    return method != "HEAD" and status >= 200 and status not in (204, 304)


def send_head(h, status, content_type, chunked):
    """Starts a reply; with chunked=False the reply has no body at all."""
    h.send_response(status)
    if content_type:  # the Codex backend streams SSE with no content-type
        h.send_header("content-type", content_type)
    h.send_header("cache-control", "no-cache")
    if chunked:
        h.send_header("transfer-encoding", "chunked")
    h.end_headers()


def is_sse(content_type, accept):
    if content_type:
        return content_type.startswith("text/event-stream")
    return "text/event-stream" in (accept or "")


def read_body(h):
    """The request body, by Content-Length or chunked transfer coding."""
    if "chunked" in (h.headers.get("transfer-encoding") or "").lower():
        parts = []
        while True:
            size = int(h.rfile.readline().split(b";")[0].strip() or b"0", 16)
            if size == 0:
                while h.rfile.readline() not in (b"\r\n", b"\n", b""):
                    pass  # trailers
                return b"".join(parts)
            parts.append(h.rfile.read(size))
            h.rfile.readline()
    length = int(h.headers.get("content-length") or 0)
    return h.rfile.read(length) if length else b""


def write_chunk(h, data):
    if data:
        h.wfile.write(f"{len(data):X}\r\n".encode() + data + b"\r\n")
        h.wfile.flush()


def write_end(h):
    h.wfile.write(b"0\r\n\r\n")
    h.wfile.flush()


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "model-proxy"
    sys_version = ""

    def _dispatch(self):
        self.server.app.serve(self, self.command, read_body(self))

    do_GET = do_HEAD = do_POST = do_PUT = do_DELETE = do_PATCH = _dispatch

    def log_message(self, *args):
        pass


class Server(http.server.ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True

    def __init__(self, port, app):
        super().__init__(("127.0.0.1", port), Handler)
        self.app = app

    def handle_error(self, request, client_address):
        exc = sys.exc_info()[1]
        if isinstance(exc, (ConnectionResetError, BrokenPipeError)):
            return  # a client dropping a kept-alive connection
        super().handle_error(request, client_address)


# Record -------------------------------------------------------------------

class Recorder:
    def __init__(self, directory, anthropic, codex):
        self.dir = directory
        self.bases = {"anthropic": anthropic, "codex": codex}
        self.lock = threading.Lock()
        self.seq = 0
        self.saved = 0
        self.skipped = 0
        os.makedirs(directory, exist_ok=True)
        if any(name.endswith(".json") for name in os.listdir(directory)):
            raise SystemExit(f"model-proxy: {directory} already holds recordings; clear it first")

    def serve(self, h, method, body):
        with self.lock:
            self.seq += 1
            seq = self.seq
        info = describe(method, h.path, h.headers.get("content-encoding", ""), body)
        # Logged on arrival too, so a request still in flight shows up.
        say(f"record seq={seq:04d} {method} {h.path} <- key={info['key'][:8]} msgs={info['msgs']}")
        upstream = self.bases["codex" if h.path.startswith("/backend-api/") else "anthropic"]
        headers = {k: v for k, v in h.headers.items() if k.lower() not in HOP}
        started = time.monotonic()
        try:
            conn, resp = open_upstream(upstream, method, h.path, body or None, headers)
        except OSError as exc:
            say(f"record seq={seq:04d} {method} {h.path} -> upstream unreachable ({type(exc).__name__})")
            h.send_error(502)
            return
        status = resp.status
        ctype = resp.getheader("content-type", "")
        streaming = is_sse(ctype, h.headers.get("accept"))
        ok = 200 <= status < 300
        body_follows = has_body(method, status)
        send_head(h, status, ctype, chunked=body_follows)
        events, body_parts, pending = [], [], b""
        client_ok = True
        try:
            while body_follows:
                chunk = resp.read1(65536)
                if not chunk:
                    break
                try:
                    write_chunk(h, chunk)
                except OSError:
                    client_ok = False
                    break
                if not ok:
                    continue
                if streaming:
                    pending += chunk
                    while b"\n\n" in pending:
                        raw, pending = pending.split(b"\n\n", 1)
                        events.append({"ms": int((time.monotonic() - started) * 1000),
                                       "raw": raw.decode("utf-8", "replace") + "\n\n"})
                else:
                    body_parts.append(chunk)
            if client_ok and body_follows:
                write_end(h)
        except OSError:
            client_ok = False
        finally:
            conn.close()
        if not ok or not client_ok:
            self.skipped += 1
            say(f"record seq={seq:04d} {method} {h.path} -> {status}, not saved"
                + ("" if client_ok else " (client went away)"))
            return
        if streaming and pending:
            events.append({"ms": int((time.monotonic() - started) * 1000), "raw": pending.decode("utf-8", "replace")})
        saved = {
            "seq": seq, "method": method, "path": urllib.parse.urlsplit(h.path).path,
            "key": info["key"], "model": info["model"], "msgs": info["msgs"], "ids": info["ids"],
            "status": status, "content_type": ctype,
        }
        if body_follows:
            if streaming:
                saved["events"] = events
            else:
                saved["body"] = b"".join(body_parts).decode("utf-8", "replace")
        path = os.path.join(self.dir, f"{seq:04d}.json")
        with open(path + ".tmp", "w", encoding="utf-8") as f:
            json.dump(saved, f, ensure_ascii=False, indent=1)
        os.replace(path + ".tmp", path)
        with self.lock:
            self.saved += 1
        unreadable = f" unreadable={info['unreadable']}" if "unreadable" in info else ""
        say(f"record seq={seq:04d} {method} {h.path} -> {status} key={info['key'][:8]} "
            f"msgs={info['msgs']} ids={len(info['ids'])} events={len(events)}{unreadable}")

    def summary(self):
        return f"record: saved={self.saved} skipped={self.skipped}"


# Replay -------------------------------------------------------------------

class Replayer:
    def __init__(self, directory, speed, max_gap):
        self.speed = speed
        self.max_gap = max_gap
        self.lock = threading.Lock()
        self.queues = {}
        self.served = 0
        self.unmatched = 0
        recs = []
        for name in sorted(os.listdir(directory)):
            if name.endswith(".json"):
                with open(os.path.join(directory, name), encoding="utf-8") as f:
                    recs.append(json.load(f))
        for rec in sorted(recs, key=lambda r: r["seq"]):
            qkey = (rec["method"], rec["path"], rec["key"])
            self.queues.setdefault(qkey, deque()).append(rec)
        self.total = len(recs)
        if not recs:
            raise SystemExit(f"model-proxy: no recordings in {directory}")
        say(f"replay: loaded {self.total} recorded replies under {len(self.queues)} conversation keys")

    def serve(self, h, method, body):
        path = urllib.parse.urlsplit(h.path).path
        info = describe(method, h.path, h.headers.get("content-encoding", ""), body)
        with self.lock:
            queue = self.queues.get((method, path, info["key"]))
            rec = queue.popleft() if queue else None
            if rec is None:
                self.unmatched += 1
        if rec is None:
            say(f"replay: UNMATCHED {method} {h.path} key={info['key'][:8]} msgs={info['msgs']} -> 500")
            body_out = json.dumps({"type": "error", "error": {
                "type": "api_error", "message": "model-proxy replay: no recorded reply for this request"}}).encode()
            h.send_response(500)
            h.send_header("content-type", "application/json")
            h.send_header("content-length", str(len(body_out)))
            h.end_headers()
            if method != "HEAD":
                h.wfile.write(body_out)
            return
        mapping = build_mapping(rec["ids"], info["ids"])
        if len(rec["ids"]) != len(info["ids"]):
            say(f"replay: warn seq={rec['seq']:04d} tool-result id count recorded={len(rec['ids'])} live={len(info['ids'])}")
        if rec.get("msgs") != info["msgs"]:
            say(f"replay: warn seq={rec['seq']:04d} message count recorded={rec.get('msgs')} live={info['msgs']}")
        rewriter = Rewriter(mapping) if mapping else None
        streaming = "events" in rec
        if rewriter and not self._mentions(rec, mapping):
            rewriter = None
        body_follows = has_body(method, rec["status"])
        send_head(h, rec["status"], rec["content_type"], chunked=body_follows)
        swapped = 0
        if body_follows:
            if streaming:
                swapped = self._stream(h, rec, rewriter)
            else:
                text = rec.get("body", "")
                if rewriter:
                    text = rewriter.sub(text)
                    swapped = rewriter.swapped
                write_chunk(h, text.encode())
            write_end(h)
        with self.lock:
            self.served += 1
        pairs = ", ".join(f"{old[:8]}->{new[:8]}" for old, new in list(mapping.items())[:4])
        note = f" rewrote={swapped} ({pairs})" if rewriter else ""
        say(f"replay: seq={rec['seq']:04d} {method} {h.path} -> {rec['status']} key={info['key'][:8]} "
            f"events={len(rec.get('events', []))}{note}")

    def _mentions(self, rec, mapping):
        text = "".join(e["raw"] for e in rec["events"]) if "events" in rec else rec.get("body", "")
        return any(norm(m.group()) in mapping for m in ID_RE.finditer(text))

    def _stream(self, h, rec, rewriter):
        prev = 0
        for event in rec["events"]:
            gap = min((event["ms"] - prev) / 1000 * self.speed, self.max_gap)
            prev = event["ms"]
            if gap > 0:
                time.sleep(gap)
            pieces = rewriter.raw_event(event["raw"]) if rewriter else [event["raw"]]
            for piece in pieces:
                write_chunk(h, piece.encode())
        if rewriter:
            for ev in rewriter.finish():
                write_chunk(h, frame(ev).encode())
            return rewriter.swapped
        return 0

    def summary(self):
        unused = sum(len(q) for q in self.queues.values())
        return f"replay: served={self.served} unmatched={self.unmatched} unused={unused} of {self.total}"


# Entry point --------------------------------------------------------------

def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="mode", required=True)
    for name in ("record", "replay"):
        p = sub.add_parser(name)
        p.add_argument("--dir", required=True, help="recording directory")
        p.add_argument("--port", type=int, default=43332, help="127.0.0.1 port, 43332 to 43337")
    rec = sub.choices["record"]
    rec.add_argument("--anthropic-upstream", default=ANTHROPIC)
    rec.add_argument("--codex-upstream", default=CODEX)
    rep = sub.choices["replay"]
    rep.add_argument("--speed", type=float, default=1.0, help="multiply recorded gaps by this")
    rep.add_argument("--max-gap", type=float, default=1.0, help="cap each recorded gap, seconds")
    args = parser.parse_args(argv)
    if args.port not in PORTS:
        parser.error("port must be between 43332 and 43337")

    if args.mode == "record":
        app = Recorder(args.dir, args.anthropic_upstream, args.codex_upstream)
    else:
        app = Replayer(args.dir, args.speed, args.max_gap)
    server = Server(args.port, app)

    def stop(*_):
        threading.Thread(target=server.shutdown, daemon=True).start()

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    say(f"{args.mode}: listening on 127.0.0.1:{args.port}")
    try:
        server.serve_forever()
    finally:
        server.server_close()
        say(app.summary())
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
