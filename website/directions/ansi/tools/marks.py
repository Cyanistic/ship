"""The three marks, drawn on one 16 x 16 cell grid.

Each mark is a colour map plus a one-colour map. In the one-colour map
'#' is ink and everything else is paper, so the one-colour drawing is
designed, not derived by flattening the colour version.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from pixel import Canvas, PALETTE  # noqa: E402

# ---- model sheet parts -------------------------------------------------
# k ink, w sail/cap white, r flag red, y wood, Y hull stripe, g gunport,
# b sea, c sea light, t terminal bezel, s screen, e eye, B cap band

LEGEND = {
    "k": "0", "w": "f", "r": "1", "y": "3", "Y": "b", "g": "0",
    "b": "4", "c": "c", "t": "7", "s": "0", "e": "f", "B": "4",
}

AVATAR = """
.......kk.......
.......kkrrrr...
.......kkrr.....
....kkkkkkkk....
....kwwwwwwk....
...kwwwwwwwwk...
...kkkkkkkkkk...
..kwwwwwwwwwwk..
..kwwwwwwwwwwk..
..kwwwwwwwwwwk..
...kkkkkkkkkk..k
kk.....kk.....kk
kykkkkkkkkkkkkyk
.kygyygyygyygyk.
..kkkkkkkkkkkk..
bbb.bbbbbbbb.bbb
"""

AVATAR_MONO = """
.......##.......
.......######...
.......####.....
....########....
....#......#....
...#........#...
...##########...
..#..........#..
..#..........#..
..#..........#..
...##########..#
##.....##.....##
#.############.#
.#.#..#..#..#.#.
..############..
###.########.###
"""

APP = """
.......kk.......
.......kkrrrr...
....kkkkkkkk....
....kwwwwwwk....
...kwwwwwwwwk...
...kkkkkkkkkk...
..kkkk.kk.kkkk..
.kwwwwk.kkwwwwk.
.kBBBBk..kBBBBk.
.kssssk..kssssk.
.ksesek..kesesk.
.kssssk..kssssk.
kykkkkkkkkkkkkyk
.kygyygyygyygyk.
..kkkkkkkkkkkk..
bbb.bbbbbbbb.bbb
"""

APP_MONO = """
.......##.......
.......######...
....########....
....#......#....
...#........#...
...##########...
..####.##.####..
.#....#.##....#.
.######..######.
.#....#..#....#.
.#.#.##..##.#.#.
.#....#..#....#.
#.############.#
.#.#..#..#..#.#.
..############..
###.########.###
"""

BOT = """
................
.....kkkkkk.....
....kwwwwwwk....
...kBBBBBBBBk...
.kkwwwwwwwwwwkk.
.kttttttttttttk.
.ktsssssssssstk.
.ktsseesseesstk.
.ktsseesseesstk.
.ktsssssssssstk.
.ktssssseessstk.
.ktsssssssssstk.
.kttttttttttttk.
.kkkkkkkkkkkkkk.
................
................
"""

BOT_MONO = """
................
.....######.....
....#......#....
...##########...
.##..........##.
.#............#.
.#.##########.#.
.#.##..##..##.#.
.#.##..##..##.#.
.#.##########.#.
.#.#####..###.#.
.#.##########.#.
.#............#.
.##############.
................
................
"""

MARKS = {
    "avatar": (AVATAR, AVATAR_MONO, "shipmux: a pixel sailing ship"),
    "app": (APP, APP_MONO, "Ship: a pixel sailing ship with two terminal crewmates"),
    "bot": (BOT, BOT_MONO, "ship-crewmate: a terminal crewmate in a sailor cap"),
}

INK = PALETTE["0"]
PAPER = PALETTE["f"]


def colour(name):
    m = MARKS[name][0]
    return Canvas.from_map(m, LEGEND)


def mono(name):
    return Canvas.from_map(MARKS[name][1], {"#": "ink"})


def build(out_dir, variants_dir):
    os.makedirs(out_dir, exist_ok=True)
    os.makedirs(variants_dir, exist_ok=True)
    for name, (_, _, title) in MARKS.items():
        c = colour(name)
        with open(os.path.join(out_dir, f"{name}.svg"), "w") as f:
            f.write(c.svg(unit=16, title=title, size=(256, 256)))
        m = mono(name)
        with open(os.path.join(variants_dir, f"{name}-one-colour.svg"), "w") as f:
            f.write(m.svg(unit=16, colors={"ink": INK}, title=title + " (one colour)", size=(256, 256)))
        with open(os.path.join(variants_dir, f"{name}-reversed.svg"), "w") as f:
            f.write(m.svg(unit=16, colors={"ink": PAPER}, title=title + " (reversed)", size=(256, 256)))


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(here)
    build(os.path.join(root, "marks"), os.path.join(root, "marks", "test"))
