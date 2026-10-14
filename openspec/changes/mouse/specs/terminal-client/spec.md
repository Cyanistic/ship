# Spec Delta

## MODIFIED Requirements

### Requirement: Status line
The status line SHALL be a one-row overlay drawn over the bottom row of the pane area, the area right of the sidebar, never over the sidebar. It SHALL NOT change the reported area or any pane's geometry, and SHALL hide the pane cells beneath it only while it is shown. It SHALL be shown only while it has content: outside normal mode, while reconnecting, while the selected pane's program has exited, and while an error is pending. It SHALL NOT show tab labels, pane labels or a session.

Outside normal mode it SHALL show the active mode's name and key hints for that mode: each bound chord in the config file's order, labeled by its binding's `label` when set, else by a word from its action: the mode's name, the direction, the key sent, else the action's name. Hints that don't fit SHALL be cut from the end. It SHALL show a disconnected indicator while reconnecting, and the selected pane's exit status once its program has exited. It SHALL show the latest config error, a key action's failure or "not available yet" until the next key action.

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
- **THEN** the status line appears showing `resize` with `h left  j down  k up  l right  shift-h sidebar left  shift-l sidebar right  esc normal`, and disappears after `esc`

#### Scenario: Overlay leaves sizes alone
- **WHEN** a pane runs `stty size` in normal mode, and again after the user presses `alt-r`
- **THEN** both report the same size, and the status line covers only the pane area's bottom row, not the sidebar

#### Scenario: Custom binding hint
- **WHEN** the file binds `"f" = { action = { server.pane.zoom = {} } }` in `resize` mode and the user enters it
- **THEN** the hints include `f zoom`

#### Scenario: Set label
- **WHEN** the file binds `"f" = { action = { server.pane.zoom = {} }, label = "fill" }` in `resize` mode and the user enters it
- **THEN** the hints include `f fill` and not `f zoom`

### Requirement: Derived labels
An unnamed pane's label SHALL be its program's current title, else its command. An unnamed tab's label SHALL be the last component of its first own pane's current directory in layout order, `~` for the home directory, else that pane's label, else its 1-based position among its siblings. Labels SHALL update as titles and current directories change and SHALL return to the derived form when a name is cleared.

#### Scenario: Label follows the title
- **WHEN** an unnamed pane runs a shell and the user starts Neovim in it
- **THEN** the pane's label changes to Neovim's title, and changes again when Neovim exits and resets it

#### Scenario: Clear a tab name
- **WHEN** a user clears a named tab's name with `ship tab set --tab <id> --name ""`
- **THEN** the tab's label becomes its first own pane's current directory's last component, else that pane's label

#### Scenario: Tab label follows the directory
- **WHEN** an unnamed tab's first pane runs a shell that reports its directory and the user runs `cd ~/Documents/projects/ship`
- **THEN** the tab's sidebar row reads `ship`, reads `~` after `cd ~`, and shows the pane's title while the pane has no current directory

#### Scenario: Swap changes first pane
- **WHEN** a swap places a different pane first in an unnamed tab's layout
- **THEN** the tab's derived label follows that pane without changing either pane's identity

### Requirement: Pane frames and screens
A tab with two or more panes SHALL draw labeled borders around its visible splits with a visually distinct selected-pane border and shared adjoining edges. A lone or zoomed pane SHALL fill its area without a border. Each visible pane SHALL display its own faithful screen, with the selected pane's cursor placed inside its content rectangle. A border held by a drag SHALL be drawn in the selected-border color until release.

#### Scenario: Editor beside shell and agent
- **WHEN** a user runs Neovim, a shell and Pi in three splits, types into each and closes one
- **THEN** each pane keeps its own screen, remaining programs redraw into the reclaimed area, and input and the visible cursor follow the selection

#### Scenario: Alternate screen in one split
- **WHEN** one split enters and exits an alternate-screen program
- **THEN** that split switches cleanly and its neighbors' screens are not disturbed

#### Scenario: Borderless lone pane
- **WHEN** a tab has one pane or is zoomed onto one pane
- **THEN** the visible pane has no border

#### Scenario: Held border
- **WHEN** a user presses and holds a border between two unselected panes
- **THEN** that border is drawn in the selected-border color until release

### Requirement: Tab sidebar presentation
The sidebar SHALL show a plain `tabs` title above one row per visible tab in tree order, indented by depth and using derived labels. Tabs with children SHALL have expanded/folded markers. It SHALL highlight the selected tab or the selected pane's tab, and no row when nothing is selected. A zoomed tab's row SHALL carry a zoom marker; while the sidebar is hidden, no zoom marker is shown. `client.sidebar.toggle` SHALL hide/show it per client while preserving its folds and width and giving freed width to panes. The `tabs` heading SHALL end with a button that opens the global menu (see context-menus). While a row is dragged, the drop line SHALL be an underline on the cells of the row above the gap, from the target depth's indent to the divider; the gap above the first row SHALL use the blank row under the heading.

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

#### Scenario: Heading button
- **WHEN** a client shows its sidebar
- **THEN** the `tabs` heading ends with a button, and the rows below it are unchanged

#### Scenario: Drop line
- **WHEN** a user drags a row over the gap below a top-level tab with the pointer one indent further right
- **THEN** the row above the gap is underlined from that indent to the divider, and no row moves until release

### Requirement: Ordered input
A client's bound actions, menu items, typed keys, pastes, `client.send`, clicks, fold clicks, wheel turns, drags and mouse input to programs SHALL take effect on the server in the order they happened. A border position still waiting its turn SHALL be replaced in place by a newer position for the same edge, as mouse-input's Border drags requires. Anything that depends on the selection, the layout or the folds SHALL be resolved against the state after everything before it has taken effect. The one exception is a server action, which can take effect before keys typed just before it.

#### Scenario: Type right after a new tab
- **WHEN** 500 ms of delay is added each way between client and server, and the user presses `alt-n` then types `echo hi` and Enter at once
- **THEN** `hi` is printed in the new tab's shell, and nothing reaches the pane that was selected before, in ten runs of ten

#### Scenario: Zoom what was just split
- **WHEN** 500 ms of delay is added each way and the user presses `alt-|` then `alt-f` at once
- **THEN** the new pane is the one zoomed, in ten runs of ten

#### Scenario: Idle typing
- **WHEN** nothing is waiting and the user types into a shell on a local server
- **THEN** each key is sent at once and feels no slower than before
