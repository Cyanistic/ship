# Spec Delta

Planning draft. The child-tab argument form, the resize step and the swap targeting forms follow the gate answers in `../../design.md`.

## MODIFIED Requirements

### Requirement: Default keys
With no bindings in the file, normal mode SHALL bind `alt-n` (new sibling tab with shell), `alt-shift-n` (new child tab with shell), `alt-x` (close tab), `alt--`/`alt-|` (split down/right), `alt-shift-x` (close pane), `alt-left`/`alt-right` (previous/next visible tab row), `alt-h/j/k/l` (pane focus), `alt-tab` (next pane), `alt-b` (sidebar), `alt-g`/`alt-r` (tabs/resize mode), `alt-f` (zoom), `alt-shift-h/j/k/l` (client directional swap) and `alt-q` (detach). `alt-1..9` SHALL NOT be bound by default. The default resize mode's `h/j/k/l` SHALL omit `amount`, moving one cell per press.

#### Scenario: Herdr chords on the first run
- **WHEN** a user with no config runs `ship` and presses `alt-n`, `alt-|`, `alt-right`, `alt-left`, `alt-tab` and `alt-q`
- **THEN** a tab with a shell is created and selected, a pane is split right and selected, the selection moves between visible tab rows and panes, and the client detaches

#### Scenario: Unbound key reaches the program
- **WHEN** a pane runs `cat -v` and the user presses `ctrl-b` in normal mode
- **THEN** the pane shows `^B`

#### Scenario: Row-number defaults removed
- **WHEN** a user with no config presses `alt-1` in normal mode with a pane selected
- **THEN** the chord reaches the program rather than attempting row selection

#### Scenario: Zoom and swap defaults
- **WHEN** a user with no config presses `alt-f` and later uses `alt-shift-l` in an unzoomed split tab
- **THEN** `alt-f` invokes `server.pane.zoom`, and `alt-shift-l` invokes `client.pane.swap` to resolve the right neighbor and submit its explicit ID pair

### Requirement: Server and client actions
Every binding SHALL name one action under `server.` or `client.`, a list of such actions, or be `"none"`. A `server.` action SHALL change shared state and SHALL be the `ship` command at the same path without `server.`, taking the same action arguments. A `client.` action SHALL change only that client's view and SHALL be reachable only from keys and the mouse, except `client.pane.swap`, which SHALL resolve direction locally and submit the explicit server swap pair. `client.pane.swap` SHALL have no CLI form. Any action SHALL be bindable in any mode.

#### Scenario: Same action from a key and the CLI
- **WHEN** a user binds `"alt-y" = { server.tab.close = {} }`, presses it with a tab selected, and on another tab runs `ship tab close --tab <id>`
- **THEN** both tabs are closed the same way

#### Scenario: Resize action in normal mode
- **WHEN** a user binds a `server.pane.resize` action in normal mode
- **THEN** the file loads without error

#### Scenario: Local resolution submits shared swap
- **WHEN** a user invokes `client.pane.swap` right in an unzoomed tab
- **THEN** that client resolves the neighbor using published geometry and local recency, while all viewers observe the resulting server-owned layout edit

### Requirement: Action lists
A binding written as a list SHALL run its actions in order, each after the one before has finished and the client shows its result. It SHALL stop at the first action that fails, keep what the earlier actions did, and show the error on the status line. An empty list, or `"none"` inside a list, SHALL be an error.

#### Scenario: Tab with two panes
- **WHEN** the file binds `"alt-m" = [{ server.tab.create.starter = "shell" }, { server.pane.create = {} }]` and the user presses `alt-m`
- **THEN** a new tab is created with two panes running the shell

#### Scenario: A failure stops the list
- **WHEN** the file binds a row-selection action followed by tab creation and the user invokes it while row selection is still unavailable
- **THEN** the status line shows "not available yet" and no tab is created

### Requirement: New tabs from keys
`server.tab.create` from a key with no placement SHALL create the tab after the selected tab under the same parent, or at the end of the top level when nothing is selected, and select it. Child placement SHALL append under the selected tab or the selected pane's tab, falling back to the top level with nothing selected. With `starter = "shell"`, the tab SHALL open with one pane running the configured shell, and that pane SHALL be selected.

#### Scenario: New tab after the selected one
- **WHEN** the first of three top-level tabs is selected and the user presses `alt-n`
- **THEN** a new tab with a shell sits second among the top-level tabs and its pane is selected

#### Scenario: New last child
- **WHEN** a pane's tab already has two children and the user presses `alt-shift-n`
- **THEN** a new tab with a shell is appended as its third child and its pane is selected

#### Scenario: Child key with no selection
- **WHEN** nothing is selected and the user presses `alt-shift-n`
- **THEN** a top-level tab with a shell is created and its pane is selected

### Requirement: Actions before their feature
Every action SHALL be accepted in the config even when its feature isn't built. Using one whose feature isn't built SHALL report "not available yet" and change nothing. Until layout is available, `server.pane.create` SHALL add a shell pane to the selected tab or selected pane's tab and select it, ignoring `direction`. Layout, focus, folding and sidebar actions SHALL work once provided by this change. `client.tab.select` SHALL remain accepted and unavailable, with no default row-number bindings.

#### Scenario: Not available yet
- **WHEN** a user binds `client.tab.select` with a valid row and invokes it
- **THEN** the status line shows "not available yet" and the selection stays unchanged

#### Scenario: Pane creation before layout
- **WHEN** layout is not yet available, a pane is selected and the user presses `alt-|`
- **THEN** a pane running the shell is added to the selected pane's tab and selected

#### Scenario: Pane creation with layout
- **WHEN** layout is available, a pane is selected and the user presses `alt-|`
- **THEN** a pane running the shell is split right from that pane and selected

### Requirement: Pane cycling
`client.pane.next` and `client.pane.prev` SHALL select the next and previous own pane in the viewed tab's layout order, wrapping around. With a tab selected rather than a pane, they SHALL select its first or last own pane. Next from a zoomed pane SHALL first unzoom the tab for everyone.

#### Scenario: Cycle panes
- **WHEN** a tab has three panes and the user presses `alt-tab` three times
- **THEN** selection visits each pane in layout order and returns to the first

#### Scenario: Empty parent with populated child
- **WHEN** an empty tab with a populated child is selected and pane-next is invoked
- **THEN** selection stays on the parent rather than entering the child's panes

## ADDED Requirements

### Requirement: Directional client swap
`client.pane.swap` SHALL resolve its direction using published geometry and client-local recency with focus's tie rules, and send an explicit same-tab pair. In a zoomed tab it SHALL first unzoom the tab for everyone and wait for the unzoomed geometry before resolving; with no neighbor the tab SHALL stay unzoomed. With no neighbor it SHALL make no swap request. `ship pane swap` SHALL require an explicit `--other` pane and SHALL NOT accept a direction.

#### Scenario: Interactive recency tie
- **WHEN** B and C are adjacent to the right of A and the client last selected C before swapping right from A
- **THEN** it sends the pair A and C, not direction or recency

#### Scenario: Direct CLI pair
- **WHEN** the CLI names a source and an explicit other pane
- **THEN** it submits that pair without neighbor resolution or a geometry read

#### Scenario: Swap out of zoom
- **WHEN** a client invokes `client.pane.swap` right from a zoomed pane with a right neighbor
- **THEN** the tab unzooms for every viewer, and the pane is then exchanged with the neighbor found in the unzoomed geometry

#### Scenario: Zoomed swap without a neighbor
- **WHEN** a client invokes `client.pane.swap` left from a zoomed pane at the tab's left edge
- **THEN** the tab unzooms for every viewer and no swap request is sent

#### Scenario: Missing neighbor
- **WHEN** a directional swap has no neighbor in an unzoomed tab
- **THEN** no swap request or layout change occurs

#### Scenario: CLI without a target
- **WHEN** a user runs `ship pane swap` without `--other`
- **THEN** it rejects the arguments before submitting a swap
