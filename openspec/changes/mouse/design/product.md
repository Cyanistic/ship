# Mouse drag and menus

Status: Locked on 2026-10-13, approved by Cyan in Plannotator review ("LGTM") after a first review of the earlier draft ("LGTM!") and a revision that added `set`, the passthrough bool and the removal of `rename`. Written on 2026-10-13 from a design conversation with Cyan. The approval doesn't authorize implementation; Cyan requests that separately. It supersedes `openspec/changes/mouse-drag/design/product.md` (locked 2026-10-11 with amendment A-1). That paper was written before server-owned selection and the ordered input lane, so this one carries its stories forward against the current code and adds right-click menus. The table at the end says where each old requirement went.

## Goal

Ship's mouse can select panes and tabs but can't rearrange anything or reach any action without a key. This change adds three drags the keyboard and CLI can already do: dragging a pane border resizes panes, dragging the sidebar's edge resizes the sidebar, and dragging a sidebar row moves a tab. It also adds right-click menus for panes, tabs and the whole workspace, built from config the same way key bindings are. A pane setting lets a program have the right-clicks on its contents instead, and that setting arrives through a new `set` command for tab and pane attributes, which replaces `rename`. It closes the "mouse pane resizing" item in milestone 1 of the product brief and covers what Herdr users reach for with the mouse.

Today a press on a sidebar row acts at once. After this change, a press that moves at least one cell before release becomes a drag, and one that doesn't acts as a click on release.

## User stories

### P1: Resize panes by dragging a border

Given a tab with two or more panes, a user grabs the border between them and drags it, and the panes resize as the pointer moves.

- **Given** a left and a right pane, **when** the user presses on the vertical border between them and drags 5 columns right, **then** the left pane is 5 columns wider and the right pane 5 columns narrower, and both programs redraw at their new size during the drag.
- **Given** the right pane is selected, **when** the user presses on the border and drags, **then** the right pane stays selected, even if earlier input is still waiting.
- **Given** a 2x2 grid, **when** the user presses on the center junction and drags diagonally, **then** both the vertical and the horizontal edge move.
- **Given** a neighbor with 3 columns of content, **when** the user drags the border 10 columns into it, **then** the border stops with the neighbor at 1 column, as `ship pane resize` clamps.
- **Given** a script, **when** it runs `ship pane resize --direction right --to 40` on the left pane, **then** the border moves to column 40 of the tab, or as close as clamping allows.

Tested independently by dragging borders in a split tab and checking the result with `ship pane get` and by eye, and by running `ship pane resize --to`.

### P2: Move a tab by dragging its sidebar row

A user drags a sidebar row to another place in the tree. The row the pointer is over picks the gap above it, and the pointer's column picks the depth. A line drawn at the target indent shows where the tab will land.

- **Given** top-level tabs A, B, C, **when** the user drags C's row to the gap above A at A's indent and releases, **then** the order is C, A, B.
- **Given** top-level tabs A and B, **when** the user drags B onto the gap below A with the pointer one indent further right than A, **then** B becomes A's last child, with its subtree and panes, and its programs keep running.
- **Given** B is a child of A, **when** the user drags B to the left edge below the last row, **then** B becomes the last top-level tab.
- **Given** A is folded, **when** a tab is dropped as A's child, **then** A unfolds and the moved row is visible.
- **Given** the user drags A over one of A's own descendants, **then** no drop line is shown, and releasing there changes nothing.
- **Given** a press on a row that never moves a cell, **then** on release it selects the tab or toggles the fold marker, as a click does today.

Tested independently by dragging rows and checking `ship tab list` against the drop line shown before release.

### P3: Resize the sidebar by dragging its edge or with keys

- **Given** the sidebar is shown, **when** the user drags its right divider 10 columns right, **then** the sidebar is 10 columns wider for that client and the pane area shrinks to match.
- **Given** a dragged width, **when** the user saves an unrelated config change, **then** the width stays; **when** `sidebar_width` itself changes, **then** the configured width replaces it.
- **Given** a second client attached, **then** its sidebar width is unaffected.
- **Given** the default keys, **when** the user enters resize mode and presses `shift-l` three times, **then** the sidebar is 3 columns wider, under the same limits and lifetime as a dragged width.
- **Given** a binding `"alt-[" = { action = { client.sidebar.resize = { direction = "left", amount = 5 } } }`, **when** the user presses it, **then** the sidebar is 5 columns narrower.

Tested independently by dragging the divider and pressing the resize keys in two attached clients, and editing the config while attached.

### P4: Act on a pane, a tab or the workspace from a right-click menu

A right-click opens a menu of actions for what was clicked. The items come from config, as key bindings do, so a user can add, remove or reorder them.

- **Given** an unselected pane, **when** the user right-clicks in it, **then** that pane is selected and the pane menu opens at the pointer with split right, split down, zoom, right-click passthrough and close.
- **Given** the pane menu is open, **when** the user clicks "split right", **then** the pane splits to the right, the new pane is selected, and the menu closes.
- **Given** the tab menu opened on tab B's row, **when** the user picks "new tab", **then** a tab with a shell is created right after B under B's parent and its pane is selected; **when** they pick "new child tab" instead, **then** it is appended as B's last child.
- **Given** the sidebar is shown, **when** the user left-clicks the button at the right end of the `tabs` heading, or right-clicks the heading or the empty space below the last row, **then** the global menu opens with new tab, hide sidebar, reload config and detach.
- **Given** a menu is open, **when** the user presses `j` twice and Enter, **then** the third item runs; **when** they press Esc or click outside the menu instead, **then** the menu closes, nothing runs, and the click goes nowhere.
- **Given** a config with `[client.menus] pane = [{ action = { server.pane.zoom = {} }, label = "fill" }]`, **when** the user right-clicks a pane, **then** the menu shows only "fill"; **given** `pane = []` instead, **then** no menu opens and the right-click acts as it does today.

Tested independently by right-clicking panes, rows, the heading and the button, with the default menus and with a config that replaces and empties them, checking results with `ship tab list`.

### P5: Let a program have right-clicks in its pane

Some programs use right-click themselves. A pane can be set to send right-clicks on its contents to its program instead of opening the menu. The setting belongs to the pane, so every client sees the same behavior.

- **Given** a pane running a program that asked for mouse input, **when** the user picks "right-click passthrough" from its menu and then right-clicks in its contents, **then** the program receives the right-click and no menu opens.
- **Given** that pane, **when** the user right-clicks its label or border, **then** the pane menu opens, and picking "right-click passthrough" again turns passthrough off.
- **Given** a passthrough pane whose program hasn't asked for mouse input, such as a shell at its prompt, **when** the user right-clicks its contents, **then** the pane menu opens.
- **Given** a script, **when** it runs `ship pane create --right-click-passthrough -- nvim`, **then** the new pane has passthrough on, and `ship pane get` shows `rightClickPassthrough: true` for it.
- **Given** two attached clients, **when** one turns passthrough on for a pane, **then** a right-click on that pane's contents in the other client also reaches the program.

Tested independently with a mouse-aware program such as Neovim with `mouse=a`, from two clients, and with `ship pane create`, `ship pane set` and `ship pane get`.

### P6: Change tab and pane attributes with `set`

Names and the passthrough setting are attributes, changed through one `set` command per entity. `set` replaces `rename`.

- **Given** a pane, **when** a script runs `ship pane set --pane <id> --name build --right-click-passthrough true`, **then** both change in one edit, and `ship pane get` shows them.
- **Given** a named tab, **when** a script runs `ship tab set --tab <id> --name ""`, **then** the name is cleared and the sidebar shows its derived label.
- **Given** a script, **when** it runs `ship pane set` with no attribute, **then** nothing changes and it exits 0.
- **Given** a script, **when** it runs `ship tab rename`, **then** the CLI reports an unknown command and exits 2.

Tested independently with `ship tab set`, `ship pane set`, `ship tab get` and `ship pane get`.

## Non-goals

- Closing a pane when its program exits (#16). It gets its own change.
- Dragging panes to swap or move them.
- Scrolling the sidebar, including auto-scroll during a tab drag. Rows past the sidebar's bottom edge stay out of reach, as they are today.
- Writing a dragged sidebar width to the config file, or keeping it after the client exits.
- Hover effects other than the drop line, the held border's color and the highlighted menu item.
- Cancelling a drag with Esc or any other key. Keys keep going where they go today.
- Drags with any button other than the left one.
- A rename item or any other item that needs a text prompt. Ship has no in-UI prompt yet; names are set from the CLI with `set`.
- A one-time "send this right-click through" item, or a modifier that bypasses the menu for one click.
- Submenus, separators, icons, disabled items or items whose label reflects state.
- A mouse entry point to the global menu while the sidebar is hidden.
- Keeping `right_click_passthrough` across a server restart. Restore isn't built; when it is, the setting is reset to off.
- A `right_click_passthrough` option on tab create. A tab's first pane always starts with it off.
- Keeping `rename` as an alias for `set`.
- Any new server operation or CLI command other than `to` on pane resize, `right_click_passthrough` on pane create, and `server.tab.set` and `server.pane.set` with the fields FR-040 and FR-041 name. Any new key action other than `client.sidebar.resize`.

## Functional requirements

### Drags in general

- **FR-001** A left-button press on a pane border, the sidebar divider or a sidebar row MUST become a drag once the pointer moves at least one cell from where it was pressed. A press released before that MUST act as a click.
- **FR-002** A drag MUST belong to the client that started it and MUST end on release. No other client's view or selection may change because of the drag itself; they see only the resulting server edit.
- **FR-003** With mouse capture off (`[client] mouse = false`), none of these drags and no menu may happen.
- **FR-004** Drag results MUST take their turn in the client's input order, with keys, actions and other mouse input, as terminal-client's Ordered input already requires of drags.

### Sidebar clicks

- **FR-005** A left click on a sidebar row or fold marker MUST act on release, not on press, and only when the press didn't become a drag. It then selects the tab or folds the row as today, taking its turn in input order at release.

### Pane borders

- **FR-006** A press on a cell of one border between panes MUST grab that border. A press on a cell where borders meet MUST grab every border through that cell, moving each in its own axis.
- **FR-007** A left press on a border MUST NOT change which pane is selected and MUST NOT reach any program, including while earlier input is still waiting. This narrows mouse-input's rule that a press on any pane selects it while input waits: that rule covers pane contents, not borders.
- **FR-008** `server.pane.resize` and `ship pane resize` MUST accept `to`, a position in cells counted from the tab's left edge for `left` and `right` or its top edge for `up` and `down`, starting at 0, in place of `amount`. With `to`, the edge that `direction` selects, including the opposite-edge fallback at the tab boundary, MUST move so its border is drawn at that position, or as close as clamping allows: no pane below 1 column and 1 row of content. An edge already there MUST change nothing. A request giving both `to` and `amount` MUST be rejected without changing anything, and the CLI MUST exit 2 for it. Without `to`, resize behaves as before.
- **FR-009** While dragging, the grabbed edge MUST follow the pointer through FR-008, naming the edge by a pane beside it and a direction, so a layout change made elsewhere mid-drag still moves the same edge while that pane exists. If the pane is gone, the drag MUST end without another resize.
- **FR-010** A border drag MUST have at most one position waiting its turn. A newer position MUST replace a waiting one, so the border catches up to the pointer rather than replaying each step, and a position equal to the last one sent MUST NOT be sent. On release, the final position MUST be sent if it differs from the last one sent.
- **FR-011** The layout shown during a drag MUST be the layout the server publishes. The client MUST NOT draw a predicted border position.
- **FR-012** A held border MUST be drawn in the selected-border color until release.
- **FR-013** A lone or zoomed pane has no border, so there MUST be nothing to grab.

### Sidebar width

- **FR-014** A drag on the sidebar's right divider MUST set that client's sidebar width to follow the pointer, with the pane area taking the remaining columns.
- **FR-015** The width MUST stay between 10 columns and the width that leaves the pane area 20 columns. When the terminal is too narrow for both limits, the pane area's minimum MUST win.
- **FR-016** A changed width MUST apply only to that client, MUST NOT be written to the config file, and MUST last until the client exits or the config's `sidebar_width` value changes. Reloading a config whose `sidebar_width` is unchanged MUST keep it.
- **FR-017** While the sidebar is hidden, there MUST be no divider to drag. Hiding and showing it again MUST keep a changed width.
- **FR-018** A client action `client.sidebar.resize` MUST take a `direction` of `left` or `right` and an optional `amount` in columns, defaulting to 1, and MUST move the sidebar's right edge that way. It MUST follow FR-015 and FR-016 as a dragged width does, MUST do nothing while the sidebar is hidden, and MUST have no CLI command.
- **FR-019** The default keys MUST bind `shift-h` and `shift-l` in resize mode to `client.sidebar.resize` left and right, beside the existing `h`, `j`, `k` and `l` pane resizes.

### Tab rows

- **FR-020** While a row is dragged, the drop target MUST be the gap above the row under the pointer, or the gap after the last row when the pointer is below it. The pointer's column MUST pick a depth from the depth of the row below the gap (0 after the last row) to one more than the depth of the row above it, clamped to that range.
- **FR-021** A line MUST be drawn at the drop target's gap, starting at the target depth's indent, for as long as the target is valid.
- **FR-022** A target inside the dragged tab's own subtree, or one that would leave the tab where it already is, MUST show no line, and release there MUST change nothing.
- **FR-023** Release on a valid target MUST perform one tab move to that place, carrying the tab's subtree and panes, as `ship tab move` does. Release outside the sidebar MUST change nothing.
- **FR-024** The drop MUST be resolved at its turn against the tree as it is then. If the target no longer exists or is no longer valid, the drop MUST change nothing and show no error.
- **FR-025** Dropping a tab as the child of a folded tab MUST unfold that tab in the dragging client.
- **FR-026** Dragging a row MUST NOT change selection. A selected tab that is moved MUST stay selected.

### Menus in the config

- **FR-027** The client config MUST accept `[client.menus]` with the targets `pane`, `tab` and `global`. Each target MUST be an ordered list of items, each a table with exactly one action under `action`, written as in a key binding, and an optional `label`. An item without `label` MUST be labeled as a key binding's hint is derived. An unknown target, an unknown key in an item, or an item without `action` MUST be a config error with its line and column.
- **FR-028** Ship's defaults MUST define all three targets. A target in the user's file MUST replace that target's default list whole; targets the file doesn't name keep their defaults. An empty list MUST mean that target has no menu.
- **FR-029** The defaults MUST be:

  | Target | Label | Action |
  | --- | --- | --- |
  | pane | split right | `server.pane.create = { direction = "right", select = true }` |
  | pane | split down | `server.pane.create = { direction = "down", select = true }` |
  | pane | zoom | `server.pane.zoom = {}` |
  | pane | right-click passthrough | `server.pane.set = { right_click_passthrough = "toggle" }` |
  | pane | close | `server.pane.close = {}` |
  | tab | new tab | `server.tab.create = { starter = "shell", select = true }` |
  | tab | new child tab | `server.tab.create = { starter = "shell", child = true, select = true }` |
  | tab | close | `server.tab.close = {}` |
  | global | new tab | `server.tab.create = { starter = "shell", select = true }` |
  | global | hide sidebar | `client.sidebar.toggle = {}` |
  | global | reload config | `client.config.reload = {}` |
  | global | detach | `client.detach = {}` |

### Opening a menu

- **FR-030** A right-click MUST open:
  - the pane menu on a pane's contents (subject to FR-036), its label, or its border. On a border cell, the pane is the one whose label is there; otherwise the pane below a horizontal edge or right of a vertical edge, and at a junction the pane below and to the right; on the tab's outer edges, the pane inside.
  - the tab menu on a sidebar row, including its fold marker.
  - the global menu on the sidebar's `tabs` heading or on sidebar space below the last row.
- **FR-031** The sidebar heading MUST show a button at its right end, and a left click on it MUST open the global menu.
- **FR-032** A right-click that opens the pane or tab menu MUST first select the clicked pane or tab for that client, as a left click on it does, taking its turn in input order. Opening the global menu MUST NOT change selection.
- **FR-033** When the target's list is empty, a right-click MUST act as it does without this change, and MUST NOT select anything it wouldn't select today.

### Using an open menu

- **FR-034** A menu MUST open at the pointer and be moved as needed to fit fully on the client's screen. Its items MUST appear in config order with the first highlighted. A menu MUST belong to the client that opened it; other clients see nothing of it.
- **FR-035** While a menu is open:
  - a left click on an item MUST run it and close the menu;
  - up and down, and `j` and `k`, MUST move the highlight, and Enter MUST run the highlighted item and close the menu;
  - Esc MUST close the menu without running anything;
  - a press of any button outside the menu MUST close it and MUST NOT act anywhere else;
  - no key may reach a pane or run a binding.
- **FR-036** An item's action MUST take its turn in input order and behave exactly as the same action bound to a key, acting on the selection at its turn. A failure MUST show on the status line as a key action's does.

### Right-click passthrough and `set`

- **FR-037** Every pane MUST have a `right_click_passthrough` setting, on or off, defaulting to off. It is shared state: one value per pane, the same for every client, shown by `ship pane get` and in `ship tab list` output.
- **FR-038** With passthrough on, a right-click on the pane's contents MUST reach its program as mouse input, without a menu and without changing selection, whenever the program has asked for mouse input. When it hasn't, the pane menu MUST open as with passthrough off. A right-click on the pane's label or border MUST open the pane menu either way.
- **FR-039** `server.pane.create` MUST accept `right_click_passthrough = true`, and `ship pane create` the switch `--right-click-passthrough`, to create the pane with passthrough on.
- **FR-040** A server action `server.pane.set`, and the CLI command `ship pane set`, MUST change a pane's attributes in one edit. Each attribute is an optional field, and a field left out MUST stay as it is. Its fields are:
  - `name` (`--name`): a blank name MUST clear it, as `rename` clears today.
  - `right_click_passthrough` (`--right-click-passthrough`): `true`, `false` or `toggle`. `toggle` MUST switch to the other value, resolved against the pane's value when the action takes effect.

  It MUST act on the selected pane from keys and menus, and on `--pane` or the caller's pane from the CLI, like other pane actions. A `set` with no field MUST change nothing and succeed.
- **FR-041** A server action `server.tab.set`, and the CLI command `ship tab set`, MUST change a tab's attributes on the same terms as FR-040, acting on the selected tab or `--tab`. Its only field is `name`, with the same blank-clears rule.
- **FR-042** `server.tab.rename`, `server.pane.rename`, `ship tab rename` and `ship pane rename` MUST be removed. A config binding that names a rename action MUST be a config error with its line and column, as any unknown action is.
- **FR-043** Toggling MUST be offered only on attributes with exactly two values.

## Success criteria

- **SC-001** On macOS and Linux, each user story's scenarios pass by hand in at least two terminals per platform, with any terminal-specific gaps recorded. Platforms or terminals not run are reported as unverified.
- **SC-002** A fast border drag across a split running Neovim and a shell feels as responsive as dragging a tmux border, by Cyan's judgment, with no backlog of resizes after the pointer stops. The same holds with 100 ms of delay added each way, where the border may lag but must settle at the pointer's final position.
- **SC-003** Every existing click behavior in the mouse-input and selection-clipboard specs still works, apart from sidebar clicks acting on release (FR-005) and border presses not selecting (FR-007): selecting panes and rows, fold markers, text selection, and mouse input to programs.
- **SC-004** A menu opens on the frame after the right-click on a local server, and keyboard navigation in it feels instant.
- **SC-005** The only server or `ship-core` changes are FR-008 and FR-037 to FR-042, and whatever the server must publish so a client can tell whether a pane's program has asked for mouse input. Any other one is brought to Cyan before it's made.
- **SC-006** The change adds about 650 to 900 lines of implementation Rust in the client and about 100 to 160 in the server and `ship-core`. Going well past that is a signal to revisit the design, not a hard limit.

## Open unknowns

None. These choices were made while drafting and approved with the paper:

- An empty list (`pane = []`) for "no menu", rather than `pane = {}` (FR-028).
- Which pane a right-click on a shared border belongs to (FR-030).
- The button's look on the `tabs` heading (FR-031). This paper only requires that it is there and opens the global menu.

## Carried forward from mouse-drag

| Old | New | Change |
| --- | --- | --- |
| FR-001 to FR-003 | FR-001 to FR-003 | FR-003 also covers menus. FR-004 is new: drags take their turn in input order. |
| FR-004, FR-005 | FR-006, FR-007 | FR-007 now holds even while input waits. |
| FR-006, FR-007 | — | Replaced by A-1 in the old paper. |
| FR-008 to FR-010 | FR-011 to FR-013 | None. |
| FR-011 to FR-017 | FR-014 to FR-019 | `client.sidebar.resize`'s CLI rule folded into FR-018. |
| FR-018 | FR-020 | The depth range is narrower: from the row below's depth to one more than the row above's. |
| FR-019 to FR-021, FR-023, FR-024 | FR-021 to FR-023, FR-025, FR-026 | None. |
| FR-022 | FR-024 | Resolved at the drop's turn in input order, not at release. |
| A-1 FR-025, FR-026 | FR-008, FR-009 | None. |
| A-1 FR-027 | FR-010 | Positions wait their turn in input order, with newer ones replacing waiting ones, instead of one per frame sent without waiting. Server-selection's Ordered input requires drags to keep their order. |
| SC-001 to SC-003 | SC-001 to SC-003 | SC-002 adds a delayed-link check; SC-003 names the two intended click changes. |
| A-1 SC-006, SC-007 | SC-005, SC-006 | Widened for the menus and the pane setting. |
