# Spec Delta

## ADDED Requirements

### Requirement: Ordered input
A client's bound actions, typed keys, pastes, `client.send`, clicks, fold clicks, wheel turns, drags and mouse input to programs SHALL take effect on the server in the order they happened. Anything that depends on the selection, the layout or the folds SHALL be resolved against the state after everything before it has taken effect. The one exception is a server action, which can take effect before keys typed just before it.

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

### Requirement: Input across a disconnect or detach
When the connection drops, input not yet taken effect SHALL be dropped and SHALL NOT be sent after reconnecting. Detaching SHALL take effect after all input before it.

#### Scenario: Proxy cut mid-burst
- **WHEN** 500 ms of delay is added each way, the user presses `alt-n` and types, and the connection is cut before the tab appears
- **THEN** after reconnecting, none of the waiting keys reach any pane and the create is not sent again

#### Scenario: Detach after typing
- **WHEN** on a slow link the user types `touch done` and Enter, then presses `alt-q` at once
- **THEN** the file `done` exists after the client has detached

## MODIFIED Requirements

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

### Requirement: Status line
The status line SHALL be a one-row overlay drawn over the bottom row of the pane area, the area right of the sidebar, never over the sidebar. It SHALL NOT change the reported area or any pane's geometry, and SHALL hide the pane cells beneath it only while it is shown. It SHALL be shown only while it has content: outside normal mode, while reconnecting, while the selected pane's program has exited, and while an error is pending. It SHALL NOT show tab labels, pane labels or a session.

Outside normal mode it SHALL show the active mode's name and key hints for that mode: each bound chord in the config file's order, labeled by its binding's `hint` when set, else by a word from its action: the mode's name, the direction, the key sent, else the action's name. Hints that don't fit SHALL be cut from the end. It SHALL show a disconnected indicator while reconnecting, and the selected pane's exit status once its program has exited. It SHALL show the latest config error, a key action's failure or "not available yet" until the next key action.

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
- **THEN** the status line appears showing `resize` with `h left  j down  k up  l right  esc normal`, and disappears after `esc`

#### Scenario: Overlay leaves sizes alone
- **WHEN** a pane runs `stty size` in normal mode, and again after the user presses `alt-r`
- **THEN** both report the same size, and the status line covers only the pane area's bottom row, not the sidebar

#### Scenario: Custom binding hint
- **WHEN** the file binds `"f" = { action = { server.pane.zoom = {} } }` in `resize` mode and the user enters it
- **THEN** the hints include `f zoom`

#### Scenario: Set hint
- **WHEN** the file binds `"f" = { action = { server.pane.zoom = {} }, hint = "fill" }` in `resize` mode and the user enters it
- **THEN** the hints include `f fill` and not `f zoom`
