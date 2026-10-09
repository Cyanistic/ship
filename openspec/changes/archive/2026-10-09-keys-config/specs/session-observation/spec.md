# Spec Delta

## MODIFIED Requirements

### Requirement: Cycle top-level tabs
`client.tab.next` (by default `alt-right`) SHALL select the next top-level tab and `client.tab.prev` (by default `alt-left`) the previous one, in order, wrapping around. With nothing selected they SHALL select the first and last top-level tab. Selecting a tab this way SHALL select its first pane in tree order, or the tab itself when it has no panes. The server SHALL accept a client's change of selection to any existing tab or pane, or to nothing.

#### Scenario: Cycle tabs
- **WHEN** three top-level tabs exist and a client on the first presses `alt-right` three times
- **THEN** the client shows each tab's first pane in turn and returns to the first

#### Scenario: From nothing selected
- **WHEN** a client has nothing selected and presses `alt-left`
- **THEN** the last top-level tab's first pane is selected

#### Scenario: Tab without panes
- **WHEN** a client cycles to a top-level tab with no panes
- **THEN** the tab itself is selected and the client shows the no-pane hint

### Requirement: Attach a client
Bare `ship` SHALL attach a full-screen client to the whole server, as described by the `terminal-client` capability. On attach it SHALL show the server's current state and, once something is selected, the selected pane's current screen, including changes made while it was attaching, without waiting for another change. It SHALL update after each later change. `ship attach` SHALL no longer exist.

#### Scenario: Attach during edits
- **WHEN** a user opens `ship` while another process is creating top-level tabs
- **THEN** the client's state includes every created tab without any further edit

#### Scenario: Quiet pane on attach
- **WHEN** a user opens `ship`, presses `alt-right`, and the first tab's first pane shows a shell prompt and produces no further output
- **THEN** the client shows that prompt at once

#### Scenario: Two observers converge
- **WHEN** two clients view the same pane and a separate CLI process edits the tree
- **THEN** both clients show the same resulting state and screen

### Requirement: Independent optional selection
Each attached client SHALL have its own selection: a tab, a pane, or nothing. A newly opened client SHALL select nothing, except as "First tab from server startup" describes. A client SHALL be able to select an empty tab or a pane. Changing one client's selection SHALL NOT change another's. Each client SHALL show its own selection.

#### Scenario: Open with nothing selected
- **WHEN** a server has three top-level tabs and a user runs `ship`
- **THEN** the client selects nothing and shows `3 tabs · alt-right to open one`

#### Scenario: Different selections
- **WHEN** two clients are on different panes and one presses `alt-tab`
- **THEN** only that client's selection and screen change

### Requirement: Attachments outlive tabs
An attachment SHALL end only when its client detaches or the server shuts down. Removing tabs, including every tab, SHALL NOT end it.

#### Scenario: Remove every tab
- **WHEN** a client is attached and a user removes every top-level tab
- **THEN** the client stays attached and shows `no tabs · ship tab create`

#### Scenario: Tabs return
- **WHEN** a client shows no tabs and a user then runs `ship tab create` twice
- **THEN** the client's hint updates to `2 tabs · alt-right to open one` without reattaching
