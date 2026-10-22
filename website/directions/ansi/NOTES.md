# ansi: the ship drawn in terminal material

## Summary for Cyan

The `ansi` direction is finished and polished. The landing page is the whole cross-section (sky, deck, waterline, below deck, the hold, seabed), there is a real docs page, and both work from 1440px down to 390px. Everything is drawn on one pixel grid in "Ship 16", the site's own tuning of the 16 ANSI colours. Open `index.html`, `docs.html`, `moods.html` and `marks.html` straight from the folder, no build step. `art.html` is a sheet of every drawing.

The polish pass did four things.

- The crew moves. Every crewmate has a stepped loop of its own, so nothing animates in time with anything else. The five deck crew keep their state face and their plate in every frame, the five cutaway rooms are small scenes, and the nest lookout, the diver and the three docs note crew have loops too. Everything stops on its first frame under `prefers-reduced-motion`, and a loop pauses while it is off screen.
- Text sits on one type scale and the page has one set of edges. The sticky bar lines up with the page column, and at 390px it is 81px tall where it was 98px.
- Small things got fixed along the way: the deck's ten items are a ruled ledger, the hold has less dead space, long lines scroll with a dithered hint, the copy button is a keycap that says when it worked, and external links in the docs sidebar carry a small arrow.
- Three layout bugs I found by reading the screenshots: the tabs sketch cut "claude blocked" in half at every width, that sketch's panes cut their words between 1100 and 1360px, and "the ocean of slop" ran 7px past the column on a phone.

Decisions that are yours, still open and untouched: the interactive demo and the server recording hook, `ship --skill` and binary-printed keys, the art license and whether AI-made art is acceptable, and the real domain (`shipmux.dev` is a placeholder). The two choices you reserved are kept: crew bodies are told apart by hats with colour meaning state only, and the hull is ANSI yellow. The hero copy, the section order and the three marks are unchanged.

Two things to look at with your own eyes: how the loops feel live (I judged them from paused frames, not by watching them run), and the docs search, which is a stand-in that only searches the page you are on.

## What is where

- `index.html`, `css/site.css` (tokens, type scale, terminals, bar, the loop mechanism, shared pieces), `css/landing.css`, `js/site.js` (copy buttons, places menu, scroll spy, the glance, pausing loops off screen).
- `docs.html`, `css/docs.css`, `js/docs.js` (search, `/` shortcut, on-page contents).
- `art/` has 125 generated drawings and `tools/` generates them: `python3 tools/build.py`. Never edit an SVG in `art/` by hand. The marks have their own command, `python3 tools/marks.py`, and were not touched.
- Generators: `tools/crew.py` (crewmates, poses, the knobs a loop turns), `tools/loops.py` (every crew loop sheet), `tools/rooms.py` (the five rooms, each a sheet), `tools/scene.py` and `tools/scenery.py` (the nest, the ship, the seabed, the link arrow).
- `tools/sheet.py` writes `art.html`. `tools/shot.py` takes true full-page screenshots through DevTools. `tools/strips.py` cuts a tall PNG into strips for reading. `tools/check.py` is the browser check. `tools/contrast.py` lists every colour pair the pages use with pass or fail.
- `screenshots/`: `landing-1440.png` (1440 x 8399), `landing-390.png` (390 x 15666), `docs-1440.png`, `docs-390.png`, plus `mood-sheet.png` and `marks-test-sheet.png`. The real footer is the last rows of each page shot.

## The landing, in order

- Sky: the locked hero, the install command with a copy button, the lookout's agent list in Ship's real order and glyphs, clouds and gulls.
- Deck: ten things that work today in the README's words, set as a ruled ledger with a pixel-face name on the left and the text on the right. Then the "not built yet" box and the crew stage. The five crew map to the five states (lookout error, navigator blocked, deckhand working, cook unseen, captain idle). That mapping is mine.
- Waterline: the ocean of slop (the drifting phrases are aria-hidden decoration), the two paragraphs, and the dashed "placeholder, not a recording" frame for the VHS. The sketch inside is labelled hand drawn.
- Below deck: four lit rooms (galley is the server, chart is tabs and panes, telegraph is the CLI, handoff is agents driving agents), each beside real copy and a real command or a diagram in the terminal's own boxes.
- The hold: install with the build needs and the three links into the docs, the 16 default keys, config. A cargo room sits beside the heading. Hatch, ladder and lantern show from 1360px up, where there is free space.
- Seabed: "run a tight ship" and the install command, then the reef and the diver.
- Every place has an anchor, a plain label next to the nautical one (`deck` and "what works today"), and an entry in the places menu in the bar.

## The loops

Each loop is one SVG sheet with a frame per step, played with `steps()`. Frame 0 is always the still pose, so the page reads the same with motion off. Each loop has its own frame count and pace, and `check.py` keeps the CSS numbers and the sheets in agreement.

Deck crew (36 x 28):

- Lookout, error, 6 frames: smoke puffs drift up and right, the arms flail, two sideways jolts. The face stays the magenta error face.
- Navigator, blocked, 8 frames: the raised hand waves a pixel either way, the right foot taps, a red `!` blinks over the hand. The face stays the red blocked face.
- Deckhand, working, 8 frames: swabbing. The mop head slides out and back, the feet shuffle, the body leans a pixel, water drips on two frames.
- Cook, unseen, 10 frames: holds out the parcel and lifts it a row, a cyan sparkle grows from one pixel to a plus and the parcel flashes bright yellow.
- Captain, idle, 41 frames: hands behind the head, three steps right, back, three left, back, with a foot lifting on each step and rests between legs.

Rooms and props:

- Galley, 4 frames: the cook stirs, the fire flickers, the soup bubbles.
- Chart, 8 frames: the navigator taps panes with a pointer stick and the tapped pane takes the cyan selected border, as in the page's own sketch.
- Telegraph, 16 frames: the lookout pulls the lever down to hand height and the needle jumps between three marks.
- Handoff, 12 frames: the note shuttles between the captain's hand and the deckhand's.
- Hold, 12 frames: the cook ticks the clipboard line by line and the lantern sways on its rope.
- Nest spyglass, 14 frames: sweeps level, down toward the agent list, level, up toward the horizon, with a glint at the ends.
- Diver, 8 frames: waves and breathes out bubbles that stay clear of the hose.

Docs notes (28 x 24, mostly at rest and then a short burst): the deckhand writes lines on the clipboard, the navigator taps toward the text, the cook beckons.

Heads never move in room frames, so the blink and glance patches still line up.

## The docs page, and what it does not do

Source is `docs/guide.md` Configuration, lowercased in the site voice, code identical to the guide. Relative guide links became GitHub URLs. Three sentences from the guide are set as notes with a neutral crewmate (your file only needs what you change, open windows pick up changes and `ship config check` points at the first mistake, server settings apply to new panes). They replace those paragraphs rather than repeating them. Nothing from "Not built yet" is claimed.

- Only Configuration is on the site. The other ten guide sections in the left list open the guide on GitHub, with an arrow, and the page says so.
- Search is provisional. It searches the text of the open page only (every word must match), opens with `/`, shows up to eight matches with links, and says plainly when there are none. Real docs search needs an index built when the site is published, and the static generator is not chosen.
- `docs.html` is static HTML written from a throwaway script that is not kept, so edit it by hand. There is no versioning (`/docs/vX/`), because there is no generator yet.
- The three note crewmates are the only motion on the docs page. To take it out, turn the three `.a-clipboard`, `.a-point` and `.a-reach` spans back into `<img>` tags.
- At 390px the contents list folds into a `<details>` with the terminal's own fold marks.

## Type scale

Reading text uses 14, 16, 18, 20 and 22px. Code uses 13 and 14px. The pixel face uses multiples of 11 (11, 22, 33, 44, 66) so Departure Mono lands on its grid. The tokens are `--t-sm`, `--t-md`, `--t-base`, `--t-lg`, `--t-xl`, `--code-sm` and `--code-md` in `css/site.css`, and `check.py` fails on any text node outside the scale. Text that used to be 17px is now 18px and 19px is now 20px, so some columns got a little taller.

## Weak spots from the first pass, and where they ended up

- The empty block in the hold's left column is reduced, not removed. At 1440 the install column is 696px tall and keys 903px, a gap of about 207px (it was 364px); at 1280 the gap is about 86px. I moved the three links into the install column and gave keys more width. A sticky install column was tried and dropped, because Chrome kept it inside the whole grid and it slid over the config block. Removing the gap fully needs content moved or new art, so I left it for you.
- Text sizes: now one scale, above.
- The deck's ten items: a ledger, the README's words untouched. It makes the deck about 300px taller.
- The sticky bar at 390px: 81px on the landing and 79px on the docs, from 98px. `check.py` allows 84.
- Ambient motion is plentiful, and now more so. It all stops under reduced motion.

## Evidence

- `python3 tools/check.py`: 93 checks, all pass, at 1440, 1100, 768 and 390, about 95 seconds. It covers links, images, overflow, text inside the page gutters, keyboard, reduced motion, the copy buttons, the places menu, docs search, and for the loops: every sheet is exactly n frames of the size the CSS says, `steps(n)` timing, whole art pixels, a distinct pace for each, glyph and word match the state, off-screen loops pause, reduced motion shows frame 0. The art checks read the generators: every deck loop frame keeps its mood colour on the screen, frame 0 matches the still body, and the diver's bubbles stay clear of the hose.
- `python3 tools/contrast.py`: 70 of 70 pairs pass.
- Every loop was viewed in Chrome with the animation paused on each frame, in page context, then as a sheet.
- Screenshots were re-shot after the last edit and read in strips at full size: landing 1440 (10 strips), landing 390 (18), docs 1440 (4), docs 390 (6). Each ends on a flat footer colour, so none is a blank band. I also read landing at 1100 and 768, and the tabs sketch at 1440, 1280, 1150, 1100 and 768.
- `moods.html` and `marks.html` render the same as before the pass, and `marks/` and `tools/marks.py` have no diff.
- Not verified: Safari and Firefox, a screen reader, real touch devices, Linux (all checks ran in headless Chrome on macOS), and the loops watched live rather than as stills. The fonts load from CDNs, so screenshots taken offline would show fallbacks.
- Known and left: at 320px the hero headline runs about 14px past the column because its three lines are locked with no-wrap, and at 360px a long inline code chip in the docs can end about 3px past it. Neither width is one the checks run at.

## Uncertain decisions

- Lit tan rooms inside the dark hull, as the answer to ink outlines vanishing on dark. Low to medium confidence on the look. The diver's bright blue outline is a second, separate answer.
- The `diver` hat was added to the cast for the seabed. Crew names stay role names only.
- The crew to state mapping, the stage plates, the crew to room mapping and the mast spanning two sections are my composition choices inside the fixed layout.
- The places menu (a `<details>` dropdown listing all six places, nautical word plus plain label) replaces the two inline links the cut-down version had.
- The ledger replaced the two-column list for the deck. It is presentation only and it is taller.
- The bar now aligns to the 1328px page column, so the logo sits at 56px at 1440 where it was 28px.
- The link arrow is a 5 x 5 sprite shown at 10px, which is off the page's 4px grid because it is a link mark and not a sprite.
- Docs now has motion, three note crewmates, although the earlier pass kept the docs still on purpose. This follows your request that crew come alive.
- The red `!` over the blocked navigator and the sparkle over the cook are extra drawn marks. The glyph and plate still carry the state.
- Every `--n` frame count in the CSS is hard-coded and must match its generator. `check.py` enforces it.
- Inside the hold, the three docs links now sit between install and keys in the single-column layout. DOM order of the three groups is unchanged.
- Inline `code` now breaks anywhere when a path is longer than the column, site-wide. The waterline heading wraps below 600px instead of holding "the ocean of slop" on one line. The tabs sketch stacks its three panes when its box is narrower than 450px, which changes how it looks between 1100 and 1360px.
- Docs: which three sentences became notes, which neutral crewmate stands by each, the search stand-in, the plain bar (no places menu, a search box instead), and nav links `install`, `keys` and `config` that go to the landing's hold rather than staying in the docs.
- Docs background is `#fbf8f1`, which the detector reads as cream. It is the palette's bright white, shared with the marks.
- The hatch, ladder and lantern appear only from 1360px.
- Fonts still come from CDNs, and Departure Mono is unpinned (`@main`). Pin it before shipping.
- Shared CSS changed earlier: `.cmd` rules moved into `css/site.css`, `.term` has a 2px border, `code` and `kbd` are styled. `moods.html` got `border:0` on `.tag` to stay identical.
- Not settled and untouched: the interactive demo and recording hook, `ship --skill` and binary-printed keys, the art license and AI-made art, and the real domain (links point at GitHub, `shipmux.dev` is a placeholder).
