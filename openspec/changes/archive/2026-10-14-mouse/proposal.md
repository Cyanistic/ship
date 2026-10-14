# Proposal

This is the planning artifact for a locked plan. Cyan locked the product, architecture and program papers in `design/` on 2026-10-13. Program gate G1 amended architecture decision 5 at lock, as recorded in the program paper's deviation log. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

The mouse can select panes and tabs but can't rearrange anything or reach an action without a key. Resizing panes with the mouse is an open item in milestone 1 of the product brief, and Herdr users expect it, along with dragging tabs and a right-click menu. The superseded mouse-drag change was planned before server-owned selection and the ordered input lane. This change carries its stories forward against the current code and adds menus.

## What Changes

- **Border drags.** Dragging a pane border resizes the panes, and a junction moves both edges.
  - Each position is an absolute `to` that waits its turn in the client's input order. A newer position replaces one still waiting.
  - A left press on a border never selects and never reaches a program.
- **Sidebar width.** Dragging the sidebar's divider changes that client's sidebar width.
  - So does the new `client.sidebar.resize` action, bound to `shift-h` and `shift-l` in resize mode.
  - The width is client-local and isn't written to the config. It lasts until the client exits or `sidebar_width` changes.
- **Tab drops.** Dragging a sidebar row moves the tab, and a line at the target indent shows where it will land.
  - The drop is re-checked at its turn, and a stale drop does nothing.
  - Dropping under a folded tab unfolds it.
- **Sidebar clicks on release.** A sidebar row or fold marker now acts on release, not press, and only when the press didn't become a drag.
- **Right-click menus.**
  - Right-clicking a pane, its label or its border opens the pane menu. A sidebar row opens the tab menu.
  - The `tabs` heading, the sidebar space below the last row, and a new button on the heading open the global menu.
  - Items come from `[client.menus]`. A target the file names replaces its default list, and an empty list means no menu.
  - An item runs exactly as the same action bound to a key.
- **Right-click passthrough.** A pane with `right_click_passthrough` on gives right-clicks on its contents to its program when the program has asked for mouse input. The setting is shared, so every client sees it.
- `ship pane resize --to N` and `server.pane.resize = { to = N }` move an edge to an absolute position. `to` with `amount` is rejected.
- `ship tab set`, `ship pane set`, `server.tab.set` and `server.pane.set` change attributes in one edit.
  - A pane's attributes are `name` and `right_click_passthrough` (`true`, `false` or `toggle`). A tab's only attribute is `name`.
  - They reuse `PATCH /api/v0/tabs/{id}` and `/panes/{id}`.
- `ship pane create --right-click-passthrough` and `server.pane.create = { right_click_passthrough = true }` create a pane with passthrough on.
- Each published pane screen carries `mouse: true` while the program tracks the mouse.
- **BREAKING**: `ship tab rename`, `ship pane rename`, `server.tab.rename` and `server.pane.rename` are removed for `set`. A config binding to a rename action is an unknown action.
- **BREAKING**: protocol version 10 goes to 11. The PATCH bodies change shape. Server and client upgrade together.

Not in this change (the product paper lists them all):
- Dragging panes to move or swap them.
- Sidebar scrolling.
- Cancelling a drag with a key.
- Persisting a dragged width.
- Prompted menu items such as rename.
- Submenus.
- A passthrough option on tab create.
- Keeping `rename` as an alias.

## Capabilities

### New Capabilities

- `context-menus`:
  - `[client.menus]` and its defaults.
  - What a right-click or the heading button opens, and which pane a border cell belongs to.
  - Using an open menu by mouse and keys.
  - Items running as key actions.

### Modified Capabilities

- `mouse-input`:
  - A border press doesn't select.
  - Sidebar clicks act on release.
  - Border, divider and row drags.
  - Right-click passthrough at the router.
- `session-structure`:
  - `to` on directional resize.
  - `set` in place of `rename`.
  - The `right_click_passthrough` attribute and its create option.
- `session-observation`:
  - The mouse-tracking flag on published screens.
  - `set` responses in the published-tab rule.
- `terminal-client`:
  - Menu items in input order, with border positions replacing waiting ones.
  - The held border's color.
  - The sidebar's heading button and drop line.
  - The resize-mode hints.
  - Clearing a name with `set`.
- `keymap`:
  - The resize-mode sidebar defaults.
  - `client.sidebar.resize`.
  - Rename actions are no longer accepted.
- `config-file`: the sidebar width's client-local override and how it survives reload.
- `health-exchange`: protocol version 11.

## Impact

- `ship-core`:
  - `to` on `pane::Resize` and `ResizePane`.
  - `tab::Set` and `pane::Set` in place of `Rename`, with `SetTab` and `SetPane` bodies. `Named` is removed.
  - `Switch`, and `Pane.right_click_passthrough`.
  - The create flag.
  - `Screen.mouse`.
  - Protocol version 11.
- `ship-server`:
  - `layout::resize` takes a relative or absolute `Change`.
  - `Set` handlers resolve `toggle` and commit only on change.
  - `set_tab` and `set_pane` routes.
  - The pane actor copies the tracking flag.
- `vendor/ratatui-ghostty`: the rendered snapshot records `is_mouse_tracking()`. It is counted apart from implementation Rust.
- `ship-client`:
  - Gestures and the owner rule in the mouse router.
  - A new `menu` module.
  - Sidebar widths, gaps and the drop line.
  - `Resize` and `Move` lane items.
  - `Local.menu` and `Scene`.
  - `Menus` and `client.sidebar.resize` in the keymap.
  - `set` in the API client.
- `ship`: the CLI derives `tab set`, `pane set`, `--to` and `--right-click-passthrough`, and `rename` goes away, with no hand-written changes.
- Docs: `docs/guide.md` and the README's "What works today" and keys. The product brief's milestone-1 item is closed.
- No new dependency.
- Measured with `cloc` against `dac695b`: +820 client code lines and +113 server/core code lines of implementation Rust, plus +5 vendored. No authored tests added. Full counts, including physical lines, and verification are in [tasks.md](tasks.md).
