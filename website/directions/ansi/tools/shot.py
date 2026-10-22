"""Full-page screenshots through the DevTools protocol.

usage: shot.py <html> <png> <width> [--reduced-motion]

Sets a 900 pixel tall viewport of the given width, waits for the fonts, and
captures the whole document, so the PNG is the real full page. Prints its size.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp import Page  # noqa: E402

html, png, width = os.path.abspath(sys.argv[1]), sys.argv[2], int(sys.argv[3])
page = Page(width, reduced_motion="--reduced-motion" in sys.argv)
try:
    page.goto(html)
    h = page.full_page_png(png)
    overflow = page.eval("document.documentElement.scrollWidth")
    print(f"{png}: {width}x{h}" + (f"  WARNING scroll width {overflow}" if overflow > width else ""))
finally:
    page.close()
