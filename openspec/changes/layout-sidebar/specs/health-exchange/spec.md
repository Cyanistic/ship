# Spec Delta

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocolVersion` equal to 8, and informational string `version`. Health response JSON field names SHALL use camelCase; Rust fields retain snake_case. The client SHALL validate service/protocol compatibility before any other request. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

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
- **WHEN** a client built for protocol version 1 to 7 contacts a server reporting protocol version 8
- **THEN** the client reports the incompatible protocol version and exits 1 without attaching

#### Scenario: New client against old server
- **WHEN** a version-8 client contacts a version-7 server
- **THEN** it reports incompatibility before attempting a layout request or attach and does not replace the running server
