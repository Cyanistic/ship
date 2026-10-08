# Spec Delta

## REMOVED Requirements

### Requirement: Sessions, recursive tabs and logical panes
**Reason**: Panes are no longer metadata only. Every pane now runs a program, so "pane creation starts no process" no longer holds.
**Migration**: Replaced by "Sessions, recursive tabs and panes" below; program behavior is in the `pane-programs` capability.

## ADDED Requirements

### Requirement: Sessions, recursive tabs and panes
The server SHALL hold sessions containing ordered root tabs, tabs containing ordered child tabs to any depth, and tabs owning panes. There SHALL be no workspace entity. A pane SHALL belong to exactly one tab, SHALL contain no tabs or panes, and SHALL run a program as described by the `pane-programs` capability. Empty sessions and tabs SHALL be valid. Creating a session or tab SHALL NOT create child tabs, panes or terminal processes, except that attaching by a new session name creates a starter tab and pane. All structure is held in memory only.

#### Scenario: Build a nested structure
- **WHEN** a user creates a session, a root tab, a child tab under that tab and two panes under the child tab
- **THEN** inspecting the session shows the root tab containing the child tab, the child tab containing both panes in creation order, and no other entities

#### Scenario: Session and tab creation start no process
- **WHEN** a user runs `ship session create work` and then `ship tab create work`
- **THEN** the session and tab are empty and the server has started no child process

## MODIFIED Requirements

### Requirement: Naming rules
Session names SHALL be non-empty after trimming whitespace, SHALL NOT contain `:`, and SHALL be unique on the server, including after rename. Tab and pane names SHALL be optional. An empty or whitespace-only tab or pane name SHALL mean no name, on create and on rename, and renaming a tab or pane without a name SHALL clear its name. Duplicate tab and pane names SHALL be allowed anywhere.

#### Scenario: Duplicate session name
- **WHEN** a user creates a session with a name already in use, or renames a session to such a name
- **THEN** the command fails with a conflict error and the structure is unchanged

#### Scenario: Name containing a colon
- **WHEN** a user creates or renames a session to a name containing `:`
- **THEN** the command fails clearly and the structure is unchanged

#### Scenario: Blank session name
- **WHEN** a user creates or renames a session with an empty or whitespace-only name
- **THEN** the command fails and the structure is unchanged

#### Scenario: Duplicate tab names
- **WHEN** a user creates two tabs with the same name under the same parent
- **THEN** both tabs exist with distinct IDs

#### Scenario: Unnamed tabs and panes
- **WHEN** a user runs `ship tab create work` and `ship pane create <tab>` without names, or with `--name "  "`
- **THEN** both entities are created and their JSON shows `"name": null`

#### Scenario: Clear a name
- **WHEN** a user runs `ship tab rename <id>` or `ship pane rename <id>` with no name, `""` or `"  "`
- **THEN** the entity's name becomes `null`

### Requirement: Session names in the CLI
Wherever the CLI accepts a session ID, it SHALL also accept a session name. Any argument containing `:` SHALL be treated as an ID and any other argument as a session name. Tab and pane arguments SHALL accept IDs only. Name lookup SHALL NOT change results: output still identifies entities by ID. Tab and pane names SHALL be given with `--name` on creation and as an optional argument on rename.

#### Scenario: Same session by name and ID
- **WHEN** a user runs `ship session get work` and `ship session get <id of work>`
- **THEN** both print the same entity

#### Scenario: Tab parent by session name
- **WHEN** a user runs `ship tab create work --name editor`
- **THEN** a root tab named `editor` is created in session `work`

#### Scenario: Unknown session name
- **WHEN** a user names a session that does not exist in a non-attach command
- **THEN** the command fails with a not-found error and creates nothing
