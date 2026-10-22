"""Count the pixels that differ between two screenshots and write a mask of them (red on grey).
usage: diff.py <a.png> <b.png> <mask.png>"""
import sys
from PIL import Image, ImageChops
a, b = Image.open(sys.argv[1]).convert("RGB"), Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size:
    print(f"sizes differ: {a.size} vs {b.size}")
    w, h = min(a.size[0], b.size[0]), min(a.size[1], b.size[1])
    a, b = a.crop((0, 0, w, h)), b.crop((0, 0, w, h))
d = ImageChops.difference(a, b).convert("L").point(lambda v: 255 if v > 8 else 0)
px = d.tobytes()
n = sum(1 for v in px if v)
rows = sorted({i // a.size[0] for i, v in enumerate(px) if v})
print(f"{n} of {a.size[0] * a.size[1]} pixels differ" + (f", rows {rows[0]}..{rows[-1]}" if rows else ""))
grey = a.convert("L").convert("RGB").point(lambda v: v // 3)
grey.paste((255, 0, 0), mask=d)
grey.save(sys.argv[3])
