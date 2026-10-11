# Spec Delta

## ADDED Requirements

### Requirement: Server scrollback setting
`[server] scrollback_bytes` SHALL limit each pane's history in bytes, with a default of 10,000,000. The server SHALL read it when it starts a pane, so a saved change applies to panes created afterwards. When the file is bad, the server SHALL use the default and log the error.

#### Scenario: Smaller history for new panes
- **WHEN** a user sets `scrollback_bytes = 20000`, saves, and creates a pane on a running server
- **THEN** the new pane keeps about 20 KB of history and existing panes keep theirs

### Requirement: Client mouse setting
`[client] mouse` SHALL turn the client's mouse capture on or off, defaulting to `true`, and SHALL reload like other `[client]` settings.

#### Scenario: Default capture
- **WHEN** a user with no config runs `ship` and clicks a pane
- **THEN** the click selects the pane
