# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Send a chord
`client.send` SHALL send its chord as if typed at its turn in the client's input order, to the selected pane or the landing pane of a selected tab, without changing the selection.

#### Scenario: Pass the prefix through
- **WHEN** `prefix` binds `"ctrl-b" = { action = { client.send = "ctrl-b" } }`, a pane runs `cat -v`, and the user enters `prefix` and presses `ctrl-b`
- **THEN** the pane shows one `^B`

## REMOVED Requirements

### Requirement: Action lists
**Reason**: A binding holds one action (keys-config FR-012 and FR-030, amended by server-selection FR-018). The lists in use were a create followed by its starter or a selection, which `starter` and `select = true` now do in one edit, and one action per binding settles error handling and the mixing of client and server actions.
**Migration**: Rewrite `"chord" = [{ … }]` as `"chord" = { action = { … } }`, and `[]` as `{}`. Bind a multi-step sequence as separate keys. Strict list rejection is waived; incidental Serde positional forms are left alone, not supported as multi-action bindings.
