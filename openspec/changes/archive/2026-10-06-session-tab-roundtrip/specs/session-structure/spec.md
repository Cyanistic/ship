# Spec Delta

## Purpose

Let users organize server-owned work into sessions, recursive tabs and metadata-only logical panes through explicit CLI commands, before terminals or layouts exist.

## ADDED Requirements

### Requirement: Sessions, recursive tabs and logical panes
The server SHALL hold sessions containing ordered root tabs, tabs containing ordered child tabs to any depth, and tabs owning named logical panes. There SHALL be no workspace entity. A pane SHALL belong to exactly one tab and SHALL contain no tabs or panes. Empty sessions and tabs SHALL be valid, and creating any entity SHALL NOT create child tabs, panes or terminal processes. All structure is held in memory only.

#### Scenario: Build a nested structure
- **WHEN** a user creates a session, a root tab, a child tab under that tab and two panes under the child tab
- **THEN** inspecting the session shows the root tab containing the child tab, the child tab containing both panes in creation order, and no other entities

#### Scenario: Pane creation starts no process
- **WHEN** a user creates a pane
- **THEN** no terminal or child process is started by the server

### Requirement: Stable kind-prefixed IDs
Every session, tab and pane SHALL have an ID of the form `<kind>:<identifier>`, where the kind is `session`, `tab` or `pane`. IDs SHALL NOT change on rename, reordering or movement, including when a pane's owning tab moves to another session. Any argument or request field expecting one kind SHALL reject an ID of another kind or a malformed ID.

#### Scenario: IDs survive rename and cross-session move
- **WHEN** a user renames a tab and then moves it, with its child tabs and panes, to another session
- **THEN** the tab, its descendants and their panes keep the same IDs

#### Scenario: Wrong kind of ID
- **WHEN** a user passes a session ID where a tab ID is required
- **THEN** the command fails with an error naming the expected kind and changes nothing

### Requirement: Machine-readable creation results
Creating a session, tab or pane SHALL print the created entity as JSON on stdout, including its ID, in the same representation used for inspection. Callers SHALL NOT need any later update to learn the ID.

#### Scenario: Extract a created ID
- **WHEN** a user runs `ship session create work | jq -r .id`
- **THEN** the output is the new session's ID, beginning with `session:`

### Requirement: CLI create, inspect, rename and remove
The CLI SHALL create, list or inspect, rename and remove sessions, tabs and panes. Inspecting a session or tab SHALL include its contained tabs and panes. Removing a session or tab SHALL remove its entire subtree, including every pane owned by a removed tab. Removing a pane SHALL remove only that pane.

#### Scenario: Recursive tab removal
- **WHEN** a user removes a tab that has child tabs and panes
- **THEN** the tab, all its descendants and all their panes disappear, and sibling tabs are unaffected

#### Scenario: Single pane removal
- **WHEN** a user removes one pane from a tab
- **THEN** the tab, its other panes and its child tabs remain

#### Scenario: Session removal
- **WHEN** a user removes a session
- **THEN** the session and all its tabs and panes disappear from inspection

### Requirement: Tab placement and movement
Creating a tab SHALL append it to its parent's children. A tab SHALL be movable to a session or another tab, in the same session or a different one, carrying its whole subtree and panes. Movement SHALL accept placement before or after a named sibling in the destination, and SHALL append when no placement is given. Panes SHALL NOT be moved or reordered independently of their owning tab.

#### Scenario: Reorder siblings
- **WHEN** a user moves the last of three sibling tabs to before the first
- **THEN** inspection lists the moved tab first and the other two in their original order

#### Scenario: Move across sessions
- **WHEN** a user moves a tab with descendants and panes to another session without placement
- **THEN** the subtree appears as the last root tab of the destination and is gone from the source

### Requirement: Session and tab parents
Tab creation and movement SHALL accept either a session or a tab as the parent, distinguished by the kind of the supplied identifier. Pane creation SHALL accept only a tab parent and SHALL reject a session or pane parent.

#### Scenario: Pane under a session
- **WHEN** a user creates a pane with a session as the parent
- **THEN** the command fails clearly and the structure is unchanged

### Requirement: Naming rules
Session names SHALL be non-empty, SHALL NOT contain `:`, and SHALL be unique on the server, including after rename. Duplicate tab and pane names SHALL be allowed anywhere.

#### Scenario: Duplicate session name
- **WHEN** a user creates a session with a name already in use, or renames a session to such a name
- **THEN** the command fails with a conflict error and the structure is unchanged

#### Scenario: Name containing a colon
- **WHEN** a user creates or renames a session to a name containing `:`
- **THEN** the command fails clearly and the structure is unchanged

#### Scenario: Duplicate tab names
- **WHEN** a user creates two tabs with the same name under the same parent
- **THEN** both tabs exist with distinct IDs

### Requirement: Session names in the CLI
Wherever the CLI accepts a session ID, it SHALL also accept a session name. Any argument containing `:` SHALL be treated as an ID and any other argument as a session name. Tab and pane arguments SHALL accept IDs only. Name lookup SHALL NOT change results: output still identifies entities by ID.

#### Scenario: Same session by name and ID
- **WHEN** a user runs `ship session get work` and `ship session get <id of work>`
- **THEN** both print the same entity

#### Scenario: Tab parent by session name
- **WHEN** a user runs `ship tab create work editor`
- **THEN** a root tab named `editor` is created in session `work`

#### Scenario: Unknown session name
- **WHEN** a user names a session that does not exist in a non-attach command
- **THEN** the command fails with a not-found error and creates nothing

### Requirement: Atomic rejection of invalid operations
Invalid operations SHALL fail with an understandable error on stderr and leave the structure exactly as it was. This covers nonexistent targets, wrong parent kinds, moving a tab into itself or its own descendant, placement relative to a tab that is not a sibling in the destination, and the naming violations above. An invalid command-line argument, such as an ID of the wrong kind or a session name containing `:`, SHALL be rejected before any request and exit 2. Any other failed operation SHALL exit 1.

#### Scenario: Move into own descendant
- **WHEN** a user moves a tab under one of its own descendants
- **THEN** the command fails and inspection of every session is identical to before the attempt

#### Scenario: Placement relative to a non-sibling
- **WHEN** a user moves a tab with `--before` naming a tab that is not a child of the destination
- **THEN** the command fails and nothing moves

### Requirement: Versioned HTTP API
The session, tab, pane and attachment HTTP routes SHALL be served under `/api/v0` and SHALL address entities by ID only. The existing `/health` route SHALL remain at `/health`. Errors SHALL use HTTP 400 for malformed input, 404 for a missing target, 409 for a session name conflict, 422 for an invalid structural relationship or placement, and 503 when server machinery is unavailable. Malformed input means a malformed path ID or a request body that is not valid JSON. A JSON request body that does not fit the operation, such as a session name containing `:` or an ID of the wrong kind, SHALL be rejected with 422.

#### Scenario: Prefixed route
- **WHEN** a client sends `GET /api/v0/sessions` to a running server
- **THEN** it receives the ordered session collection, while `GET /health` still returns the health response
