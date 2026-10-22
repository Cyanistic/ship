"""WCAG contrast for the colour pairs the pages use.

usage: python3 tools/contrast.py          the pairs the pages use, with pass or fail
       python3 tools/contrast.py matrix   every palette colour on every ground, for picking new pairs

A pair needs 4.5 for text, 3 for large text and for focus rings and other non-text parts.
Decorative things (the slop rows, dividers, art) are not listed: they are aria-hidden or carry no meaning.
"""
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from pixel import PALETTE as P


def lum(h):
    h = h.lstrip('#'); r, g, b = (int(h[i:i+2], 16) / 255 for i in (0, 2, 4))
    f = lambda c: c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)


def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def mix(fg, bg, a):
    """fg at opacity a over bg, as a hex colour (for the 13% tint behind inline code)."""
    f = [int(fg.lstrip('#')[i:i+2], 16) for i in (0, 2, 4)]
    b = [int(bg.lstrip('#')[i:i+2], 16) for i in (0, 2, 4)]
    return '#%02x%02x%02x' % tuple(round(a * x + (1 - a) * y) for x, y in zip(f, b))


GROUNDS = {
    'ink': P['0'], 'paper': P['f'], 'sky-high': '#a9d3ea', 'sky': '#cfe7f3', 'haze': '#e4f2f8',
    'sea': '#1f4fa8', 'deep': '#14306b', 'wall': '#252232', 'hold': '#1c1a27', 'seabed': '#0f1a38',
    'room': P['7'], 'footer': P['8'], 'keycap': '#2b2840',
}
EXTRA = {'muted-light': '#4a4660', 'muted-dark': '#b9b3c9', 'sea-text': '#1f4fa8'}

# (where, foreground, ground, minimum). Text is 4.5; a focus ring or a large pixel heading is 3.
INK, PAPER, MUTED_L, MUTED_D, SEA = P['0'], P['f'], EXTRA['muted-light'], EXTRA['muted-dark'], EXTRA['sea-text']
G = GROUNDS
USED = [
    # the bar, on every page
    ('bar link', P['7'], G['ink'], 4.5), ('bar link, current or hover', P['f'], G['ink'], 4.5),
    ('places label', MUTED_D, G['ink'], 4.5), ('places place names', P['e'], G['ink'], 4.5),
    ('search placeholder', MUTED_D, G['keycap'], 4.5), ('search text', P['f'], G['keycap'], 4.5),
    ('search / key hint', P['7'], '#3a3650', 4.5),
    ('focus ring on the bar', P['e'], G['ink'], 3),
    # sky and deck (light)
    ('body on sky', INK, G['sky'], 4.5), ('body on haze', INK, G['haze'], 4.5), ('body on paper', INK, G['paper'], 4.5),
    ('muted on sky', MUTED_L, G['sky'], 4.5), ('muted on haze', MUTED_L, G['haze'], 4.5), ('muted on paper', MUTED_L, G['paper'], 4.5),
    ('place word on haze', SEA, G['haze'], 4.5), ('place word on sky', SEA, G['sky'], 4.5), ('docs links on paper', SEA, G['paper'], 4.5),
    ('inline code on haze', INK, mix(INK, G['haze'], .13), 4.5), ('inline code on paper', INK, mix(INK, G['paper'], .13), 4.5),
    ('docs banner', INK, G['sky-high'], 4.5), ('focus ring on light', INK, G['sky'], 3),
    ('fold mark on paper', SEA, G['paper'], 3), ('docs search / bullet marks on paper', SEA, G['paper'], 3),
    # terminals, everywhere
    ('terminal text', P['7'], G['ink'], 4.5), ('terminal dim text', MUTED_D, G['ink'], 4.5),
    ('state working', P['3'], G['ink'], 4.5), ('state blocked', P['1'], G['ink'], 4.5), ('state unseen', P['6'], G['ink'], 4.5),
    ('state idle', P['2'], G['ink'], 4.5), ('state error', P['5'], G['ink'], 3),
    ('toml string', P['a'], G['ink'], 4.5), ('toml key', P['c'], G['ink'], 4.5), ('toml table', P['b'], G['ink'], 4.5), ('toml number', P['d'], G['ink'], 4.5),
    ('stage plate', P['f'], G['ink'], 4.5), ('stage plate role', MUTED_D, G['ink'], 4.5),
    ('copy button', INK, P['6'], 4.5), ('copy button hover', INK, P['e'], 4.5), ('copy button, copied', INK, P['a'], 4.5),
    ('wide code focus ring', P['e'], G['ink'], 3),
    # waterline (sea)
    ('body on sea', P['f'], G['sea'], 4.5), ('place word on sea', P['e'], G['sea'], 4.5), ('why-ship chip', INK, P['e'], 4.5),
    ('placeholder label', INK, P['b'], 4.5), ('placeholder text', P['7'], G['ink'], 4.5),
    # below deck, hold, seabed (dark)
    ('body on wall', P['f'], G['wall'], 4.5), ('text on wall', P['7'], G['wall'], 4.5), ('muted on wall', MUTED_D, G['wall'], 4.5),
    ('place word on wall', P['e'], G['wall'], 4.5), ('inline code on wall', P['7'], mix(P['7'], G['wall'], .13), 4.5),
    ('text on hold', P['7'], G['hold'], 4.5), ('white on hold', P['f'], G['hold'], 4.5), ('link on hold', P['e'], G['hold'], 4.5),
    ('hold column heading', P['b'], G['hold'], 4.5), ('inline code on hold', P['7'], mix(P['7'], G['hold'], .13), 4.5),
    ('keycap', P['f'], G['keycap'], 4.5), ('keys table text', P['7'], G['hold'], 4.5),
    ('text on deep water', P['7'], G['deep'], 4.5), ('text on seabed', P['7'], G['seabed'], 4.5),
    ('link on deep water', P['e'], G['deep'], 4.5), ('link on seabed', P['e'], G['seabed'], 4.5), ('heading on seabed', P['f'], G['seabed'], 4.5),
    ('focus ring on dark', P['e'], G['hold'], 3), ('focus ring on sea', P['e'], G['sea'], 3),
    ('footer text', P['f'], G['footer'], 4.5),
    # docs search results
    ('result text', P['7'], G['ink'], 4.5), ('result heading', P['e'], G['ink'], 4.5), ('result highlight', INK, P['b'], 4.5),
    ('result note', MUTED_D, G['ink'], 4.5),
    # the skip link
    ('skip link', INK, P['b'], 4.5),
]


def used():
    bad = 0
    for where, fg, bg, need in USED:
        r = ratio(fg, bg)
        ok = r >= need
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {r:5.2f}  needs {need:<3} {where:28} {fg} on {bg}")
    print(f"\n{len(USED) - bad} of {len(USED)} pairs pass")
    return bad


def matrix():
    for bg, bv in GROUNDS.items():
        row = []
        for k, v in list(P.items()) + list(EXTRA.items()):
            r = ratio(v, bv)
            row.append(f"{k}:{r:4.1f}{'*' if r >= 4.5 else ('+' if r >= 3 else ' ')}")
        print(f"{bg:9}", ' '.join(row))


if __name__ == '__main__':
    if sys.argv[1:] == ['matrix']:
        matrix()
    else:
        sys.exit(1 if used() else 0)
