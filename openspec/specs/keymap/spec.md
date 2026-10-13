# keymap Specification

## Purpose

Let users drive the client from the keyboard through modes of chord bindings, where each binding names a shared server action or a client-only view action, with Alt-chord defaults.

## Requirements

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

### Requirement: Server and client actions
Every binding SHALL name one action, under `server.` or `client.`. A `server.` action SHALL change shared state and SHALL be the `ship` command at the same path without `server.`, taking the same action arguments. A `client.` action SHALL change only that client's view and SHALL be reachable only from keys and the mouse, except `client.pane.swap`, which SHALL resolve direction locally and submit the explicit server swap pair. `client.pane.swap` SHALL have no CLI form. Any action SHALL be bindable in any mode.

#### Scenario: Same action from a key and the CLI
- **WHEN** a user binds `"alt-y" = { action = { server.tab.close = {} } }`, presses it with a tab selected, and on another tab runs `ship tab close --tab <id>`
- **THEN** both tabs are closed the same way

#### Scenario: Resize action in normal mode
- **WHEN** a user binds a `server.pane.resize` action in normal mode
- **THEN** the file loads without error

#### Scenario: Local resolution submits shared swap
- **WHEN** a user invokes `client.pane.swap` right in an unzoomed tab
- **THEN** that client resolves the neighbor using published geometry and local recency, while all viewers observe the resulting server-owned layout edit

### Requirement: Chord spelling
Chords SHALL be case-insensitive and written as crokey parses them: `"alt-X"` means `alt-x`, Shift is written as `shift-`, and modifier order does not matter. When two spellings of one chord appear in the same mode, one of them SHALL win; which one is unspecified.

#### Scenario: Two spellings of one chord
- **WHEN** a mode binds both `"alt-n"` and `"Alt-N"`
- **THEN** the file loads and `alt-n` runs one of the two bindings

### Requirement: Overriding defaults
A chord in the file SHALL replace the default binding on that chord in that mode, and other default chords SHALL stay bound. `{}` SHALL unbind a chord. `clear_defaults = true` SHALL drop all of a mode's default bindings before the file's apply.

#### Scenario: Unbind a default
- **WHEN** the file binds `"alt-q" = {}` in normal mode and the user presses `alt-q`
- **THEN** the client stays attached and the program receives `alt-q`

#### Scenario: Rebind a default
- **WHEN** the file binds `"alt-n" = { action = { server.tab.create = {} } }` and the user presses `alt-n`
- **THEN** an empty tab is created and the selection stays where it was: the default's `starter` and `select` don't carry over

#### Scenario: Override under another spelling
- **WHEN** the file binds `"Alt-N" = { action = { server.tab.close = {} } }` in normal mode
- **THEN** the file loads without error and `alt-n` closes the selected tab

#### Scenario: Clear a mode
- **WHEN** the file sets `clear_defaults = true` on `resize` and binds only `esc`
- **THEN** in resize mode `h` does nothing and `esc` returns to normal

### Requirement: Modes
`normal` SHALL always exist and SHALL be the mode a client starts in. Setting `kind` on `normal` SHALL change nothing. Users SHALL be able to define modes under `[client.modes.<name>]` with `kind` set to `sticky` or `oneshot`. `client.mode` SHALL enter the named mode. Naming an undefined mode SHALL load, and pressing the key SHALL show the error on the status line and change nothing. The defaults SHALL define sticky `tabs` and `resize` and an empty one-shot `prefix` that no key enters.

#### Scenario: Undefined mode
- **WHEN** a binding is `{ action = { client.mode = "pnaes" } }`, no `pnaes` mode is defined, and the user presses it
- **THEN** the status line says there is no mode named `pnaes` and the client stays in its mode

#### Scenario: tmux-style prefix
- **WHEN** the file binds `"ctrl-b" = { action = { client.mode = "prefix" } }` in normal and `"c" = { action = { server.tab.create = {} } }` in `prefix`, and the user presses `ctrl-b` then `c`
- **THEN** a tab is created and the client is back in normal mode

### Requirement: Unbound keys by mode
In normal mode, an unbound key SHALL reach the selected pane's program. In a sticky mode other than normal, a mode SHALL stay active until another mode is entered, and an unbound key SHALL do nothing. A one-shot mode SHALL return to normal after one key, bound or not, unless that key's action enters a mode.

#### Scenario: Sticky mode swallows keys
- **WHEN** the client is in `tabs` mode and the user presses `x`, which `tabs` does not bind
- **THEN** nothing is sent to the program and the client stays in `tabs`

#### Scenario: One-shot mode exits on an unbound key
- **WHEN** the client is in a one-shot mode and the user presses an unbound key
- **THEN** the client returns to normal and nothing else happens

### Requirement: Key actions act on the selection
A key binding SHALL act on the client's selection as it is at the binding's turn in the client's input order, not at press (see terminal-client's Ordered input). A `server.` action whose target is not selected, such as `server.pane.close` with only a tab selected, SHALL report that on the status line and change nothing. `client.mode`, `client.sidebar.toggle` and `client.config.reload` SHALL apply at press, ahead of any input still waiting.

#### Scenario: Close the selected pane
- **WHEN** a pane is selected and the user presses `alt-shift-x`
- **THEN** that pane closes

#### Scenario: Pane action with a tab selected
- **WHEN** a tab without panes is selected and the user presses `alt-shift-x`
- **THEN** the status line reports that no pane is selected and nothing changes

#### Scenario: Close what the previous key made
- **WHEN** on a slow link the user presses `alt-|` then `alt-shift-x` before the split appears
- **THEN** the new pane closes, not the one that was selected at the first press

#### Scenario: Mode change while input waits
- **WHEN** on a slow link the user presses `alt-n` then `alt-r` before the tab appears
- **THEN** the status line shows `resize` at once, and the tab is created as usual

### Requirement: New tabs from keys
`server.tab.create` from a key with no placement SHALL create the tab after the selected tab under the same parent, or at the end of the top level when nothing is selected. Child placement SHALL append under the selected tab or the selected pane's tab, falling back to the top level with nothing selected. With `starter = "shell"`, the tab SHALL open with one pane running the configured shell. Whether the client selects the new tab or its pane follows Selecting creates from keys.

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

### Requirement: Send a chord
`client.send` SHALL send its chord as if typed at its turn in the client's input order, to the selected pane or the landing pane of a selected tab, without changing the selection.

#### Scenario: Pass the prefix through
- **WHEN** `prefix` binds `"ctrl-b" = { action = { client.send = "ctrl-b" } }`, a pane runs `cat -v`, and the user enters `prefix` and presses `ctrl-b`
- **THEN** the pane shows one `^B`

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

### Requirement: Binding syntax
The documented binding syntax SHALL be a table with an optional `action`, holding exactly one action, and an optional `hint`, the binding's status-line hint: `"alt-r" = { action = { client.mode = "resize" } }`. `{}` SHALL unbind the chord. An action written without `action`, or any other key in the table, SHALL be a config error with its line and column. Strict rejection of positional list forms is not required, per Cyan's slice-1 amendment; their incidental acceptance by Serde is not documented binding syntax.

#### Scenario: Accepted forms
- **WHEN** a file binds `"alt-t" = { action = { server.tab.create = {} } }`, `"alt-y" = { action = { server.tab.close = {} }, hint = "close" }` and `"alt-q" = {}`, and the user runs `ship config check`
- **THEN** it prints nothing and exits 0

#### Scenario: An action without `action`
- **WHEN** a file binds `"alt-t" = { server.tab.create = {} }` and the user runs `ship config check`
- **THEN** it reports the unknown key `server` at that binding's line and column and exits 1

#### Scenario: A misspelled key
- **WHEN** a file binds `"alt-t" = { action = { server.tab.create = {} }, hnt = "x" }` and the user runs `ship config check`
- **THEN** it reports the unknown key `hnt` with its line and column and exits 1

### Requirement: Selecting creates from keys
`server.tab.create` and `server.pane.create` SHALL accept `select = true`, which selects what they create for the pressing client in the same edit: the new pane, or the new tab when it has none. Without it, a create from a key SHALL NOT change any selection. The defaults SHALL set `select = true` on `alt-n`, `alt-shift-n`, `alt--` and `alt-|`. The config SHALL NOT name an attachment.

#### Scenario: Default split lands on the new pane
- **WHEN** a user with no config presses `alt-|` then `alt-f`
- **THEN** the new pane is selected and zoomed

#### Scenario: Create without select
- **WHEN** the file binds `"alt-t" = { action = { server.pane.create = { direction = "down" } } }` and the user presses `alt-t` with a pane selected
- **THEN** a pane is split below it and the original pane stays selected

#### Scenario: Attachment in the config
- **WHEN** a file binds `"alt-t" = { action = { server.tab.create = { select = true, attachment = "…" } } }` and the user runs `ship config check`
- **THEN** it reports the unknown field `attachment` and exits 1
