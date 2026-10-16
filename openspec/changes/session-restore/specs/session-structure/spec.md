# Spec Delta

## MODIFIED Requirements

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
