# Spec Delta

## ADDED Requirements

### Requirement: Sidebar resize action
`client.sidebar.resize` SHALL take a `direction` of `left` or `right` and an optional `amount` in columns, defaulting to 1, and SHALL move the sidebar's right edge that way under config-file's Sidebar width setting, as a dragged width does. It SHALL apply at press, SHALL do nothing while the sidebar is hidden, and SHALL have no CLI command. Any other direction SHALL be a config error with its line and column.

#### Scenario: Bound amount
- **WHEN** the file binds `"alt-[" = { action = { client.sidebar.resize = { direction = "left", amount = 5 } } }` and the user presses it
- **THEN** the sidebar is 5 columns narrower

#### Scenario: Vertical direction
- **WHEN** the file binds `client.sidebar.resize = { direction = "up" }` and the user runs `ship config check`
- **THEN** it reports the bad value with its line and column and exits 1

#### Scenario: Hidden sidebar
- **WHEN** the sidebar is hidden and the user presses `shift-l` in resize mode
- **THEN** nothing changes, and showing the sidebar again shows its earlier width

### Requirement: Set actions replace rename
`server.tab.set` and `server.pane.set` SHALL be bindable with the fields of session-structure's Set attributes, written in snake_case, such as `server.pane.set = { right_click_passthrough = "toggle" }`. `server.tab.rename` and `server.pane.rename` SHALL NOT exist, so a binding naming either SHALL be a config error with its line and column, as any unknown action is.

#### Scenario: Toggle from a key
- **WHEN** the file binds `"alt-p" = { action = { server.pane.set = { right_click_passthrough = "toggle" } } }` and the user presses it twice
- **THEN** the selected pane's passthrough turns on and then off

#### Scenario: Old rename binding
- **WHEN** the file binds `"alt-e" = { action = { server.tab.rename = { name = "x" } } }` and the user runs `ship config check`
- **THEN** it reports the unknown action with its line and column and exits 1

## MODIFIED Requirements

### Requirement: Default keys
With no bindings in the file, normal mode SHALL bind `alt-n` (new sibling tab with shell), `alt-shift-n` (new child tab with shell), `alt-x` (close tab), `alt--`/`alt-|` (split down/right), `alt-shift-x` (close pane), `alt-left`/`alt-right` (previous/next visible tab row), `alt-h/j/k/l` (pane focus), `alt-tab` (next pane), `alt-b` (sidebar), `alt-g`/`alt-r` (tabs/resize mode), `alt-f` (zoom), `alt-shift-h/j/k/l` (client directional swap), `alt-pageup`/`alt-pagedown` (scroll the selected pane up/down one page) and `alt-q` (detach). `alt-1..9` SHALL NOT be bound by default. The default resize mode's `h/j/k/l` SHALL omit `amount`, moving one cell per press. Resize mode SHALL also bind `shift-h` and `shift-l` to `client.sidebar.resize` left and right, with the hints `sidebar left` and `sidebar right`.

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
