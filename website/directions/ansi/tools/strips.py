"""Cut a tall screenshot into viewport-sized strips for inspection.

usage: strips.py <png> <outdir> [strip height, default 900]
Writes <outdir>/strip-00.png, strip-01.png and so on, and prints the count.
Also reports whether the last rows are one flat colour, which is the sign of a
capture that ran past the end of the page (a blank band).
"""
import os
import sys

from PIL import Image

src, out = sys.argv[1], sys.argv[2]
step = int(sys.argv[3]) if len(sys.argv) > 3 else 900
os.makedirs(out, exist_ok=True)
im = Image.open(src).convert("RGB")
w, h = im.size
n = 0
for y in range(0, h, step):
    im.crop((0, y, w, min(h, y + step))).save(os.path.join(out, f"strip-{n:02d}.png"))
    n += 1
tail = im.crop((0, h - 40, w, h)).getcolors(10000)
flat = tail is not None and len(tail) == 1
colour = "#%02x%02x%02x" % tail[0][1] if flat else None
print(f"{w}x{h} -> {n} strips; last 40 rows " + (f"are one flat colour {colour}: check it is a real footer, not a blank band" if flat else "have content"))
