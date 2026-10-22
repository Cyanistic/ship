"""Crewmates: walking terminals, built from the bot mark's head.

One body, five hats, five moods. A mood is a face drawn on the screen in
that state's colour plus a pose. The face colour repeats the state colour
but never carries the state alone: every page that shows a mood also shows
the glyph and the word.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from pixel import Canvas  # noqa: E402
from marks import BOT, LEGEND  # noqa: E402

W, H = 28, 24
BX, BY = 6, 5          # where the body's 16-wide box sits
SCREEN = (BX + 3, BY + 5)  # top-left of the 10 x 6 screen

# state -> (glyph, palette key for the face, word)
MOODS = {
    "working": ("●", "3", "yellow"),
    "blocked": ("▲", "1", "red"),
    "unseen": ("◆", "6", "cyan"),
    "idle": ("○", "2", "green"),
    "error": ("✖", "5", "magenta"),
}

FACES = {
    "neutral": """
..........
..##..##..
..##..##..
..........
.....##...
..........
""",
    "working": """
..........
.#......#.
..#....#..
.#......#.
....##....
..........
""",
    "blocked": """
..##..##..
..##..##..
..........
...####...
...#..#...
...####...
""",
    "unseen": """
..........
..#....#..
.#.#..#.#.
..........
...#..#...
....##....
""",
    "idle": """
..........
..........
.###..###.
..........
....##....
..........
""",
    "error": """
.#.#..#.#.
..#....#..
.#.#..#.#.
..........
...####...
..........
""",
    "blink": """
..........
..........
..##..##..
..........
.....##...
..........
""",
    "glance": """
..##..##..
..##..##..
..........
..........
.....##...
..........
""",
}

HATS = {
    # each hat replaces body rows 0..3 (and may rise above them)
    "deckhand": None,  # the sailor cap already on the bot head
    "captain": """
....kkkkkkkk....
...kBBBBBBBBk...
..kBBBBYYBBBBk..
.kkkkkkkkkkkkkk.
""",
    "cook": """
.....kkkkkk.....
....kwwwwwwk....
...kwwwwwwwwk...
...kwwwwwwwwk...
...kwwwwwwwwk...
....kkkkkkkk....
.kkwwwwwwwwwwkk.
""",
    "lookout": """
.......kk.......
......kwwk......
.....kkkkkk.....
....kcccccck....
...kcccccccck...
.kkcccccccccckk.
""",
    "navigator": """
................
....kkkkkkk.....
...k8888888kk...
.kk88888888888k.
""",
    # seabed only: a brass diving helmet in place of a cap
    "diver": """
.....kkkkkk.....
...kkYYYY33kk...
..kY33333333Yk..
.kYY3333333333k.
.kkkkkkkkkkkkkk.
""",
}


def body(hat="deckhand"):
    rows = BOT.strip("\n").split("\n")[1:14]   # the 13 rows of head and body
    c = Canvas(16, 13)
    c.blit_map("\n".join(rows), 0, 0, LEGEND)
    if HATS.get(hat):
        hmap = HATS[hat].strip("\n").split("\n")
        # clear the cap rows first, then draw the hat bottom-aligned on row 3
        for y in range(0, 4):
            for x in range(16):
                c.set(x, y, None)
        top = 4 - len(hmap)
        big = Canvas(16, 13 - top)
        big.blit(c, 0, -top)
        big.blit_map("\n".join(hmap), 0, 0, {**LEGEND, "Y": "b", "8": "8"})
        return big, top
    return c, 0


def face(name, key):
    f = Canvas(10, 6)
    f.blit_map(FACES[name], 0, 0, {"#": key})
    return f


def legs(c, gait=0):
    """Two legs. gait lifts a foot one row for a step: 0 both down, 1 left foot up, 2 right foot up."""
    lx, rx = BX + 4, BX + 10
    for side, x in ((1, lx), (2, rx)):
        up = 1 if gait == side else 0
        for y in ((BY + 13,) if up else (BY + 13, BY + 14)):
            c.set(x, y, "0"); c.set(x + 1, y, "0")
        c.rect(x - 1 if side == 1 else x, BY + 15 - up, 3, 1, "0")


def arm(c, pts, hand):
    for p in pts:
        c.set(p[0], p[1], "0")
    c.set(hand[0], hand[1], "0")


L0, R0 = BX, BX + 15          # shoulder columns
SY = BY + 8                   # shoulder row


def arms_down(c):
    arm(c, [(L0, SY), (L0 - 1, SY + 1), (L0 - 1, SY + 2)], (L0 - 1, SY + 3))
    arm(c, [(R0, SY), (R0 + 1, SY + 1), (R0 + 1, SY + 2)], (R0 + 1, SY + 3))


LANTERN = """
..k..
.kkk.
kkkkk
kbbbk
kbfbk
kbbbk
kkkkk
"""

CLIPBOARD = """
.kkk.
kfffk
k8f8k
kfffk
k8f8k
kfffk
kkkkk
"""


def apply_pose(c, pose, fx=None):
    """Arms and hand props for a neutral crewmate at work. Props that need more
    room than the 28 pixel sprite are drawn by the scene that uses the pose.
    fx tunes a pose for a loop: reach and point take `up` (0 to 2, the hand rises),
    clipboard takes `up` (0 or 1, the pen hand dips) and `ticks` (0 to 2 lines written)."""
    fx = fx or {}
    up = fx.get("up", 0)
    arm(c, [(L0, SY), (L0 - 1, SY + 1), (L0 - 1, SY + 2)], (L0 - 1, SY + 3))
    if pose == "reach":
        # right arm straight out at shoulder height, hand open; up lifts the hand like a beckon
        arm(c, [(R0, SY), (R0 + 1, SY - (1 if up else 0)), (R0 + 2, SY - up)], (R0 + 3, SY - up - (1 if up > 1 else 0)))
    elif pose == "lantern":
        arm(c, [(R0, SY), (R0 + 1, SY - 1), (R0 + 2, SY - 2), (R0 + 3, SY - 3)], (R0 + 4, SY - 4))
        c.blit_map(LANTERN, R0 + 2, SY - 4, {**LEGEND, "b": "b", "f": "f"})
    elif pose == "clipboard":
        dip = 1 if up else 0
        arm(c, [(R0, SY), (R0 + 1, SY - 1 + dip)], (R0 + 1, SY - 2 + dip))
        c.blit_map(CLIPBOARD, R0 + 1, SY - 4, {**LEGEND, "8": "8"})
        for i in range(fx.get("ticks", 0)):       # the pen has been down: this line is written
            c.set(R0 + 2, SY - 2 + 2 * i, "0"); c.set(R0 + 4, SY - 2 + 2 * i, "0")
    elif pose == "bare":
        pass                                      # the right arm is drawn by the scene (a pointer, a lever)
    elif pose == "point":
        # right arm up and out, one finger at something above; up lowers it for a tap
        arm(c, [(R0, SY), (R0 + 1, SY - 1), (R0 + 2, SY - 2 + (1 if up else 0)), (R0 + 3, SY - 3 + up)], (R0 + 4, SY - 4 + up + (1 if up > 1 else 0)))
    else:
        raise ValueError(pose)


def crewmate(hat="deckhand", mood=None, frame=None, pose=None, outline=None, fx=None):
    """mood: one of MOODS or None for neutral. frame: optional face override
    (blink, glance) for ambient animation. pose: an at-work pose for neutral
    crew (reach, lantern, clipboard, point, bare). fx: knobs for the loops (see build_loops); with none, the pose is the still one."""
    fx = fx or {}
    gait = fx.get("gait", 0)
    c = Canvas(W, H)
    b, top = body(hat)
    key = MOODS[mood][1] if mood else "f"
    lift = 0
    c.blit(b, BX, BY + top + lift)
    c.rect(SCREEN[0], SCREEN[1] + lift, 10, 6, "0")
    fname = frame or (mood or "neutral")
    c.blit(face(fname, key), SCREEN[0], SCREEN[1] + lift)

    if mood == "working":
        # swabbing: both hands on a mop to the right. sw is the stroke, 0 to 3: the head slides out and back
        sw = fx.get("sw", 0)
        sh = (0, 1, 2, 1)[sw]
        arm(c, [(L0, SY), (L0 - 1, SY + 1), (L0 - 1, SY + 2)], (L0 - 1, SY + 3))
        arm(c, [(R0, SY), (R0 + 1, SY)], (R0 + 2, SY))
        top_x, bot_x = R0 + 3 + (sh // 2), R0 + 1 + sh
        for i in range(9):
            c.set(top_x + int((bot_x - top_x) * i / 8), SY - 2 + i, "3")
        c.rect(R0 + sh, BY + 15, 4, 1, "7")
        c.rect(R0 - 1 + sh, BY + 16, 6, 1, "7")
        for x in range(R0 - 1 + sh, R0 + 5 + sh):
            c.set(x, BY + 17, "0")
        if fx.get("drip"):
            c.set(R0 + 6 + sh, BY + 14, "c"); c.set(R0 + 7 + sh, BY + 13, "c")
        legs(c, gait)
    elif mood == "blocked":
        # a raised hand, calling for you. wave sways the hand a pixel either way
        wave = fx.get("wave", 0)
        arm(c, [(L0, SY), (L0 - 1, SY + 1), (L0 - 1, SY + 2)], (L0 - 1, SY + 3))
        arm(c, [(R0, SY), (R0 + 1, SY - 1), (R0 + 1 + (wave > 0), SY - 2), (R0 + 2 + wave, SY - 3), (R0 + 2 + wave, SY - 4)],
            (R0 + 2 + wave * 2, SY - 6))
        legs(c, gait)
    elif mood == "unseen":
        # holding out the finished parcel; up lifts it a row
        u = fx.get("up", 0)
        arm(c, [(L0, SY), (L0 - 1, SY + 1), (L0 - 1, SY + 2)], (L0 - 1, SY + 3))
        for x in range(R0, R0 + 2):
            c.set(x, SY + 1 - u, "0")
        c.rect(R0 + 2, SY - 2 - u, 5, 5, "0")
        c.rect(R0 + 3, SY - 1 - u, 3, 3, "b" if fx.get("flash") else "3")      # flash: the parcel catches the light
        for y in (-1, 0, 1):
            c.set(R0 + 4, SY + y - u, "f" if fx.get("flash") else "b")
        legs(c, gait)
    elif mood == "idle":
        # hands behind the head
        for pts in ([(L0, SY), (L0 - 1, SY - 1), (L0 - 1, SY - 2), (L0, SY - 3)],
                    [(R0, SY), (R0 + 1, SY - 1), (R0 + 1, SY - 2), (R0, SY - 3)]):
            for p in pts:
                c.set(p[0], p[1], "0")
        legs(c, gait)
    elif mood == "error":
        # arms flung out, smoke rising. flail 1 throws them up instead of out
        fl = fx.get("flail", 0)
        arm(c, [(L0, SY), (L0 - 1, SY - fl), (L0 - 2, SY + 1 - 2 * fl)], (L0 - 3, SY + 1 - 2 * fl))
        arm(c, [(R0, SY), (R0 + 1, SY - fl), (R0 + 2, SY + 1 - 2 * fl)], (R0 + 3, SY + 1 - 2 * fl))
        if fx.get("smoke", True):
            y0 = max(0, BY + top - 4)
            for x, y in ((BX + 12, 1), (BX + 13, 1), (BX + 13, 0), (BX + 14, 0),
                         (BX + 15, 1), (BX + 14, 2), (BX + 16, 0)):
                c.set(x + (3 if top < -1 else 0), y0 + y, "8")
        legs(c, gait)
    elif pose:
        apply_pose(c, pose, fx)
        legs(c, gait)
    else:
        arms_down(c)
        legs(c, gait)
    if outline:
        # for dark grounds: every ink pixel outside the screen takes another key
        sx, sy = SCREEN
        for (x, y), k in list(c.px.items()):
            if k == "0" and not (sx <= x < sx + 10 and sy <= y < sy + 6):
                c.px[(x, y)] = outline
    return c


def prop_spyglass(c):
    """Lookout's spyglass, held up to the right side of the screen."""
    y = SY - 2
    c.rect(R0, y, 6, 2, "3")
    c.rect(R0 + 6, y - 1, 2, 4, "0")
    for x in range(R0, R0 + 6):
        c.set(x, y - 1, "0"); c.set(x, y + 2, "0")
    arm(c, [(R0, SY + 1)], (R0 + 2, SY + 1))
    return c


def build():
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(os.path.dirname(here), "art")
    os.makedirs(out, exist_ok=True)

    def write(name, c):
        with open(os.path.join(out, name), "w") as f:
            f.write(c.svg(unit=1, decorative=True))

    roles = [h for h in HATS if h != "diver"]
    for mood in [None] + list(MOODS):
        for hat in roles:
            write(f"crew-{hat}-{mood or 'neutral'}.svg", crewmate(hat, mood))
    # ambient frames: neutral crew blink and glance up, the deckhand's second swab
    for hat in roles:
        for fr in ("blink", "glance"):
            write(f"crew-{hat}-neutral-{fr}.svg", crewmate(hat, None, frame=fr))
    # at-work poses
    for pose in ("reach", "lantern", "clipboard", "point"):
        for hat in roles:
            write(f"crew-{hat}-{pose}.svg", crewmate(hat, None, pose=pose))
    # a screen patch to lay over a neutral crewmate's face: blink and glance up
    for fr in ("blink", "glance"):
        patch = Canvas(10, 6)
        patch.rect(0, 0, 10, 6, "0")
        patch.blit(face(fr, "f"), 0, 0)
        write(f"face-{fr}.svg", patch)
    # the diver, for dark water: ink outlines are drawn in bright blue
    write("crew-diver-neutral.svg", crewmate("diver", None, outline="c"))
    write("crew-diver-reach.svg", crewmate("diver", None, pose="reach", outline="c"))
    write("crew-diver-point.svg", crewmate("diver", None, pose="point", outline="c"))


if __name__ == "__main__":
    build()
