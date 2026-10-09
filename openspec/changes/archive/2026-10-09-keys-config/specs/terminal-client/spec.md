# Spec Delta

## MODIFIED Requirements

### Requirement: Empty state
When nothing is selected, the client SHALL show the number of top-level tabs and the default key that opens one, such as `3 tabs · alt-right to open one`, or `no tabs · ship tab create` when there are none. When the selection is a tab rather than a pane, the client SHALL show a hint for creating a pane, such as `no pane: ship pane create --tab <tab-id>`.

#### Scenario: Last pane removed
- **WHEN** the last pane of the viewed tab is removed
- **THEN** the client shows the no-pane hint for that tab

#### Scenario: Nothing selected
- **WHEN** a client opens on a server with two top-level tabs
- **THEN** it shows `2 tabs · alt-right to open one` and keys that reach programs are dropped

### Requirement: Status line
The status line SHALL show the viewed tab and pane labels, the pane's exit status once its program has exited, and a disconnected indicator while reconnecting. It SHALL NOT show a session. Outside normal mode it SHALL show the active mode's name. It SHALL show the latest config error, a key action's failure or "not available yet" until the next key action.

#### Scenario: Exited program
- **WHEN** the selected pane's program exits with code 3
- **THEN** the status line shows `exited (3)`

#### Scenario: Labels without a session
- **WHEN** a client selects the pane `nvim` in a tab named `work`
- **THEN** the status line begins with `work › nvim`

#### Scenario: Active mode
- **WHEN** the user presses `alt-r`
- **THEN** the status line shows `resize`, and stops showing it after `esc`

## REMOVED Requirements

### Requirement: Fixed pane keys
**Reason**: The `C-b` prefix keys are replaced by configurable modes and Alt-chord defaults.
**Migration**: Use the `keymap` capability. `alt-tab` selects the next pane, `alt-q` detaches, and unbound keys such as `ctrl-b` reach the program. `C-b` habits come back by binding `ctrl-b` to `{ client.mode = "prefix" }`.
