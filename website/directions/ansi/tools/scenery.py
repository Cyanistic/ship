"""Scenery for the cross-section below the sky: the deck, the hull, the sea's
flotsam and fish, the interior with its rooms, the hold and the seabed.

Same rules as the rest of the direction. Everything is on the 16 colour Ship
palette, drawn on a square grid, shown at four CSS pixels per art pixel. Wood
is ANSI yellow like the hull of the marks. Underwater and stone are blues and
greys, so no state colour is spent on scenery that is not a crewmate.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from pixel import Canvas  # noqa: E402
from crew import crewmate  # noqa: E402

OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "art")


# ---- deck and hull --------------------------------------------------------

def planks():
    """The deck's face, 48 wide and 18 tall, repeating sideways. Four boards,
    butt joints staggered, a nail by every joint."""
    c = Canvas(48, 18)
    c.rect(0, 0, 48, 1, "0")
    c.rect(0, 1, 48, 1, "b")
    c.rect(0, 2, 48, 14, "3")
    for y in (4, 8, 12, 16):
        c.rect(0, y, 48, 1, "0")
    c.rect(0, 17, 48, 1, "0")
    boards = [(2, 3, [9, 33]), (5, 7, [21, 45]), (9, 11, [3, 27]), (13, 15, [15, 39])]
    for y0, y1, xs in boards:
        for x in xs:
            c.rect(x, y0, 1, y1 - y0 + 1, "0")
            c.set(x - 2, y0 + 1, "0")
            c.set(x + 2, y0 + 1, "0")
    # a few lighter flecks of grain
    for x, y in ((4, 3), (17, 6), (29, 3), (38, 6), (12, 10), (26, 10), (44, 10), (8, 14), (33, 14)):
        c.set(x, y, "b")
    return c


def hull():
    """The side of the ship above the water, 16 wide and 14 tall: the avatar's
    hull, ink rail, yellow strakes, open gunports."""
    c = Canvas(16, 14)
    c.rect(0, 0, 16, 1, "0")
    c.rect(0, 1, 16, 1, "b")
    c.rect(0, 2, 16, 11, "3")
    c.rect(0, 13, 16, 1, "0")
    # strake line under the ports
    c.rect(0, 10, 16, 1, "0")
    # the gunport: an ink window with a glint
    c.rect(4, 3, 8, 6, "0")
    c.rect(5, 4, 6, 4, "0")
    c.set(5, 4, "8"); c.set(6, 4, "8")
    c.rect(3, 2, 10, 1, "b")
    return c


def rail():
    """A rail with balusters, 12 wide, 14 tall, for the far side of the deck."""
    c = Canvas(12, 14)
    c.rect(0, 0, 12, 2, "0")
    c.rect(0, 0, 12, 1, "b")
    c.rect(0, 1, 12, 1, "3")
    c.rect(0, 2, 12, 1, "0")
    c.rect(4, 3, 4, 11, "0")
    c.rect(5, 3, 2, 11, "3")
    c.rect(5, 3, 1, 11, "b")
    c.rect(0, 13, 12, 1, "0")
    return c


# ---- props ---------------------------------------------------------------

def barrel():
    c = Canvas(12, 14)
    insets = (3, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 2, 3)
    for y, inset in enumerate(insets):
        c.rect(inset, y, 12 - 2 * inset, 1, "0")
        if y not in (0, 13):
            c.rect(inset + 1, y, 12 - 2 * inset - 2, 1, "3")
    c.rect(3, 1, 6, 1, "b")
    c.rect(4, 2, 1, 11, "b")
    for y in (4, 9):                      # hoops
        c.rect(insets[y], y, 12 - 2 * insets[y], 1, "0")
    return c


def crate(w=14, h=14, word=None):
    """A slatted crate. With a word it is a stencilled shipping crate instead."""
    c = Canvas(w, h)
    c.rect(0, 0, w, h, "0")
    c.rect(1, 1, w - 2, h - 2, "3")
    c.rect(1, 1, w - 2, 1, "b")
    c.rect(1, 1, 1, h - 2, "b")
    if word:
        tw = len(word) * 4 - 1
        c.text((w - tw) // 2, (h - 5) // 2 + 1, word, "0")
        c.rect(1, 3, w - 2, 1, "0")
        c.rect(1, h - 4, w - 2, 1, "0")
    else:
        # inner panel with an X brace
        c.rect(3, 3, w - 6, 1, "0"); c.rect(3, h - 4, w - 6, 1, "0")
        c.rect(3, 3, 1, h - 6, "0"); c.rect(w - 4, 3, 1, h - 6, "0")
        c.line(4, 4, w - 5, h - 5, "0")
        c.line(w - 5, 4, 4, h - 5, "0")
    return c


def rope_coil():
    """A coil of rope seen from slightly above, 16 wide and 8 tall: a tan ring with its twist, a dark hole."""
    c = Canvas(16, 8)
    c.ellipse(7.5, 3.5, 7, 3.5, "7")
    c.ellipse(7.5, 3.5, 3.4, 1.4, "0")
    for (x, y), k in list(c.px.items()):
        if k == "7" and (x + 2 * y) % 4 == 0:     # the twist, a diagonal of darker strands
            c.set(x, y, "8")
    c.outline("0")
    return c


def bucket():
    return Canvas.from_map("""
0000000000
0bb3333330
.03333330.
.00000000.
.03333330.
.03333330.
..000000..
""", {"b": "b", "3": "3"})


def sack():
    c = Canvas(11, 12)
    c.ellipse(5, 7, 4.5, 4.5, "7")
    c.rect(3, 2, 5, 3, "7")
    c.outline("0")
    c.rect(3, 4, 5, 1, "0")                 # the tie
    for x, y in ((3, 8), (4, 9), (7, 6), (6, 9)):
        c.set(x, y, "8")
    return c


def anchor():
    m = """
.......00.......
......0000......
.......00.......
.....000000.....
.......00.......
.......00.......
.......00.......
.......00.......
..0....00....0..
..00...00...00..
..000..00..000..
...00000000000..
....000000000...
.....0000000....
"""
    c = Canvas.from_map(m, {"0": "8"})
    c.outline("0")
    return c


def chain():
    """A chain link tile, 4 wide, 8 tall, repeating down."""
    c = Canvas(4, 8)
    c.rect(1, 0, 2, 4, "8")
    c.rect(0, 1, 1, 2, "8"); c.rect(3, 1, 1, 2, "8")
    c.rect(1, 2, 2, 4, "7")
    c.rect(1, 4, 2, 4, "8")
    c.rect(0, 5, 1, 2, "8"); c.rect(3, 5, 1, 2, "8")
    c.set(1, 6, "7")
    return c


# ---- flotsam and fish -----------------------------------------------------

def bottle():
    return Canvas.from_map("""
..00..
..33..
..00..
.0ee0.
0eeee0
0effe0
0eeeee0
0eeeee0
0eeeee0
.00000.
""".replace("0eeeee0", "0eeeee0"), {"e": "e", "f": "f", "3": "3"})


def buoy():
    c = Canvas(16, 10)
    c.ellipse(7.5, 4.5, 7, 4, "f")
    c.ellipse(7.5, 4.5, 3, 1.2, None)
    c.outline("0")
    for x in (2, 3, 12, 13):              # ink bands round the ring
        for y in range(10):
            if c.px.get((x, y)) == "f":
                c.set(x, y, "0")
    return c


def float_plank():
    c = Canvas(18, 4)
    c.rect(0, 0, 18, 4, "0")
    c.rect(1, 1, 16, 2, "3")
    c.rect(1, 1, 16, 1, "b")
    c.set(0, 0, None); c.set(17, 3, None)
    c.set(4, 2, "0"); c.set(13, 2, "0")
    return c


def float_sign():
    """A board adrift with a slop word on it."""
    c = Canvas(18, 12)
    c.rect(0, 0, 18, 9, "0")
    c.rect(1, 1, 16, 7, "3")
    c.rect(1, 1, 16, 1, "b")
    c.text(2, 2, "10x", "0")
    c.rect(8, 9, 2, 3, "0")
    return c


def fish():
    """Two frames: tail up, tail down. Bright blue on the deep sea."""
    frames = []
    for up in (True, False):
        c = Canvas(12, 6)
        c.ellipse(5, 3, 4, 1.5, "c")
        c.set(9, 2 if up else 4, "c"); c.set(10, 1 if up else 5, "c"); c.set(10, 2 if up else 4, "c")
        c.set(10, 3, "c"); c.set(11, 1 if up else 5, "c")
        c.set(3, 2, "f")
        frames.append(c)
    return Canvas.sheet(frames)


def ext_arrow():
    """A small arrow pointing up and out, 5 by 5, marking a link that leaves the site."""
    return Canvas.from_map("""
..###
...##
..#.#
.#...
#....
""", {"#": "8"})


def bubble():
    return Canvas.from_map("""
.c.
c.c
.c.
""", {"c": "c"})


ART = {
    "planks": planks,
    "hull": hull,
    "rail": rail,
    "barrel": barrel,
    "crate": crate,
    "crate-cargo": lambda: crate(22, 16, "cargo"),
    "rope-coil": rope_coil,
    "bucket": bucket,
    "sack": sack,
    "anchor": anchor,
    "chain": chain,
    "bottle": bottle,
    "buoy": buoy,
    "float-plank": float_plank,
    "float-sign": float_sign,
    "fish": fish,
    "bubble": bubble,
    "ext": ext_arrow,
}


def build(out=OUT):
    for name, fn in ART.items():
        with open(os.path.join(out, f"{name}.svg"), "w") as f:
            f.write(fn().svg(unit=1, decorative=True))


if __name__ == "__main__":
    build()
