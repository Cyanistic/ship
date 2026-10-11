# Slice 2: simultaneous screens and directional focus

Tasks 2.1–2.4 are implemented and verified on macOS. Linux and Windows were not run and are unverified. Slice 3 has not started.

## Implementation

- `crates/ship-core/src/geometry.rs`: `TabGeometry::neighbor`. A candidate's frame must touch the selected frame's edge on the requested side (bordered frames overlap by one cell) and share more than a corner cell with it. Ties go to the first candidate in the client's recency list, then the nearest center (squared distance in half cells), then layout order.
- `crates/ship-client/src/ui/panes.rs` (new): draws the viewed tab from its published geometry, offset to the pane area. Two or more panes get `Block::bordered().merge_borders(MergeStrategy::Exact)`, with the selected pane drawn last, thick and cyan. Labels are drawn in a second pass because a later border merges over earlier title text. A lone pane gets no border. Screens paint into content rectangles clipped to the client area. The selected pane's cursor is placed only inside its shown content. The filler dots the area outside the shared tab size and any content a lagging screen doesn't reach. `fill`, `paint`, `style` and `color` moved here from `draw.rs`.
- `crates/ship-client/src/ui/memory.rs` (new): client-local recency (newest first) and last pane per tab. It is pruned against each new replica and never serialized. `landing` returns the zoomed pane, else the last pane, else the first own pane, else the tab.
- `crates/ship-client/src/ui/observer.rs`: a screen event is visible when its pane is in the viewed tab's published geometry, not only when it is selected.
- `crates/ship-client/src/ui/draw.rs`: the pane area goes to `panes::draw` whenever the viewed tab's geometry has panes, else to the existing hints.
- `crates/ship-client/src/ui/mod.rs`: `Step` gains `Focus(Direction)`; `client.pane.focus` goes through `navigate` like next/prev. Memory observes the record selection after each replica event. Top-level tab cycling lands through `Memory::landing`, the spec's own-pane landing applied to today's top-level ring. Slice 3 replaces that ring with visible sidebar rows.
- `README.md`: status, keys table, the unavailable-defaults note and a "Working in splits" section.

There is no server or wire change. Protocol stays 8.

## Verification

Temporary harness, outside the repository: `/tmp/ship-slice2-0Wrw/`. `harness.py` runs an owned server on a random port and a raw TCP proxy that logs every client-to-server byte. Each `ship` client runs in a real PTY whose output is rendered by pyte (run with `uv run --with pyte`). Logs: `s2.log`, `s3.log` output inline, `s4.log`, `s2-wire.log`.

`s2.py`, debug build, 29/29 passed:

- After `alt-right`, `alt-|` and `alt--`, `[A | [B / C]]` renders with merged junctions (`┬ ┤ ┢ ┪ ┺`), labels on every top border and C thick. One pane before the first split rendered borderless.
- `sleep 2; echo BG_A_DONE` in A, followed at once by `alt-l` to C: A's output appeared while C stayed selected, without a keypress.
- Focus: `alt-h` C→A; `alt-l` A→C (C more recent than B); `alt-k` C→B; `alt-h` B→A; `alt-l` A→B (now B more recent); `alt-j` B→C; `alt-j` at the bottom edge does nothing. `alt-tab` from C visits A, B, C.
- `echo IN_C` typed after focusing C appeared only inside C's content.
- `printf '\033[31mRED\033[0m é 日本語|\n'` in B: red foreground, wide characters in consecutive double cells, and B's right border intact on that row.
- A second 80x20 client with fresh memory landed on A. Its `alt-l` from A chose B, where B's and C's centers are equidistant from A's (both 9 half cells), resolved by layout order. The 100x30 client then showed the 80x19 shared geometry with dotted unused columns 80–99 and rows 19–28.
- `nvim -u NONE -i NONE` in C: the terminal cursor sat inside C's published content rectangle, right after the typed `hello nvim`. A and B were byte-identical before entry, during and after `:q!`. C's shell screen returned.
- `ship pane close --pane C`, the most recent pane: the selection repaired to B, and `alt-h` then `alt-l` went A→B.
- Wire: 18 view PUTs, all with exactly `selection` and `area`; the byte log contains no `recent`.
- Exit: `alt-q` exited 0, and the final output leaves the alternate screen, disables bracketed paste and shows the cursor.

`s3.py`: `[A | [B / [C / D]]]` built from the CLI; a fresh client's `alt-l` from A chose C, the nearest center (7 half cells against 14 for B and 21 for D).

`s4.py`, release build, 140x42, 6/6 passed: shell, Neovim, Pi (UI only; no prompt sent) and `yes ship-flood` in four splits. Five rounds of `alt-|` then `alt-shift-x` left Neovim's and the flood pane's screens intact, returned to the shell and kept four panes. The terminal was restored on exit.

Keystroke echo, measured from writing `Z` to the client's PTY until the client's own output contains it, 30 samples each:

| Case | Median ms | Max ms |
| --- | --- | --- |
| Ship, one pane | 4.6 | 7.1 |
| Ship, four quiet panes | 4.7 | 6.5 |
| Ship, four panes, `yes` flooding a neighbor | 3.0 | 3.8 |
| zellij, one pane, same method (throwaway session, temp config dir) | 12.9 | 14.7 |

The flood case is faster most likely because the client is already redrawing continuously; it is not a regression. tmux and Herdr aren't installed and weren't compared. An earlier run that timed against pyte's rendered screen measured pyte's emulation cost rather than Ship's, so it was discarded.

### Emulator note

After the second client shrank the layout, pyte showed leftover wide-character continuation cells where `日本語` had been. ratatui-core 0.1.2 deliberately doesn't rewrite a default-styled wide character's trailing cell (`buffer/diff.rs`, `shrinking_wide_glyph_clears_trailing_cell`). It relies on the terminal blanking the right half when the left half is overwritten. Common terminals do that; pyte doesn't. This predates slice 2 and isn't a Ship defect. It hasn't been looked at in a GUI terminal.

### Checks

- `cargo fmt --all -- --check`: passed.
- Debug and release builds; `cargo check -p ship-core` with and without `--features clap`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: fails on findings from `4b2dbf8` (enabling `absolute_paths`, `unused_qualifications` and others), which stops at ship-core. Linting every crate with warnings non-fatal gives the same 79 findings before and after this slice, so this slice adds none. A clean strict Clippy run is not claimed.
- `openspec validate layout-sidebar --strict`: valid.
- Size: implementation Rust 6,977 → 7,234 lines (+257). Tests: 0; no permanent test code was added.

### Mixed builds within protocol 8

Cyan's first manual run showed only the empty-tab hint on a tab that had a pane. The running server was started at 02:31, before the slice-1 correction (about 03:19) moved geometry from `Replica.geometry` onto each tab. Both builds report protocol 8, so health passed. The old server published no `tab.geometry` and the client dropped the old map as an unknown field, so the client treated the tab as having nothing to draw. Restarting the server on the current build fixed it, and Cyan confirmed splits render. This only affects builds from between those two points, all of them local. Protocol stays 8.

## Unverified

- Linux and Windows.
- A GUI terminal looked at by a person. All rendered checks went through pyte on real client PTY output.
