"""Checks on the two pages in a real browser (headless Chrome through cdp.py).

usage: python3 tools/check.py [width ...]      default widths: 1440 1100 768 390

What it checks, per page and width:
  anchors      every #fragment link has a target on its page; every link between the pages and to local files resolves
  images       every <img> has an alt attribute and loaded
  overflow     no element pokes past the right edge of the window unless an ancestor clips it
               (html and body use overflow-x: clip, so scrollWidth cannot be trusted)
  column       headings and reading text stay inside the page gutters
  motion       with prefers-reduced-motion there are no running animations; without it there are
  keyboard     Tab walks the first stops, each one visible with a focus outline; the places menu opens with Enter
               and closes with Esc and returns focus to its button
  places       the landing's place label follows the scroll
  glance       a neutral crewmate looks up when its room scrolls into view
  copy         a copy button puts its text on the clipboard (real click)
  search       the docs search finds text, says when there is none, opens with / and closes with Esc
  loops        every crew loop: the sheet is exactly n frames of the size the CSS says, it runs, it has its own pace,
               it pauses off screen, it stops with reduced motion on frame 0; the deck crew keep their plate and their
               state colour in every frame and the diver's bubbles stay clear of the hose (the art check reads the generators)
  edges        the bar lines up with the page column; the sticky bar at 390 stays compact
  scale        every piece of text uses a step of the type scale (14 16 18 20 22 for reading text, 13 14 for code,
               multiples of 11 for the pixel face)
Prints one line per check and exits 1 if any failed.
"""
import json
import os
import re
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp import Page  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PAGES = {"landing": os.path.join(ROOT, "index.html"), "docs": os.path.join(ROOT, "docs.html")}
results = []


def record(name, ok, detail=""):
    results.append(ok)
    print(("ok   " if ok else "FAIL ") + name + (f"  {detail}" if detail else ""))


def ids_of(page, path):
    page.goto(path, settle=0.3)
    return set(page.eval("[...document.querySelectorAll('[id]')].map(e => e.id)"))


OVERFLOW_JS = """(() => {
  const vw = document.documentElement.clientWidth, bad = [];
  const clipped = el => { for (let p = el.parentElement; p && p !== document.body && p !== document.documentElement; p = p.parentElement) {
    const o = getComputedStyle(p).overflowX; if (o !== 'visible') return true; } return false; };
  for (const el of document.querySelectorAll('body *')) {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.right <= vw + 0.5) continue;
    if (getComputedStyle(el).position === 'fixed') continue;
    if (clipped(el)) continue;
    bad.push(el.tagName.toLowerCase() + (el.className && typeof el.className === 'string' ? '.' + el.className.split(' ')[0] : '') + ' right=' + Math.round(r.right));
  }
  return bad.slice(0, 8);
})()"""

FOCUS_JS = """(() => { const e = document.activeElement; if (!e || e === document.body) return null;
  const r = e.getBoundingClientRect(), s = getComputedStyle(e);
  return { tag: e.tagName.toLowerCase(), text: (e.textContent || e.getAttribute('aria-label') || '').trim().slice(0, 28),
    outline: s.outlineStyle + ' ' + s.outlineWidth, width: parseFloat(s.outlineWidth), style: s.outlineStyle,
    visible: r.width > 0 && r.height > 0 && s.visibility !== 'hidden', inView: r.bottom > 0 && r.top < innerHeight }; })()"""


COLUMN_JS = """(() => { const vw = document.documentElement.clientWidth;
  const gut = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--gut')) || 0;
  const out = [];
  for (const el of document.querySelectorAll('h1, h2, h3, p, li, figcaption, summary, dt, dd')) {
    if (el.closest('pre, [aria-hidden=true], .term, .crew, .bar, .results, nav')) continue;
    const rg = document.createRange(); rg.selectNodeContents(el);
    const t = rg.getBoundingClientRect();
    if (!t.width) continue;
    if (t.right > vw - gut + 1.5 || t.left < gut - 1.5)
      out.push(el.tagName.toLowerCase() + ' "' + el.textContent.trim().slice(0, 24) + '" ' + Math.round(t.left) + ' to ' + Math.round(t.right));
  }
  return out; })()"""


def static_checks(page, name, width, all_ids):
    path = PAGES[name]
    page.goto(path, settle=0.5)
    tag = f"{name}@{width}"

    # anchors
    here = all_ids[name]
    hrefs = page.eval("[...document.querySelectorAll('a[href]')].map(a => a.getAttribute('href'))")
    missing = []
    for h in hrefs:
        if h.startswith(("http://", "https://", "mailto:")):
            continue
        base, _, frag = h.partition("#")
        target = {"": name, "index.html": "landing", "docs.html": "docs"}.get(base)
        if base and target is None:      # a local file such as marks/avatar.svg
            if not os.path.exists(os.path.join(ROOT, base)):
                missing.append(h)
        elif frag and frag not in all_ids[target]:
            missing.append(h)
    record(f"anchors   {tag}", not missing, f"{len(hrefs)} links" + (f", unresolved: {missing[:6]}" if missing else ""))

    # images
    bad = page.eval("[...document.images].filter(i => !i.hasAttribute('alt') || !i.complete || i.naturalWidth === 0).map(i => i.getAttribute('src'))")
    n = page.eval("document.images.length")
    record(f"images    {tag}", not bad, f"{n} images" + (f", bad: {bad[:6]}" if bad else ""))

    # overflow
    over = page.eval(OVERFLOW_JS)
    record(f"overflow  {tag}", not over, ", ".join(over))

    # column: headings and reading text stay inside the page gutters (overflow above only trips at the window's edge)
    out = page.eval(COLUMN_JS)
    record(f"column    {tag} text stays inside the page gutters", not out, "; ".join(out[:4]))

    # the crew stage sits inside a clipping section, so overflow there would hide crew without tripping the check above
    if name == "landing":
        crew = page.eval("""(() => { const vw = document.documentElement.clientWidth;
          const li = [...document.querySelectorAll('.crew > li')].filter(l => getComputedStyle(l).display !== 'none');
          return { shown: li.length, members: li.filter(l => l.classList.contains('member')).length,
                   out: li.filter(l => { const r = l.getBoundingClientRect(); return r.right > vw + 0.5 || r.left < -0.5; }).length }; })()""")
        record(f"crew      {tag} all five crew on screen", crew["members"] == 5 and crew["out"] == 0, f"{crew['members']} crew shown, {crew['out']} outside the window")


def motion_checks(name):
    for reduced in (True, False):
        page = Page(1440, reduced_motion=reduced)
        try:
            page.goto(PAGES[name], settle=1.0)
            n = page.eval("document.getAnimations().length")
            looks = page.eval("matchMedia('(prefers-reduced-motion: reduce)').matches")
            if reduced:
                record(f"motion    {name} reduced", n == 0 and looks, f"{n} running animations")
            else:
                # the docs page has no ambient motion by design, so only the landing must be moving
                record(f"motion    {name} normal", n > 0 or name == "docs", f"{n} running animations" + (" (none by design)" if name == "docs" else ""))
        finally:
            page.close()


def keyboard_checks(name, width):
    page = Page(width)
    try:
        page.goto(PAGES[name], settle=0.5)
        stops, problems = [], []
        for i in range(26):
            page.key("Tab")
            f = page.eval(FOCUS_JS)
            if not f:
                problems.append(f"stop {i + 1}: nothing focused")
                continue
            stops.append(f["tag"])
            if not f["visible"] or f["style"] == "none" or f["width"] < 2 or not f["inView"]:
                problems.append(f"stop {i + 1} {f['tag']} '{f['text']}': outline {f['outline']}, visible {f['visible']}, in view {f['inView']}")
        record(f"keyboard  {name}@{width} tab order", not problems and len(stops) == 26, f"26 stops, first: {stops[:6]}" + (f"; {problems[:3]}" if problems else ""))

        if name == "landing":
            page.goto(PAGES[name], settle=0.5)
            page.eval("document.querySelector('.places summary').focus()")
            page.key("Enter")
            opened = page.eval("document.querySelector('.places').open")
            page.key("Escape")
            closed = not page.eval("document.querySelector('.places').open")
            back = page.eval("document.activeElement === document.querySelector('.places summary')")
            record(f"keyboard  landing@{width} places menu", opened and closed and back, f"opened {opened}, closed on Esc {closed}, focus returned {back}")
            # choosing a place from the menu goes there and closes it
            page.eval("document.querySelector('.places summary').focus()")
            page.key("Enter")
            page.eval("document.querySelector('.places a[href=\"#below-deck\"]').focus()")
            page.key("Enter")
            time.sleep(0.5)
            y = page.eval("Math.round(document.getElementById('below-deck').getBoundingClientRect().top)")
            shut = not page.eval("document.querySelector('.places').open")
            record(f"keyboard  landing@{width} choose a place", shut and abs(y) < 140, f"menu closed {shut}, section top at {y}px under the bar")
    finally:
        page.close()


def landing_behaviour(width):
    page = Page(width)
    try:
        page.goto(PAGES["landing"], settle=0.5)
        first = page.eval("document.querySelector('[data-here]').textContent")
        seen = {}
        for target in ("deck", "waterline", "below-deck", "hold", "seabed"):
            page.eval(f"document.getElementById('{target}').scrollIntoView()")
            time.sleep(0.5)
            seen[target] = page.eval("document.querySelector('[data-here]').textContent")
        want = {"deck": "deck", "waterline": "waterline", "below-deck": "below deck", "hold": "the hold", "seabed": "seabed"}
        ok = all(seen[k].startswith(v) for k, v in want.items())
        current = page.eval("document.querySelector('.places [aria-current]')?.getAttribute('href')")
        record(f"places    landing@{width} follows the scroll", ok and current == "#seabed", f"top '{first}', then {json.dumps(seen)}")

        page.eval("window.scrollTo(0, 0)")
        page.eval("document.querySelector('.floor .room').scrollIntoView({block: 'center'})")
        time.sleep(0.7)
        looking = page.eval("document.querySelectorAll('.room.looking, .nest.looking').length")
        record(f"glance    landing@{width} crew looks up", looking >= 1, f"{looking} room(s) looking")
    finally:
        page.close()


def copy_check(name, width):
    page = Page(width)
    try:
        page.grant_clipboard()
        page.goto(PAGES[name], settle=0.5)
        info = page.eval("[...document.querySelectorAll('.copy')].map(b => ({ok: !!b.dataset.copy && b.dataset.copy.length > 5, label: b.textContent}))")
        bad = [i for i in info if not i["ok"]]
        page.eval("document.querySelector('.copy').scrollIntoView({block: 'center'})")
        r = page.eval("(() => { const r = document.querySelector('.copy').getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()")
        page.click(*r)
        time.sleep(0.4)
        label = page.eval("document.querySelector('.copy').textContent")
        want = page.eval("document.querySelector('.copy').dataset.copy")
        try:
            got = page.eval("navigator.clipboard.readText()", await_promise=True)
        except Exception as e:   # headless Chrome may refuse a read even with permission
            got = None
            note = f"clipboard read refused ({str(e)[:50]})"
        else:
            note = ""
        ok = label == "copied" and (got is None or got == want)
        record(f"copy      {name}@{width}", ok and not bad,
               f"{len(info)} buttons, all with data-copy {not bad}; click -> label '{label}'" + (f", clipboard matches {got == want}" if got is not None else f", {note}"))
    finally:
        page.close()


def search_checks(width):
    page = Page(width)
    try:
        page.goto(PAGES["docs"], settle=0.5)
        page.eval("document.activeElement && document.activeElement.blur()")
        page.key("/")
        focused = page.eval("document.activeElement && document.activeElement.id")
        page.type_text("oneshot")
        time.sleep(0.2)
        n = page.eval("document.querySelectorAll('#results a').length")
        shown = page.eval("!document.getElementById('results').hidden")
        first = page.eval("document.querySelector('#results a')?.getAttribute('href')")
        marked = page.eval("document.querySelectorAll('#results mark').length")
        record(f"search    docs@{width} finds text", focused == "q" and shown and n >= 1 and marked >= 1, f"/ focused '{focused}', {n} results, first -> {first}")
        page.key("Enter")
        time.sleep(0.4)
        h = page.eval("location.hash")
        hidden = page.eval("document.getElementById('results').hidden")
        record(f"search    docs@{width} Enter follows the first result", h == first and hidden, f"hash {h}, panel hidden {hidden}")
        page.eval("document.getElementById('q').value = ''; document.getElementById('q').focus()")
        page.type_text("zzqx")
        time.sleep(0.2)
        empty = page.eval("!!document.querySelector('#results .empty')")
        page.key("Escape")
        cleared = page.eval("document.getElementById('q').value === '' && document.getElementById('results').hidden")
        record(f"search    docs@{width} no match, Esc clears", empty and cleared, f"empty message {empty}, cleared {cleared}")
        # arrow keys move through results
        page.eval("document.getElementById('q').focus()")
        page.type_text("keys")
        time.sleep(0.2)
        page.key("ArrowDown")
        on_result = page.eval("document.activeElement.closest('#results') !== null")
        page.key("ArrowUp")
        back = page.eval("document.activeElement.id === 'q'")
        record(f"search    docs@{width} arrow keys", on_result and back, f"down lands in results {on_result}, up returns to input {back}")
        # the on-page contents follow the scroll (wide layout only)
        if width >= 1200:
            page.eval("document.getElementById('menus').scrollIntoView()")
            time.sleep(0.5)
            cur = page.eval("document.querySelector('.toc a[aria-current]')?.getAttribute('href')")
            record(f"toc       docs@{width} follows the scroll", cur == "#menus", f"current {cur}")
    finally:
        page.close()


LOOPS_JS = """[...document.querySelectorAll('.act')].map(el => {
  const s = getComputedStyle(el), v = n => parseFloat(s.getPropertyValue(n));
  const r = el.getBoundingClientRect();
  return { cls: el.className.split(' ').filter(c => c.startsWith('a-'))[0], w: v('--w'), h: v('--h'), n: v('--n'),
    img: (s.backgroundImage.match(/url\\("?([^")]+)"?\\)/) || [])[1], name: s.animationName, dur: s.animationDuration,
    ease: s.animationTimingFunction, bx: s.backgroundPositionX, boxw: r.width, boxh: r.height,
    running: el.getAnimations().length, off: el.classList.contains('off'), px: v('--px') };
})"""


def loop_checks(name, width):
    """The loops are sheets of n frames: the CSS and the drawing must agree, and each loop must run at its own pace."""
    page = Page(width)
    try:
        page.goto(PAGES[name], settle=0.8)
        rows = page.eval(LOOPS_JS)
        want = 12 if name == "landing" else 3
        bad = []
        for r in rows:
            path = os.path.normpath(os.path.join(ROOT, "art", os.path.basename(r["img"] or "")))
            m = re.search(r'viewBox="0 0 (\d+) (\d+)"', open(path).read())
            vw, vh = int(m[1]), int(m[2])
            if (vw, vh) != (int(r["n"] * r["w"]), int(r["h"])):
                bad.append(f"{r['cls']} sheet {vw}x{vh} is not {int(r['n'])} frames of {int(r['w'])}x{int(r['h'])}")
            if r["name"] != "act" or r["ease"] != f"steps({int(r['n'])})":
                bad.append(f"{r['cls']} animation {r['name']} {r['ease']}")
            if abs(r["boxw"] - r["w"] * r["px"]) > 0.5 or abs(r["boxh"] - r["h"] * r["px"]) > 0.5:
                bad.append(f"{r['cls']} box {r['boxw']}x{r['boxh']} is not whole art pixels")
        durs = [r["dur"] for r in rows]
        record(f"loops     {name}@{width} {len(rows)} loops, sheets match their frames", len(rows) == want and not bad,
               "; ".join(bad[:4]) if bad else f"{len(rows)} loops, steps()-stepped, whole art pixels")
        record(f"loops     {name}@{width} each loop has its own pace", len(set(durs)) == len(durs), ", ".join(sorted(set(durs)))[:120])

        if name == "landing":
            # the deck crew keep their state: the plate under each shows the glyph and the word the loop acts out
            plates = page.eval("""[...document.querySelectorAll('.crew .member')].map(li => ({ cls: [...li.querySelector('.act').classList].find(c => c.startsWith('a-')),
              glyph: li.querySelector('.plate .g')?.textContent, text: li.querySelector('.plate')?.textContent.trim() }))""")
            mapping = {"a-error": ("✖", "error"), "a-blocked": ("▲", "blocked"), "a-working": ("●", "working"), "a-unseen": ("◆", "unseen"), "a-idle": ("○", "idle")}
            ok = len(plates) == 5 and all(p["cls"] in mapping and p["glyph"] == mapping[p["cls"]][0] and mapping[p["cls"]][1] in p["text"] for p in plates)
            record(f"loops     landing@{width} deck crew keep glyph, word and loop together", ok, ", ".join(f"{p['cls']}:{p['glyph']}" for p in plates))
            # a loop off screen is paused; one in view runs
            page.eval("window.scrollTo(0, 0)")
            time.sleep(0.4)
            top = page.eval("document.querySelector('.a-spyglass').classList.contains('off')")
            bottom_off = page.eval("document.querySelector('.a-diver').classList.contains('off')")
            page.eval("document.querySelector('.a-working').scrollIntoView({block: 'center'})")
            time.sleep(0.5)
            near = page.eval("!document.querySelector('.a-working').classList.contains('off')")
            paused = page.eval("document.querySelector('.a-working').getAnimations()[0].playState")
            record(f"loops     landing@{width} off-screen loops pause", (not top) and bottom_off and near and paused == "running",
                   f"spyglass at the top off {top}, diver far below off {bottom_off}, deck crew in view running {near} ({paused})")
        # with reduced motion nothing runs and every loop shows frame 0
        page2 = Page(width, reduced_motion=True)
        try:
            page2.goto(PAGES[name], settle=0.8)
            st = page2.eval("[...document.querySelectorAll('.act')].map(el => [getComputedStyle(el).animationName, getComputedStyle(el).backgroundPositionX, el.getAnimations().length])")
            ok = all(a == "none" and x in ("0px", "0%") and n == 0 for a, x, n in st)
            record(f"loops     {name}@{width} reduced motion: still, on frame 0", ok and len(st) == want, f"{len(st)} loops, {sum(n for _, _, n in st)} running")
        finally:
            page2.close()
    finally:
        page.close()


def art_checks():
    """The deck crew's loops, read from the generators: every frame keeps the mood's face (its colour, no neutral white)
    on the screen, and the first frame of every loop is the still pose the page shows with motion off."""
    import loops as L
    from crew import FACES, MOODS, SCREEN, crewmate
    problems = []
    moods = {"crew-lookout-error-loop": ("lookout", "error"), "crew-navigator-blocked-loop": ("navigator", "blocked"),
             "crew-deckhand-working-loop": ("deckhand", "working"), "crew-cook-unseen-loop": ("cook", "unseen"),
             "crew-captain-idle-loop": ("captain", "idle")}
    for name, (hat, mood) in moods.items():
        frames = L.DECK[name]()
        key = MOODS[mood][1]
        sx, sy = L.OX + SCREEN[0], L.OY + SCREEN[1]
        for i, f in enumerate(frames):
            # the screen, allowing for a sideways pace or lean of up to three pixels
            if key not in {f.px.get((x, y)) for dx in range(-3, 4) for x in range(sx + dx, sx + dx + 10) for y in range(sy, sy + 6)}:
                problems.append(f"{name} frame {i} has no {mood} colour on its screen")
        still = crewmate(hat, mood)
        f0 = frames[0]
        # frame 0 is the still pose, apart from what a loop adds around it (smoke, a bang, a sparkle, a lifted arm)
        shown = {(x - L.OX, y - L.OY): k for (x, y), k in f0.px.items()}
        body = {p: k for p, k in still.px.items() if 6 <= p[0] < 22 and 5 <= p[1] < 18}      # the head and body box
        if any(shown.get(p) != k for p, k in body.items()):
            problems.append(f"{name} frame 0 differs from the still {mood} body")
    record("art       deck loops keep their mood colour in every frame, frame 0 is the still body", not problems, "; ".join(problems[:3]) if problems else "5 loops")
    # the diver's bubbles rise beside the hose (art columns 12 and 13 in the rows above the diver), never over it
    clash = [i for i, f in enumerate(L.diver_loop()) if any(12 - 1 <= x <= 13 + 1 and y < L.DY for (x, y) in f.px)]
    record("art       diver bubbles stay clear of the hose", not clash, f"frames {clash} touch it" if clash else "8 frames, one column of air either side")


def edge_checks(name, width):
    """The bar's content lines up with the page's column, and the sticky bar stays compact on a phone."""
    page = Page(width)
    try:
        page.goto(PAGES[name], settle=0.5)
        info = page.eval("""(() => { const l = s => Math.round(document.querySelector(s).getBoundingClientRect().left);
          const right = s => Math.round(document.querySelector(s).getBoundingClientRect().right);
          return { home: l('.bar .home'), page: l(%s), bar: Math.round(document.querySelector('.bar').getBoundingClientRect().height),
                   barRight: right(%s), pageRight: right(%s) }; })()""" % (
            ("'.hero-text'" if name == "landing" else "'.docs-grid'"), ("'.places'" if name == "landing" else "'.search'"),
            ("'.hero'" if name == "landing" else "'.docs-grid'")))
        record(f"edges     {name}@{width} bar lines up with the page column", abs(info["home"] - info["page"]) <= 1 and abs(info["barRight"] - info["pageRight"]) <= 1,
               f"bar logo at {info['home']}, page at {info['page']}; bar control ends {info['barRight']}, page ends {info['pageRight']}")
        if width <= 600:
            record(f"edges     {name}@{width} sticky bar stays compact", info["bar"] <= 84, f"{info['bar']}px tall")
    finally:
        page.close()


SCALE_JS = """(() => {
  const out = {};
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let t; (t = walker.nextNode());) {
    if (!t.textContent.trim()) continue;
    const el = t.parentElement;
    if (el.closest('script, style, [aria-hidden="true"], .sr-only, .slop, .eyes')) continue;
    if (el.closest('details:not([open])') && !el.closest('summary')) continue;
    const s = getComputedStyle(el);
    if (s.display === 'none' || s.visibility === 'hidden') continue;
    const fam = /Departure/.test(s.fontFamily) ? 'pixel' : (/Atkinson|system-ui/.test(s.fontFamily) && !/ui-monospace/.test(s.fontFamily.split(',')[0]) ? 'read' : 'code');
    // inline code is 0.88em of its sentence and keeps that relation by design
    if (el.tagName === 'CODE' && !el.closest('.term, .bar')) continue;
    const key = fam + ' ' + parseFloat(s.fontSize);
    (out[key] = out[key] || []).push((el.className && typeof el.className === 'string' ? el.className.split(' ')[0] : el.tagName.toLowerCase()));
  }
  return Object.fromEntries(Object.entries(out).map(([k, v]) => [k, [...new Set(v)].slice(0, 4)]));
})()"""


def scale_checks(name, width):
    page = Page(width)
    try:
        page.goto(PAGES[name], settle=0.5)
        page.eval("document.querySelectorAll('details').forEach(d => d.open = true)")      # the closed contents hide text too
        sizes = page.eval(SCALE_JS)
        reading, code = {14, 16, 18, 20, 22}, {13, 14}
        bad = []
        for key, who in sizes.items():
            fam, px = key.split(" ")
            px = float(px)
            if fam == "pixel":
                ok = px % 11 == 0
            elif fam == "read":
                ok = px in reading
            else:
                ok = px in code
            if not ok:
                bad.append(f"{key} ({', '.join(who)})")
        record(f"scale     {name}@{width} text sits on the type scale", not bad, "; ".join(bad[:5]) if bad else f"{len(sizes)} sizes in use")
    finally:
        page.close()


def main():
    widths = [int(a) for a in sys.argv[1:]] or [1440, 1100, 768, 390]
    probe = Page(1440)
    try:
        all_ids = {n: ids_of(probe, p) for n, p in PAGES.items()}
    finally:
        probe.close()
    for width in widths:
        page = Page(width)
        try:
            for name in PAGES:
                static_checks(page, name, width, all_ids)
        finally:
            page.close()
    motion_checks("landing")
    motion_checks("docs")
    for width in (1440, 390):
        for name in PAGES:
            keyboard_checks(name, width)
        landing_behaviour(width)
        copy_check("landing", width)
        copy_check("docs", width)
    search_checks(1440)
    search_checks(390)
    for width in (1440, 390):
        for name in PAGES:
            loop_checks(name, width)
            edge_checks(name, width)
            scale_checks(name, width)
    art_checks()
    failed = results.count(False)
    print(f"\n{len(results) - failed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


main()
