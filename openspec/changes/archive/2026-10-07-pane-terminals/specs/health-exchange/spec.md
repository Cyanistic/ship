# Spec Delta

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocolVersion` equal to 3, and informational string `version`. Health response JSON field names SHALL use camelCase; Rust fields retain snake_case. The client SHALL validate service/protocol compatibility and print one health JSON line on stdout only on success.

#### Scenario: Compatible health
- **WHEN** `ship` contacts a compatible server
- **THEN** it prints the health response as one JSON line and exits 0

#### Scenario: Incompatible or malformed health
- **WHEN** the target returns the wrong status, invalid JSON, wrong service or incompatible protocol version
- **THEN** the client reports contextual failure, emits no success result and exits 1

#### Scenario: Client from an earlier protocol
- **WHEN** a client built for protocol version 1 or 2 contacts a server reporting protocol version 3
- **THEN** the client reports the incompatible protocol version and exits 1 without attaching
