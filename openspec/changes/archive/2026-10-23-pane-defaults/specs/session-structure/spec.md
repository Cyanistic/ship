## ADDED Requirements

### Requirement: Exit behavior setting
Every pane SHALL have an exit behavior of `close`, `keep` or `keep_failed`. It is shared state, the same for every client, shown as `exitBehavior` by `ship pane get` and in `ship tab list` output. `ship pane create --exit-behavior` and `server.pane.create` with `exit_behavior` SHALL set it, else it SHALL come from `[server.defaults.pane]`. A tab's starter pane SHALL take the default. It SHALL survive a server restart as part of the saved tabs.

#### Scenario: Create with an exit behavior
- **WHEN** a script runs `ship pane create --tab <t> --exit-behavior keep -- cargo test`
- **THEN** `ship pane get` shows `exitBehavior: keep` for the new pane

#### Scenario: Starter pane
- **WHEN** the config has no `exit_behavior` and a script runs `ship tab create --starter`
- **THEN** its starter pane shows `exitBehavior: close`

#### Scenario: Bad value
- **WHEN** a script runs `ship pane create --exit-behavior sometimes`
- **THEN** the CLI rejects the value, exits 2 and nothing is created

### Requirement: Setting the exit behavior of an exited pane
Setting the exit behavior of a pane whose program has already exited SHALL store the value and do nothing else.

#### Scenario: Exited pane stays
- **WHEN** a pane with exit behavior `keep` has exited and a script runs `ship pane set --pane <id> --exit-behavior close`
- **THEN** `ship pane get` shows `exitBehavior: close` and the pane remains

## MODIFIED Requirements

### Requirement: Set attributes
`server.tab.set` and `ship tab set`, and `server.pane.set` and `ship pane set`, SHALL change a tab's or a pane's attributes in one edit. Each attribute SHALL be an optional field, and a field left out SHALL stay as it is. A tab's only attribute SHALL be `name` (`--name`). A pane's attributes SHALL be `name` (`--name`), `right_click_passthrough` (`--right-click-passthrough`), `exit_behavior` (`--exit-behavior`) and `scrollback_bytes` (`--scrollback-bytes`). A blank name SHALL clear it. A two-valued attribute SHALL take `true`, `false` or `toggle`. `toggle` SHALL switch to the other value, resolved against the stored value when the edit takes effect. Toggling SHALL be offered only on attributes with exactly two values. From keys and menus, `set` SHALL act on the selected tab or pane. From the CLI it SHALL act on `--tab`, `--pane` or the caller's pane, as other tab and pane commands do. A `set` with no field, or one that changes nothing, SHALL succeed and SHALL NOT change the shared state's revision. Over HTTP, `PATCH /api/v0/tabs/{id}` SHALL take `{ "name"? }` and `PATCH /api/v0/panes/{id}` SHALL take `{ "name"?, "rightClickPassthrough"?, "exitBehavior"?, "scrollbackBytes"? }`. An absent or JSON `null` field SHALL leave its attribute unchanged; a present empty or whitespace-only `name` SHALL clear it. Both SHALL return the entity, and an unknown field SHALL be rejected without changing anything. `rename` SHALL NOT exist as a command or action.

#### Scenario: Two attributes in one edit
- **WHEN** a script runs `ship pane set --pane <id> --name build --right-click-passthrough true`
- **THEN** both change in one edit, and `ship pane get` shows them

#### Scenario: Exit behavior and history in one edit
- **WHEN** a script runs `ship pane set --pane <id> --exit-behavior keep_failed --scrollback-bytes 50000000`
- **THEN** both change in one edit, and `ship pane get` shows `exitBehavior: keep_failed` and `scrollbackBytes: 50000000`

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
Every pane SHALL have a `right_click_passthrough` setting, on or off. It is shared state: one value per pane, the same for every client, shown as `rightClickPassthrough` by `ship pane get` and in `ship tab list` output. `server.pane.create` SHALL accept `right_click_passthrough = true` or `false`, and `ship pane create` the switch `--right-click-passthrough` (on) or `--right-click-passthrough=false`. Otherwise a new pane, including a tab's starter pane, SHALL take `[server.defaults.pane] right_click_passthrough`, which defaults to off. The setting SHALL survive a server restart as part of the saved tabs.

#### Scenario: Create with passthrough
- **WHEN** a script runs `ship pane create --tab <t> --right-click-passthrough -- nvim`
- **THEN** `ship pane get` shows `rightClickPassthrough: true` for the new pane

#### Scenario: Create without passthrough over a default
- **WHEN** the config sets `right_click_passthrough = true` and a script runs `ship pane create --right-click-passthrough=false`
- **THEN** `ship pane get` shows `rightClickPassthrough: false` for the new pane

#### Scenario: Starter pane
- **WHEN** the config has no `right_click_passthrough` and a script runs `ship tab create --starter`
- **THEN** its starter pane shows `rightClickPassthrough: false`

#### Scenario: Survives a restart
- **WHEN** a pane has `rightClickPassthrough: true` and the server restarts
- **THEN** the restored pane shows `rightClickPassthrough: true`
