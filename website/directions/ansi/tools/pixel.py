"""Tiny pixel canvas that writes crisp SVG. Every drawing in the ansi direction
goes through here, so marks, sprites and page art share one grid and one palette.

A canvas is a dict of (x, y) -> palette key. Sprites are written as ASCII maps
where each character is a key from a legend; '.' is transparent.
"""

# Ship 16: the site's own ANSI theme. Index order is the terminal's.
PALETTE = {
    "0": "#1c1a27",   # black: ink, outlines, screens
    "1": "#e65a48",   # red: blocked
    "2": "#3fae5a",   # green: idle
    "3": "#e2a52b",   # yellow: working, wood
    "4": "#2f63cc",   # blue: sea, caps
    "5": "#c45cc4",   # magenta: error
    "6": "#25b2c0",   # cyan: unseen, selection
    "7": "#d4ccb8",   # white: terminal bezels
    "8": "#5f5a75",   # bright black: muted ink
    "9": "#ff7a68",   # bright red
    "a": "#86d873",   # bright green
    "b": "#ffd462",   # bright yellow
    "c": "#79a4ff",   # bright blue
    "d": "#f590e8",   # bright magenta
    "e": "#7fe3ec",   # bright cyan
    "f": "#fbf8f1",   # bright white: sails, caps, paper
}

NAMES = {
    "0": "black", "1": "red", "2": "green", "3": "yellow", "4": "blue",
    "5": "magenta", "6": "cyan", "7": "white", "8": "bright black",
    "9": "bright red", "a": "bright green", "b": "bright yellow",
    "c": "bright blue", "d": "bright magenta", "e": "bright cyan",
    "f": "bright white",
}


# A 3 by 5 pixel face for stencils on crates and signs (lowercase only).
FONT3x5 = {
    "a": ".#.|#.#|###|#.#|#.#", "b": "##.|#.#|##.|#.#|##.", "c": ".##|#..|#..|#..|.##",
    "d": "##.|#.#|#.#|#.#|##.", "e": "###|#..|##.|#..|###", "f": "###|#..|##.|#..|#..",
    "g": ".##|#..|#.#|#.#|.##", "h": "#.#|#.#|###|#.#|#.#", "i": "###|.#.|.#.|.#.|###",
    "j": "..#|..#|..#|#.#|.#.", "k": "#.#|#.#|##.|#.#|#.#", "l": "#..|#..|#..|#..|###",
    "m": "#.#|###|###|#.#|#.#", "n": "##.|#.#|#.#|#.#|#.#", "o": ".#.|#.#|#.#|#.#|.#.",
    "p": "##.|#.#|##.|#..|#..", "q": ".#.|#.#|#.#|##.|.##", "r": "##.|#.#|##.|#.#|#.#",
    "s": ".##|#..|.#.|..#|##.", "t": "###|.#.|.#.|.#.|.#.", "u": "#.#|#.#|#.#|#.#|###",
    "v": "#.#|#.#|#.#|#.#|.#.", "w": "#.#|#.#|###|###|#.#", "x": "#.#|#.#|.#.|#.#|#.#",
    "y": "#.#|#.#|.#.|.#.|.#.", "z": "###|..#|.#.|#..|###", " ": "...|...|...|...|...",
    "1": ".#.|##.|.#.|.#.|###", "-": "...|...|###|...|...", ".": "...|...|...|...|.#.", "0": "###|#.#|#.#|#.#|###",
}


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.px = {}

    @classmethod
    def from_map(cls, text, legend=None):
        rows = [r for r in text.strip("\n").split("\n")]
        rows = [r.split("#")[0].rstrip() if False else r for r in rows]
        w = max(len(r) for r in rows)
        c = cls(w, len(rows))
        c.blit_map(text, 0, 0, legend)
        return c

    def blit_map(self, text, ox, oy, legend=None):
        rows = text.strip("\n").split("\n")
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                if ch in ". ":
                    continue
                key = legend.get(ch, ch) if legend else ch
                if key is None:
                    continue
                self.set(ox + x, oy + y, key)
        return self

    def blit(self, other, ox, oy):
        for (x, y), k in other.px.items():
            self.set(ox + x, oy + y, k)
        return self

    def set(self, x, y, k):
        if 0 <= x < self.w and 0 <= y < self.h:
            if k is None:
                self.px.pop((x, y), None)
            else:
                self.px[(x, y)] = k

    def rect(self, x, y, w, h, k):
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                self.set(xx, yy, k)
        return self

    def outline(self, k="0", fill_keys=None):
        """Add a 1px outline of key k around every non-transparent pixel
        (4-neighbour), without overwriting existing pixels."""
        add = []
        for (x, y) in list(self.px):
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                p = (x + dx, y + dy)
                if p not in self.px and 0 <= p[0] < self.w and 0 <= p[1] < self.h:
                    add.append(p)
        for p in add:
            self.px[p] = k
        return self

    def mapped(self, mapping):
        """Return a copy with keys remapped; a key mapped to None is removed.
        Keys missing from the mapping are kept."""
        c = Canvas(self.w, self.h)
        for p, k in self.px.items():
            nk = mapping.get(k, k)
            if nk is not None:
                c.px[p] = nk
        return c

    def flipped(self):
        c = Canvas(self.w, self.h)
        for (x, y), k in self.px.items():
            c.px[(self.w - 1 - x, y)] = k
        return c


    def text(self, x, y, s, k):
        """Stamp a string in the 3 by 5 face, one blank column between letters.
        Returns the width used."""
        cx = x
        for ch in s:
            for dy, row in enumerate(FONT3x5[ch].split("|")):
                for dx, v in enumerate(row):
                    if v == "#":
                        self.set(cx + dx, y + dy, k)
            cx += 4
        return cx - x - 1

    def ellipse(self, cx, cy, rx, ry, k):
        """Fill an ellipse centred on pixel (cx, cy), radii in pixels."""
        for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
            for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
                if ((x - cx) / (rx + 0.5)) ** 2 + ((y - cy) / (ry + 0.5)) ** 2 <= 1:
                    self.set(x, y, k)
        return self

    def checker(self, x, y, w, h, k, phase=0):
        """A 1 pixel checkerboard, the dither of every terminal."""
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                if (xx + yy + phase) % 2 == 0:
                    self.set(xx, yy, k)
        return self

    def line(self, x0, y0, x1, y1, k):
        """Bresenham, one pixel wide."""
        dx, dy = abs(x1 - x0), -abs(y1 - y0)
        sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
        err = dx + dy
        while True:
            self.set(x0, y0, k)
            if x0 == x1 and y0 == y1:
                break
            e2 = 2 * err
            if e2 >= dy:
                err += dy; x0 += sx
            if e2 <= dx:
                err += dx; y0 += sy
        return self

    def crop(self, x, y, w, h):
        c = Canvas(w, h)
        for (px, py), k in self.px.items():
            if x <= px < x + w and y <= py < y + h:
                c.px[(px - x, py - y)] = k
        return c

    def copy(self):
        c = Canvas(self.w, self.h)
        c.px = dict(self.px)
        return c

    @classmethod
    def sheet(cls, frames):
        """Frames side by side, left to right, for a steps() animation."""
        c = cls(sum(f.w for f in frames), max(f.h for f in frames))
        x = 0
        for f in frames:
            c.blit(f, x, 0)
            x += f.w
        return c

    # ---- output -------------------------------------------------------

    def _rects(self, key):
        """Greedy merge of same-key pixels into rectangles."""
        todo = {p for p, k in self.px.items() if k == key}
        out = []
        for y in range(self.h):
            x = 0
            while x < self.w:
                if (x, y) not in todo:
                    x += 1
                    continue
                w = 0
                while (x + w, y) in todo:
                    w += 1
                h = 1
                while all((x + i, y + h) in todo for i in range(w)):
                    h += 1
                for yy in range(y, y + h):
                    for xx in range(x, x + w):
                        todo.discard((xx, yy))
                out.append((x, y, w, h))
                x += w
        return out

    def svg(self, unit=1, colors=None, title=None, size=None, pad=0,
            bg=None, decorative=False, extra_class=None):
        colors = colors or PALETTE
        W, H = (self.w + 2 * pad) * unit, (self.h + 2 * pad) * unit
        attrs = [f'xmlns="http://www.w3.org/2000/svg"', f'viewBox="0 0 {W} {H}"']
        if size:
            attrs.append(f'width="{size[0]}" height="{size[1]}"')
        attrs.append('shape-rendering="crispEdges"')
        if extra_class:
            attrs.append(f'class="{extra_class}"')
        if decorative:
            attrs.append('aria-hidden="true" focusable="false"')
        elif title:
            attrs.append('role="img"')
        parts = [f"<svg {' '.join(attrs)}>"]
        if title and not decorative:
            parts.append(f"<title>{title}</title>")
        if bg:
            parts.append(f'<rect width="{W}" height="{H}" fill="{bg}"/>')
        order = sorted(set(self.px.values()), key=lambda k: str(k))
        for key in order:
            d = []
            for x, y, w, h in self._rects(key):
                X, Y = (x + pad) * unit, (y + pad) * unit
                d.append(f"M{X} {Y}h{w*unit}v{h*unit}h{-w*unit}z")
            fill = colors.get(key, key)
            extra = ""
            if "@" in fill:
                fill, alpha = fill.split("@")
                extra = f' fill-opacity="{alpha}"'
            parts.append(f'<path fill="{fill}"{extra} d="{"".join(d)}"/>')
        parts.append("</svg>")
        return "\n".join(parts) + "\n"
