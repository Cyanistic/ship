# Spec Delta

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocolVersion` equal to 8, and informational string `version`. Health response JSON field names SHALL use camelCase; Rust fields retain snake_case. The current client SHALL validate service/protocol compatibility before any other request, including ordinary commands with `--server-url` or `SHIP_SERVER_URL` and `server stop`. Explicit targets, status and stop SHALL NOT start a server. Endpoint methods SHALL remain policy-free; application dispatch owns this validation. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

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
- **WHEN** a client built for protocol version 1 to 7 validates health from a server reporting protocol version 8
- **THEN** the client reports the incompatible protocol version and exits 1 without attaching

#### Scenario: Already-built protocol-7 explicit ordinary command
- **WHEN** an already-built protocol-7 client runs an ordinary command with an explicit URL against protocol 8
- **THEN** its legacy health bypass can reach the ordinary endpoint, without a compatibility guarantee
- **AND** documentation instructs users to update client and server together; this change does not add a request marker or server guard to distinguish that legacy request

This legacy exception is Cyan's approved slice-1 acceptance adjustment. It does not permit the current client to bypass health.

#### Scenario: Stop preflight
- **WHEN** the current client runs `ship server stop` against incompatible, malformed or wrong-service health
- **THEN** it fails before requesting shutdown and starts nothing

#### Scenario: New client against old server
- **WHEN** a version-8 client contacts a version-7 server
- **THEN** it reports incompatibility before attempting a layout request or attach and does not replace the running server
