"""Below the waterline: the interior with its rooms, the hold and the seabed.

Each room is one small diorama with a crewmate at work in it. Crew come from
crew.py, so a hat or a pose changed there changes here. The crew in rooms are
neutral (white face), so they carry no state and need no glyph beside them;
the crew who do carry a state are on the deck, with their labels.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from pixel import Canvas  # noqa: E402
from crew import crewmate  # noqa: E402
import scenery as sc  # noqa: E402

OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "art")


def put(canvas, sprite, x, y, flip=False):
    canvas.blit(sprite.flipped() if flip else sprite, x, y)
    return canvas


# ---- the structure of the interior -----------------------------------------

def porthole(d=14):
    """A brass-ringed round window onto the sea."""
    c = Canvas(d, d)
    r = d / 2 - 0.5
    c.ellipse(r, r, r, r, "0")
    c.ellipse(r, r, r - 1, r - 1, "3")
    c.ellipse(r, r, r - 3, r - 3, "0")
    c.ellipse(r, r, r - 4, r - 4, "4")
    # one diagonal glint on the glass
    for x, y in ((4, 5), (5, 4)):
        c.set(x, y, "c")
    # a bright edge on the ring
    for x, y in ((3, 2), (4, 1), (5, 1)):
        c.set(x, y, "b")
    return c


def bulkhead():
    """The sea side of the below-deck section: a timber, then a wall with a
    porthole, 64 wide. The wall itself is the section's own texture."""
    c = Canvas(64, 24)
    c.rect(0, 0, 64, 1, "0")
    c.rect(0, 1, 64, 1, "b")
    c.rect(0, 2, 64, 2, "3")
    c.rect(0, 4, 64, 1, "0")
    # rivets along the timber
    for x in range(4, 64, 8):
        c.set(x, 2, "0")
    put(c, porthole(), 25, 7)
    return c


def wall():
    """Plank wall for the interior: seams only, drawn translucent so the
    section's own colour shows through. 32 wide, 22 tall."""
    c = Canvas(32, 22)
    c.rect(0, 10, 32, 1, "8")
    c.rect(0, 21, 32, 1, "8")
    c.rect(5, 0, 1, 10, "8")
    c.rect(21, 11, 1, 10, "8")
    for x, y in ((3, 2), (7, 2), (3, 7), (7, 7), (19, 13), (23, 13), (19, 18), (23, 18)):
        c.set(x, y, "8")
    return c


def beam():
    """The timber between rooms, 16 wide and 6 tall."""
    c = Canvas(16, 6)
    c.rect(0, 0, 16, 1, "0")
    c.rect(0, 1, 16, 1, "b")
    c.rect(0, 2, 16, 3, "3")
    c.rect(0, 5, 16, 1, "0")
    c.set(3, 3, "0"); c.set(11, 3, "0")
    return c


def ladder():
    """A ladder tile, 12 wide and 8 tall, repeating down."""
    c = Canvas(12, 8)
    for y in range(8):
        for x0 in (0, 9):
            c.set(x0, y, "0"); c.set(x0 + 1, y, "b"); c.set(x0 + 2, y, "0")
    c.rect(3, 0, 6, 1, "0")
    c.rect(3, 1, 6, 1, "3")
    c.rect(3, 2, 6, 1, "0")
    return c


def lantern():
    """A hanging lantern, 9 wide and 15 tall, glass lit."""
    c = Canvas(9, 15)
    c.rect(4, 0, 1, 4, "8")
    c.rect(3, 4, 3, 1, "0")
    c.rect(2, 5, 5, 1, "0")
    c.rect(1, 6, 7, 1, "0")
    c.rect(1, 7, 1, 6, "0"); c.rect(7, 7, 1, 6, "0")
    c.rect(2, 7, 5, 6, "b")
    c.rect(3, 8, 3, 4, "f")
    c.rect(4, 7, 1, 6, "0") if False else None
    c.rect(1, 13, 7, 1, "0")
    c.rect(2, 14, 5, 1, "0")
    return c


def glow(r=20):
    """A dithered halo for a lantern, drawn translucent. 2r+1 square."""
    n = 2 * r + 1
    c = Canvas(n, n)
    for y in range(n):
        for x in range(n):
            d = ((x - r) ** 2 + (y - r) ** 2) ** 0.5
            if d < r * 0.42:
                if x % 2 == 0 or y % 2 == 0:     # three cells in four: dense, but still a dither, not a flat disc
                    c.set(x, y, "b")
            elif d < r * 0.72 and (x + y) % 2 == 0:
                c.set(x, y, "b")
            elif d < r and (x % 2 == 0 and y % 2 == 0):
                c.set(x, y, "b")
    return c


def steam():
    """Three frames of steam, each 9 wide and 14 tall: puffs rising."""
    frames = []
    for phase in range(3):
        c = Canvas(9, 14)
        spots = [(4, 12 - phase * 3), (3, 8 - phase * 3 + 3), (5, 5 - phase * 2 + 2)]
        for i, (x, y) in enumerate(spots):
            if 0 <= y < 14:
                c.rect(x - 1, y, 3, 2, "f")
                c.set(x, y - 1, "f") if y > 0 else None
        frames.append(c)
    return Canvas.sheet(frames)


# ---- rooms -------------------------------------------------------------------

def room(w, h):
    """The shell every room shares: a lit tan wall with plank seams, a floor,
    and an ink frame, so ink-outlined crew read against it even though the
    section around the room is dark. Floor top is row h - 5."""
    c = Canvas(w, h)
    c.rect(0, 0, w, h, "7")
    for y in range(7, h - 5, 8):
        c.rect(1, y, w - 2, 1, "s")
    for i, y in enumerate(range(0, h - 5, 8)):
        for x in range(5 + (i * 17) % 44, w - 1, 44):
            c.rect(x, y, 1, 7, "s")
    c.rect(0, h - 5, w, 5, "0")
    c.rect(1, h - 4, w - 2, 3, "3")
    c.rect(1, h - 4, w - 2, 1, "b")
    for x in range(5, w, 12):
        c.set(x, h - 3, "0")
    c.rect(0, 0, w, 1, "0"); c.rect(0, 0, 1, h, "0"); c.rect(w - 1, 0, 1, h, "0")
    return c


def crew_in(c, hat, pose, x, feet_y, flip=False, mood=None, frame=None, fx=None):
    """Stand a crewmate with its feet on row feet_y."""
    s = crewmate(hat, mood, pose=pose, frame=frame, fx=fx)
    put(c, s, x, feet_y - 20, flip)
    return s


def galley(f=0):
    """The server is a galley: the cook keeps the pot on while nobody is at
    the table. 68 by 52. Frame f (0 to 3): the spoon goes round the pot, the fire
    flickers and the soup bubbles. Frame 0 is the still room."""
    c = room(68, 52)
    fy = 52 - 6                      # feet row
    # shelf with jars, and a mug
    c.rect(34, 12, 30, 2, "0"); c.rect(34, 12, 30, 1, "3")
    for x in (37, 44, 51):
        c.rect(x, 5, 5, 7, "0")
        c.rect(x + 1, 6, 3, 5, "4" if x == 44 else "f")
        c.rect(x + 1, 5, 3, 1, "3")
    c.rect(57, 8, 5, 4, "0"); c.rect(58, 9, 3, 2, "f"); c.rect(62, 9, 1, 2, "0")
    # the stove, an iron box with the fire behind its door, and a chimney
    c.rect(31, 36, 26, 11, "0")
    c.rect(32, 37, 24, 9, "8")
    c.rect(32, 37, 24, 1, "7")
    c.rect(36, 40, 9, 6, "0")
    c.rect(38, 43, 5, 3, "3"); c.rect(39, 42, 3, 1, "b"); c.set(40, 44, "b")
    if f % 2:                                                  # the flame licks up
        c.rect(39, 41, 3, 1, "b"); c.set(40, 40, "3"); c.set(42, 43, "b")
    c.rect(31, 47, 3, 1, "0"); c.rect(54, 47, 3, 1, "0")
    c.rect(54, 14, 3, 22, "0"); c.rect(55, 14, 1, 22, "8")
    # the pot, open, soup showing
    c.rect(35, 27, 16, 10, "0")
    c.rect(36, 28, 14, 8, "f")
    c.rect(36, 28, 14, 2, "8")
    c.rect(36, 34, 14, 2, "7")
    c.rect(33, 29, 2, 3, "0"); c.rect(51, 29, 2, 3, "0")
    for x, y in [(), ((38, 32),), ((46, 31), (39, 32)), ((44, 33),)][f]:   # bubbles in the soup
        c.set(x, y, "7")
    up = f % 2
    crew_in(c, "cook", "reach", 2, fy, fx={"up": up})
    # a long spoon from the cook's hand up into the pot, its end stirring a circle
    tx, ty = [(40, 24), (43, 23), (40, 22), (37, 23)][f]
    c.line(2 + 27, fy - 20 + 13 - up, tx, ty, "0")
    c.line(2 + 27, fy - 20 + 14 - up, tx, ty + 1, "0")
    c.rect(tx - 1, ty - 2, 3, 3, "3")
    return c


def chart_room(target=None):
    """The layout of tabs and panes is a chart, pinned up and pointed at. 68 by 52.
    target is the pane the navigator taps with a pointer (left, top, bottom), which
    takes the cyan border a selected pane has in ship. None is the still room."""
    c = room(68, 52)
    fy = 52 - 6
    c.rect(30, 3, 36, 30, "0")
    c.rect(31, 4, 34, 28, "f")
    panes = {"left": (33, 6, 14, 24), "top": (49, 6, 14, 11), "bottom": (49, 19, 14, 11)}
    for name, (x, y, w, h) in panes.items():
        c.rect(x, y, w, h, "6" if name == target else "0")
        c.rect(x + 1, y + 1, w - 2, h - 2, "f")
    for y in range(9, 27, 3):
        c.rect(36, y, 8 if y % 2 else 6, 1, "8")
    for y in (9, 12):
        c.rect(52, y, 7, 1, "8")
    for y in (22, 25):
        c.rect(52, y, 8 if y == 22 else 5, 1, "8")
    c.set(32, 4, "1"); c.set(64, 4, "1")      # pins
    # a compass on a stand
    c.rect(52, 36, 2, 5, "0")
    c.ellipse(52.5, 34, 4, 4, "0"); c.ellipse(52.5, 34, 3, 3, "3")
    c.rect(52, 32, 1, 5, "0"); c.rect(50, 34, 5, 1, "0")
    c.rect(48, 41, 10, 1, "0")
    crew_in(c, "navigator", "point", 4, fy)
    if target:
        hx, hy = 29, 35                               # the pointing hand
        tx, ty = {"left": (40, 22), "top": (51, 12), "bottom": (51, 25)}[target]
        c.line(hx, hy, tx, ty, "0")
        c.rect(tx - 1, ty - 1, 2, 2, "3")
    return c


def telegraph(f=0):
    """The CLI is a telegraph: orders go down in plain words and the ship
    answers. 56 by 52. Frame f (0 to 15): the lookout pulls the lever down, the
    ship answers and the needle jumps to a new mark."""
    c = room(56, 52)
    fy = 52 - 6
    drop, needle = [(0, "a"), (0, "a"), (2, "a"), (4, "a"), (4, "b"), (2, "b"), (0, "b"), (0, "b"),
                    (0, "b"), (2, "b"), (4, "b"), (4, "c"), (2, "c"), (0, "c"), (0, "c"), (0, "a")][f]
    c.rect(37, 22, 4, 24, "0"); c.rect(38, 22, 2, 24, "8")
    c.rect(33, 42, 12, 4, "0"); c.rect(34, 43, 10, 2, "8")
    c.ellipse(39, 13, 11, 11, "0")
    c.ellipse(39, 13, 10, 10, "3")
    c.ellipse(39, 13, 8, 8, "0")
    c.ellipse(39, 13, 7, 7, "f")
    c.set(33, 5, "b"); c.set(34, 4, "b"); c.set(35, 3, "b")
    for x, y in ((39, 7), (39, 19), (33, 13), (45, 13), (35, 9), (43, 9), (35, 17), (43, 17)):
        c.set(x, y, "0")
    tip = {"a": (43, 9), "b": (45, 16), "c": (34, 9)}[needle]
    c.line(39, 13, tip[0], tip[1], "0")
    c.rect(39, 13, 2, 1, "0")
    # the lever: a bar pinned to the post, its handle swinging down toward the lookout's hand
    for x in range(29, 41):
        y = 32 + round(drop * (40 - x) / 11)
        c.set(x, y + 1, "0")
        c.set(x, y, "3" if 30 <= x <= 39 else "0")
    c.rect(27, 30 + drop, 4, 6, "0"); c.rect(28, 31 + drop, 2, 4, "b")
    crew_in(c, "lookout", "reach", 2, fy)
    return c


def handoff(nx=33):
    """Agents driving agents: one crewmate hands a note to another. 76 by 40.
    nx is where the note is on its way across (27 in the first hand, 39 in the second)."""
    c = room(76, 40)
    fy = 40 - 6
    crew_in(c, "captain", "reach", 2, fy)
    crew_in(c, "deckhand", "reach", 46, fy, flip=True)
    put(c, porthole(), 31, 3)
    ny = 18
    c.rect(nx, ny, 10, 13, "0")
    c.rect(nx + 1, ny + 1, 8, 11, "f")
    for x, y in ((nx + 2, ny + 3), (nx + 3, ny + 4), (nx + 2, ny + 5)):
        c.set(x, y, "0")
    c.rect(nx + 5, ny + 5, 3, 1, "0")
    c.rect(nx + 2, ny + 8, 6, 1, "8")
    c.rect(nx + 2, ny + 10, 4, 1, "8")
    return c


def hold_room(f=0):
    """The hold: a lit corner where the cargo is counted. 76 by 52. Frame f (0 to 11):
    the cook ticks the list line by line and the lantern sways on its rope."""
    c = room(76, 52)
    fy = 52 - 6
    ticks, dip, sway = [(0, 0, 0), (0, 1, 0), (1, 0, 1), (1, 1, 1), (1, 0, 1), (2, 1, 0),
                        (2, 0, 0), (2, 1, -1), (2, 0, -1), (2, 0, -1), (0, 0, 0), (0, 0, 0)][f]
    # crates stacked against the wall, one with the stencil
    put(c, sc.crate(22, 16, "cargo"), 52, fy - 15)
    put(c, sc.crate(14, 14), 54, fy - 29)
    put(c, sc.crate(14, 14), 62, fy - 29)
    put(c, sc.crate(14, 14), 42, fy - 13)
    put(c, sc.barrel(), 34, fy - 13)
    # a lantern hung from the beam
    c.line(18, 1, 18 + sway, 9, "0")
    put(c, lantern(), 14 + sway, 9)
    crew_in(c, "cook", "clipboard", 2, fy, fx={"ticks": ticks, "up": dip})
    return c


def hold_pile(flip=False):
    """Cargo heaped in a corner of the hold, for the dark. 64 by 40."""
    c = Canvas(64, 40)
    put(c, sc.crate(22, 16, "cargo"), 2, 24)
    put(c, sc.crate(14, 14), 26, 26)
    put(c, sc.crate(14, 14), 6, 10)
    put(c, sc.barrel(), 42, 26)
    put(c, sc.barrel(), 54, 26)
    put(c, sc.crate(14, 14), 30, 12)
    put(c, sc.sack(), 48, 14)
    return c.flipped() if flip else c


def hold_pile_small():
    """A smaller heap for the other corner. 40 by 28."""
    c = Canvas(40, 28)
    put(c, sc.barrel(), 2, 14)
    put(c, sc.crate(14, 14), 15, 14)
    put(c, sc.sack(), 30, 16)
    put(c, sc.rope_coil(), 12, 8)
    put(c, sc.crate(14, 14), 17, 0) if False else None
    return c


def keel():
    """The underside of the hull as the sea sees it, 48 wide and 16 tall,
    repeating: a dark plank edge, barnacles, and weed hanging down."""
    c = Canvas(48, 16)
    c.rect(0, 0, 48, 2, "0")
    c.rect(0, 2, 48, 1, "8")
    for x in (3, 4, 15, 22, 23, 36, 41):
        c.set(x, 3, "7")
    for x, h in ((6, 9), (19, 12), (28, 7), (40, 10)):
        for y in range(3, 3 + h):
            c.set(x + (1 if (y // 3) % 2 else 0), y, "c" if y % 4 else "4")
    return c


def sand():
    """The seabed's top, 32 wide and 14 tall, repeating: tan sand with a
    bright rim and dithered shade."""
    c = Canvas(32, 14)
    tops = [2, 1, 1, 0, 0, 1, 1, 2, 2, 2, 1, 1, 0, 0, 0, 1, 1, 2, 2, 1, 1, 1, 0, 0, 1, 1, 2, 2, 2, 1, 1, 2]
    for x, t in enumerate(tops):
        c.rect(x, t, 1, 14 - t, "7")
        c.set(x, t, "f")
    c.checker(0, 6, 32, 3, "8", 0)
    c.rect(0, 9, 32, 5, "8")
    c.checker(0, 9, 32, 1, "7", 1)
    for x, y in ((5, 4), (14, 5), (24, 3), (29, 5)):
        c.set(x, y, "8")
    return c


def kelp():
    """Two frames of a weed strand, each 9 wide and 30 tall, swaying a pixel."""
    frames = []
    for sway in (0, 1):
        c = Canvas(9, 30)
        for y in range(30):
            t = (29 - y) / 29.0                       # 0 at the root, 1 at the tip
            x = 4 + int(round(2.2 * t * (1 if sway else -1) * (1 if (y // 9) % 2 == 0 else 0.4)))
            c.set(x, y, "c")
            c.set(x + 1, y, "4")
            if y in (22, 14, 6):
                c.rect(x - 2, y - 1, 2, 2, "c")
            if y in (18, 10, 3):
                c.rect(x + 2, y - 1, 2, 2, "4")
        frames.append(c)
    return Canvas.sheet(frames)


def crab():
    frames = []
    for up in (True, False):
        c = Canvas(16, 9)
        c.ellipse(7.5, 5.5, 4.5, 2, "f")
        c.outline("8")
        for x in (3, 5, 10, 12):
            c.set(x, 8, "8")
        # claws
        ly = 1 if up else 3
        c.rect(1, ly, 2, 2, "f"); c.rect(13, ly, 2, 2, "f")
        c.set(2, ly + 2, "8"); c.set(13, ly + 2, "8")
        c.set(6, 3, "0"); c.set(9, 3, "0")
        frames.append(c)
    return Canvas.sheet(frames)


def starfish():
    return Canvas.from_map("""
.....7.....
.....7.....
....777....
7777777777.7
.777777777.
..7777777..
..777.777..
.777...777.
.77.....77.
""".replace("7777777777.7", "77777777777"), {"7": "7"})


def rocks():
    c = Canvas(26, 11)
    c.ellipse(7, 7, 7, 3.5, "8")
    c.ellipse(18, 6, 6, 4.5, "8")
    c.outline("0")
    for x, y in ((4, 5), (5, 5), (16, 3), (17, 3), (18, 3)):
        c.set(x, y, "7")
    return c


def mast_base():
    """Where the mast meets the deck: a collar with bolts, 14 wide and 8 tall."""
    c = Canvas(14, 8)
    c.rect(0, 2, 14, 6, "0")
    c.rect(1, 3, 12, 4, "3")
    c.rect(1, 3, 12, 1, "b")
    c.rect(4, 0, 6, 3, "0")
    c.rect(5, 0, 4, 3, "3")
    for x in (3, 10):
        c.set(x, 5, "0")
    return c


ART = {
    "pile": hold_pile,
    "pile-small": hold_pile_small,
    "keel": keel,
    "sand": sand,
    "kelp": kelp,
    "crab": crab,
    "starfish": starfish,
    "rocks": rocks,
    "mast-base": mast_base,
    "porthole": porthole,
    "bulkhead": bulkhead,
    "wall": wall,
    "beam": beam,
    "ladder": ladder,
    "lantern": lantern,
    "glow": glow,
    "steam": steam,
}

# the rooms are sheets: each frame is one still of the room, and frame 0 is the room at rest
HANDOFF_X = [33, 36, 39, 39, 39, 36, 33, 30, 27, 27, 27, 30]
ROOMS = {
    "room-galley": [galley(f) for f in range(4)],
    "room-chart": [chart_room(t) for t in (None, "left", "left", "top", "top", "bottom", "bottom", "top")],
    "room-telegraph": [telegraph(f) for f in range(16)],
    "room-handoff": [handoff(x) for x in HANDOFF_X],
    "room-hold": [hold_room(f) for f in range(12)],
}

SEAM = {"s": "#c2baa9"}      # the tan wall with a tenth of ink in it
COLORS = {
    "glow": {"b": "#ffd462@0.22"},
    "wall": {"8": "#5f5a75@0.17"},
    "room-galley": SEAM, "room-chart": SEAM, "room-telegraph": SEAM, "room-handoff": SEAM,
    "room-hold": SEAM,
}


def build(out=OUT):
    from pixel import PALETTE
    sheets = {name: fn() for name, fn in ART.items()}
    sheets.update({name: Canvas.sheet(frames) for name, frames in ROOMS.items()})
    for name, c in sheets.items():
        with open(os.path.join(out, f"{name}.svg"), "w") as f:
            f.write(c.svg(unit=1, decorative=True, colors={**PALETTE, **COLORS.get(name, {})}))


if __name__ == "__main__":
    build()
