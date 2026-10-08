# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Attach a text observer`
- TO: `### Requirement: Attach a client`

## MODIFIED Requirements

### Requirement: Attach a client
`ship attach <session>` SHALL attach a full-screen client to the named or identified session, as described by the `terminal-client` capability. On attach it SHALL show the session's current state and the selected pane's current screen, including changes made while it was attaching, without waiting for another change. It SHALL update after each later change. The text observer and its stdin `select` and `switch` controls SHALL no longer exist.

#### Scenario: Attach during edits
- **WHEN** a user attaches while another process is creating panes in the session's first tab
- **THEN** the client's state includes every created pane without any further edit

#### Scenario: Quiet pane on attach
- **WHEN** a user attaches to a session whose first pane shows a shell prompt and produces no further output
- **THEN** the client shows that prompt at once

#### Scenario: Two observers converge
- **WHEN** two clients are attached to one session, view the same pane, and a separate CLI process edits the session
- **THEN** both clients show the same resulting state and screen

### Requirement: Attach or create by name
Attaching by session name SHALL attach to the existing session with that name, or, if none exists, create it with one unnamed tab holding one pane running the user's login shell, select that pane and attach. Attaching by session ID SHALL never create a session and SHALL fail if the ID is unknown. Concurrent attaches by the same new name SHALL result in exactly one session with that name. `ship session create` SHALL still create an empty session.

#### Scenario: Attach to a missing name
- **WHEN** a user runs `ship attach scratch` and no session named `scratch` exists
- **THEN** a session named `scratch` is created with one tab holding one shell pane, and the client attaches with that pane selected

#### Scenario: Concurrent attach-or-create
- **WHEN** two `ship attach scratch` commands run at the same time with no `scratch` session present
- **THEN** both attach to the same single `scratch` session

#### Scenario: Unknown ID
- **WHEN** a user attaches by a session ID that does not exist
- **THEN** the command fails with a not-found error and no session is created

#### Scenario: Empty session from the CLI
- **WHEN** a user runs `ship session create x` and then `ship attach x`
- **THEN** the session has no tabs and the client shows the empty state

### Requirement: Independent selection
Each attached client SHALL have its own selection of the most specific session, tab or pane. A newly attached client SHALL select the session's first pane in tree order when one exists, and the session otherwise. A client SHALL be able to select an empty session, an empty tab or a pane. Changing one client's selection SHALL NOT change another's. Each client SHALL show its own selection.

#### Scenario: Attach selects the first pane
- **WHEN** a user attaches to an existing session whose first tab in tree order holds two panes
- **THEN** the client selects and shows the first of those panes

#### Scenario: Different selections
- **WHEN** two clients of one session are on different panes and one presses `C-b n`
- **THEN** only that client's selection and screen change

### Requirement: Selection fallback
When a client's selected pane is removed, its selection SHALL move to the next pane in the same tab, or the previous pane if the removed pane was last, and SHALL fall back to the tab only when no panes remain in it. When a selected tab is removed, or an ancestor of the selection is removed, the selection SHALL move to the nearest surviving ancestor within the same session, ultimately the session itself. When a selected tab or pane moves within the same session, the selection SHALL stay on it. When it moves to another session, the client SHALL stay attached to its original session and select the nearest surviving parent there.

#### Scenario: Selected pane removed
- **WHEN** a client's selected pane is removed from a tab holding three panes
- **THEN** its selection becomes the next pane in that tab, or the previous one if the removed pane was last

#### Scenario: Last pane in a tab removed
- **WHEN** a client's selected pane is the only pane in its tab and is removed
- **THEN** its selection becomes the tab and the client shows the empty state

#### Scenario: Selected tab's ancestor removed
- **WHEN** an ancestor of a client's selected tab is removed
- **THEN** its selection becomes the nearest surviving ancestor, or the session if none survives

#### Scenario: Selected pane moved to another session
- **WHEN** the tab owning a client's selected pane moves to another session
- **THEN** the client remains on its original session and selects that tab's former parent

### Requirement: Session removal kicks observers out
When a session is removed, every client attached to it SHALL restore the terminal, report that the session was removed and exit with status 0. It SHALL NOT stay running unattached or attach to another session on its own.

#### Scenario: Remove an observed session
- **WHEN** two clients are attached to a session and a user removes that session
- **THEN** both clients restore their terminals, report the removal and exit

### Requirement: Latest state for slow observers
A client SHALL never be shown state or a screen older than what it has already shown. A client that stops reading SHALL, once it resumes, receive the latest state and the latest screen of each pane it shows, even if no further change occurs. Intermediate states and screens are not guaranteed to be shown.

#### Scenario: Stalled observer resumes
- **WHEN** a client process is suspended, fifty edits are made and its pane prints more output, and the process is resumed
- **THEN** it shows the final state and the pane's final screen without any further change

### Requirement: Shutdown ends observation
Server shutdown by SIGINT or SIGTERM SHALL end open attach streams so that the server still exits within its five-second shutdown bound. Attached clients SHALL restore the terminal, report that the server stopped and exit.

#### Scenario: Shutdown with observers attached
- **WHEN** the server receives SIGTERM while two clients are attached
- **THEN** it exits within five seconds, and both clients restore their terminals, report that the server stopped and exit

## REMOVED Requirements

### Requirement: Switch sessions while attached
**Reason**: The stdin `switch` control is removed with the text observer, so a client can no longer switch to a session by name or ID.
**Migration**: Use `C-b )` and `C-b (` in the client ("Cycle sessions while attached"). Any existing selection can still be set through `PUT /api/v0/attach/view`.

## ADDED Requirements

### Requirement: Cycle sessions while attached
A running client SHALL switch sessions without restarting. `C-b )` SHALL select the next session and `C-b (` the previous one, in creation order, wrapping around. Switching SHALL select the new session's first pane in tree order, or the session itself when it has no panes. The server SHALL accept a client's change of selection to any existing session, tab or pane.

#### Scenario: Cycle sessions
- **WHEN** two sessions exist and a client attached to the first presses `C-b )`
- **THEN** the client shows the second session's first pane

#### Scenario: Session without panes
- **WHEN** a client switches to a session that has no panes
- **THEN** the session itself is selected and the client shows the empty state
