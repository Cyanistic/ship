## ADDED Requirements

### Requirement: Pane defaults
`[server.defaults.pane]` SHALL hold what a new pane starts with: `shell`, `scrollback_bytes`, `right_click_passthrough` and `exit_behavior`, defaulting to the login shell, 10,000,000, `false` and `"close"`. The server SHALL read the table each time it creates a pane, so a saved change applies to panes created afterwards and not to existing ones. A value given in the create request SHALL override the table for that pane.

#### Scenario: Change applies to the next pane
- **WHEN** a user sets `exit_behavior = "keep"` in `[server.defaults.pane]`, saves, and creates a pane on a running server
- **THEN** the new pane shows `exitBehavior: keep` and existing panes keep their values

#### Scenario: Create request overrides the table
- **WHEN** the table sets `exit_behavior = "keep"` and a script runs `ship pane create --exit-behavior close`
- **THEN** the new pane shows `exitBehavior: close`

### Requirement: Pane shell setting
`[server.defaults.pane] shell` SHALL name the program a pane runs when created without a command, started as a login shell. A relative path SHALL resolve from the config file's folder. When it is absent, the pane SHALL run the login shell.

#### Scenario: Shell for the next pane
- **WHEN** a user sets `shell = "/bin/sh"` in `[server.defaults.pane]`, saves, and creates a pane without a command on a running server
- **THEN** the new pane runs `/bin/sh` as a login shell, and existing panes are unchanged

### Requirement: Bad file gives built-in pane defaults
When the config file can't be read or has errors, the server SHALL create panes with the built-in pane defaults and log the error.

#### Scenario: Broken file
- **WHEN** the file has a syntax error and a user creates a pane without a command
- **THEN** the pane runs the login shell with the built-in defaults, and the server log records the error

### Requirement: Old server settings are rejected
`[server] shell` and `[server] scrollback_bytes` SHALL no longer be accepted. `ship config check` SHALL report either one as an unknown field with its line and column.

#### Scenario: Leftover shell setting
- **WHEN** a config file still has `shell = "/bin/zsh"` under `[server]` and the user runs `ship config check`
- **THEN** it reports the unknown field `shell` with its line and column and exits 1

## REMOVED Requirements

### Requirement: Server shell setting
**Reason**: The shell moves into `[server.defaults.pane]` with the other settings a new pane starts with.
**Migration**: Move `shell` from `[server]` to `[server.defaults.pane]`. There is no alias.

### Requirement: Server scrollback setting
**Reason**: History size becomes a per-pane setting whose default lives in `[server.defaults.pane]`.
**Migration**: Move `scrollback_bytes` from `[server]` to `[server.defaults.pane]`. There is no alias.
