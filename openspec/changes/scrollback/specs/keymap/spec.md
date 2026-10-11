# Spec Delta

## MODIFIED Requirements

### Requirement: Default keys
With no bindings in the file, normal mode SHALL bind `alt-n` (new sibling tab with shell), `alt-shift-n` (new child tab with shell), `alt-x` (close tab), `alt--`/`alt-|` (split down/right), `alt-shift-x` (close pane), `alt-left`/`alt-right` (previous/next visible tab row), `alt-h/j/k/l` (pane focus), `alt-tab` (next pane), `alt-b` (sidebar), `alt-g`/`alt-r` (tabs/resize mode), `alt-f` (zoom), `alt-shift-h/j/k/l` (client directional swap), `alt-pageup`/`alt-pagedown` (scroll the selected pane up/down one page) and `alt-q` (detach). `alt-1..9` SHALL NOT be bound by default. The default resize mode's `h/j/k/l` SHALL omit `amount`, moving one cell per press.

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
