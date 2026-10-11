# Proposal

Planning artifact for a locked plan. The product, architecture and program papers in `design/` were locked by Cyan on 2026-10-11, after an amendment the same day deferred edit-scrollback. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

Panes keep about 10 KB of history and nothing can scroll it, so a long build log or test run is lost once it leaves the screen. Ship doesn't capture the mouse, so clicks can't select panes, and agents have no way to read a sibling pane's output. Together with layout, this finishes milestone 1's terminal workspace.

## What Changes

- Every pane gets history you can scroll back through, capped at 10 MB by default (`[server] scrollback_bytes`), with one scroll position shared by every client viewing it.
- `server.pane.scroll` and `ship pane scroll --direction up|down|top|bottom [--amount N]` scroll by lines, by a page when the amount is omitted, or to the top or bottom. `alt-pageup` and `alt-pagedown` scroll a page by default.
- A scrolled pane stays on its content while output arrives, shows how far up it is, and returns to the bottom on input.
- `ship pane read` prints a pane's visible screen, or its last N lines with `--lines`, joining soft-wrapped lines with `--unwrap` and keeping styles with `--ansi`, regardless of scroll position.
- The client captures the mouse unless `[client] mouse = false`. A click selects a pane or a sidebar row, or folds a row at its marker. Programs that ask for the mouse get it. Otherwise the wheel scrolls history, or sends arrows on the alternate screen, and acts on the pane under the pointer without selecting it.
- Dragging, double-clicking and triple-clicking in a pane whose program hasn't asked for the mouse selects text, highlighted for every viewer, and copies it to that client's clipboard on release through OSC 52. Clipboard writes from programs reach every client viewing the pane; programs can't read any clipboard.
- Exited panes still scroll, read and select.
- **BREAKING**: protocol version 8 to 9, for mouse input frames, clipboard events and the scroll offset on published screens. Server and client upgrade together.

Copy mode, search, edit-scrollback, a bindable run action and `ship clipboard copy` are deferred with tracking issues. Until search lands, `ship pane read` stands in for milestone 1's search evidence.

## Capabilities

### New Capabilities

- `pane-history`: a pane's scrollback, its shared scroll position, scrolling it from keys and the CLI, and reading its text.
- `mouse-input`: mouse capture and what clicks and the wheel do in panes and the sidebar.
- `selection-clipboard`: selecting text with the mouse, highlighting it for every viewer, and delivering copies and program clipboard writes to clients.

### Modified Capabilities

- `config-file`: the `[server] scrollback_bytes` and `[client] mouse` settings.
- `keymap`: `alt-pageup` and `alt-pagedown` join the defaults.
- `pane-programs`: exited panes ignore program input but still scroll, read and select.
- `terminal-client`: the scroll indicator, and terminal restoration turning mouse capture off.
- `health-exchange`: protocol version 9.

## Impact

- Vendored `ratatui-ghostty` wrapper: recorded changes 12 to 16 for the history cap, outliving the PTY, scrolling, reading, pointer input and gestures, drawing the selection, and clipboard events. It isn't counted toward the size budget.
- `ship-core`: `InputFrame::Mouse`, `SseEvent::Clipboard`, `Screen.scrolled`, and the `scroll` and `read` commands.
- `ship-server`: pane task commands and forwarding after exit, the history setting, two routes, mouse frames on the input route, and clipboard events on the attach stream. The state actor, `Pane` and `Layout` are unchanged.
- `ship-client`: mouse capture and hit-testing, sidebar clicks, the scroll indicator, OSC 52 output through crossterm's `osc52` feature, and the new commands.
- No new crates. About 320 to 420 lines of implementation Rust, plus about 300 uncounted wrapper lines.
