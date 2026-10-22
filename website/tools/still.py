"""Full-page screenshot with prefers-reduced-motion on, so every loop sits on frame 0 and two
renders of the same page can be compared pixel for pixel.
usage: still.py <url-or-html> <png> <width>"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp import Page
src, png, width = sys.argv[1], sys.argv[2], int(sys.argv[3])
if "://" not in src: src = os.path.abspath(src)
page = Page(width, reduced_motion=True)
try:
    page.goto(src, settle=3.0)
    print(f"{png}: {width}x{page.full_page_png(png)}")
finally:
    page.close()
