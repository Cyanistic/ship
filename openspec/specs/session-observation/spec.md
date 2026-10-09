# session-observation Specification

## Purpose

Let independently running full-screen clients attach to a session, see its live state and pane screens, and keep their own selections without affecting each other.

## Requirements

### Requirement: Attach a client
Bare `ship` SHALL attach a full-screen client to the whole server, as described by the `terminal-client` capability. On attach it SHALL show the server's current state and, once something is selected, the selected pane's current screen, including changes made while it was attaching, without waiting for another change. It SHALL update after each later change. `ship attach` SHALL no longer exist.

#### Scenario: Attach during edits
- **WHEN** a user opens `ship` while another process is creating top-level tabs
- **THEN** the client's state includes every created tab without any further edit

#### Scenario: Quiet pane on attach
- **WHEN** a user opens `ship`, presses `C-b )`, and the first tab's first pane shows a shell prompt and produces no further output
- **THEN** the client shows that prompt at once

#### Scenario: Two observers converge
- **WHEN** two clients view the same pane and a separate CLI process edits the tree
- **THEN** both clients show the same resulting state and screen

### Requirement: First tab from server startup
When bare `ship` starts the local server itself, the server SHALL begin with one top-level tab holding one pane running the user's login shell in the server user's home directory, and the client SHALL open with that tab's pane selected. Opening a server that was already running SHALL NOT create a tab, even when it has none.

#### Scenario: Start on a stopped server
- **WHEN** no server is running and a user runs `ship` from a project directory
- **THEN** a server starts with one tab, and the client opens on its shell, whose working directory is home

#### Scenario: Open an empty running server
- **WHEN** a running server has no tabs and a user runs `ship`
- **THEN** the client shows `no tabs · ship tab create`, and `ship tab list` reads the same before and after

#### Scenario: Two clients start the server together
- **WHEN** two `ship` commands run at the same time with no server running
- **THEN** exactly one tab exists and both clients open on it

### Requirement: Independent optional selection
Each attached client SHALL have its own selection: a tab, a pane, or nothing. A newly opened client SHALL select nothing, except as "First tab from server startup" describes. A client SHALL be able to select an empty tab or a pane. Changing one client's selection SHALL NOT change another's. Each client SHALL show its own selection.

#### Scenario: Open with nothing selected
- **WHEN** a server has three top-level tabs and a user runs `ship`
- **THEN** the client selects nothing and shows `3 tabs · C-b ) to open one`

#### Scenario: Different selections
- **WHEN** two clients are on different panes and one presses `C-b n`
- **THEN** only that client's selection and screen change

### Requirement: Selection repair
When a client's selected pane is removed, its selection SHALL move to the next pane in the same tab, or the previous pane if the removed pane was last, and SHALL fall back to the tab only when no panes remain in it. When a selected tab is removed, or an ancestor of the selection is removed, the selection SHALL move to the nearest surviving ancestor, or to nothing when none survives. When a selected tab or pane moves anywhere on the server, the selection SHALL stay on it.

#### Scenario: Selected pane removed
- **WHEN** a client's selected pane is removed from a tab holding three panes
- **THEN** its selection becomes the next pane in that tab, or the previous one if the removed pane was last

#### Scenario: Last pane in a tab removed
- **WHEN** a client's selected pane is the only pane in its tab and is removed
- **THEN** its selection becomes the tab and the client shows the no-pane hint

#### Scenario: Selected tab's ancestor removed
- **WHEN** an ancestor of a client's selected tab is removed
- **THEN** its selection becomes the nearest surviving ancestor, or nothing if none survives

#### Scenario: Selected tab moved
- **WHEN** a client's selected pane's tab moves to the top level
- **THEN** the client keeps that pane selected

### Requirement: Attachments outlive tabs
An attachment SHALL end only when its client detaches or the server shuts down. Removing tabs, including every tab, SHALL NOT end it.

#### Scenario: Remove every tab
- **WHEN** a client is attached and a user removes every top-level tab
- **THEN** the client stays attached and shows `no tabs · ship tab create`

#### Scenario: Tabs return
- **WHEN** a client shows no tabs and a user then runs `ship tab create` twice
- **THEN** the client's hint updates to `2 tabs · C-b ) to open one` without reattaching

### Requirement: Latest state for slow observers
A client SHALL never be shown state or a screen older than what it has already shown. A client that stops reading SHALL, once it resumes, receive the latest state and the latest screen of each pane it shows, even if no further change occurs. Intermediate states and screens are not guaranteed to be shown.

#### Scenario: Stalled observer resumes
- **WHEN** a client process is suspended, fifty edits are made and its pane prints more output, and the process is resumed
- **THEN** it shows the final state and the pane's final screen without any further change

### Requirement: Compressed observation stream
The observation stream SHALL be served as server-sent events with negotiated zstd compression, with gzip available, and each event SHALL reach the observer without waiting for the stream to end.

#### Scenario: zstd negotiated
- **WHEN** a client requests the observation stream accepting zstd
- **THEN** the response is zstd-encoded and individual events arrive while the stream stays open

### Requirement: Shutdown ends observation
Server shutdown by SIGINT or SIGTERM SHALL end open attach streams so that the server still exits within its five-second shutdown bound. Attached clients SHALL restore the terminal, report that the server stopped and exit.

#### Scenario: Shutdown with observers attached
- **WHEN** the server receives SIGTERM while two clients are attached
- **THEN** it exits within five seconds, and both clients restore their terminals, report that the server stopped and exit

### Requirement: Cycle top-level tabs
`C-b )` SHALL select the next top-level tab and `C-b (` the previous one, in order, wrapping around. With nothing selected they SHALL select the first and last top-level tab. Selecting a tab this way SHALL select its first pane in tree order, or the tab itself when it has no panes. The server SHALL accept a client's change of selection to any existing tab or pane, or to nothing.

#### Scenario: Cycle tabs
- **WHEN** three top-level tabs exist and a client on the first presses `C-b )` three times
- **THEN** the client shows each tab's first pane in turn and returns to the first

#### Scenario: From nothing selected
- **WHEN** a client has nothing selected and presses `C-b (`
- **THEN** the last top-level tab's first pane is selected

#### Scenario: Tab without panes
- **WHEN** a client cycles to a top-level tab with no panes
- **THEN** the tab itself is selected and the client shows the no-pane hint
