# session-structure Specification

## Purpose

Let users organize server-owned work into top-level tabs, recursive tabs and panes through explicit CLI commands.

## Requirements

### Requirement: Top-level tabs, recursive tabs and panes
The server SHALL hold an ordered list of top-level tabs, tabs containing ordered child tabs to any depth, and tabs owning panes. There SHALL be no session or workspace entity, and no pane outside a tab. A pane SHALL belong to exactly one tab, SHALL contain no tabs or panes, and SHALL run a program as described by the `pane-programs` capability. A server with no tabs and empty tabs SHALL be valid. All structure is held in memory only.

#### Scenario: Build a nested structure
- **WHEN** a user creates a top-level tab, a child tab under it and two panes under the child tab
- **THEN** `ship tab list` shows the top-level tab containing the child tab, the child tab containing both panes in creation order, and no other entities

#### Scenario: Tab creation starts no process
- **WHEN** a user runs `ship tab create`
- **THEN** the new top-level tab is empty and the server has started no child process for it

### Requirement: Stable tab and pane IDs
Every tab and pane SHALL have an ID of the form `<kind>:<identifier>`, where the kind is `tab` or `pane`. IDs SHALL NOT change on rename, reordering or movement, including when a tab moves to or from the top level. Any argument or request field expecting one kind SHALL reject an ID of another kind or a malformed ID.

#### Scenario: IDs survive rename and move to the top level
- **WHEN** a user renames a nested tab and then moves it, with its child tabs and panes, to the top level
- **THEN** the tab, its descendants and their panes keep the same IDs

#### Scenario: Wrong kind of ID
- **WHEN** a user passes a pane ID where a tab ID is required
- **THEN** the command fails with an error naming the expected kind and changes nothing

### Requirement: Machine-readable creation results
Creating a tab or pane SHALL print the created entity as JSON on stdout, including its ID, in the same representation used for inspection. Callers SHALL NOT need any later update to learn the ID.

#### Scenario: Extract a created ID
- **WHEN** a user runs `ship tab create | jq -r .id`
- **THEN** the output is the new tab's ID, beginning with `tab:`

### Requirement: CLI tab and pane commands
The CLI SHALL create, list or inspect, rename and close tabs and panes, with `ship tab close` and `ship pane close`; `ship tab rm` and `ship pane rm` SHALL NOT exist. Inspecting a tab SHALL include its contained tabs and panes. Closing a tab SHALL remove its entire subtree, including every pane owned by a removed tab. Closing a pane SHALL remove only that pane. Closing the last top-level tab SHALL leave a valid server with no tabs.

#### Scenario: Recursive tab removal
- **WHEN** a user closes a tab that has child tabs and panes
- **THEN** the tab, all its descendants and all their panes disappear, and sibling tabs are unaffected

#### Scenario: Single pane removal
- **WHEN** a user closes one pane in a tab
- **THEN** the tab, its other panes and its child tabs remain

#### Scenario: Remove every tab
- **WHEN** a user closes every top-level tab
- **THEN** `ship tab list` prints `{}` and the server keeps running

#### Scenario: Old command name
- **WHEN** a user runs `ship tab rm --tab <id>`
- **THEN** the CLI rejects the unknown subcommand, exits 2 and nothing changes

### Requirement: Tab placement and one-destination moves
Creating a tab SHALL append it to its parent's children, or to the top level, or place it before or after a sibling under that sibling's parent when given `--before` or `--after`. A tab SHALL be movable, carrying its whole subtree and panes, to exactly one destination: appended under a parent tab, appended at the top level, or placed before or after a sibling, under that sibling's parent. Neither a create nor a move SHALL name both a parent and a sibling. Panes SHALL NOT be moved or reordered independently of their owning tab.

#### Scenario: Reorder siblings
- **WHEN** a user moves the last of three sibling tabs with `--before` naming the first
- **THEN** inspection lists the moved tab first and the other two in their original order

#### Scenario: Move to the top level
- **WHEN** a user runs `ship tab move --tab <id>` on a nested tab with descendants and panes
- **THEN** the subtree appears as the last top-level tab, is gone from its old parent, and its programs keep running

#### Scenario: Place next to a tab at another level
- **WHEN** a user moves a top-level tab with `--after` naming a nested tab
- **THEN** the moved tab sits right after that tab, under the same parent

#### Scenario: Parent and sibling together
- **WHEN** a user runs `ship tab move --tab <id> <parent> --before <sibling>`
- **THEN** the CLI rejects the arguments, exits 2 and nothing moves

#### Scenario: Create next to a sibling
- **WHEN** a user runs `ship tab create --after <id>` where `<id>` is the first of two top-level tabs
- **THEN** the new tab sits second among the top-level tabs

### Requirement: Tab parents
Tab creation SHALL accept a tab as the parent, or no parent for the top level. Pane creation SHALL take its tab as `--tab ID`, defaulting as described in Targets from the current pane, and SHALL reject a pane ID.

#### Scenario: Create at the top level
- **WHEN** a user runs `ship tab create --name notes`
- **THEN** a top-level tab named `notes` is appended after the existing top-level tabs

#### Scenario: Pane under a pane
- **WHEN** a user runs `ship pane create --tab <pane-id>`
- **THEN** the command fails clearly and the structure is unchanged

### Requirement: Optional names
Tab and pane names SHALL be optional. An empty or whitespace-only tab or pane name SHALL mean no name, on create and on rename, and renaming a tab or pane without a name SHALL clear its name. Duplicate tab and pane names SHALL be allowed anywhere, including among top-level tabs.

#### Scenario: Duplicate tab names
- **WHEN** a user creates two top-level tabs with the same name
- **THEN** both tabs exist with distinct IDs

#### Scenario: Unnamed tabs and panes
- **WHEN** a user runs `ship tab create` and `ship pane create --tab <tab>` without names, or with `--name "  "`
- **THEN** both entities are created and their JSON shows `"name": null`

#### Scenario: Clear a name
- **WHEN** a user runs `ship tab rename --tab <id>` or `ship pane rename --pane <id>` with no name, `""` or `"  "`
- **THEN** the entity's name becomes `null`

### Requirement: IDs in the CLI
Every CLI argument naming a tab or pane SHALL take its ID. Commands acting on an existing tab or pane SHALL take it as `--tab ID` or `--pane ID`. Names SHALL NOT be used to look anything up. `ship tab list` SHALL print the top-level tabs and their descendants as JSON keyed by ID, in order. Tab and pane names SHALL be given with `--name` on creation and as an optional argument on rename.

#### Scenario: List the tree
- **WHEN** a user creates two top-level tabs and runs `ship tab list`
- **THEN** both appear in creation order, keyed by their IDs, with their child tabs and panes

#### Scenario: A name is not an ID
- **WHEN** a user runs `ship tab get --tab notes`
- **THEN** the command rejects `notes` as a malformed tab ID and exits 2

### Requirement: Atomic rejection of invalid edits
Invalid operations SHALL fail with an understandable error on stderr and leave the structure exactly as it was. This covers nonexistent targets, wrong parent kinds, and moving a tab into itself or its own descendant, including next to a sibling inside it. An invalid command-line argument, such as an ID of the wrong kind or a move naming both a parent and a sibling, SHALL be rejected before any request and exit 2. Any other failed operation SHALL exit 1.

#### Scenario: Move into own descendant
- **WHEN** a user moves a tab under one of its own descendants
- **THEN** the command fails and `ship tab list` is identical to before the attempt

#### Scenario: Place next to own child
- **WHEN** a user moves a tab with `--before` naming one of its own child tabs
- **THEN** the command fails and nothing moves

#### Scenario: Missing sibling
- **WHEN** a user moves a tab with `--after` naming a tab ID that does not exist
- **THEN** the command fails with a not-found error and nothing moves

### Requirement: Versioned HTTP API
The tab, pane and attachment HTTP routes SHALL be served under `/api/v0` and SHALL address entities by ID only. The existing `/health` route SHALL remain at `/health`. Errors SHALL use HTTP 400 for malformed input, 404 for a missing target, 422 for an invalid structural relationship, and 503 when server machinery is unavailable. Malformed input means a malformed path ID or a request body that is not valid JSON. A JSON request body that does not fit the operation, such as an ID of the wrong kind or a move body naming no destination, SHALL be rejected with 422.

#### Scenario: Prefixed route
- **WHEN** a client sends `GET /api/v0/tabs` to a running server
- **THEN** it receives the ordered top-level tabs with their descendants, while `GET /health` still returns the health response

#### Scenario: Empty move body
- **WHEN** a client sends `POST /api/v0/tabs/{id}/move` with `{}`
- **THEN** it receives 422 and nothing moves, while `{"parent": null}` moves the tab to the top level

### Requirement: Targets from the current pane
Commands that act on a tab SHALL also accept `--pane ID`, meaning the tab holding that pane, and `--tab` SHALL win when both are given. Run inside a Ship pane, `--pane` SHALL default to that pane and `--tab` to the tab holding it. Run outside a pane without the flag, the command SHALL fail, say to pass the flag or run inside a pane, and change nothing. An explicit flag SHALL win over the default.

#### Scenario: Close the pane a command runs in
- **WHEN** a program in pane p5 runs `ship pane close`
- **THEN** p5 closes, whatever any client has selected

#### Scenario: Close the tab a command runs in
- **WHEN** a program in a pane runs `ship tab close`
- **THEN** the tab holding that pane closes

#### Scenario: Outside a pane
- **WHEN** a user outside any Ship pane runs `ship pane close`
- **THEN** the command fails saying to pass `--pane` or run inside a pane, exits 1 and nothing changes

#### Scenario: A tab by one of its panes
- **WHEN** a user runs `ship tab close --pane <p>`
- **THEN** the tab holding `<p>` closes

#### Scenario: An explicit tab inside a pane
- **WHEN** a program in a pane runs `ship tab rename --tab <other> notes`
- **THEN** `<other>` is renamed and the tab holding the pane is unchanged

### Requirement: Tab with a starter shell
`ship tab create --starter` SHALL create the tab with one pane running the configured shell in the server user's home directory, and print the tab. Without `--starter`, the tab SHALL be empty.

#### Scenario: Starter tab
- **WHEN** a user runs `ship tab create --starter`
- **THEN** the printed tab holds one pane running the shell
