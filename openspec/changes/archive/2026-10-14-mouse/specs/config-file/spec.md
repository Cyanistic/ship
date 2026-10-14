# Spec Delta

## MODIFIED Requirements

### Requirement: Sidebar width setting
`[client] sidebar_width` SHALL set the sidebar's configured width in columns, defaulting to 26. Labels SHALL NOT resize the sidebar. A client MAY replace the configured width with its own through a divider drag or `client.sidebar.resize`. That width SHALL apply only to that client, SHALL NOT be written to the file, and SHALL last until the client exits or the file's `sidebar_width` value changes; reloading a file whose `sidebar_width` is unchanged SHALL keep it. Either width SHALL stay between 10 columns and the width that leaves the pane area 20 columns, and when the terminal is too narrow for both limits the pane area's minimum SHALL win. The setting SHALL use the existing config selection, client reload and error-reporting contract; the server SHALL NOT read it as server configuration.

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

#### Scenario: Dragged width survives an unrelated save
- **WHEN** a user drags the sidebar wider and then saves a config change that leaves `sidebar_width` alone
- **THEN** the dragged width stays, and a later save that changes `sidebar_width` replaces it

#### Scenario: Narrow terminal
- **WHEN** a user drags the divider right on a 40-column terminal
- **THEN** the sidebar stops where the pane area keeps 20 columns
