> Superseded on 2026-10-13 by `openspec/changes/archive/2026-10-14-mouse/design/product.md`, which carries this change forward with right-click menus against server-owned selection and the ordered input lane.

# Mouse drag

Status: Locked on 2026-10-11, approved by Cyan in Plannotator review ("LGTM"). Written on 2026-10-11 from a design conversation with Cyan. Cyan accepted every recommendation in the interview round ("all recommendations. looks good to me! write it up!"). In the first review Cyan asked for a key action to resize the sidebar like panes, and approved `client.sidebar.resize` bound to `shift-h` and `shift-l` in resize mode ("looks good to me"). The approval doesn't authorize implementation; Cyan requests that separately.

Amendment A-1: Locked on 2026-10-11, approved by Cyan in Plannotator review ("LGTM"). Written the same day. During the architecture discussion Cyan chose absolute border positions over relative steps for border drags ("i think i prefer the amended version. let's go for it!"). A-1 at the end of this paper supersedes FR-006, FR-007, SC-004, SC-005 and the non-goal on server operations. Nothing below this paper exists yet, so the ripple rule reopens nothing else.

## Goal

Ship's mouse can select panes and tabs but can't rearrange anything. This change lets the mouse do three things the keyboard and CLI already can: drag a pane border to resize panes, drag the sidebar's edge to resize the sidebar, and drag a sidebar row to move a tab, including into or out of another tab. Since sidebar width becomes adjustable, it also gets a key action, bound in resize mode beside the pane resizes. It closes the "mouse pane resizing" item in milestone 1 of the product brief and follows what Herdr users expect.

Every drag ends in an existing operation (`pane resize`, `tab move`) or in client-local presentation (sidebar width), so the server doesn't change. Today a press on a sidebar row or a pane always means a click. After this change, a press that moves at least one cell before release becomes a drag, and one that doesn't stays the click it is today.

## User stories

### P1: Resize panes by dragging a border

Given a tab with two or more panes, a user grabs the border between them and drags it, and the panes resize as the pointer moves.

- **Given** a left and a right pane, **when** the user presses on the vertical border between them and drags 5 columns right, **then** the left pane is 5 columns wider and the right pane 5 columns narrower, and both programs redraw at their new size during the drag.
- **Given** the right pane is selected, **when** the user presses on the border and drags, **then** the right pane stays selected.
- **Given** a 2x2 grid, **when** the user presses on the center junction and drags diagonally, **then** both the vertical and the horizontal edge move.
- **Given** a neighbor with 3 columns of content, **when** the user drags the border 10 columns into it, **then** the border stops with the neighbor at 1 column, exactly as `ship pane resize` clamps.

Tested independently by dragging borders in a split tab and checking the result with `ship pane get` and by eye.

### P2: Move a tab by dragging its sidebar row

A user drags a sidebar row to another place in the tree. The row the pointer is over picks the gap above it; the pointer's column picks the depth. A line drawn at the target indent shows where the tab will land.

- **Given** top-level tabs A, B, C, **when** the user drags C's row to the gap above A at A's indent and releases, **then** the order is C, A, B.
- **Given** top-level tabs A and B, **when** the user drags B onto the gap below A with the pointer one indent further right than A, **then** B becomes A's last child, with its subtree and panes, and its programs keep running.
- **Given** B is a child of A, **when** the user drags B to the left edge below the last row, **then** B becomes the last top-level tab.
- **Given** A is folded, **when** a tab is dropped as A's child, **then** A unfolds and the moved row is visible.
- **Given** the user drags A over one of A's own descendants, **then** no drop line is shown, and releasing there changes nothing.
- **Given** a press on a row that never moves a cell, **then** it selects the tab or toggles the fold marker, as today.

Tested independently by dragging rows and checking `ship tab list` against the drop line shown before release.

### P3: Resize the sidebar by dragging its edge

- **Given** the sidebar is shown, **when** the user drags its right divider 10 columns right, **then** the sidebar is 10 columns wider for that client and the pane area shrinks to match.
- **Given** a dragged width, **when** the user saves an unrelated config change, **then** the width stays; **when** `sidebar_width` itself changes, **then** the configured width replaces it.
- **Given** a second client attached, **then** its sidebar width is unaffected.
- **Given** the default keys, **when** the user enters resize mode and presses `shift-l` three times, **then** the sidebar is 3 columns wider, under the same limits and lifetime as a dragged width.
- **Given** a binding `{ client.sidebar.resize = { direction = "left", amount = 5 } }`, **when** the user presses it, **then** the sidebar is 5 columns narrower.

Tested independently by dragging the divider and pressing the resize keys in two attached clients, and editing the config while attached.

## Non-goals

- Closing a pane when its program exits (#16). It gets its own change.
- Dragging panes to swap or move them.
- Scrolling the sidebar, including auto-scroll during a tab drag. Rows past the sidebar's bottom edge stay out of reach, as they are today.
- Writing a dragged sidebar width to the config file, or keeping it after the client exits.
- Hover effects other than the drop line and the held border's color.
- Cancelling a drag with Esc or any other key. Keys keep going to the selected pane.
- Any new server operation or CLI command, and any new key action other than `client.sidebar.resize`.
- Drags with buttons other than the left one.

## Functional requirements

### Drags in general

- **FR-001** A left-button press on a pane border, the sidebar divider or a sidebar row MUST become a drag once the pointer moves at least one cell from where it was pressed. A press released before that MUST act as the click it is today.
- **FR-002** A drag MUST belong to the client that started it and MUST end on release. No other client's view or selection may change because of the drag itself; they see only the resulting server edit.
- **FR-003** With mouse capture off (`[client] mouse = false`), none of these drags may happen.

### Pane borders

- **FR-004** A press on a cell of one border between panes MUST grab that border. A press on a cell where borders meet MUST grab every border through that cell, moving each in its own axis.
- **FR-005** A press on a border MUST NOT change which pane is selected and MUST NOT reach any program.
- **FR-006** While dragging, the grabbed edge MUST follow the pointer through the existing directional edge resize, with the same clamping: no pane may drop below 1 column and 1 row of content.
- **FR-007** The client MUST have at most one resize from a border drag in flight. Pointer movement while one is in flight MUST be folded into the next resize, so the border catches up to where the pointer is rather than replaying each step.
- **FR-008** The layout shown during a drag MUST be the layout the server publishes. The client MUST NOT draw a predicted border position.
- **FR-009** A held border MUST be drawn in the selected-border color until release.
- **FR-010** A lone or zoomed pane has no border, so there MUST be nothing to grab.

### Sidebar width

- **FR-011** A drag on the sidebar's right divider MUST set that client's sidebar width to follow the pointer, with the pane area taking the remaining columns.
- **FR-012** The dragged width MUST stay between 10 columns and the width that leaves the pane area 20 columns. When the terminal is too narrow for both limits, the pane area's minimum MUST win.
- **FR-013** A dragged width MUST apply only to that client, MUST NOT be written to the config file, and MUST last until the client exits or the config's `sidebar_width` value changes. Reloading a config whose `sidebar_width` is unchanged MUST keep it.
- **FR-014** While the sidebar is hidden, there MUST be no divider to drag. Hiding and showing it again MUST keep a dragged width.

### Sidebar width from keys

- **FR-015** A client action `client.sidebar.resize` MUST take a `direction` of `left` or `right` and an optional `amount` in columns, defaulting to 1, and MUST move the sidebar's right edge that way. It MUST follow FR-012 and FR-013 exactly as a dragged width does, and MUST do nothing while the sidebar is hidden.
- **FR-016** The default keys MUST bind `shift-h` and `shift-l` in resize mode to `client.sidebar.resize` left and right, beside the existing `h`, `j`, `k` and `l` pane resizes.
- **FR-017** `client.sidebar.resize` MUST have no CLI command, because the sidebar width is client-local.

### Tab rows

- **FR-018** While a row is dragged, the drop target MUST be the gap above the row under the pointer, or the gap after the last row when the pointer is below it. The pointer's column MUST pick the depth among the depths valid at that gap: becoming a child of the row above, or a sibling at that row's level or any of its ancestors' levels.
- **FR-019** A line MUST be drawn at the drop target's gap, starting at the target depth's indent, for as long as the target is valid.
- **FR-020** A target inside the dragged tab's own subtree, or one that would leave the tab where it already is, MUST show no line, and release there MUST change nothing.
- **FR-021** Release on a valid target MUST perform one tab move to that place, carrying the tab's subtree and panes, as `ship tab move` does. Release outside the sidebar MUST change nothing.
- **FR-022** The target MUST be resolved again on release against the current tree. If the tree has changed so that the target no longer exists or is no longer valid, the drop MUST change nothing.
- **FR-023** Dropping a tab as the child of a folded tab MUST unfold that tab in the dragging client.
- **FR-024** Dragging a row MUST NOT change selection. A selected tab that is moved MUST stay selected.

## Success criteria

- **SC-001** On macOS and Linux, each user story's scenarios pass by hand in at least two terminals per platform, with any terminal-specific gaps recorded. Platforms or terminals not run are reported as unverified.
- **SC-002** A fast border drag across a split running Neovim and a shell feels as responsive as dragging a tmux border, by Cyan's judgment, with no backlog of resizes after the pointer stops.
- **SC-003** Every existing click behavior in the mouse-input and selection-clipboard specs still works: selecting panes and rows, fold markers, text selection, and mouse input to programs.
- **SC-004** No server or `ship-core` change is needed. If one turns out to be needed, it is brought to Cyan before it's made.
- **SC-005** The change adds about 350 to 500 lines of implementation Rust. Going well past that is a signal to revisit the design, not a hard limit.

## Open unknowns

None. The interview round settled the junction rule, live resizing, border presses not selecting, the sidebar width's lifetime and limits, folded and below-last-row drops, cancelling, selection during a drag, mid-drag tree changes, and the held border's color. The first review settled the sidebar key action and its default binding.

## Amendment A-1: absolute border positions

A border drag says where the border should be, not how far to move it. Each message is complete on its own, so the client sends one per frame without waiting for the previous one, and a late or repeated message does no harm. The server's resize already computes an absolute position internally before clamping; A-1 lets the caller supply that position directly through the existing resize operation.

### Superseded

- **FR-006**, **FR-007**: replaced by FR-025 to FR-027.
- **Non-goals**, "Any new server operation or CLI command": now reads "Any new server operation or CLI command other than the `to` option on pane resize (FR-025), and any new key action other than `client.sidebar.resize`."
- **SC-004**: replaced by SC-006.
- **SC-005**: replaced by SC-007.

### Requirements

- **FR-025** `server.pane.resize` and `ship pane resize` MUST accept `to`, a position in cells counted from the tab's left edge for `left` and `right` or its top edge for `up` and `down`, starting at 0, in place of `amount`. With `to`, the edge that `direction` selects, including the opposite-edge fallback at the tab boundary, MUST move so its border is drawn at that position, or as close as the existing clamping allows: no pane below 1 column and 1 row of content. An edge already there MUST change nothing. A request giving both `to` and `amount` MUST be rejected without changing anything, and the CLI MUST exit 2 for it. Without `to`, resize behaves as before.
- **FR-026** While dragging, the grabbed edge MUST follow the pointer through FR-025, naming the edge by a pane beside it and the direction, not by the layout's internal structure, so a layout change made elsewhere mid-drag still moves the same edge while that pane exists. If the pane is gone, the drag MUST end without another resize.
- **FR-027** The client MUST send at most one resize per frame from a border drag, without waiting for earlier ones to finish, and only when the pointer's position along the drag axis has changed since the last one sent. On release it MUST send the final position if that differs from the last one sent.

### Success criteria

- **SC-006** The only server or `ship-core` change is FR-025. Any other one is brought to Cyan before it's made.
- **SC-007** The change adds about 350 to 500 lines of implementation Rust in the client and about 30 to 50 in the server and `ship-core`. Going well past that is a signal to revisit the design, not a hard limit.
