"""The app mark at large sizes: the same ship as the 16 x 16 app mark, drawn on a
72 x 72 cell grid so the crew can stand on deck as themselves, in their states.

The small marks in directions/ansi/tools/marks.py stay the drawing for 16 to 64px.
This one is for 128px and up (README header, org avatar, social preview).

The crew come straight from crew.py with the landing page's role to state
mapping (deckhand working, cook unseen, captain idle), so face, face colour,
pose and prop are crew.py's. The cook is mirrored to hold the parcel out over
the stern, away from the captain.

Every part is drawn twice, once in colour and once for one colour. The one-colour
drawing follows the small mono maps: lines and dark parts are ink, fills are paper,
the crew's screens stay ink with paper faces. It is chosen part by part, not
flattened from the colour drawing.

The avatar variant is the same ship with a shorter sea, centred on a larger
canvas so every cell sits inside the circle GitHub crops an org avatar to.

usage: python3 website/tools/marks_large.py [out_dir]   (default website/public/marks)
"""
import math
import os
import sys

sys.dont_write_bytecode = True  # directions/ is frozen: leave no caches in it
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "directions", "ansi", "tools"))
from pixel import Canvas, PALETTE  # noqa: E402
from crew import crewmate, body, BX, BY, SCREEN, R0, SY  # noqa: E402

N = 72                 # the grid
CX = 36                # the mast stands on columns 35 and 36
RAIL = 52              # the deck: crew stand on it, the hull hangs below it
CREW_TOP = RAIL - 16   # body row 0 (the cap); feet are on body row 15, the row above the rail
# hat, mood, left column of the 16-wide body, mirrored
CREW = (("cook", "unseen", 9, True), ("captain", "idle", 28, False), ("deckhand", "working", 47, False))
UNIT = 4               # SVG units per cell


def span(c, y, x0, x1, k):
    for x in range(x0, x1 + 1):
        c.set(x, y, k)


def mirror_span(c, y, half, k):
    """A row centred on the mast, `half` cells either side of the centre line."""
    span(c, y, CX - half, CX - 1 + half, k)


# ---- parts ----------------------------------------------------------------

def flag(c, mono):
    # a pennant streaming right from the masthead, cut like the small mark's
    k = "#" if mono else "1"
    for y, x1 in ((1, 47), (2, 45), (3, 42), (4, 40)):
        span(c, y, CX + 2, x1, k)


def mast(c, mono):
    for y in range(0, CREW_TOP + 1):
        for x, k in ((CX - 2, "0"), (CX - 1, "3"), (CX, "3"), (CX + 1, "0")):
            # one colour: the wood is paper, so the mast stays apart from the flag and the captain's cap
            c.set(x, y, ("#" if k == "0" else None) if mono else k)
    span(c, 0, CX - 2, CX + 1, "#" if mono else "0")


def sail(c, top, rows, mono):
    """A square sail on its yard. rows: half-widths from the yard down, so the
    sides belly out. The last row is the ink foot; the first row of cloth is
    tan, the canvas gathered at the yard."""
    ink = "#" if mono else "0"
    n = len(rows)
    mirror_span(c, top, rows[0] + 2, ink)          # the yard, wider than the sail
    for i, hw in enumerate(rows):
        y = top + 1 + i
        mirror_span(c, y, hw, ink)
        if i == n - 1:
            continue
        for x in range(CX - hw + 1, CX + hw - 1):
            c.set(x, y, None if mono else ("7" if i == 0 else "f"))
        if mono and 0 < i:
            # one colour: the sides of the cloth carry a second line, for weight
            c.set(CX - hw + 1, y, ink)
            c.set(CX + hw - 2, y, ink)


def limbs(hat, mood):
    """Sprite cells of the arms and legs: ink outside the head, body and hat,
    and outside the parcel. Unflipped sprite coordinates."""
    s = crewmate(hat, mood)
    b, top = body(hat)
    shell = {(x + BX, y + BY + top) for (x, y) in b.px}
    parcel = set()
    if mood == "unseen":
        parcel = {(x, y) for x in range(R0 + 2, R0 + 7) for y in range(SY - 2, SY + 3)}
    return {p for p, k in s.px.items() if k == "0" and p not in shell and p not in parcel and p[1] < BY + 16}


def crew_sprite(hat, mood, flip):
    s = crewmate(hat, mood)
    return s.flipped() if flip else s


def crew_cells(mono):
    """grid cell -> key for the three crew, with the pixels that were dropped."""
    out, dropped = {}, []
    sx, sy = SCREEN
    for hat, mood, x0, flip in CREW:
        s = crew_sprite(hat, mood, flip)
        lim = {((27 - x) if flip else x, y) for x, y in limbs(hat, mood)}
        for (px, py), k in s.px.items():
            if not mono and (px, py) in lim:
                k = "8"                                    # limbs in bright black: they show on paper and on ink
            x, y = px - BX + x0, py - BY + CREW_TOP
            if y > RAIL or (y == RAIL and k == "0"):
                # under the deck line: the mop's floor line. The mop head's lower row lies on the rail.
                dropped.append((hat, mood, px, py, k))
                continue
            if mono:
                in_screen = sx <= px < sx + 10 and sy <= py < sy + 6
                in_body = BX < px < BX + 15
                if in_screen:
                    k = None if k != "0" else "#"          # ink screen, paper face
                elif k == "7" and not in_body:
                    k = "#"                                # the mop head
                else:
                    k = {"0": "#", "4": "#", "3": "#", "8": "#"}.get(k)   # cloth, bezels, the parcel's light: paper
            if (x, y) in out:
                raise ValueError(f"crew overlap at {(x, y)}")
            out[(x, y)] = k
    return out, dropped


def crew(c, mono):
    cells, _ = crew_cells(mono)
    for p, k in cells.items():
        if p in c.px and p[1] != RAIL and p[0] not in (CX - 2, CX - 1, CX, CX + 1):
            raise ValueError(f"crew collides with the ship at {p}")
        if k is None:
            c.px.pop(p, None)
        else:
            c.set(p[0], p[1], k)


def hull(c, mono):
    """Yellow hull with gunports, posts rising at bow and stern, ink outline.
    In one colour the hull is solid ink and its ports and stripe are paper,
    so the ship keeps the small one-colour mark's weight."""
    body = Canvas(N, N)
    halves = [34, 34, 33, 33, 32, 31, 30, 28, 26, 23]     # from the rail down: a bowl
    for i, hw in enumerate(halves):
        mirror_span(body, RAIL + 1 + i, hw, "3")
    for side in (-1, 1):                                   # bow and stern posts
        for j in range(3):
            x = (CX - 35 + j) if side < 0 else (CX + 34 - j)
            for y in range(RAIL - 3 + j, RAIL + 1):
                body.set(x, y, "3")
    body.outline("0")
    span(body, RAIL, CX - 34, CX + 33, "0")                # the rail: the deck line
    for x in range(N):
        if body.px.get((x, RAIL + 1)) == "3":
            body.set(x, RAIL + 1, "b")                     # lit strake under the rail
        if body.px.get((x, RAIL + 2)) == "3":
            body.set(x, RAIL + 2, "0")                     # the wale
        if body.px.get((x, RAIL + 7)) == "3":
            body.set(x, RAIL + 7, "0")                     # a plank seam
    ports = [CX - 31 + i * 8 for i in range(8)]
    for gx in ports:
        body.rect(gx, RAIL + 4, 3, 2, "0")                 # gunports
    for p, k in body.px.items():
        if not mono:
            c.px[p] = k
        elif k in ("0", "3"):
            c.px[p] = "#"
        else:
            c.px.pop(p, None)
    if mono:
        # paper ports and strake: the colour drawing's ink ports become holes
        for gx in ports:
            for dx in range(3):
                for dy in range(2):
                    c.px.pop((gx + dx, RAIL + 4 + dy), None)


def sea(c, mono, short=False):
    """Two broken lines of swell under the hull, like the small mark's sea.
    short: the avatar's sea, cut to the hull's length so it fits a circle."""
    blue = "#" if mono else "4"
    lit = "#" if mono else "c"
    x0, x1 = (6, N - 7) if short else (0, N - 1)
    for x in range(x0, x1 + 1):
        for y in range(62, 65):
            c.set(x, y, blue)
    for x in ((11, 12, 59, 60) if not short else (14, 15, 56, 57)):   # breaks, as in the small mark
        for y in range(62, 65):
            c.set(x, y, None)
    lower = ((4, 21), (27, 44), (50, 67)) if not short else ((12, 25), (29, 42), (46, 59))
    for a, b in lower:
        for x in range(a, b + 1):
            c.set(x, 67, blue)
            c.set(x, 68, blue)
    if not mono:
        for a in ((1, 16, 30, 45, 63) if not short else (8, 18, 31, 45, 59)):
            span(c, 62, a, a + 3, lit)
        for a, _ in lower:
            span(c, 67, a + 2, a + 5, lit)


def draw(mono=False, short_sea=False):
    c = Canvas(N, N)
    flag(c, mono)
    mast(c, mono)
    sail(c, 6, [8, 9, 10, 10, 11, 11], mono)                              # topsail
    sail(c, 14, [14, 16, 17, 18, 18, 19, 19, 19, 19, 19, 18, 18], mono)   # course
    hull(c, mono)
    crew(c, mono)                       # after the hull: the crew stand on the deck line
    sea(c, mono, short_sea)
    return Canvas(N, N).blit(c, 0, 1)   # one row down: a cell of air above the masthead


def avatar(mono=False):
    """The ship on a canvas big enough that every cell lies inside the centred circle."""
    ship = draw(mono, short_sea=True)
    xs = [x for x, _ in ship.px]
    ys = [y for _, y in ship.px]
    for size in range(N, 2 * N):
        ox = (size - (max(xs) + min(xs) + 1)) // 2
        oy = (size - (max(ys) + min(ys) + 1)) // 2
        r = size / 2
        if all(math.hypot(x + ox + dx - r, y + oy + dy - r) <= r
               for x, y in ship.px for dx in (0, 1) for dy in (0, 1)):
            return Canvas(size, size).blit(ship, ox, oy)
    raise ValueError("no circle fits")


TITLE = "Ship: a pixel sailing ship with three crewmates on deck"
INK = PALETTE["0"]


def svg(c, mono, title):
    px = c.w * UNIT
    if mono:
        return c.svg(unit=UNIT, colors={"#": INK}, title=title + " (one colour)", size=(px, px))
    return c.svg(unit=UNIT, title=title, size=(px, px))


def build(out_dir):
    os.makedirs(out_dir, exist_ok=True)
    files = {
        "app-large.svg": svg(draw(), False, TITLE),
        "app-large-one-colour.svg": svg(draw(True), True, TITLE),
        "app-large-avatar.svg": svg(avatar(), False, TITLE),
        "app-large-avatar-one-colour.svg": svg(avatar(True), True, TITLE),
    }
    for name, text in files.items():
        with open(os.path.join(out_dir, name), "w") as f:
            f.write(text)
    return files


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "..", "public", "marks")
    for name in build(out):
        print(os.path.join(out, name))
    _, dropped = crew_cells(False)
    print("crew pixels dropped at the deck:", dropped)
    print("avatar grid:", avatar().w)
