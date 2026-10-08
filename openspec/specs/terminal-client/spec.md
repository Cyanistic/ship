# terminal-client Specification

## Purpose

Let a user drive server-owned panes from a full-screen client in their own terminal: see the selected pane, type into it, move between panes, and always get their terminal back.

## Requirements

### Requirement: Full-screen client
`ship attach <session>` SHALL open a full-screen client in the current terminal that shows the selected pane's screen and a one-line status line. No other user interface is required.

#### Scenario: Attach to a shell
- **WHEN** a user runs `ship attach work` and `work`'s first pane runs a shell
- **THEN** the client fills the terminal with that shell's screen above a status line, and the user can type into it at once

### Requirement: Faithful screens
The client SHALL show screen changes as programs produce them, including colors, styles, Unicode, wide characters, the cursor and full-screen programs. It SHALL NOT leave a stale screen after output stops.

#### Scenario: Full-screen program
- **WHEN** a pane runs Neovim, which switches to its full-screen view and back on exit
- **THEN** the client shows each screen correctly, including colors and the cursor

#### Scenario: Output stops
- **WHEN** a pane runs `yes` and the program is then interrupted
- **THEN** the client's screen matches the program's final output

### Requirement: Input delivery
The client SHALL deliver typed text, special keys such as arrows, Ctrl combinations and function keys, and pastes to the selected pane in the order the user produced them. A paste SHALL arrive as one paste.

#### Scenario: Paste into an editor
- **WHEN** a user pastes a multi-line indented block into Neovim in insert mode
- **THEN** the block appears exactly as pasted, without added indentation

#### Scenario: Fast typing
- **WHEN** a user types quickly into a shell
- **THEN** the shell receives every key in the order typed

### Requirement: Shared panes
Several clients SHALL be able to view the same pane and send it input at the same time. There SHALL be no exclusive writer.

#### Scenario: Two clients type
- **WHEN** two clients view the same pane and both type
- **THEN** the pane receives input from both and both clients show the same screen

### Requirement: Fixed pane keys
In the client, `C-b n` SHALL select the next pane in the tab, `C-b p` the previous one, both wrapping around. `C-b d` SHALL detach. `C-b C-b` SHALL send one `C-b` to the pane. Every key not preceded by `C-b` SHALL go to the pane, and a key after `C-b` that has no binding SHALL be dropped.

#### Scenario: Cycle panes
- **WHEN** a tab has three panes and the user presses `C-b n` three times
- **THEN** the client shows each pane in turn and returns to the first

#### Scenario: Send the prefix
- **WHEN** a pane runs `cat -v` and the user presses `C-b C-b`
- **THEN** the pane shows one `^B`

### Requirement: Empty state
When the selection is a tab or a session rather than a pane, the client SHALL show an empty state with a hint for creating a pane, such as `no pane: ship pane create <tab-id>`. `C-b n` and `C-b p` SHALL then select the first or last pane of the selected tab.

#### Scenario: Last pane removed
- **WHEN** the last pane of the viewed tab is removed
- **THEN** the client shows the empty state and its hint

### Requirement: Pane size
A tab's panes SHALL be sized to the smallest client currently viewing that tab, minus the status line, and SHALL follow when a viewing client resizes its terminal. A client viewing another tab SHALL NOT constrain it. A tab nobody views SHALL keep its last size. A client larger than the tab SHALL draw the pane in its top-left corner and mark the unused area.

#### Scenario: Resize the terminal
- **WHEN** the only client viewing a tab resizes its terminal
- **THEN** the program in the selected pane sees the new size and redraws to fit

#### Scenario: Two clients of different sizes
- **WHEN** two clients of different sizes view the same tab, and the smaller one then moves to a pane in another tab
- **THEN** both first show identical screens at the smaller size with the larger client's unused area marked, and afterwards the tab grows to the larger client's size

### Requirement: Status line
The status line SHALL show the session, tab and pane labels, the pane's exit status once its program has exited, and a disconnected indicator while reconnecting.

#### Scenario: Exited program
- **WHEN** the selected pane's program exits with code 3
- **THEN** the status line shows `exited (3)`

### Requirement: Derived labels
An unnamed pane's label SHALL be its program's current title, else its command. An unnamed tab's label SHALL be its first pane's label, else its 1-based position among its siblings. Labels SHALL update as titles change and SHALL return to the derived form when a name is cleared.

#### Scenario: Label follows the title
- **WHEN** an unnamed pane runs a shell and the user starts Neovim in it
- **THEN** the pane's label changes to Neovim's title, and changes again when Neovim exits and resets it

#### Scenario: Clear a tab name
- **WHEN** a user renames a named tab with no name
- **THEN** the tab's label becomes its first pane's label

### Requirement: Terminal restoration
The client SHALL restore the user's terminal to its state before attaching on every exit path, including detach, session removal, server shutdown, panics and signals it can handle.

#### Scenario: Client killed
- **WHEN** the client receives SIGTERM, or panics in a temporary probe build
- **THEN** the outer terminal is usable without running `reset`
