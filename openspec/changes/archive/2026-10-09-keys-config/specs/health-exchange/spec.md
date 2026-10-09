# Spec Delta

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocolVersion` equal to 7, and informational string `version`. Health response JSON field names SHALL use camelCase; Rust fields retain snake_case. The client SHALL validate service/protocol compatibility before any other request. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

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
- **WHEN** a client built for protocol version 1 to 6 contacts a server reporting protocol version 7
- **THEN** the client reports the incompatible protocol version and exits 1 without attaching

### Requirement: Server starter
`ship server --starter` SHALL create one top-level tab holding one pane running the configured shell, else the login shell, in the server user's home directory before it accepts any connection. `ship server` without it SHALL start with no tabs. A server SHALL create tabs on its own only at startup. If the starter's program cannot start, the server SHALL exit with that error instead of serving.

#### Scenario: With and without the starter
- **WHEN** a user runs `ship server`, and separately `ship server --starter`
- **THEN** `ship tab list` prints `{}` for the first and one tab with one pane for the second

#### Scenario: Broken starter
- **WHEN** a user runs `HOME=/nonexistent ship server --starter`
- **THEN** the server exits 1 with an error naming what failed and never accepts a connection
