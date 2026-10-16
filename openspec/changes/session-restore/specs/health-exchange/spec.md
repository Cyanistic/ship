# Spec Delta

## MODIFIED Requirements

### Requirement: Default-local startup and reuse
Without an explicit target, bare `ship` and the tab and pane commands SHALL use `http://127.0.0.1:43179`, reuse compatible health, and launch a background server with the starter only upon positively established connection refusal. That server SHALL restore the port's saved tabs. They SHALL NOT replace or kill an occupied, incompatible or unhealthy listener. When bare `ship` launched the server, its client SHALL open on the first top-level tab's first pane, whether that tab was restored or created by the starter. A client attaching to a server that was already running SHALL start with nothing selected.

#### Scenario: Absent default server
- **WHEN** the default connection is refused, nothing is saved for the port, and the user runs `ship`
- **THEN** Ship launches a local background server with the starter, waits for compatible readiness and opens the client on the starter tab

#### Scenario: Absent default server with saved tabs
- **WHEN** the default connection is refused, the port has saved tabs, and the user runs `ship`
- **THEN** Ship launches a local background server, which restores the saved tabs without adding a starter tab, and opens the client on the first top-level tab's first pane

#### Scenario: Repeated launches
- **WHEN** the default server is stopped and started through bare `ship` ten times
- **THEN** the number of top-level tabs is the same as before the first start

#### Scenario: Existing compatible server
- **WHEN** compatible health is already available at the default endpoint
- **THEN** `ship` reuses it without starting another server or creating a tab

#### Scenario: Occupied or uncertain endpoint
- **WHEN** the default endpoint times out, fails other than refusal, or responds incompatibly
- **THEN** `ship` fails without spawning a replacement or terminating the listener

### Requirement: Explicit server and custom target
`ship server [--port PORT] [--starter] [--no-restore]` SHALL run a foreground server bound only to loopback and reject port zero. An explicit `--server-url URL`, including the default URL, SHALL use only that HTTP/HTTPS target without automatic local launch or fallback. Combining that client option with running a server SHALL be rejected; `ship server stop` and `ship server status` accept it.

#### Scenario: Explicit custom-port server
- **WHEN** a user runs `ship server --port 43180` and `ship --server-url http://127.0.0.1:43180 server status`
- **THEN** health succeeds against that server without launching a default server

#### Scenario: Explicit unreachable target
- **WHEN** the user supplies an unreachable URL, including the default URL explicitly
- **THEN** the client fails with safe target context and does not start or switch to a local server

#### Scenario: Invalid command combination
- **WHEN** the user runs a server with a client target or supplies port zero
- **THEN** the invocation is rejected rather than silently choosing a mode or ephemeral server port

### Requirement: Server starter
`ship server --starter` SHALL create one top-level tab holding one pane running the configured shell, else the login shell, in the server user's home directory before it accepts any connection, only when the server has no tabs after restoring. That includes a first run with no saved file, saved tabs that held no tabs, `--no-restore`, and an unreadable saved file. Its help SHALL say so. The starter tab SHALL be saved like any other change. `ship server` without it SHALL start with no tabs when there is nothing to restore. A server SHALL create tabs on its own only at startup. If the starter's program cannot start, the server SHALL exit with that error instead of serving.

#### Scenario: With and without the starter
- **WHEN** nothing is saved for the port and a user runs `ship server`, and separately `ship server --starter`
- **THEN** `ship tab list` prints `{}` for the first and one tab with one pane for the second

#### Scenario: Starter after a restore
- **WHEN** the port has saved tabs with two top-level tabs and a user runs `ship server --starter`
- **THEN** `ship tab list` shows the two restored tabs and no starter tab

#### Scenario: Starter with --no-restore
- **WHEN** the port has saved tabs and a user runs `ship server --starter --no-restore`
- **THEN** `ship tab list` shows one tab with one pane, and the new tab schedules a save 5 seconds later; after that save completes, the saved file holds that tab and `<port>.json.prev` holds the earlier saved tabs

#### Scenario: Broken starter
- **WHEN** nothing is saved for the port and a user runs `HOME=/nonexistent ship server --starter`
- **THEN** the server exits 1 with an error naming what failed and never accepts a connection
