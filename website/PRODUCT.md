# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Astro with Starlight, built with pnpm. The site deploys as static files with no server, and docs search is Pagefind, which Starlight builds into the output. The docs are plain GitHub markdown in `docs/site/`, so they read the same on GitHub as on the site. Starlight's component overrides let the docs keep the ansi direction's markup and CSS exactly instead of approximating it with a theme.

Versions are one Astro build per git ref, merged into one output by `website/scripts/build-versions.mjs`: the landing page and site chrome from the current checkout, `/docs/` from the newest `v*` tag, `/docs/vX/` for each tag, and `/docs/main/` with the "unreleased" banner. Each version gets its own search index. The starlight-versions plugin was passed over because it keeps every version's pages in the working tree; building from tags keeps old docs exactly as released. `website/README.md` has the commands.

## Users

- **Curious developers** who run coding agents (Claude Code, Codex, Pi) and found Ship on GitHub or a link. They decide in a few seconds whether to keep reading. They already know tmux or Zellij, or have heard of Herdr.
- **Returning users** who know exactly what they want: the install command, a key, a config setting, a CLI flag. They should reach it in one or two moves without watching anything.
- **Coding agents** reading the docs on a user's behalf. They need plain text paths (llms.txt, a skill file) more than pages.

## Product Purpose

Ship is a small terminal workspace for running several coding agents at once. A background server keeps shells, editors and agents alive in tabs and split panes after the window closes, shows which agent is working, blocked, finished or idle, and lets agents open panes and hand work to each other through a JSON CLI.

The site exists to turn "oh that's pretty cool" into "let me actually try this", and then to get out of the way of people who came for the docs.

## Positioning

Ship covers most of Herdr's daily value with a much smaller codebase, because it rents terminal emulation (Ghostty), PTYs and transport from libraries and only writes the tab, pane and agent parts itself. The name treats agents as your crew and plays on shipping software.

Herdr (herdr.dev) is the reference. The goal is to go further than it on character and charm.

## Operating Context

Ship runs in a terminal on Linux and macOS (Windows best effort). Users install it with `cargo install` from source today; there are no releases yet. Default keys are Alt chords. Config is a TOML file that applies on save.

## Capabilities and Constraints

Use `README.md` as the source of truth for what works today and what isn't built. Never claim a feature from "Not built yet" (releases, history search, keyboard copy mode, resuming agents after restart, direct remote connections).

Agent display states, from `openspec/specs/agent-attention/spec.md`. Any crewmate mood must sit next to its glyph and colour, never replace it:

| State                                        | Glyph | Colour  |
| -------------------------------------------- | ----- | ------- |
| working                                      | `●`   | yellow  |
| blocked                                      | `▲`   | red     |
| unseen (finished while you looked elsewhere) | `◆`   | cyan    |
| idle                                         | `○`   | green   |
| error                                        | `✖`   | magenta |

Open decisions that stay with Cyan; do not settle them in the site:

- an interactive "play with it" demo that replays real server snapshots needs a recording hook in the server
- `ship skill` (a subcommand, tracked in shipmux/ship#61) and printing default keys from the binary are public CLI changes
- the real domain (use `shipmux.dev` as a placeholder)

## Brand Commitments

**Hero, locked:**

> **staying afloat in an ocean of slop**
>
> ship is a small terminal workspace that gives you and your coding agents somewhere steady to work while everyone else keeps shipping more slop.

The headline's layers are the point: everyone else ships so much slop it's an ocean, and Ship helps you ship while floating above it. Don't flatten it, don't merge it with other lines, and don't swap in a benefit-first headline.

**Closing call to action**, above the install command at the bottom: "run a tight ship".

**Voice:** dry, lowercase copy, with the crewmates supplying the warmth. Flowing sentences, not punchy staccato fragments. No hype, no em dashes, no marketing filler.

**Crew:** small terminal crewmates, named by role for now (lookout, deckhand, navigator, cook and so on); proper names come later. They have five mood poses, one per agent state above.

**Marks:** three, drawn from one model sheet. Static marks show identity, never a state.

- org avatar (shipmux): the ship alone
- app (Ship): the ship with its crew
- bot (the ship-crewmate GitHub bot): one neutral crewmate face

All three must read at 16px and in one colour.

**The ship as a place.** The site is themed as the ship and its surroundings. The landing page is one tall cross-section through the ship, read top to bottom with native scrolling, so going down the page is going down the ship. This layout is fixed; directions compete on everything else.

| Section | Place               | Content                                                     |
| ------- | ------------------- | ----------------------------------------------------------- |
| top     | sky and crow's nest | the hero; the lookout watching the five agent states        |
|         | deck                | what works today, the crew at work                          |
|         | waterline           | the ocean of slop, just below the ship                      |
|         | below deck          | how it works: the server, panes, CLI, agents driving agents |
|         | the hold            | install, keys, config, with links into the docs             |
| bottom  | seabed              | "run a tight ship" and the install command                  |

Each section is static content with its own anchor (for example `#install`). Liveliness comes from small ambient touches, such as the waterline moving or a crewmate glancing up as its section comes into view, never from scroll-driven sequences.

The docs sit outside the cross-section as plain, fast pages "below deck", themed only in small touches (header, crewmates beside notes). Elsewhere: the captain's log for release notes, and "lost at sea" for the 404.

Every place keeps a plain label next to its nautical one, so someone looking for the docs sees "docs", not a riddle.

## Evidence on Hand

- `assets/ship.svg`: the current logo, a tall sailing ship crewed by three little terminal crewmates. Treat it as the existing identity to evolve, not discard without reason.
- `README.md` and `docs/site/`: real copy, keys, config and CLI examples.
- The GitHub repo description already reads "staying afloat in an ocean of slop".
- There are no screenshots, recordings, testimonials, user counts or press. Don't fabricate any. A VHS recording of the real terminal is planned; until then use a clearly marked placeholder.

## Product Principles

1. **Alive, never in the way.** Motion is ambient and small (water, a crewmate blinking or looking up). Native scrolling only: no scroll hijacking, no scroll-locked sequences, no animation the reader must wait through to reach content. Respect `prefers-reduced-motion`.
2. **Findable in one move.** Navigation is always visible and plain. Install, docs, keys and config are reachable from every page. Docs have search, ideally on `/`. Every place has its own URL.
3. **Charm in the details, clarity in the structure.** Personality lives in illustration, crewmates and small touches. Layout, labels and docs stay boring in the good way.
4. **Honest.** The site says what works and what doesn't, the same split as the README.
5. **Terminal-true.** Anything showing Ship looks like the real terminal UI, with the real glyphs and colours.

## Accessibility & Inclusion

WCAG 2.2 AA. Crewmate moods never carry state alone; the glyph and a text label always accompany them. Illustrations are decorative unless they convey content, and then they have text alternatives. All navigation works by keyboard.
