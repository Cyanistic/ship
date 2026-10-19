# mouse-input Specification

## Purpose

Let users drive the client with the mouse: select panes and sidebar rows by clicking, scroll with the wheel, and give the mouse to programs that ask for it.

## Requirements

### Requirement: Mouse capture
The client SHALL capture the mouse unless `[client] mouse = false`. A saved change SHALL apply like other `[client]` settings. With capture off, Ship SHALL handle no mouse events and the outer terminal SHALL keep its own selection.

#### Scenario: Turn capture off
- **WHEN** a user saves `mouse = false` under `[client]` while attached
- **THEN** the client stops capturing the mouse without a restart and a drag selects text in the outer terminal

### Requirement: Click selects a pane
A click in a pane's contents when the pane isn't selected SHALL select it for that client and SHALL NOT reach its program. While earlier input is still waiting to take effect, a press on any pane's contents, selected or not, SHALL select it and SHALL NOT reach its program. A press on a border SHALL NOT select, as Border drags requires. A right press SHALL follow context-menus' Opening a menu and Right-click passthrough, and follows this rule only when the pane menu is empty.

#### Scenario: Click between panes
- **WHEN** a tab has three shell panes and a user clicks each in turn
- **THEN** each becomes selected and no mouse escape sequence reaches any shell

#### Scenario: Click on the selected pane while input waits
- **WHEN** a pane running Neovim with `mouse=a` is selected, and the user clicks in it while a create is still waiting on a slow link
- **THEN** the click selects that pane and Neovim receives no click

### Requirement: Mouse input to programs
In the selected pane, when its program has asked for mouse input, clicks, drags, releases, motion and the wheel SHALL reach the program in the form it asked for, with coordinates relative to the pane.

#### Scenario: Neovim with the mouse on
- **WHEN** the selected pane runs Neovim with `mouse=a` and a user clicks a line and turns the wheel
- **THEN** Neovim moves its cursor to that line and scrolls its buffer

### Requirement: Wheel without program mouse input
When the program hasn't asked for mouse input, the wheel SHALL scroll the pane's history on the normal screen. On the alternate screen it SHALL send arrow keys when the program has alternate scroll on, and do nothing when it has it off. The wheel over a pane that isn't selected SHALL act on that pane by these rules and by mouse input to programs, without selecting it.

#### Scenario: Wheel in less
- **WHEN** a pane runs `less` on a long file and a user turns the wheel over it
- **THEN** `less` scrolls by receiving arrow keys

#### Scenario: Wheel over another pane
- **WHEN** a user has the left pane selected and turns the wheel over the right pane, which runs a shell with a long history
- **THEN** the right pane's history scrolls and the left pane stays selected

### Requirement: Sidebar clicks
A click on a tab row SHALL select that tab as the keyboard does. A click on a row's fold marker SHALL fold or unfold it. A click on an agent's entry SHALL select its pane. All three SHALL act on release, not on press, and only when the press didn't become a drag, taking their turn in input order at release. The wheel over a sidebar section SHALL scroll that section and nothing else.

#### Scenario: Click a row and a fold marker
- **WHEN** a user clicks a tab's row, then clicks the fold marker of a row with children
- **THEN** the first tab is selected as navigation keys would select it, and the second row folds

#### Scenario: Press that moves
- **WHEN** a user presses a row, moves the pointer a cell and releases over the same row
- **THEN** the press is a drag, so the tab isn't selected and no fold changes

#### Scenario: Click an agent
- **WHEN** a user clicks any line of an agent's entry for a pane in another tab
- **THEN** that pane is selected

#### Scenario: Wheel over the agents section
- **WHEN** the agents section overflows and a user turns the wheel over it
- **THEN** the agents section scrolls, and the tabs section and the selected pane don't

### Requirement: Clicks keep their target
A click on a pane, a sidebar row or a fold marker SHALL take its turn in the client's input order (see terminal-client's Ordered input) and act on what was clicked, not on what is selected at its turn. When that pane or tab is gone by its turn, the click SHALL do nothing and show no error.

#### Scenario: Click during a pending create
- **WHEN** 500 ms of delay is added each way, the user presses `alt-n` and at once clicks a pane in the current tab
- **THEN** the tab is created, and the client ends on the clicked pane

#### Scenario: Clicked pane closed first
- **WHEN** a user clicks a pane while earlier input is waiting, and another client closes that pane before the click's turn
- **THEN** the click does nothing and the status line shows no error

### Requirement: Drags
A left-button press on a pane border, the sidebar divider or a sidebar row SHALL become a drag once the pointer moves at least one cell from where it was pressed. A press released before that SHALL act as a click. A drag SHALL belong to the client that started it and SHALL end on release. No other client's view or selection may change because of the drag itself; they see only the resulting server edit. Drag results SHALL take their turn in the client's input order. With `[client] mouse = false`, no drag SHALL happen. Drags with other buttons and cancelling a drag with a key are not provided.

#### Scenario: Press without motion
- **WHEN** a user presses on a pane border and releases without moving a cell
- **THEN** nothing changes

#### Scenario: Another client watches
- **WHEN** one client drags a border while a second client views the same tab
- **THEN** the second client's selection is unchanged and it shows the resized layout as the server publishes it

### Requirement: Border drags
A press on a cell of one border between panes SHALL grab that border. A press on a cell where borders meet SHALL grab every border through that cell, each moving in its own axis. A left press on a border SHALL NOT change which pane is selected and SHALL NOT reach any program, including while earlier input is still waiting. A lone or zoomed pane has no border, so there SHALL be nothing to grab. While dragging, each grabbed edge SHALL follow the pointer through `server.pane.resize` with `to`, naming the edge by a pane beside it and a direction, so a layout change made elsewhere mid-drag still moves the same edge while that pane exists. If the pane is gone, the drag SHALL end without another resize. A border drag SHALL have at most one position per edge waiting its turn: a newer position SHALL replace a waiting one, and a position equal to the last one sent SHALL NOT be sent. On release, the final position SHALL be sent if it differs from the last one sent. The layout shown during a drag SHALL be the layout the server publishes; the client SHALL NOT draw a predicted border position.

#### Scenario: Drag a vertical border
- **WHEN** a user presses on the vertical border between a left and a right pane and drags 5 columns right
- **THEN** the left pane is 5 columns wider and the right pane 5 columns narrower, and both programs redraw at their new size during the drag

#### Scenario: Selection stays
- **WHEN** the right pane is selected, a delayed create is still waiting, and the user presses on the border and drags
- **THEN** the right pane stays selected and no program receives the press

#### Scenario: Junction
- **WHEN** a user presses on the center junction of a 2x2 grid and drags diagonally
- **THEN** both the vertical and the horizontal edge move

#### Scenario: Clamped
- **WHEN** a user drags a border 10 columns into a neighbor with 3 columns of content
- **THEN** the border stops with the neighbor at 1 column of content

#### Scenario: Slow link
- **WHEN** 100 ms of delay is added each way and a user drags a border quickly and stops
- **THEN** the border trails the pointer, settles at the pointer's final position, and no backlog of resizes follows

#### Scenario: Pane closed mid-drag
- **WHEN** a script closes the grabbed edge's pane during a drag
- **THEN** the drag ends and no error is shown

### Requirement: Sidebar divider drag
A drag on the sidebar's right divider SHALL set that client's sidebar width to follow the pointer, with the pane area taking the remaining columns, under config-file's Sidebar width setting. While the sidebar is hidden there SHALL be no divider to drag.

#### Scenario: Widen the sidebar
- **WHEN** the sidebar is shown and a user drags its divider 10 columns right
- **THEN** the sidebar is 10 columns wider for that client, the pane area shrinks to match, and a second client's sidebar is unchanged

### Requirement: Tab row drags
While a row is dragged, the drop target SHALL be the gap above the row under the pointer, or the gap after the last row when the pointer is below it. The pointer's column, in indent steps, SHALL pick a depth from the depth of the row below the gap (0 after the last row) to one more than the depth of the row above it, clamped to that range. The deepest depth SHALL mean a child of the row above. A shallower depth SHALL mean after the row above's ancestor at that depth. A line SHALL be drawn at the target's gap, starting at the target depth's indent, while the target is valid. A target inside the dragged tab's own subtree, or one that would leave the tab where it is, SHALL show no line, and release there SHALL change nothing. Release on a valid target SHALL perform one tab move to that place, carrying the tab's subtree and panes, as `ship tab move` does. Release outside the sidebar SHALL change nothing. The drop SHALL be resolved at its turn against the tree as it is then. If the target no longer exists or is no longer valid, the drop SHALL change nothing and show no error. Dropping a tab as the child of a folded tab SHALL unfold that tab in the dragging client at the drop's turn, after the validity re-check and before sending the move. If the move then fails, that parent SHALL remain unfolded. Dragging a row SHALL NOT change selection, and a selected tab that is moved SHALL stay selected.

#### Scenario: Reorder top-level tabs
- **WHEN** top-level tabs A, B, C exist and the user drags C's row to the gap above A at A's indent and releases
- **THEN** the order is C, A, B

#### Scenario: Make a child
- **WHEN** the user drags B onto the gap below A with the pointer one indent further right than A
- **THEN** B becomes A's last child, with its subtree and panes, and its programs keep running

#### Scenario: Back to the top level
- **WHEN** B is a child of A and the user drags B to the left edge below the last row
- **THEN** B becomes the last top-level tab

#### Scenario: Into a folded tab
- **WHEN** A is folded and a tab is dropped as A's child
- **THEN** A unfolds and the moved row is visible

#### Scenario: Into its own subtree
- **WHEN** the user drags A over one of A's own descendants
- **THEN** no line is shown, and releasing there changes nothing

#### Scenario: Target gone before its turn
- **WHEN** a script closes the target's parent before a waiting drop's turn
- **THEN** the drop changes nothing and no error is shown

### Requirement: Right-click passthrough
A right press on the contents of a pane whose `right_click_passthrough` is on SHALL reach its program as mouse input, without a menu and without changing selection, whenever the program has asked for mouse input. The drag and release that follow SHALL reach the same program. When the program hasn't asked for mouse input, the right press SHALL open the pane menu as with passthrough off. A right press on the pane's label or border SHALL open the pane menu either way.

#### Scenario: Program gets the right-click
- **WHEN** a pane runs Neovim with `mouse=a`, its passthrough is on, and the user right-clicks its contents
- **THEN** Neovim receives the right-click, no menu opens, and selection doesn't change

#### Scenario: Label still opens the menu
- **WHEN** the user right-clicks that pane's label or border
- **THEN** the pane menu opens, and picking `right-click passthrough` turns passthrough off

#### Scenario: Shell at its prompt
- **WHEN** a passthrough pane runs a shell at its prompt and the user right-clicks its contents
- **THEN** the pane menu opens

#### Scenario: Shared between clients
- **WHEN** one client turns passthrough on for a pane running Neovim with `mouse=a`
- **THEN** a right-click on that pane's contents in a second client also reaches Neovim

### Requirement: Sidebar split drag
A drag on the divider row between the sidebar's sections SHALL move the split to follow the pointer, within terminal-client's Sidebar split limits.

#### Scenario: Drag past the limit
- **WHEN** a user drags the divider row to the bottom of the sidebar
- **THEN** the split stops at 90% of the height, or higher when that would leave the agents section fewer than 3 rows
