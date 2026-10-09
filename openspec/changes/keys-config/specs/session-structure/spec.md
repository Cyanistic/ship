# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
