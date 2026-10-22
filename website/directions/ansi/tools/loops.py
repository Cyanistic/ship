"""Loops: the crew at work, drawn as sprite sheets for the page's steps() animations.

A loop is a row of equal frames in one SVG. The page shows one frame at a time and
steps along the row (see .act in css/site.css), so every frame sits on the same
pixel grid as the still art and nothing is ever blended between two poses.

Frame 0 is always the still pose, so the page reads the same with motion off.
The deck crew keep their state: a mood is a face, a glyph and a pose, and the loop
only moves within that pose (the error crewmate sputters, it never calms down).
Each loop has its own length and its own step time in the CSS, so they drift
against each other instead of moving in sync.

Rooms are sheets too, built in rooms.py; the lookout's spyglass is in scene.py.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from pixel import Canvas  # noqa: E402
from crew import crewmate, BX, BY, SY, R0  # noqa: E402
import scenery as sc  # noqa: E402
import scene  # noqa: E402

OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "art")

# A deck frame is 36 x 28: the 28 x 24 sprite stands at (4, 4), which leaves room
# to pace sideways and for smoke above a hat. The page centres it on the sprite's old spot.
FW, FH, OX, OY = 36, 28, 4, 4


def stand(hat, mood, dx=0, fx=None):
    c = Canvas(FW, FH)
    c.blit(crewmate(hat, mood, fx=fx), OX + dx, OY)
    return c


def puff(c, x, y, life):
    """A grey puff of smoke that swells and thins as it drifts up and away (life 0 to 3)."""
    if life == 0:
        c.rect(x, y, 3, 2, "8")
    elif life == 1:
        c.rect(x, y, 3, 3, "8"); c.rect(x - 1, y + 1, 1, 1, "8")
    elif life == 2:
        c.rect(x, y, 3, 2, "7"); c.set(x + 1, y - 1, "7")
    else:
        c.rect(x, y, 2, 1, "7")


def lookout_error():
    """Error: it sputters. Smoke rises in puffs, the arms flail and now and then it jolts."""
    jolts = [(0, 0), (0, 1), (0, 0), (-1, 1), (1, 0), (0, 1)]       # (sideways, arms thrown up)
    frames = []
    for f, (dx, flail) in enumerate(jolts):
        c = stand("lookout", "error", dx, {"flail": flail, "smoke": False})
        for off in (0, 3):
            life = (f + off) % 6
            if life < 4:
                puff(c, 22 + 2 * life, 6 - 2 * life, life)
        frames.append(c)
    return frames


def navigator_blocked():
    """Blocked: a raised hand calling for you, a tapping foot, and now and then a red bang above it."""
    steps = [(0, 0, 0), (1, 2, 1), (0, 0, 1), (-1, 2, 0), (0, 0, 0), (1, 2, 1), (0, 0, 1), (-1, 0, 0)]   # (wave, foot, bang)
    frames = []
    for wave, gait, bang in steps:
        c = stand("navigator", "blocked", 0, {"wave": wave, "gait": gait})
        if bang:
            x = OX + R0 + 2 + wave * 2
            c.rect(x, 2, 2, 3, "1"); c.rect(x, 6, 2, 1, "1")
        frames.append(c)
    return frames


def deckhand_working():
    """Working: swabbing, the mop sliding out and back, the feet shuffling, the body leaning along with it."""
    steps = [(0, 0, 0, 0), (1, 0, 1, 0), (2, 1, 0, 1), (3, 1, 2, 1), (0, 1, 0, 0), (1, 1, 1, 0), (2, 0, 0, 1), (3, 0, 2, 0)]  # (stroke, lean, foot, drip)
    return [stand("deckhand", "working", lean, {"sw": sw, "gait": gait, "drip": drip}) for sw, lean, gait, drip in steps]


def cook_unseen():
    """Unseen: holding out the finished parcel, which lifts and catches the light."""
    steps = [(0, 0), (0, 0), (0, 0), (1, 0), (1, 1), (1, 2), (1, 1), (0, 0), (0, 0), (0, 0)]     # (parcel lifted, sparkle size)
    frames = []
    for up, spark in steps:
        c = stand("cook", "unseen", 0, {"up": up, "flash": spark == 2})
        x, y = OX + R0 + 7, OY + SY - 4 - up
        if spark:
            for dx, dy in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)) if spark == 2 else ((0, 0),):
                c.set(x + dx, y + dy, "6")
            if spark == 2:
                c.set(x, y, "e")
        frames.append(c)
    return frames


def captain_idle():
    """Idle: hands behind the head, strolling three steps one way, back, three the other way, back."""
    path, pos, step = [(0, 0)] * 4, 0, 0                # frame 0 is the still pose
    plan = [(+3, 3), (-3, 3), (-3, 3), (+3, 4)]        # (steps to take, frames to stand still afterwards)
    for n, rest in plan:
        for _ in range(abs(n)):
            pos += 1 if n > 0 else -1
            path.append((pos, 1 + step % 2))           # a foot up as the body moves on a pixel
            path.append((pos, 0))                      # then both feet planted
            step += 1
        path += [(pos, 0)] * rest
    return [stand("captain", "idle", dx, {"gait": g}) for dx, g in path]


DECK = {
    # name: (frames, hat, mood)
    "crew-lookout-error-loop": lookout_error,
    "crew-navigator-blocked-loop": navigator_blocked,
    "crew-deckhand-working-loop": deckhand_working,
    "crew-cook-unseen-loop": cook_unseen,
    "crew-captain-idle-loop": captain_idle,
}


# ---- the notes on the docs page: small, with long rests -----------------------

def note_loop(hat, pose, steps):
    return [crewmate(hat, None, pose=pose, fx=fx) for fx in steps]


def note_deckhand():
    """Writing on the clipboard, a line at a time, then a fresh page."""
    steps = [(0, 0)] * 6 + [(0, 1), (1, 0), (1, 1), (2, 0), (2, 1), (2, 0), (2, 0), (2, 0)]     # (lines written, pen dipped)
    return note_loop("deckhand", "clipboard", [{"ticks": t, "up": d} for t, d in steps])


def note_navigator():
    """Tapping the air toward the note."""
    ups = [0] * 8 + [1, 2, 1, 2, 1, 0, 0, 0]
    return note_loop("navigator", "point", [{"up": u} for u in ups])


def note_cook():
    """Beckoning: this way."""
    ups = [0] * 9 + [1, 2, 1, 2, 1, 2, 0]
    return note_loop("cook", "reach", [{"up": u} for u in ups])


NOTES = {
    "crew-deckhand-clipboard-loop": note_deckhand,
    "crew-navigator-point-loop": note_navigator,
    "crew-cook-reach-loop": note_cook,
}


# ---- the diver ------------------------------------------------------------------

DW, DH, DY = 28, 34, 10       # the diver sprite stands at (0, 10); the rows above are for the bubbles


def diver_loop():
    """The diver waves now and then and breathes out a trail of bubbles."""
    ups = [0, 0, 1, 2, 1, 0, 0, 0]
    frames = []
    for f, up in enumerate(ups):
        c = Canvas(DW, DH)
        c.blit(crewmate("diver", None, pose="reach", outline="c", fx={"up": up}), 0, DY)
        for off, dx in ((0, 0), (4, 1)):
            life = (f + off) % 8
            if life < 5:
                b = sc.bubble()
                if life >= 3:
                    b = Canvas(2, 2); b.rect(0, 0, 2, 2, "c")
                c.blit(b, 17 + dx + (life % 2), 12 - 3 * life)
        frames.append(c)
    return frames


# ---- the lookout's spyglass in the nest -----------------------------------------

def spyglass_loop():
    """The lookout sweeps the spyglass from the horizon down to the agent list and back."""
    tilts = [0, 0, 1, 2, 2, 2, 1, 0, -1, -2, -2, -1, 0, 0]
    return [scene.spyglass(t, glint=abs(t) == 2 and i % 2 == 0) for i, t in enumerate(tilts)]


def build(out=OUT):
    def write(name, frames):
        with open(os.path.join(out, name + ".svg"), "w") as f:
            f.write(Canvas.sheet(frames).svg(unit=1, decorative=True))

    for name, fn in DECK.items():
        write(name, fn())
    for name, fn in NOTES.items():
        write(name, fn())
    write("crew-diver-loop", diver_loop())
    write("spyglass-loop", spyglass_loop())


if __name__ == "__main__":
    build()
