# ratatui-retained

A prototype of retained rendering for ratatui. A plain ratatui frame paints every widget into an empty buffer and diffs the whole screen. This keeps the last frame instead. Layers report which cells changed, and only those cells are reset, replayed through every layer that covers them, and diffed against the terminal.

It is an experiment toward a possible standalone crate. Ship doesn't use it.

## Results

The numbers below come from `cargo run --release --example bench`. The screen is 200x60, Ship-shaped: a sidebar, a bordered pane with 58 rows of styled text, a second pane overlapping it, and a status line. Both paths run the same widget code. "Plain" is `Terminal::draw` painting every layer.

| Workload | Plain ratatui | Retained | Bytes per frame (plain / retained) |
|---|---|---|---|
| typing | 325 µs | 10 µs | 40 / 22 |
| typing under an open popup | 331 µs | 12 µs | 40 / 22 |
| scroll (every pane row changes) | 385 µs | 392 µs | 6902 / 6894 |
| sidebar selection | 327 µs | 85 µs | 62 / 53 |

These are medians of 5000 frames on an M4 Max under macOS. Linux is unverified.

## Correctness

`cargo run --release --example check [seeds] [frames]` drives the same app with random edits: typing, wide characters, emoji with VS16, combining marks, tabs, scrolling, rows added and removed, a popup opening and moving, a style-only tint, split toggles, sidebar resizes, renames, cursor moves and terminal resizes. After every frame it requires two things:

1. The retained buffer equals a from-scratch render of the same layers.
2. A `vt100` parser fed our bytes shows the same cells and cursor as one fed plain ratatui's `Terminal::draw` output.

`vt100` panics on some byte streams, including plain ratatui's. When it can't parse ratatui's bytes, the seed stops early and the summary counts it.

Each of these deliberate bugs makes the checker fail:

- not resetting damaged cells
- painting without the scratch buffer
- forgetting a layer's old area when it moves
- not damaging removed rows
- letting rows paint unclipped
- a footprint that never touches
- reusing the old frame after a whole-screen swap
- caching a footprint across value changes (caught in about 1 seed in 8)

## How it works

- `Layer` is the core contract (`src/compositor.rs`). `damage` reports every cell whose painted value may have changed. `paint` must be deterministic, and each cell it writes depends only on the layer's state and that cell's previous value. Ratatui's widgets behave this way.
- `Compositor::draw` collects damage (including old and new areas of moved layers) and resets those cells. It then replays every layer that covers them, bottom to top. A layer that would paint outside the damage paints into a scratch buffer, and only the damaged cells are copied back. Damaged rows are diffed with ratatui's own `Buffer::diff`, so wide characters behave as in a normal frame.
- When at least half the screen is damaged, it draws a plain ratatui frame instead and swaps buffers, so a full-screen change costs what ratatui costs.
- A widget's footprint is measured by rendering it over two sentinel backgrounds that differ in every field. A layer skips repainting when the damage misses every cell it writes. That's how a pane's border `Block` stays out of a typed row. The measurement runs once per value change.
- `widget` and `rows` (`src/reactive.rs`) build layers from `reactive-scope` signals. A widget layer damages its area only when its recomputed value differs (`PartialEq`). A rows layer damages only rows that differ from the last painted ones.

## Limits

- The contract is checked, not enforced. A widget that reads cells other than the one it writes, or a row painter that strays outside its row, breaks it. `Compositor::render_full` plus `buffer()` is the way to check an app.
- `reactive-scope` is 0.1 from a single author. It keeps a thread-local graph and panics when a signal is read with tracking outside an effect. The `Compositor` and `Layer` don't depend on it.
- Cell writes that change only `diff_option` or the deprecated `skip` are detected by the footprint but untested.
