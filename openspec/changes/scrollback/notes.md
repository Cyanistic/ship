# Scrollback: decisions from conversation

Status: input for the product paper, not a paper. Recorded on 2026-10-09 from the layout and sidebar design conversation with Cyan, so the decisions survive until `/product-design` turns them into `design/product.md`. Nothing here authorizes implementation.

This change lands after layout and sidebar (`openspec/changes/layout-sidebar/design/product.md`) and before floating panes. It covers scrollback, copy mode, search, edit-scrollback, `ship pane read` and mouse support.

## Decided

### Scroll belongs to the pane

- Scroll position, copy mode's cursor and copy mode's selection are shared state on the pane, not client-local. Two clients viewing one pane see the same thing, including how far it is scrolled. Cyan: "if two clients are on the same pane, they should both see the same thing? not different scroll levels?"
- This matches Ship's existing rule that clients on one tab see identical screens (terminal-client, scenario "Two clients of different sizes"), and tmux, where copy mode is a mode of the pane.
- The cost is accepted: while one client scrolls up, another client watching that pane also sees history instead of live output.

### Edit-scrollback is a server action

- The server holds the history, so it writes the pane's scrollback to a temporary file and opens the user's `$EDITOR` on it in a new pane. Every client sees that pane like any other. The editor runs where the server runs, as every pane program does.
- It is a `server.` action with a matching `ship` command, like every other `server.` action.

### Mouse support lives here

- Capturing the mouse makes the outer terminal's drag-to-select need a modifier (usually Shift), and copy mode is what replaces it. The wheel over a shell should scroll history, which does not exist until this change. Mouse support alone would feel broken, so it ships with scrollback.
- Scope from the layout paper's former mouse story: clicking a pane selects it, clicking a sidebar row selects that tab, and mouse events inside a pane whose program asked for mouse input (for example Neovim with `mouse=a`) reach that program. Mouse capture can be turned off in `[client]`, so the outer terminal's own selection works.

## Known facts

- Each pane's Ghostty terminal already keeps up to 1,000 lines of history: `crates/ship-server/src/pane.rs` spawns sessions with `SessionConfig::default()`, whose `scrollback` is 1000 (`vendor/ratatui-ghostty/src/session.rs`).
- The vendored wrapper scrolls the pane's single viewport with `send_scroll` (`ScrollViewport::Delta`). Because scroll is shared, published screens can simply show the scrolled viewport; no history-fetch protocol is needed for viewing.
- Mouse mechanics are covered by existing dependencies:
  - crossterm turns capture on and decodes events into `MouseEvent`, which is serializable with the `serde` feature Ship already enables.
  - ratatui's `Rect::contains` finds the clicked pane.
  - `tui-tree-widget` 0.24.1, planned for the layout sidebar, has `TreeState::click_at` and `rendered_at` for sidebar rows.
  - `ratatui-ghostty` converts a crossterm `MouseEvent` into Ghostty's encoder (`vendor/ratatui-ghostty/src/input.rs:58`) and exposes `send_mouse`. Ghostty tracks whether the program asked for the mouse and in which encoding.
- Ship's owned mouse code is the glue: find the rectangle under the event, then act on it or forward it with pane-relative coordinates, plus one new client-to-server input message.
- Today the client never captures the mouse, so the wheel either does nothing or the outer terminal turns it into arrow keys for the pane's program.

## Follow-ups

- AGENTS.md lists "viewing position" as client-local presentation state. Reword it to mean which tab or pane a client views, not scroll position, as part of this change.

## Open

- Whether libghostty-vt can extract history as text. Edit-scrollback and `ship pane read` depend on it. Not checked.
- Whether libghostty-vt offers search over history. Not checked.
- Clipboard for copy mode: a crate or OSC 52.
- Default keys. Cyan's Herdr bindings are `alt-s` for copy mode and `alt-e` for edit-scrollback; neither is bound in Ship's defaults today.
- History size: keep 1,000 lines or make it configurable.
- `ship pane read` covers the product brief's "reading visible output and recent scrollback" for agents; its exact shape is open.

Rough size from the conversation, implementation Rust only: shared scroll and keys about 100 lines, copy mode 400 to 600, search 150 to 250, edit-scrollback and `pane read` about 200, mouse about 250.
