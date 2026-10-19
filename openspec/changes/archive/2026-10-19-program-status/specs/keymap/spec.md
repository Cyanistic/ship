# Spec Delta

## MODIFIED Requirements

### Requirement: Default keys
With no bindings in the file, normal mode SHALL bind `alt-n` (new sibling tab with shell), `alt-shift-n` (new child tab with shell), `alt-x` (close tab), `alt--`/`alt-|` (split down/right), `alt-shift-x` (close pane), `alt-left`/`alt-right` (previous/next visible tab row), `alt-h/j/k/l` (pane focus), `alt-tab` (next pane), `alt-b` (sidebar), `alt-g`/`alt-r` (tabs/resize mode), `alt-f` (zoom), `alt-shift-h/j/k/l` (client directional swap), `alt-pageup`/`alt-pagedown` (scroll the selected pane up/down one page), `alt-a` (`client.agents.next`) and `alt-q` (detach). `alt-1..9` SHALL NOT be bound by default. The default resize mode's `h/j/k/l` SHALL omit `amount`, moving one cell per press. Resize mode SHALL also bind `shift-h` and `shift-l` to `client.sidebar.resize` left and right, with the hints `sidebar left` and `sidebar right`, and `shift-j` and `shift-k` to `client.sidebar.resize` down and up, with the hints `split down` and `split up`.

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

#### Scenario: Scroll defaults
- **WHEN** a user with no config presses `alt-pageup` in a shell pane with a long history, then `alt-pagedown`
- **THEN** the pane scrolls up one page and back down, without either chord reaching the shell

#### Scenario: Sidebar keys in resize mode
- **WHEN** a user with no config presses `alt-r` and then `shift-l` three times
- **THEN** the sidebar is 3 columns wider for that client

#### Scenario: Split keys in resize mode
- **WHEN** a user with no config presses `alt-r` and then `shift-j` twice
- **THEN** the divider between the sidebar's sections moves 2 rows down for that client

#### Scenario: Jump default
- **WHEN** a user with no config has a blocked agent in another tab and presses `alt-a`
- **THEN** the blocked agent's pane is selected, and the chord doesn't reach the program

### Requirement: Sidebar resize action
`client.sidebar.resize` SHALL take a `direction` of `left`, `right`, `up` or `down` and an optional `amount`, defaulting to 1. `left` and `right` SHALL move the sidebar's right edge that many columns under config-file's Sidebar width setting, as a dragged width does. `up` and `down` SHALL move the divider between the sidebar's sections that many rows, as a dragged divider row does, within terminal-client's Sidebar split limits. It SHALL apply at press, SHALL do nothing while the sidebar is hidden, and SHALL have no CLI command. Any other direction SHALL be a config error with its line and column.

#### Scenario: Bound amount
- **WHEN** the file binds `"alt-[" = { action = { client.sidebar.resize = { direction = "left", amount = 5 } } }` and the user presses it
- **THEN** the sidebar is 5 columns narrower

#### Scenario: Vertical direction
- **WHEN** the file binds `"alt-]" = { action = { client.sidebar.resize = { direction = "up", amount = 3 } } }` and the user presses it
- **THEN** the divider between the sidebar's sections moves 3 rows up, unless the tabs section would drop below its limit

#### Scenario: Unknown direction
- **WHEN** the file binds `client.sidebar.resize = { direction = "sideways" }` and the user runs `ship config check`
- **THEN** it reports the bad value with its line and column and exits 1

#### Scenario: Hidden sidebar
- **WHEN** the sidebar is hidden and the user presses `shift-l` in resize mode
- **THEN** nothing changes, and showing the sidebar again shows its earlier width

## ADDED Requirements

### Requirement: Agents action
`client.agents.next = {}` SHALL select the next pane that needs the user, as agent-attention's Jump to what needs the user describes. It SHALL have no CLI command.

#### Scenario: Custom binding
- **WHEN** the file binds `"alt-y" = { action = { client.agents.next = {} } }` and an agent is blocked
- **THEN** pressing `alt-y` selects the blocked agent's pane
