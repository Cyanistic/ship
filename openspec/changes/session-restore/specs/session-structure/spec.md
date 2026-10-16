# Spec Delta

## MODIFIED Requirements

### Requirement: Top-level tabs, recursive tabs and panes
The server SHALL hold an ordered list of top-level tabs, tabs containing ordered child tabs to any depth, and tabs owning panes in a shared nested split layout. There SHALL be no session or workspace entity, and no pane outside a tab. A pane SHALL belong to exactly one tab, SHALL contain no tabs or panes, and SHALL run a program as described by the `pane-programs` capability. A server with no tabs and empty tabs SHALL be valid.

#### Scenario: Build a nested structure
- **WHEN** a user creates a top-level tab, a child tab under it and two panes under the child tab
- **THEN** `ship tab list` shows the top-level tab containing the child tab and the child tab's layout containing both panes in layout order, with no session or workspace entity

#### Scenario: Tab creation starts no process
- **WHEN** a user runs `ship tab create`
- **THEN** the new top-level tab is empty and the server has started no child process for it

### Requirement: Right-click passthrough setting
Every pane SHALL have a `right_click_passthrough` setting, on or off, defaulting to off. It is shared state: one value per pane, the same for every client, shown as `rightClickPassthrough` by `ship pane get` and in `ship tab list` output. `server.pane.create` SHALL accept `right_click_passthrough = true`, and `ship pane create` the switch `--right-click-passthrough`, to create the pane with it on. A tab's first pane SHALL start with it off. The setting SHALL survive a server restart as part of the saved tabs.

#### Scenario: Create with passthrough
- **WHEN** a script runs `ship pane create --tab <t> --right-click-passthrough -- nvim`
- **THEN** `ship pane get` shows `rightClickPassthrough: true` for the new pane

#### Scenario: Starter pane
- **WHEN** a script runs `ship tab create --starter`
- **THEN** its starter pane shows `rightClickPassthrough: false`

#### Scenario: Survives a restart
- **WHEN** a pane has `rightClickPassthrough: true` and the server restarts
- **THEN** the restored pane shows `rightClickPassthrough: true`
