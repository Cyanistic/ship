# session-observation Specification

## Purpose

Let independently running text clients attach to a session, see its live structure, and keep their own selections without affecting each other.

## Requirements

### Requirement: Attach a text observer
`ship attach <session>` SHALL attach a text observer to the named or identified session. On attach it SHALL report its attachment ID on stderr and print the session's current structure on stdout, including changes committed while it was attaching, without waiting for another edit. It SHALL reprint the structure after each later change. No graphical interface is required.

#### Scenario: Attach during edits
- **WHEN** a user attaches while another process is creating tabs in that session
- **THEN** the observer shows the final structure without any further edit

#### Scenario: Two observers converge
- **WHEN** two observers are attached to one session and a separate CLI process edits it
- **THEN** both observers show the same resulting structure

### Requirement: Attach or create by name
Attaching by session name SHALL attach to the existing session with that name, or create it and then attach if none exists. Attaching by session ID SHALL never create a session and SHALL fail if the ID is unknown. Concurrent attaches by the same new name SHALL result in exactly one session with that name.

#### Scenario: Attach to a missing name
- **WHEN** a user runs `ship attach scratch` and no session named `scratch` exists
- **THEN** a session named `scratch` is created and the observer attaches to it

#### Scenario: Concurrent attach-or-create
- **WHEN** two `ship attach scratch` commands run at the same time with no `scratch` session present
- **THEN** both attach to the same single `scratch` session

#### Scenario: Unknown ID
- **WHEN** a user attaches by a session ID that does not exist
- **THEN** the command fails with a not-found error and no session is created

### Requirement: Independent selection
Each attached observer SHALL have its own selection of the most specific session, tab or pane, starting at the session itself. An observer SHALL be able to select an empty session, an empty tab or a logical pane. Changing one observer's selection SHALL NOT change another's. The observer SHALL mark its own selection in its output.

#### Scenario: Different selections
- **WHEN** two observers of one session select different tabs, and then one selects a pane
- **THEN** each observer's output marks only its own selection

### Requirement: Selection fallback
When an observer's selected entity is removed, its selection SHALL move to the nearest surviving ancestor within the same session, ultimately the session itself. Removing a selected pane SHALL select its owning tab if that tab survives. When a selected tab or pane moves within the same session, the selection SHALL stay on it. When it moves to another session, the observer SHALL stay attached to its original session and select the nearest surviving parent there.

#### Scenario: Selected pane removed
- **WHEN** an observer's selected pane is removed
- **THEN** its selection becomes the pane's owning tab

#### Scenario: Selected tab's ancestor removed
- **WHEN** an ancestor of an observer's selected tab is removed
- **THEN** its selection becomes the nearest surviving ancestor, or the session if none survives

#### Scenario: Selected pane moved to another session
- **WHEN** the tab owning an observer's selected pane moves to another session
- **THEN** the observer remains on its original session and selects that tab's former parent

### Requirement: Switch sessions while attached
A running observer SHALL be able to switch to another session, by name or ID, without restarting. Its selection SHALL then be the new session itself.

#### Scenario: Switch by name
- **WHEN** an attached observer is told to switch to session `api`
- **THEN** it shows `api`'s structure with `api` itself selected

### Requirement: Session removal kicks observers out
When a session is removed, every observer attached to it SHALL report that the session was removed and exit with status 0. It SHALL NOT stay running unattached or attach to another session on its own.

#### Scenario: Remove an observed session
- **WHEN** two observers are attached to a session and a user removes that session
- **THEN** both observers report the removal and exit

### Requirement: Latest state for slow observers
An observer SHALL never be shown state older than state it has already shown. An observer that stops reading SHALL, once it resumes, receive the latest state even if no further change occurs. Intermediate states are not guaranteed to be shown.

#### Scenario: Stalled observer resumes
- **WHEN** an observer process is suspended, fifty edits are made, and the process is resumed
- **THEN** it shows the final structure without any further edit

### Requirement: Compressed observation stream
The observation stream SHALL be served as server-sent events with negotiated zstd compression, with gzip available, and each event SHALL reach the observer without waiting for the stream to end.

#### Scenario: zstd negotiated
- **WHEN** a client requests the observation stream accepting zstd
- **THEN** the response is zstd-encoded and individual events arrive while the stream stays open

### Requirement: Shutdown ends observation
Server shutdown by SIGINT or SIGTERM SHALL end open observation streams so that the server still exits within its five-second shutdown bound. Observers SHALL report the disconnection.

#### Scenario: Shutdown with observers attached
- **WHEN** the server receives SIGTERM while two observers are attached
- **THEN** it exits within five seconds and both observers report disconnection
