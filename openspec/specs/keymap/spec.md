# keymap Specification

## Purpose

Let users drive the client from the keyboard through modes of chord bindings, where each binding names a shared server action or a client-only view action, with Alt-chord defaults.

## Requirements

### Requirement: Default keys
With no bindings in the file, normal mode SHALL bind `alt-n` (new tab with a shell), `alt-x` (close tab), `alt--` and `alt-|` (new pane down and right), `alt-shift-x` (close pane), `alt-left`/`alt-right` (previous and next tab), `alt-1` to `alt-9` (tab by row), `alt-h/j/k/l` (focus pane by direction), `alt-tab` (next pane), `alt-b` (sidebar), `alt-g` and `alt-r` (enter `tabs` and `resize`) and `alt-q` (detach).

#### Scenario: Herdr chords on the first run
- **WHEN** a user with no config runs `ship` and presses `alt-n`, `alt-|`, `alt-right`, `alt-left`, `alt-tab` and `alt-q`
- **THEN** a tab with a shell is created and selected, a pane is added to it, the selection moves between tabs and panes, and the client detaches

#### Scenario: Unbound key reaches the program
- **WHEN** a pane runs `cat -v` and the user presses `ctrl-b` in normal mode
- **THEN** the pane shows `^B`

### Requirement: Server and client actions
Every binding SHALL name one action under `server.` or `client.`, a list of such actions, or be `"none"`. A `server.` action SHALL change shared state and SHALL be the `ship` command at the same path without `server.`, taking the same arguments. A `client.` action SHALL change only that client's view and SHALL be reachable only from keys and the mouse. Any action SHALL be bindable in any mode.

#### Scenario: Same action from a key and the CLI
- **WHEN** a user binds `"alt-y" = { server.tab.close = {} }`, presses it with a tab selected, and on another tab runs `ship tab close --tab <id>`
- **THEN** both tabs are closed the same way

#### Scenario: Resize action in normal mode
- **WHEN** a user binds a `server.pane.resize` action in normal mode
- **THEN** the file loads without error

### Requirement: Action lists
A binding written as a list SHALL run its actions in order, each after the one before has finished and the client shows its result. It SHALL stop at the first action that fails, keep what the earlier actions did, and show the error on the status line. An empty list, or `"none"` inside a list, SHALL be an error.

#### Scenario: Tab with two panes
- **WHEN** the file binds `"alt-m" = [{ server.tab.create.starter = "shell" }, { server.pane.create = {} }]` and the user presses `alt-m`
- **THEN** a new tab is created with two panes running the shell

#### Scenario: A failure stops the list
- **WHEN** the file binds `"alt-f" = [{ server.pane.resize.direction = "left" }, { server.tab.create = {} }]` before resize exists, and the user presses `alt-f`
- **THEN** the status line shows "not available yet" and no tab is created

### Requirement: Chord spelling
Chords SHALL be case-insensitive and written as crokey parses them: `"alt-X"` means `alt-x`, Shift is written as `shift-`, and modifier order does not matter. Two spellings of one chord in the same mode SHALL be an error naming both.

#### Scenario: Two spellings of one chord
- **WHEN** a mode binds both `"alt-n"` and `"Alt-N"`
- **THEN** loading fails with an error naming both as one chord bound twice

#### Scenario: Modifier order
- **WHEN** a mode binds both `"ctrl-alt-x"` and `"alt-ctrl-x"`
- **THEN** loading fails with the same duplicate-chord error

### Requirement: Overriding defaults
A chord in the file SHALL replace the default action on that chord in that mode, and other default chords SHALL stay bound. `"none"` SHALL unbind a chord. `clear_defaults = true` SHALL drop all of a mode's default bindings before the file's apply.

#### Scenario: Unbind a default
- **WHEN** the file binds `"alt-q" = "none"` in normal mode and the user presses `alt-q`
- **THEN** the client stays attached and the program receives `alt-q`

#### Scenario: Rebind a default
- **WHEN** the file binds `"alt-n" = { server.tab.create = {} }` and the user presses `alt-n`
- **THEN** an empty tab is created: the default's `starter` doesn't carry over

#### Scenario: Override under another spelling
- **WHEN** the file binds `"Alt-N" = { server.tab.close = {} }` in normal mode
- **THEN** the file loads without error and `alt-n` closes the selected tab

#### Scenario: Clear a mode
- **WHEN** the file sets `clear_defaults = true` on `resize` and binds only `esc`
- **THEN** in resize mode `h` does nothing and `esc` returns to normal

### Requirement: Modes
`normal` SHALL always exist and SHALL be the mode a client starts in. Setting `kind` on `normal` SHALL be an error. Users SHALL be able to define modes under `[client.modes.<name>]` with `kind` set to `sticky` or `oneshot`. `client.mode` SHALL enter the named mode, and naming an undefined mode SHALL be an error. The defaults SHALL define sticky `tabs` and `resize` and an empty one-shot `prefix` that no key enters.

#### Scenario: Undefined mode
- **WHEN** a binding is `{ client.mode = "pnaes" }` and no `pnaes` mode is defined
- **THEN** loading fails naming the binding and the undefined mode

#### Scenario: tmux-style prefix
- **WHEN** the file binds `"ctrl-b" = { client.mode = "prefix" }` in normal and `"c" = { server.tab.create = {} }` in `prefix`, and the user presses `ctrl-b` then `c`
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
A key binding SHALL act on the client's selection. A `server.` action whose target is not selected, such as `server.pane.close` with only a tab selected, SHALL report that on the status line and change nothing. Key actions SHALL run one at a time in the order pressed.

#### Scenario: Close the selected pane
- **WHEN** a pane is selected and the user presses `alt-shift-x`
- **THEN** that pane closes

#### Scenario: Pane action with a tab selected
- **WHEN** a tab without panes is selected and the user presses `alt-shift-x`
- **THEN** the status line reports that no pane is selected and nothing changes

### Requirement: New tabs from keys
`server.tab.create` from a key SHALL create the tab after the selected tab under the same parent, or at the end of the top level when nothing is selected, and select it. With `starter = "shell"`, the tab SHALL open with one pane running the configured shell, and that pane SHALL be selected.

#### Scenario: New tab after the selected one
- **WHEN** the first of three top-level tabs is selected and the user presses `alt-n`
- **THEN** a new tab with a shell sits second among the top-level tabs and its pane is selected

### Requirement: Actions before their feature
Every action SHALL be accepted in the config even when its feature isn't built. Using one whose feature isn't built SHALL report "not available yet" and change nothing. Until the layout change lands, `server.pane.create` SHALL add a pane running the configured shell to the selected tab, or the selected pane's tab, and select it, ignoring `direction`.

#### Scenario: Not available yet
- **WHEN** the user presses `alt-b` before the sidebar exists
- **THEN** the status line shows "not available yet" and nothing else changes

#### Scenario: Pane creation before layout
- **WHEN** a pane is selected and the user presses `alt-|`
- **THEN** a pane running the shell is added to the selected pane's tab and selected

### Requirement: Pane cycling
`client.pane.next` and `client.pane.prev` SHALL select the next and previous pane in the viewed tab, wrapping around. With a tab selected rather than a pane, they SHALL select its first or last pane.

#### Scenario: Cycle panes
- **WHEN** a tab has three panes and the user presses `alt-tab` three times
- **THEN** the client shows each pane in turn and returns to the first

### Requirement: Send a chord
`client.send` SHALL send its chord to the selected pane as if typed.

#### Scenario: Pass the prefix through
- **WHEN** `prefix` binds `"ctrl-b" = { client.send = "ctrl-b" }`, a pane runs `cat -v`, and the user enters `prefix` and presses `ctrl-b`
- **THEN** the pane shows one `^B`
