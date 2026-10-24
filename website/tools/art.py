"""Sprites added to public/art/ since the ansi direction was frozen. Same canvas and palette as
directions/ansi/tools (imported from there, never edited), same rule: change this file and run it,
don't edit the SVG.

    python3 tools/art.py

face-look.svg: nine faces of a neutral crewmate on a 3 by 3 sheet, for the eyes that follow the pointer.
Column 0 looks left, 1 straight on, 2 right; row 0 looks up, 1 level, 2 down. The middle cell is the
plain neutral face, so a sheet can always rest on it.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "directions", "ansi", "tools"))
from pixel import Canvas  # noqa: E402

OUT = os.path.join(HERE, "..", "public", "art")
FW, FH = 10, 6  # the screen of a crewmate: the same patch the blink and glance faces cover


def face(dx, dy):
    """The neutral face with its eyes turned (dx, dy) pixels: two 2 by 2 eyes and a small mouth.
    The mouth follows sideways, and drops a row when the eyes look down, so the face stays whole."""
    c = Canvas(FW, FH)
    c.rect(0, 0, FW, FH, "0")
    for ex in (2, 6):
        c.rect(ex + dx, 1 + dy, 2, 2, "f")
    c.rect(5 + dx, 4 + (1 if dy > 0 else 0), 2, 1, "f")
    return c


def look():
    sheet = Canvas(FW * 3, FH * 3)
    for row, dy in enumerate((-1, 0, 1)):
        for col, dx in enumerate((-1, 0, 1)):
            sheet.blit(face(dx, dy), col * FW, row * FH)
    return sheet


def build(out=OUT):
    with open(os.path.join(out, "face-look.svg"), "w") as f:
        f.write(look().svg(decorative=True))


if __name__ == "__main__":
    build()
    print("face-look.svg")
