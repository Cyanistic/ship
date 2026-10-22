"""A very small Chrome DevTools Protocol client, standard library only.

Headless Chrome's --screenshot cuts a tall page short and leaves a blank band
at the bottom, so the page shots here go through the protocol instead: set a
real viewport, load, wait for fonts, then capture the whole document beyond
the viewport. The same connection runs the checks in check.py.
"""
import base64
import json
import os
import socket
import struct
import subprocess
import tempfile
import time
import urllib.request

CHROME = os.environ.get("CHROME", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")


def _free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


class WS:
    """Just enough of RFC 6455 to talk to DevTools: text frames, no extensions."""

    def __init__(self, url):
        host, rest = url[len("ws://"):].split("/", 1)
        h, p = host.split(":")
        self.sock = socket.create_connection((h, int(p)))
        key = base64.b64encode(os.urandom(16)).decode()
        self.sock.sendall((f"GET /{rest} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\n"
                           f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n").encode())
        buf = b""
        while b"\r\n\r\n" not in buf:
            buf += self.sock.recv(4096)
        self.buf = buf.split(b"\r\n\r\n", 1)[1]

    def _read(self, n):
        while len(self.buf) < n:
            chunk = self.sock.recv(1 << 20)
            if not chunk:
                raise ConnectionError("devtools socket closed")
            self.buf += chunk
        out, self.buf = self.buf[:n], self.buf[n:]
        return out

    def send(self, text):
        data = text.encode()
        head = bytearray([0x81])
        n = len(data)
        if n < 126:
            head.append(0x80 | n)
        elif n < 65536:
            head += bytes([0x80 | 126]) + struct.pack(">H", n)
        else:
            head += bytes([0x80 | 127]) + struct.pack(">Q", n)
        mask = os.urandom(4)
        head += mask
        self.sock.sendall(bytes(head) + bytes(b ^ mask[i % 4] for i, b in enumerate(data)))

    def recv(self):
        message = b""
        while True:
            b1, b2 = self._read(2)
            n = b2 & 0x7F
            if n == 126:
                n = struct.unpack(">H", self._read(2))[0]
            elif n == 127:
                n = struct.unpack(">Q", self._read(8))[0]
            payload = self._read(n)
            op = b1 & 0x0F
            if op == 0x9:      # ping: answer and keep reading
                continue
            message += payload
            if b1 & 0x80:
                return message.decode()


class Page:
    def __init__(self, width, height=900, reduced_motion=False):
        self.port = _free_port()
        self.profile = tempfile.mkdtemp(prefix="ansi-cdp-")
        self.proc = subprocess.Popen(
            [CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--no-first-run",
             "--no-default-browser-check", f"--remote-debugging-port={self.port}",
             f"--user-data-dir={self.profile}", "about:blank"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        target = None
        for _ in range(100):
            try:
                tabs = json.load(urllib.request.urlopen(f"http://127.0.0.1:{self.port}/json/list", timeout=1))
                target = next(t for t in tabs if t["type"] == "page")
                break
            except Exception:
                time.sleep(0.1)
        if not target:
            raise RuntimeError("could not reach Chrome's DevTools port")
        self.ws = WS(target["webSocketDebuggerUrl"])
        self.n = 0
        self.call("Page.enable")
        self.call("Emulation.setDeviceMetricsOverride", width=width, height=height, deviceScaleFactor=1, mobile=False)
        self.call("Emulation.setFocusEmulationEnabled", enabled=True)
        self.set_reduced_motion(reduced_motion)
        self.width, self.height = width, height

    def call(self, method, **params):
        self.n += 1
        self.ws.send(json.dumps({"id": self.n, "method": method, "params": params}))
        while True:
            msg = json.loads(self.ws.recv())
            if msg.get("id") == self.n:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg.get("result", {})

    def set_reduced_motion(self, on):
        self.call("Emulation.setEmulatedMedia", features=[
            {"name": "prefers-reduced-motion", "value": "reduce" if on else "no-preference"}])

    def goto(self, path, settle=2.0):
        self.call("Page.navigate", url="file://" + path)
        deadline = time.time() + 30
        while time.time() < deadline:
            if self.eval("document.readyState") == "complete":
                break
            time.sleep(0.1)
        self.eval("document.fonts.ready.then(() => true)", await_promise=True)
        time.sleep(settle)

    def eval(self, expr, await_promise=False):
        r = self.call("Runtime.evaluate", expression=expr, returnByValue=True, awaitPromise=await_promise)
        if "exceptionDetails" in r:
            raise RuntimeError(r["exceptionDetails"].get("exception", {}).get("description", "evaluate failed"))
        return r["result"].get("value")

    _KEYS = {"Tab": 9, "Enter": 13, "Escape": 27, "ArrowDown": 40, "ArrowUp": 38, "/": 191}

    def key(self, key, shift=False):
        """Press and release one key, the way a keyboard would (real key events, not synthetic JS)."""
        code = {"/": "Slash"}.get(key, key)
        text = {"Enter": "\r", "/": "/"}.get(key, "")
        mods = 8 if shift else 0
        vk = self._KEYS[key]
        self.call("Input.dispatchKeyEvent", type="keyDown" if text else "rawKeyDown", key=key, code=code,
                  windowsVirtualKeyCode=vk, text=text, modifiers=mods)
        self.call("Input.dispatchKeyEvent", type="keyUp", key=key, code=code, windowsVirtualKeyCode=vk, modifiers=mods)

    def type_text(self, text):
        self.call("Input.insertText", text=text)

    def click(self, x, y):
        for t in ("mousePressed", "mouseReleased"):
            self.call("Input.dispatchMouseEvent", type=t, x=x, y=y, button="left", clickCount=1)

    def grant_clipboard(self):
        self.call("Browser.grantPermissions", permissions=["clipboardReadWrite", "clipboardSanitizedWrite"])

    def full_page_png(self, out):
        h = int(self.eval("Math.ceil(document.documentElement.getBoundingClientRect().height)"))
        shot = self.call("Page.captureScreenshot", format="png", captureBeyondViewport=True,
                         clip={"x": 0, "y": 0, "width": self.width, "height": h, "scale": 1})
        with open(out, "wb") as f:
            f.write(base64.b64decode(shot["data"]))
        return h

    def viewport_png(self, out):
        shot = self.call("Page.captureScreenshot", format="png")
        with open(out, "wb") as f:
            f.write(base64.b64decode(shot["data"]))

    def close(self):
        try:
            self.proc.terminate()
            self.proc.wait(timeout=5)
        except Exception:
            self.proc.kill()
