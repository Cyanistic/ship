# Spec Delta

## ADDED Requirements

### Requirement: Set attributes
`server.tab.set` and `ship tab set`, and `server.pane.set` and `ship pane set`, SHALL change a tab's or a pane's attributes in one edit. Each attribute SHALL be an optional field, and a field left out SHALL stay as it is. A tab's only attribute SHALL be `name` (`--name`). A pane's attributes SHALL be `name` (`--name`) and `right_click_passthrough` (`--right-click-passthrough`). A blank name SHALL clear it. A two-valued attribute SHALL take `true`, `false` or `toggle`. `toggle` SHALL switch to the other value, resolved against the stored value when the edit takes effect. Toggling SHALL be offered only on attributes with exactly two values. From keys and menus, `set` SHALL act on the selected tab or pane. From the CLI it SHALL act on `--tab`, `--pane` or the caller's pane, as other tab and pane commands do. A `set` with no field, or one that changes nothing, SHALL succeed and SHALL NOT change the shared state's revision. Over HTTP, `PATCH /api/v0/tabs/{id}` SHALL take `{ "name"? }` and `PATCH /api/v0/panes/{id}` SHALL take `{ "name"?, "rightClickPassthrough"? }`. An absent or JSON `null` field SHALL leave its attribute unchanged; a present empty or whitespace-only `name` SHALL clear it. Both SHALL return the entity, and an unknown field SHALL be rejected without changing anything. `rename` SHALL NOT exist as a command or action.

#### Scenario: Two attributes in one edit
- **WHEN** a script runs `ship pane set --pane <id> --name build --right-click-passthrough true`
- **THEN** both change in one edit, and `ship pane get` shows them

#### Scenario: Toggle twice
- **WHEN** a script runs `ship pane set --pane <id> --right-click-passthrough toggle` twice
- **THEN** the setting is back to its first value

#### Scenario: Nothing to set
- **WHEN** a script runs `ship pane set --pane <id>` with no attribute
- **THEN** nothing changes, the revision stays, and it exits 0

#### Scenario: Bad switch value
- **WHEN** a script runs `ship pane set --pane <id> --right-click-passthrough maybe`
- **THEN** the CLI rejects the value, exits 2 and nothing changes

#### Scenario: Old command
- **WHEN** a script runs `ship tab rename --tab <id> notes`
- **THEN** the CLI reports an unknown subcommand, exits 2 and nothing changes

### Requirement: Right-click passthrough setting
Every pane SHALL have a `right_click_passthrough` setting, on or off, defaulting to off. It is shared state: one value per pane, the same for every client, shown as `rightClickPassthrough` by `ship pane get` and in `ship tab list` output. `server.pane.create` SHALL accept `right_click_passthrough = true`, and `ship pane create` the switch `--right-click-passthrough`, to create the pane with it on. A tab's first pane SHALL start with it off. The setting SHALL NOT survive a server restart.

#### Scenario: Create with passthrough
- **WHEN** a script runs `ship pane create --tab <t> --right-click-passthrough -- nvim`
- **THEN** `ship pane get` shows `rightClickPassthrough: true` for the new pane

#### Scenario: Starter pane
- **WHEN** a script runs `ship tab create --starter`
- **THEN** its starter pane shows `rightClickPassthrough: false`

## MODIFIED Requirements

### Requirement: CLI tab and pane commands
The CLI SHALL create, list or inspect, set attributes of, and close tabs and panes, with `ship tab close` and `ship pane close`. `ship tab rm`, `ship pane rm`, `ship tab rename` and `ship pane rename` SHALL NOT exist. Inspecting a tab SHALL include its contained tabs and panes. Closing a tab SHALL remove its entire subtree, including every pane owned by a removed tab. Closing a pane SHALL remove only that pane. Closing the last top-level tab SHALL leave a valid server with no tabs.

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

### Requirement: Optional names
Tab and pane names SHALL be optional. An empty or whitespace-only tab or pane name SHALL mean no name, on create and on `set`, and setting a blank name SHALL clear it. Duplicate tab and pane names SHALL be allowed anywhere, including among top-level tabs.

#### Scenario: Duplicate tab names
- **WHEN** a user creates two top-level tabs with the same name
- **THEN** both tabs exist with distinct IDs

#### Scenario: Unnamed tabs and panes
- **WHEN** a user runs `ship tab create` and `ship pane create --tab <tab>` without names, or with `--name "  "`
- **THEN** both entities are created and their JSON shows `"name": null`

#### Scenario: Clear a name
- **WHEN** a user runs `ship tab set --tab <id>` or `ship pane set --pane <id>` with `--name ""` or `--name "  "`
- **THEN** the entity's name becomes `null`

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
- **WHEN** a program in a pane runs `ship tab set --tab <other> --name notes`
- **THEN** `<other>` is named `notes` and the tab holding the pane is unchanged

### Requirement: Directional edge resize
`server.pane.resize` and `ship pane resize` SHALL move the requested edge of the targeted pane in the given direction by `amount` cells, or 1 cell when neither `amount` nor `to` is given. At the tab boundary they SHALL use the opposite-edge fallback in that direction. Given `to` instead of `amount`, they SHALL move that same edge so its border is drawn at `to`, a cell position starting at 0 from the tab's left edge for `left` and `right` or its top edge for `up` and `down`. The move SHALL stop where any pane would have less than 1 column and 1 row of content. An edge that can't move, or is already at `to`, SHALL change nothing. A request giving both `to` and `amount` SHALL be rejected without changing anything, and the CLI SHALL exit 2 for it. The amount and `to` SHALL remain bindable action arguments. There SHALL be no additional grow/shrink flag or resize mode.

#### Scenario: Move a shared edge
- **WHEN** a user runs `ship pane resize --pane <left-pane> --direction right --amount 5` on a two-pane horizontal split with room to move
- **THEN** its right edge moves five columns right, increasing its area and reducing its neighbor's area

#### Scenario: Opposite-edge fallback
- **WHEN** the right pane's right edge is the tab boundary and a caller resizes it right by an explicit amount with room to move
- **THEN** its left edge moves right by that amount and the pane becomes narrower

#### Scenario: Omitted amount
- **WHEN** a user runs `ship pane resize --pane <left-pane> --direction right` without `--amount` on a two-pane horizontal split with room to move
- **THEN** its right edge moves one column right

#### Scenario: Clamped at the minimum
- **WHEN** a caller resizes a pane right by 10 into a neighbor with 3 columns of content
- **THEN** the edge moves 2 columns, leaving the neighbor 1 column of content, and a further resize right changes nothing

#### Scenario: Bind the amount
- **WHEN** a user binds a directional resize with `amount = 5` to a chord outside resize mode and presses it
- **THEN** the selected pane's edge moves by the same amount as the corresponding CLI command

#### Scenario: Absolute position
- **WHEN** a user runs `ship pane resize --pane <left-pane> --direction right --to 40` on a two-pane horizontal split
- **THEN** the border between them is drawn at column 40 of the tab, as `ship tab get`'s geometry shows, and running it again changes nothing

#### Scenario: Absolute from the other side
- **WHEN** a user runs `ship pane resize --pane <right-pane> --direction left --to 40` on the same split
- **THEN** the same border moves to column 40

#### Scenario: Both `to` and `amount`
- **WHEN** a user runs `ship pane resize --pane <p> --direction right --to 5 --amount 2`, or sends both fields in an HTTP body
- **THEN** the request is rejected, the CLI exits 2, and nothing changes
