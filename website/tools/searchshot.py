import os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp import Page
url, png, width, term = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
p = Page(width, height=700)
try:
    p.goto(url)
    p.key("/")
    # type at keyboard speed: the whole term landing in one burst right after "/" can leave Pagefind's
    # result fetches aborted in headless Chrome, and the box stays empty
    for ch in term:
        time.sleep(0.08)
        p.type_text(ch)
    time.sleep(1.5)
    p.viewport_png(png)
finally:
    p.close()
