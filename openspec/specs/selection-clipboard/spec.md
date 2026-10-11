# selection-clipboard Specification

## Purpose

Let users select text in a pane with the mouse and copy it to their own clipboard, and let programs' clipboard writes reach the clients viewing them, without letting programs read any clipboard.

## Requirements

### Requirement: Mouse selection
In a pane whose program hasn't asked for mouse input, dragging SHALL select text, a double-click SHALL select a word, and a triple-click SHALL select a line. Dragging past the pane's top or bottom edge SHALL scroll it while selecting.

#### Scenario: Word and line
- **WHEN** a user double-clicks a word in a shell pane, then triple-clicks a line
- **THEN** the word is selected, then the whole line

#### Scenario: Drag past the top edge
- **WHEN** a user drags from a shell's last line up past the pane's top edge
- **THEN** the pane scrolls up and the selection keeps growing

### Requirement: Copy on release
Releasing the button after a selection SHALL copy its text to the clipboard of the client that made it, through its terminal's clipboard escape (OSC 52). A click without a drag SHALL copy nothing.

#### Scenario: Drag and paste elsewhere
- **WHEN** a user drag-selects a line in a shell and releases the button
- **THEN** the line is on that client's system clipboard and no other client's

#### Scenario: Plain click
- **WHEN** a user clicks in the selected shell pane without dragging
- **THEN** the clipboard is unchanged

### Requirement: Shared selection highlight
A selection SHALL be highlighted for every viewer. It SHALL follow its text as output arrives and the pane scrolls, and SHALL end at the next click in that pane, on input to it, or when its text leaves the history. Each pane SHALL have one selection; when two clients drag in one pane, the last drag wins.

#### Scenario: Highlight on two clients
- **WHEN** two clients view a pane and one drag-selects a line
- **THEN** both show the line highlighted

#### Scenario: Typing ends the selection
- **WHEN** a pane has a selection and a user types into it
- **THEN** the highlight disappears

### Requirement: Program clipboard writes
Clipboard writes from a pane's program SHALL reach the clipboard of every client viewing that pane's tab, and of no other client.

#### Scenario: Yank in Neovim
- **WHEN** Neovim yanks to the `+` register in a pane viewed by two clients, with a third client on another tab
- **THEN** the text reaches the first two clients' clipboards and not the third's

### Requirement: No clipboard reads
Programs SHALL NOT be able to read any client's clipboard.

#### Scenario: Clipboard query
- **WHEN** a program prints `\e]52;c;?\a`
- **THEN** it receives no answer
