"""Full-page screenshot of a URL or file (copied from ansi/tools/shot.py, accepts http URLs).
usage: shot.py <url-or-html> <png> <width>"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp import Page
src, png, width = sys.argv[1], sys.argv[2], int(sys.argv[3])
if "://" not in src: src = os.path.abspath(src)
page = Page(width)
try:
    page.goto(src)
    h = page.full_page_png(png)
    sw = page.eval("document.documentElement.scrollWidth")
    print(f"{png}: {width}x{h}" + (f"  WARNING scroll width {sw}" if sw > width else ""))
finally:
    page.close()
