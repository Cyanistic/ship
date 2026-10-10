# Spec Delta

Zero-content terminal handling is approved in architecture A-4 and program P-1; the slice 1 probe verifies the policy recorded in `../../design.md`. The status line as an overlay over the pane area, shown only when it has something to say, follows product A-3, architecture A-6 and program P-3.

## MODIFIED Requirements

### Requirement: Full-screen client
Bare `ship` SHALL open a full-screen client in the current terminal showing the selected tab's own panes together, or the existing empty-state hint, and a left tab-tree sidebar when shown. The status line SHALL appear only as an overlay, as the Status line requirement says. There SHALL be no tab bar. Child tabs SHALL NOT supply panes for their parent's pane area.

#### Scenario: Attach to a shell
- **WHEN** no server is running and a user runs `ship`
- **THEN** the client shows the starting shell's screen beside the sidebar, using the full height of the terminal with no status line, and the user can type into it at once

#### Scenario: Parent with panes and children
- **WHEN** a user selects a tab with both its own panes and child tabs
- **THEN** the pane area shows only that tab's own panes, with the child tabs available separately in the sidebar

### Requirement: Pane size
A tab's area SHALL follow the smallest reported viewing-client area in each dimension, excluding that client's sidebar. The status line overlay SHALL NOT reduce the area. The server SHALL publish actual pane frame/content geometry and size visible programs to their content rectangles, except that both PTY and emulator dimensions SHALL be clamped independently to at least 1 column and 1 row on startup and resize. Published geometry SHALL NOT be clamped. Cells and cursors SHALL be clipped to actual content; when either content dimension is zero, neither SHALL be drawn. A client viewing another tab SHALL NOT constrain it. Unviewed tabs and zoom-hidden panes SHALL retain their last terminal sizes. Larger clients SHALL draw the shared geometry at the pane area's top-left and mark unused space.

#### Scenario: Resize the terminal
- **WHEN** the only client viewing a split tab resizes its terminal
- **THEN** its panes fit the new tab area with unchanged stored proportions and each visible program redraws at its own content size

#### Scenario: Two clients of different sizes
- **WHEN** two clients of different pane-area sizes view the same tab, and the smaller one then moves to a pane in another tab
- **THEN** both first show identical shared pane geometry at the smaller area with the larger client's unused area marked, and afterwards the tab grows to the larger client's area

#### Scenario: Hide sidebar on smallest viewer
- **WHEN** the smallest viewer hides its sidebar
- **THEN** it reports the freed width and the tab grows within the remaining viewers' constraints

#### Scenario: Hidden programs during zoom
- **WHEN** a tab is zoomed and its viewing area changes
- **THEN** the zoomed pane follows that area while hidden panes continue running at their previous terminal sizes

### Requirement: Status line
The status line SHALL be a one-row overlay drawn over the bottom row of the pane area, the area right of the sidebar, never over the sidebar. It SHALL NOT change the reported area or any pane's geometry, and SHALL hide the pane cells beneath it only while it is shown. It SHALL be shown only while it has content: outside normal mode, while reconnecting, while the selected pane's program has exited, and while an error is pending. It SHALL NOT show tab labels, pane labels or a session.

Outside normal mode it SHALL show the active mode's name and key hints for that mode: each bound chord in the config file's order, labeled from its binding's first action by the action's scalar value when it has one, else the action's last path segment. Hints that don't fit SHALL be cut from the end. It SHALL show a disconnected indicator while reconnecting, and the selected pane's exit status once its program has exited. It SHALL show the latest config error, a key action's failure or "not available yet" until the next key action.

#### Scenario: Hidden in normal mode
- **WHEN** a connected client in normal mode selects a running pane with no pending error
- **THEN** no status line is drawn and the pane's last row is visible

#### Scenario: Exited program
- **WHEN** the selected pane's program exits with code 3
- **THEN** the status line appears showing `exited (3)` and stays while that pane is selected

#### Scenario: Labels without a session
- **WHEN** a client enters `resize` mode with the pane `nvim` selected in a tab named `work`
- **THEN** the status line does not show `work` or `nvim`

#### Scenario: Active mode
- **WHEN** the user presses `alt-r`
- **THEN** the status line appears showing `resize` with `h left  j down  k up  l right  esc normal`, and disappears after `esc`

#### Scenario: Overlay leaves sizes alone
- **WHEN** a pane runs `stty size` in normal mode, and again after the user presses `alt-r`
- **THEN** both report the same size, and the status line covers only the pane area's bottom row, not the sidebar

#### Scenario: Custom binding hint
- **WHEN** the file binds `"f" = { server.pane.zoom = {} }` in `resize` mode and the user enters it
- **THEN** the hints include `f zoom`

### Requirement: Derived labels
An unnamed pane's label SHALL be its program's current title, else its command. An unnamed tab's label SHALL be its first own pane's label in layout order, else its 1-based position among its siblings. Labels SHALL update as titles change and SHALL return to the derived form when a name is cleared.

#### Scenario: Label follows the title
- **WHEN** an unnamed pane runs a shell and the user starts Neovim in it
- **THEN** the pane's label changes to Neovim's title, and changes again when Neovim exits and resets it

#### Scenario: Clear a tab name
- **WHEN** a user renames a named tab with no name
- **THEN** the tab's label becomes its first own pane's label in layout order

#### Scenario: Swap changes first pane
- **WHEN** a swap places a different pane first in an unnamed tab's layout
- **THEN** the tab's derived label follows that pane without changing either pane's identity

## ADDED Requirements

### Requirement: Pane frames and screens
A tab with two or more panes SHALL draw labeled borders around its visible splits with a visually distinct selected-pane border and shared adjoining edges. A lone or zoomed pane SHALL fill its area without a border. Each visible pane SHALL display its own faithful screen, with the selected pane's cursor placed inside its content rectangle.

#### Scenario: Editor beside shell and agent
- **WHEN** a user runs Neovim, a shell and Pi in three splits, types into each and closes one
- **THEN** each pane keeps its own screen, remaining programs redraw into the reclaimed area, and input and the visible cursor follow the selection

#### Scenario: Alternate screen in one split
- **WHEN** one split enters and exits an alternate-screen program
- **THEN** that split switches cleanly and its neighbors' screens are not disturbed

#### Scenario: Borderless lone pane
- **WHEN** a tab has one pane or is zoomed onto one pane
- **THEN** the visible pane has no border

### Requirement: Tab sidebar presentation
The sidebar SHALL show a plain `tabs` title above one row per visible tab in tree order, indented by depth and using derived labels. Tabs with children SHALL have expanded/folded markers. It SHALL highlight the selected tab or the selected pane's tab, and no row when nothing is selected. A zoomed tab's row SHALL carry a zoom marker; while the sidebar is hidden, no zoom marker is shown. `client.sidebar.toggle` SHALL hide/show it per client while preserving folds and giving freed width to panes.

#### Scenario: Nested rows
- **WHEN** a client opens a three-level tab tree and folds an intermediate row
- **THEN** the sidebar retains that row with a folded marker and hides its descendants without hiding their programs

#### Scenario: Toggle preserves presentation
- **WHEN** a client hides its folded sidebar and shows it again
- **THEN** the same folds return and another client's sidebar visibility is unchanged

#### Scenario: Zoom marker
- **WHEN** the selected tab is zoomed
- **THEN** its sidebar row carries a zoom marker, without requiring a pane border

#### Scenario: Empty selection beside sidebar
- **WHEN** nothing is selected or an empty tab is selected
- **THEN** the existing corresponding empty-state hint appears in the pane area beside the sidebar
