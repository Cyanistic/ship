## MODIFIED Requirements

### Requirement: Restore on startup
On startup, before it accepts any connection, the server SHALL rebuild the saved tabs with the same IDs, names, structure, proportions, zoom and each pane's `right_click_passthrough`, `exit_behavior` and `scrollback_bytes`. Each restored pane SHALL run `[server.defaults.pane] shell`, else the login shell, as configured at restore time, in its saved directory, which SHALL be its starting directory for `ship pane get`. It SHALL NOT run the pane's earlier program. A missing saved file SHALL start with no restored tabs.

#### Scenario: Structure comes back
- **WHEN** a server with nested tabs, named tabs and panes, custom proportions and a zoomed pane restarts
- **THEN** `ship tab list` shows the same IDs, names, nesting, proportions and zoom

#### Scenario: Pane settings come back
- **WHEN** a pane with `exitBehavior: keep` and `scrollbackBytes: 50000000` is saved and the server restarts
- **THEN** the restored pane, a fresh shell, shows the same two values

#### Scenario: Saved before pane settings existed
- **WHEN** a saved file from before exit behavior and per-pane history size is restored
- **THEN** its panes get `exitBehavior: close` and `scrollbackBytes: 10000000`, the built-in defaults

#### Scenario: Program not rerun
- **WHEN** a pane running `nvim foo.rs` in `~/projects/ship` is restored
- **THEN** the pane runs the configured shell in `~/projects/ship` and `nvim` is not started

#### Scenario: Reconnecting client
- **WHEN** a client stays open while the server stops and starts again
- **THEN** after reconnecting it keeps the tab or pane it had selected
