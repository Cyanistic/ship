# terminal-client Specification

## Purpose

Let a user drive server-owned panes from a full-screen client in their own terminal: see the selected pane, type into it, move between panes, and always get their terminal back.

## Requirements

### Requirement: Full-screen client
Bare `ship` SHALL open a full-screen client in the current terminal showing the selected tab's own panes together, or the existing empty-state hint, and a left tab-tree sidebar when shown. The status line SHALL appear only as an overlay, as the Status line requirement says. There SHALL be no tab bar. Child tabs SHALL NOT supply panes for their parent's pane area.

#### Scenario: Attach to a shell
- **WHEN** no server is running and a user runs `ship`
- **THEN** the client shows the starting shell's screen beside the sidebar, using the full height of the terminal with no status line, and the user can type into it at once

#### Scenario: Parent with panes and children
- **WHEN** a user selects a tab with both its own panes and child tabs
- **THEN** the pane area shows only that tab's own panes, with the child tabs available separately in the sidebar

### Requirement: Faithful screens
The client SHALL show screen changes as programs produce them, including colors, styles, Unicode, wide characters, the cursor and full-screen programs. It SHALL NOT leave a stale screen after output stops.

#### Scenario: Full-screen program
- **WHEN** a pane runs Neovim, which switches to its full-screen view and back on exit
- **THEN** the client shows each screen correctly, including colors and the cursor

#### Scenario: Output stops
- **WHEN** a pane runs `yes` and the program is then interrupted
- **THEN** the client's screen matches the program's final output

### Requirement: Input delivery
The client SHALL deliver typed text, special keys such as arrows, Ctrl combinations and function keys, and pastes to the selected pane in the order the user produced them. When a tab with panes is selected, it SHALL deliver them to the pane it would land on for that tab, its zoomed pane, else its last selected pane there, else its first in layout order, and the selection SHALL stay on the tab. A paste SHALL arrive as one paste.

#### Scenario: Paste into an editor
- **WHEN** a user pastes a multi-line indented block into Neovim in insert mode
- **THEN** the block appears exactly as pasted, without added indentation

#### Scenario: Fast typing
- **WHEN** a user types quickly into a shell
- **THEN** the shell receives every key in the order typed

#### Scenario: Typing with a tab selected
- **WHEN** a script selects a tab with two shell panes for a client, and the user types `ls` and Enter
- **THEN** `ls` runs in the tab's landing pane, and the client still has the tab selected

### Requirement: Shared panes
Several clients SHALL be able to view the same pane and send it input at the same time. There SHALL be no exclusive writer.

#### Scenario: Two clients type
- **WHEN** two clients view the same pane and both type
- **THEN** the pane receives input from both and both clients show the same screen

### Requirement: Empty state
When nothing is selected, the client SHALL show the number of top-level tabs and the default key that opens one, such as `3 tabs · alt-right to open one`, or `no tabs · ship tab create` when there are none. When the selection is a tab rather than a pane, the client SHALL show a hint for creating a pane, such as `no pane: ship pane create --tab <tab-id>`.

#### Scenario: Last pane removed
- **WHEN** the last pane of the viewed tab is removed
- **THEN** the client shows the no-pane hint for that tab

#### Scenario: Nothing selected
- **WHEN** a client opens on a server with two top-level tabs
- **THEN** it shows `2 tabs · alt-right to open one` and keys that reach programs are dropped

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

### Requirement: Terminal restoration
The client SHALL restore the user's terminal to its state before opening on every exit path, including detach, server shutdown, panics and signals it can handle. Restoring SHALL include turning mouse capture off.

#### Scenario: Client killed
- **WHEN** the client receives SIGTERM, or panics in a temporary probe build
- **THEN** the outer terminal is usable without running `reset`

#### Scenario: Mouse released on detach
- **WHEN** a user detaches with `alt-q` and drags in the outer terminal
- **THEN** the outer terminal selects text and no mouse escape sequences appear at the shell prompt

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
The sidebar's tabs section SHALL show a plain `tabs` title above one row per visible tab in tree order, indented by depth and using derived labels. Tabs with children SHALL have expanded/folded markers. It SHALL highlight the selected tab or the selected pane's tab, and no row when nothing is selected. A zoomed tab's row SHALL carry a zoom marker; while the sidebar is hidden, no zoom marker is shown. A tab row SHALL end with its rollup icon as agent-attention's Tab rollup describes. `client.sidebar.toggle` SHALL hide/show it per client while preserving its folds, width and split and giving freed width to panes. The `tabs` heading SHALL end with a button that opens the global menu (see context-menus). While a row is dragged, the drop line SHALL be an underline on the cells of the row above the gap, from the target depth's indent to the divider; the gap above the first row SHALL use the blank row under the heading.

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

#### Scenario: Rollup icon
- **WHEN** a pane in a tab's child tab is `working`
- **THEN** both tabs' rows end with the working icon

### Requirement: Scroll indicator
A pane scrolled up from the bottom SHALL show how far it is from the bottom, on its border, or at its content's top right when it has no border. A pane at the bottom SHALL show no indicator. Its exact wording is settled in rendering review.

#### Scenario: Indicator appears and goes
- **WHEN** a user scrolls a bordered shell pane up one page, then types a key
- **THEN** the pane's border shows its distance from the bottom, and the indicator disappears once input returns it to the bottom

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

### Requirement: Failed actions in order
A key action that fails SHALL show its error on the status line, and input after it SHALL still take effect in order.

#### Scenario: Keys after a failure
- **WHEN** a tab without panes is selected, and the user presses `alt-shift-x`, `alt-right` to a shell pane, and types `ls` and Enter
- **THEN** the status line reports that no pane is selected, and `ls` runs in the shell

#### Scenario: Selection target removed during a request
- **WHEN** the client's navigation or click sends a selection request and another caller removes that target before the request is applied
- **THEN** the client's lane treats that selection's 404 as nothing to do, shows no error and continues later input
- **AND** a script making the same request still receives the not-found error

### Requirement: Input across a disconnect or detach
When the connection drops, queued input SHALL be dropped and an in-flight action SHALL NOT be retried after reconnecting. Already-sent input or requests are not undone. Detaching SHALL close the input channel and await its existing input request before ending the attachment, so earlier typing is not abandoned; a signal may interrupt this drain.

#### Scenario: Proxy cut mid-burst
- **WHEN** 500 ms of delay is added each way, the user presses `alt-n` and types, and the connection is cut before the tab appears
- **THEN** after reconnecting, none of the waiting keys reach any pane and the create is not sent again

#### Scenario: Detach after typing
- **WHEN** on a slow link the user types `touch done` and Enter, then presses `alt-q` at once
- **THEN** the file `done` exists after the client has detached

### Requirement: Sidebar sections
Whenever the sidebar is shown, it SHALL hold the tabs section on top and the agents section (see agent-attention) below, with a divider row between them. Each section SHALL scroll on its own when its content overflows, and the tabs section SHALL keep the selected row visible as it does today.

#### Scenario: No agents
- **WHEN** a client shows the sidebar with no pane reporting
- **THEN** the agents section below the divider row shows `no agents`

#### Scenario: Overflow
- **WHEN** a short terminal holds 30 tabs and 20 reporting panes
- **THEN** each section scrolls with the wheel without moving the other

### Requirement: Sidebar split
The split between the sidebar's sections SHALL default to half the sidebar's height. A client MAY move it by dragging the divider row or with `client.sidebar.resize` up or down. The split SHALL stay between 10% and 90% of the height with at least 3 rows for each section, SHALL apply only to that client, and SHALL last until the client exits.

#### Scenario: Default split
- **WHEN** a client shows the sidebar
- **THEN** the tabs section fills the top half and the agents section the bottom half

#### Scenario: Another client
- **WHEN** one client drags its split down
- **THEN** a second client's split is unchanged
