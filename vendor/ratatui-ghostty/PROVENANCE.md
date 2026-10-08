# Provenance

Copied on 2026-10-06 from `experiments/terminal-transport/vendor/wrapper-compat`, byte-for-byte and in full, including tests and examples. The experiment copy stays where it is. This is Ship's production copy of the wrapper; `ship-server` depends on it by path. It isn't a workspace member, so Ship's lints and line budget don't apply to it.

The wrapper section of `experiments/terminal-transport/vendor/PROVENANCE.md` carries over below.

## Baseline

- Published `ratatui-ghostty` **0.2.0**, repository <https://codeberg.org/jint/ratatui-ghostty>.
- Packaged `.cargo_vcs_info.json` records commit **`be6d2d89105ca8cb867e7963888f82ba58b52316`**. That metadata is preserved in `experiments/terminal-transport/vendor/preservation/wrapper-cargo-vcs-info.json`.
- Published `.crate` SHA-256: **`2dbd592bc403a76d2eb5c491ebe5041f1973bd20d7f439970af2fe9ea1b4ac32`**.
- `experiments/terminal-transport/vendor/wrapper-compat.patch` is a unified diff against the published crate. Differences are limited to `Cargo.toml`, `src/session.rs`, and three existing tests: `input_encode.rs`, `terminal_move_after_callback.rs`, and `widget_render.rs`. All other files match the published crate byte-for-byte.

## Local changes

The experiment made five changes:

1. Raise `libghostty-vt` from `0.1.1` to `0.2.1`.
2. Register the demo at `examples/demo/main.rs`.
3. Replace `TerminalOptions` with `Terminal::new(cols, rows)` and `set_scrollback_max_lines(Some(scrollback))`.
4. Use typed `ColorScheme::Dark` / `Light` and remove the obsolete `ffi` import.
5. Adapt those three tests to the same constructor/scrollback API. No new test cases were added.

This copy keeps all five. Two edits differ from the experiment copy:

- `Cargo.toml` names libghostty-rs by git rev instead of `"0.2.1"` plus the experiment's workspace `[patch.crates-io]`: `libghostty-vt = { git = "https://github.com/uzaaft/libghostty-rs", rev = "8953a740bc378cec3e07e1f6ca949f0595eab19b" }`. That's the checkout the experiment built against, and the tip of upstream `master` on 2026-10-06.
- rustfmt reformatted `tests/terminal_move_after_callback.rs` and `tests/widget_render.rs`, both adapted by change 5. No test logic changed.

All 80 wrapper tests pass against that rev.

## Ship changes

Made for Ship's pane-terminals slice 3 on 2026-10-06, beyond the experiment copy:

6. Colors reach the caller unresolved. `convert/to_ratatui.rs` maps a palette color to `Color::Indexed` instead of looking it up in Ghostty's palette, and `widget.rs` no longer fills unset foreground and background with Ghostty's defaults, so they stay `Reset`. A background set by erasing with a color, which Ghostty keeps in the cell content rather than its style, is read from the raw cell. A viewing terminal then shows default and palette colors in its own theme. `tests/convert_to_ratatui.rs` renames `style_fg_palette_resolves` and `style_bg_palette_resolves` to `_stays_indexed` and expects `Color::Indexed`.
7. The session thread renders during sustained output. Its loop drained queued PTY output until the queue was empty before rendering, so a program like `yes` kept it draining and nothing rendered or woke the caller until output stopped. `session.rs` now stops draining after `DRAIN_BUDGET` (8 ms), renders and wakes, then continues.

Made for Ship after pane-terminals slice 7 on 2026-10-07:

8. `SessionHandle::take_if_dirty` copies the rendered cells and cursor and clears the dirty flag under the render lock, returning `None` when nothing was rendered since the last clear. Ship published a screen per wake, reading cells and cursor under separate locks. A render could land between the caller's read and its wake, so the late wake published the same screen again; about half the screens sent while scrolling Neovim were duplicates. Renders already set the flag under that lock, so the take can't miss a render or repeat one. `blit_to`'s copy loop moved to `RenderSnapshot::blit_to`, shared by both; `blit_to` behaves as before.
9. Rendering visits only dirty rows. `TerminalWidget::incremental` renders only the rows Ghostty's render state marks dirty and clears the global and per-row dirty state; a frame marked `Full` renders every row. Each rendered cell is reset first, since `set_style` only patches a kept cell. `render_to_shared` keeps its buffer between renders instead of resetting it, renders incrementally unless the buffer was just replaced, and replaces it only when the size changed. The `Resize` command no longer replaces the buffer itself, so the render it requests is a full one. On a pane changing one line, each render had converted all of its cells, a `String` per cell.

All 80 wrapper tests pass after changes 6 to 9, run in a copy outside the repo.

Made for Ship on 2026-10-07:

10. The cursor reports when it follows the default. Ghostty resolves DECSCUSR 0 and the initial cursor to its configured default shape and doesn't expose that the cursor is following it, so a caller passing the cursor on to another terminal had to send Ghostty's block in place of that terminal's own default. `session.rs` sets the default cursor style to `BlockHollow`, which no escape sequence selects, and `widget.rs` reports it as the new `CursorStyle::Default`. The two examples map `Default` to crossterm's `DefaultUserShape`. Terminals built outside a session, as in the tests, keep Ghostty's block default. Ghostty tracks this internally as `cursor.is_default`; when its C API exposes that, read it instead of the marker. The marker depends on no escape sequence selecting a hollow block; Ghostty discussion #5876 proposes adding one to DECSCUSR, so check it whenever libghostty is bumped.

No crates.io release works yet. `libghostty-vt` 0.2.1 and 0.2.2 come from the `release/0.2.x` branch, which lacks `Terminal::new(cols, rows)` and builds Ghostty `a887df42`, which needs Zig 0.15.2. When a release ships Ghostty `22d13172` or later, swap the git dependency for that version.

## License

The published manifest declares **MIT**, but neither the package nor the recorded source commit contains a license/copyright notice file. `LICENSE` here is an explicitly labeled local notice containing standard MIT terms, not a claimed upstream file or invented copyright attribution. Resolve any missing upstream notice before public redistribution.

## Native Ghostty

`libghostty-vt-sys`'s build script at `8953a740` clones Ghostty commit `22d13172cde98a0a4dda05d3d6a3fcb0dd8ed018` into Cargo's `OUT_DIR` and builds it with whatever `zig` is on `PATH`. That commit declares `minimum_zig_version = "0.16.0"`, and `requireZig` also needs the same major.minor version, so 0.15 and 0.17 are rejected. Building `ship-server` therefore needs Zig 0.16 and network access.
