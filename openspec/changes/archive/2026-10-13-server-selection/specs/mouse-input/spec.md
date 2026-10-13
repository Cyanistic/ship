# Spec Delta

## ADDED Requirements

### Requirement: Clicks keep their target
A click on a pane, a sidebar row or a fold marker SHALL take its turn in the client's input order (see terminal-client's Ordered input) and act on what was clicked, not on what is selected at its turn. When that pane or tab is gone by its turn, the click SHALL do nothing and show no error.

#### Scenario: Click during a pending create
- **WHEN** 500 ms of delay is added each way, the user presses `alt-n` and at once clicks a pane in the current tab
- **THEN** the tab is created, and the client ends on the clicked pane

#### Scenario: Clicked pane closed first
- **WHEN** a user clicks a pane while earlier input is waiting, and another client closes that pane before the click's turn
- **THEN** the click does nothing and the status line shows no error

## MODIFIED Requirements

### Requirement: Click selects a pane
A click in a pane that isn't selected SHALL select it for that client and SHALL NOT reach its program. While earlier input is still waiting to take effect, a press on any pane, selected or not, SHALL select it and SHALL NOT reach its program.

#### Scenario: Click between panes
- **WHEN** a tab has three shell panes and a user clicks each in turn
- **THEN** each becomes selected and no mouse escape sequence reaches any shell

#### Scenario: Click on the selected pane while input waits
- **WHEN** a pane running Neovim with `mouse=a` is selected, and the user clicks in it while a create is still waiting on a slow link
- **THEN** the click selects that pane and Neovim receives no click
