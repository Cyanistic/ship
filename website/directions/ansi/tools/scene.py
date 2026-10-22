"""Page art for the sky and the waterline: crow's nest, clouds, gulls, the mast
and its ladder, and the wave strips. Every sprite is meant to be shown at four
CSS pixels per art pixel (the page's --px), so the whole page shares one grid.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from pixel import Canvas  # noqa: E402
from crew import crewmate  # noqa: E402

OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "art")


def crows_nest():
    W, H = 44, 48
    c = Canvas(W, H)
    mx = 22  # mast left column (mast is 4 wide)
    # mast, ochre with a bright edge and ink sides, running to the bottom edge
    c.rect(mx - 1, 6, 6, H - 6, "0")
    c.rect(mx, 6, 4, H - 6, "3")
    c.rect(mx, 6, 1, H - 6, "b")
    # the >_ flag from the old logo, now ANSI red
    c.rect(mx + 5, 0, 14, 9, "0")
    c.rect(mx + 5, 1, 13, 7, "1")
    for x, y in ((mx + 8, 2), (mx + 9, 3), (mx + 10, 4), (mx + 9, 5), (mx + 8, 6)):
        c.set(x, y, "f")
    c.rect(mx + 12, 6, 4, 1, "f")
    c.rect(mx - 1, 0, 6, 6, "0")
    c.rect(mx, 1, 4, 5, "3")
    c.rect(mx, 1, 1, 5, "b")
    # the lookout, upper half only, standing in the nest with a spyglass
    look = crewmate("lookout", None)
    crop = Canvas(look.w, 15)
    for (x, y), k in look.px.items():
        if y < 15:
            crop.px[(x, y)] = k
    c.blit(crop, 4, 10)
    # the spyglass is a separate sprite (spyglass() below), so it can sweep
    # the nest: a barrel around the mast
    nx, ny, nw, nh = 2, 24, 40, 9
    c.rect(nx, ny, nw, nh, "0")
    c.rect(nx + 1, ny + 1, nw - 2, nh - 2, "3")
    for y in (ny + 3, ny + 5):
        c.rect(nx + 1, y, nw - 2, 1, "b")
    for x in range(nx + 5, nx + nw - 2, 6):
        c.rect(x, ny + 1, 1, nh - 2, "0")
    c.rect(nx - 1, ny, nw + 2, 2, "0")
    # the barrel's hull below the rim, narrowing to the mast, with a lashing band
    c.rect(nx + 4, ny + nh, nw - 8, 2, "0")
    c.rect(nx + 5, ny + nh, nw - 10, 1, "3")
    # rope lashing where the nest ends and the mast tile begins
    for y in range(44, 48):
        c.rect(mx, y, 4, 1, "7" if y % 2 == 0 else "0")
    return c


def spyglass(tilt=0, glint=False):
    """The lookout's spyglass, 12 by 16, to lay over the nest at (0, 12). tilt turns the lens end up (negative)
    or down (positive) by that many rows, as a shear, so every step stays on the pixel grid. Level, it is the
    pose the nest was first drawn with."""
    base = Canvas(12, 16)
    base.rect(1, 5, 3, 6, "0")                    # the lens cap
    base.rect(2, 6, 8, 4, "0")                    # the tube
    base.rect(3, 7, 7, 2, "3")
    c = Canvas(12, 16)
    for (x, y), k in base.px.items():
        c.set(x, y + round(tilt * (9 - x) / 8), k)
    ly = round(tilt * 7 / 8)
    for y in (7, 8):
        c.set(2, y + ly, "f" if glint else "e")
    if glint:
        c.set(0, 6 + ly, "f")
    return c


def blob(circles, base, fill="f", edge="c", shade="e", pad=1):
    """A cloud: the union of circles (cx, cy, r), cut flat at row `base`, shaded
    on its last two rows, outlined. Same recipe as the first cloud, bumpier."""
    top = min(cy - r for cx, cy, r in circles)
    left = min(cx - r for cx, cy, r in circles)
    dy, dx = pad + 1 - top, pad + 1 - left
    w = int(max(cx + r for cx, cy, r in circles) + dx) + 2 + pad
    h = int(base + dy) + 2 + pad
    c = Canvas(w, h)
    for cx, cy, r in circles:
        c.ellipse(cx + dx, cy + dy, r, r, fill)
    for (x, y) in list(c.px):
        if y > base + dy:
            del c.px[(x, y)]
    for (x, y), k in list(c.px.items()):
        if y >= base + dy - 1:
            c.px[(x, y)] = shade
    c.outline(edge)
    return c


def cloud(kind="a"):
    if kind == "a":      # the big one
        return blob([(7, 13, 4), (14, 10, 6), (23, 7, 7), (32, 9, 6), (40, 12, 5), (46, 14, 3)], 17)
    if kind == "b":      # a smaller one
        return blob([(6, 9, 4), (13, 6, 6), (21, 7, 5), (27, 9, 4)], 13)
    return blob([(4, 6, 3), (10, 4, 5), (16, 6, 3)], 9)   # a wisp


def gull():
    """Two frames, side by side: wings up, wings down. An ink 'm' with a white
    breast, the way a gull looks from the ground."""
    frames = []
    for up in (True, False):
        c = Canvas(19, 9)
        c.ellipse(9, 5, 2, 1, "f")
        c.outline("0")
        pts_l = [(6, 5), (3, 2), (0, 1)] if up else [(6, 5), (3, 4), (0, 6)]
        pts_r = [(12, 5), (15, 2), (18, 1)] if up else [(12, 5), (15, 4), (18, 6)]
        for pts in (pts_l, pts_r):
            for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
                c.line(x0, y0, x1, y1, "0")
                c.line(x0, y0 + 1, x1, y1 + 1, "0")
        frames.append(c)
    return Canvas.sheet(frames)


def mast_tile():
    """The mast, 6 wide, repeating every 16 rows. Matches the nest's mast."""
    c = Canvas(6, 16)
    c.rect(0, 0, 6, 16, "0")
    c.rect(1, 0, 4, 16, "3")
    c.rect(1, 0, 1, 16, "b")
    # a worn seam and a lashing so a long mast is not a flat stripe
    c.rect(3, 5, 1, 4, "0")
    for y in (12, 13):
        c.rect(1, y, 4, 1, "7" if y == 12 else "0")
    return c


def ratlines_tile():
    """A rope ladder, 8 wide, one rung every 8 rows: tan rope with an ink shadow."""
    c = Canvas(8, 8)
    for y in range(8):
        c.set(0, y, "7"); c.set(1, y, "0"); c.set(6, y, "0"); c.set(7, y, "7")
    c.rect(0, 0, 8, 1, "7")
    c.rect(1, 1, 6, 1, "0")
    return c


def wave():
    m = """
..ff............
.fccf.......ff..
fccccf.....fccf.
cccccccc.ccccccc
cccccccccccccccc
"""
    return Canvas.from_map(m, {"c": "c", "f": "f"})


def wave_back():
    """A slower, paler swell behind the front strip."""
    m = """
.....ee.........
...eecceee....e.
.eecccccccee.eee
cccccccccccccccc
"""
    return Canvas.from_map(m, {"c": "4", "e": "c"})


ART = {
    "crows-nest": crows_nest,
    "spyglass": spyglass,
    "cloud": lambda: cloud("a"),
    "cloud-b": lambda: cloud("b"),
    "cloud-c": lambda: cloud("c"),
    "gull": gull,
    "mast": mast_tile,
    "ratlines": ratlines_tile,
    "wave": wave,
    "wave-back": wave_back,
}


def build(out=OUT):
    for name, fn in ART.items():
        with open(os.path.join(out, f"{name}.svg"), "w") as f:
            f.write(fn().svg(unit=1, decorative=True))


if __name__ == "__main__":
    build()
