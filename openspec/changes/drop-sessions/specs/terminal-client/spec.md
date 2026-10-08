# Spec Delta

## MODIFIED Requirements

### Requirement: Full-screen client
Bare `ship` SHALL open a full-screen client in the current terminal that shows the selected pane's screen, or a hint when no pane is selected, and a one-line status line. No other user interface is required.

#### Scenario: Attach to a shell
- **WHEN** no server is running and a user runs `ship`
- **THEN** the client fills the terminal with the starting shell's screen above a status line, and the user can type into it at once

### Requirement: Empty state
When nothing is selected, the client SHALL show the number of top-level tabs and the key that opens one, such as `3 tabs · C-b ) to open one`, or `no tabs · ship tab create` when there are none. When the selection is a tab rather than a pane, the client SHALL show a hint for creating a pane, such as `no pane: ship pane create <tab-id>`, and `C-b n` and `C-b p` SHALL then select the first or last pane of that tab.

#### Scenario: Last pane removed
- **WHEN** the last pane of the viewed tab is removed
- **THEN** the client shows the no-pane hint for that tab

#### Scenario: Nothing selected
- **WHEN** a client opens on a server with two top-level tabs
- **THEN** it shows `2 tabs · C-b ) to open one` and keys other than prefixed ones are dropped

### Requirement: Status line
The status line SHALL show the viewed tab and pane labels, the pane's exit status once its program has exited, and a disconnected indicator while reconnecting. It SHALL NOT show a session.

#### Scenario: Exited program
- **WHEN** the selected pane's program exits with code 3
- **THEN** the status line shows `exited (3)`

#### Scenario: Labels without a session
- **WHEN** a client selects the pane `nvim` in a tab named `work`
- **THEN** the status line begins with `work › nvim`

### Requirement: Terminal restoration
The client SHALL restore the user's terminal to its state before opening on every exit path, including detach, server shutdown, panics and signals it can handle.

#### Scenario: Client killed
- **WHEN** the client receives SIGTERM, or panics in a temporary probe build
- **THEN** the outer terminal is usable without running `reset`
