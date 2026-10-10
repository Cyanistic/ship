# Spec Delta

The external setting is settled. Its owning Rust type remains open in `../../design.md`.

## ADDED Requirements

### Requirement: Sidebar width setting
`[client] sidebar_width` SHALL set the sidebar's fixed width in columns, defaulting to 26. Labels SHALL NOT resize the sidebar. The setting SHALL use the existing config selection, client reload and error-reporting contract; the server SHALL NOT read it as server configuration.

#### Scenario: Default width
- **WHEN** a client starts without a sidebar-width setting
- **THEN** its shown sidebar uses 26 columns when space permits

#### Scenario: Explicit width
- **WHEN** the config sets `[client] sidebar_width = 40` and is loaded by a client
- **THEN** the sidebar uses the configured 40 columns when space permits and the client reports the remaining pane area

#### Scenario: Label changes do not resize
- **WHEN** a pane title changes the derived label of a sidebar row
- **THEN** the sidebar width remains configured rather than fitting the new label

#### Scenario: Invalid type
- **WHEN** a file sets `[client] sidebar_width = "wide"` and the user runs `ship config check` on it
- **THEN** the command rejects it with the file and `client.sidebar_width` location

#### Scenario: Width reload
- **WHEN** a user changes the width from 26 to 40 and saves a valid config while attached
- **THEN** the client applies the new width without restart and reports its changed pane area
