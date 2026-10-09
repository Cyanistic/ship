# Spec Delta

## ADDED Requirements

### Requirement: Server starter
`ship server --starter` SHALL create one top-level tab holding one pane running the login shell in the server user's home directory before it accepts any connection. `ship server` without it SHALL start with no tabs. A server SHALL create tabs on its own only at startup. If the starter's program cannot start, the server SHALL exit with that error instead of serving.

#### Scenario: With and without the starter
- **WHEN** a user runs `ship server`, and separately `ship server --starter`
- **THEN** `ship tab list` prints `{}` for the first and one tab with one pane for the second

#### Scenario: Broken starter
- **WHEN** a user runs ~~`SHELL=/nonexistent ship server --starter`~~ `HOME=/nonexistent ship server --starter` (amended in slice 3: portable-pty falls back to the login shell when `$SHELL` is bad)
- **THEN** the server exits 1 with an error ~~naming the failed program~~ naming what failed and never accepts a connection

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocolVersion` equal to 5, and informational string `version`. Health response JSON field names SHALL use camelCase; Rust fields retain snake_case. The client SHALL validate service/protocol compatibility before any other request. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

#### Scenario: Compatible health
- **WHEN** `ship server status` contacts a compatible server
- **THEN** it prints the health response as one JSON line and exits 0

#### Scenario: Status with no server
- **WHEN** no server is running and a user runs `ship server status`
- **THEN** it reports that no server is running, exits 1 and starts nothing

#### Scenario: Incompatible or malformed health
- **WHEN** the target returns the wrong status, invalid JSON, wrong service or incompatible protocol version
- **THEN** the client reports contextual failure, emits no success result and exits 1

#### Scenario: Client from an earlier protocol
- **WHEN** a client built for protocol version 1 to 4 contacts a server reporting protocol version 5
- **THEN** the client reports the incompatible protocol version and exits 1 without attaching

### Requirement: Default-local startup and reuse
Without an explicit target, bare `ship` and the tab and pane commands SHALL use `http://127.0.0.1:43179`, reuse compatible health, and launch a background server with the starter only upon positively established connection refusal. They SHALL NOT replace or kill an occupied, incompatible or unhealthy listener.

#### Scenario: Absent default server
- **WHEN** the default connection is refused and the user runs `ship`
- **THEN** Ship launches a local background server with the starter, waits for compatible readiness and opens the client on the starter tab

#### Scenario: Existing compatible server
- **WHEN** compatible health is already available at the default endpoint
- **THEN** `ship` reuses it without starting another server or creating a tab

#### Scenario: Occupied or uncertain endpoint
- **WHEN** the default endpoint times out, fails other than refusal, or responds incompatibly
- **THEN** `ship` fails without spawning a replacement or terminating the listener

### Requirement: Explicit server and custom target
`ship server [--port PORT] [--starter]` SHALL run a foreground server bound only to loopback and reject port zero. An explicit `--server-url URL`, including the default URL, SHALL use only that HTTP/HTTPS target without automatic local launch or fallback. Combining that client option with running a server SHALL be rejected; `ship server stop` and `ship server status` accept it.

#### Scenario: Explicit custom-port server
- **WHEN** a user runs `ship server --port 43180` and `ship --server-url http://127.0.0.1:43180 server status`
- **THEN** health succeeds against that server without launching a default server

#### Scenario: Explicit unreachable target
- **WHEN** the user supplies an unreachable URL, including the default URL explicitly
- **THEN** the client fails with safe target context and does not start or switch to a local server

#### Scenario: Invalid command combination
- **WHEN** the user runs a server with a client target or supplies port zero
- **THEN** the invocation is rejected rather than silently choosing a mode or ephemeral server port
