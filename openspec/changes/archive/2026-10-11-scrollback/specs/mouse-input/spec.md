# Spec Delta

## Purpose

Let users drive the client with the mouse: select panes and sidebar rows by clicking, scroll with the wheel, and give the mouse to programs that ask for it.

## ADDED Requirements

### Requirement: Mouse capture
The client SHALL capture the mouse unless `[client] mouse = false`. A saved change SHALL apply like other `[client]` settings. With capture off, Ship SHALL handle no mouse events and the outer terminal SHALL keep its own selection.

#### Scenario: Turn capture off
- **WHEN** a user saves `mouse = false` under `[client]` while attached
- **THEN** the client stops capturing the mouse without a restart and a drag selects text in the outer terminal

### Requirement: Click selects a pane
A click in a pane that isn't selected SHALL select it for that client and SHALL NOT reach its program.

#### Scenario: Click between panes
- **WHEN** a tab has three shell panes and a user clicks each in turn
- **THEN** each becomes selected and no mouse escape sequence reaches any shell

### Requirement: Mouse input to programs
In the selected pane, when its program has asked for mouse input, clicks, drags, releases, motion and the wheel SHALL reach the program in the form it asked for, with coordinates relative to the pane.

#### Scenario: Neovim with the mouse on
- **WHEN** the selected pane runs Neovim with `mouse=a` and a user clicks a line and turns the wheel
- **THEN** Neovim moves its cursor to that line and scrolls its buffer

### Requirement: Wheel without program mouse input
When the program hasn't asked for mouse input, the wheel SHALL scroll the pane's history on the normal screen. On the alternate screen it SHALL send arrow keys when the program has alternate scroll on, and do nothing when it has it off. The wheel over a pane that isn't selected SHALL act on that pane by these rules and by mouse input to programs, without selecting it.

#### Scenario: Wheel in less
- **WHEN** a pane runs `less` on a long file and a user turns the wheel over it
- **THEN** `less` scrolls by receiving arrow keys

#### Scenario: Wheel over another pane
- **WHEN** a user has the left pane selected and turns the wheel over the right pane, which runs a shell with a long history
- **THEN** the right pane's history scrolls and the left pane stays selected

### Requirement: Sidebar clicks
A click on a sidebar row SHALL select that tab as the keyboard does. A click on a row's fold marker SHALL fold or unfold it. The wheel over the sidebar SHALL do nothing.

#### Scenario: Click a row and a fold marker
- **WHEN** a user clicks a tab's row, then clicks the fold marker of a row with children
- **THEN** the first tab is selected as navigation keys would select it, and the second row folds
