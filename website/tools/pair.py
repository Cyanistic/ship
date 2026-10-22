"""Side-by-side: original on the left, starlight on the right, rows y0..y1, scaled.
usage: pair.py <left.png> <right.png> <out.png> <y0> <y1> [scale]"""
import sys
from PIL import Image, ImageDraw
l, r, out, y0, y1 = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
s = float(sys.argv[6]) if len(sys.argv) > 6 else 1.0
a, b = Image.open(l).convert("RGB"), Image.open(r).convert("RGB")
w = a.size[0]
ca, cb = a.crop((0, y0, w, min(y1, a.size[1]))), b.crop((0, y0, w, min(y1, b.size[1])))
h = max(ca.size[1], cb.size[1])
im = Image.new("RGB", (w * 2 + 16, h), "#ff00ff")
im.paste(ca, (0, 0)); im.paste(cb, (w + 16, 0))
if s != 1.0: im = im.resize((int(im.size[0] * s), int(im.size[1] * s)), Image.LANCZOS)
im.save(out)
