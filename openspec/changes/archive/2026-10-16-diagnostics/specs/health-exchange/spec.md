# Spec Delta

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocolVersion` equal to 13, informational string `version`, and `log`, the path of the log part the server is writing. Health response JSON field names SHALL use camelCase; Rust fields retain snake_case. The current client SHALL validate service/protocol compatibility before any other request, including ordinary commands with `--server-url` or `SHIP_SERVER_URL` and `server stop`. Explicit targets, status and stop SHALL NOT start a server. Endpoint methods SHALL remain policy-free; application dispatch owns this validation. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

#### Scenario: Compatible health
- **WHEN** `ship server status` contacts a compatible server
- **THEN** it prints the health response as one JSON line, including the server's current log part in `log`, and exits 0

#### Scenario: Log path follows rollover
- **WHEN** the server's log has moved to a new part and a user runs `ship server status`
- **THEN** `log` names the new part

#### Scenario: Status with no server
- **WHEN** no server is running and a user runs `ship server status`
- **THEN** it reports that no server is running, exits 1 and starts nothing

#### Scenario: Incompatible or malformed health
- **WHEN** the target returns the wrong status, invalid JSON, wrong service or incompatible protocol version
- **THEN** the client reports contextual failure, emits no success result and exits 1

#### Scenario: Client from an earlier protocol
- **WHEN** a client built for protocol version 1 to 12 validates health from a server reporting protocol version 13
- **THEN** the client reports the incompatible protocol version and exits 1 without attaching

#### Scenario: Already-built protocol-7 explicit ordinary command
- **WHEN** an already-built protocol-7 client runs an ordinary command with an explicit URL against protocol 8 or later
- **THEN** its legacy health bypass can reach the ordinary endpoint, without a compatibility guarantee
- **AND** documentation instructs users to update client and server together; this change does not add a request marker or server guard to distinguish that legacy request

This legacy exception is Cyan's approved slice-1 acceptance adjustment. It does not permit the current client to bypass health.

#### Scenario: Stop preflight
- **WHEN** the current client runs `ship server stop` against incompatible, malformed or wrong-service health
- **THEN** it fails before requesting shutdown and starts nothing

#### Scenario: New client against old server
- **WHEN** a version-11 client contacts a version-10 server
- **THEN** it reports incompatibility before attempting any other request or attach and does not replace the running server

### Requirement: Observable startup and private daemon logs
Server diagnostics SHALL identify startup address, PID and protocol version, and, at debug detail, health-request method/path, status and duration. Automatically launched servers SHALL write to that run's private mode-0600 file under the port's log directory, as the `diagnostics` capability describes, and no temporary log SHALL be created. Launch diagnostics SHALL expose the run file's path and PID; log-creation failures SHALL name the attempted directory without claiming a file exists.

#### Scenario: Observe request outcome
- **WHEN** a health request completes with Ship debug logging enabled
- **THEN** server diagnostics show GET `/health`, status and duration without obscuring client stdout or exposing arbitrary bodies/headers

#### Scenario: Private launch log
- **WHEN** automatic launch succeeds
- **THEN** diagnostics identify the daemon PID and its run file under `logs/<port>/`, which remains available after client exit and is linked from `current.log`

## ADDED Requirements

### Requirement: Request correlation
Every HTTP response SHALL carry `x-request-id`, preserving a caller-supplied request ID or generating a UUID when absent. Request IDs SHALL be correlation labels, not authentication credentials.

#### Scenario: Caller correlation
- **WHEN** a request supplies `x-request-id`
- **THEN** the same value appears in the response and, when enabled, the completion summary

### Requirement: Status-aware completion diagnostics
Request spans SHALL be debug-level. Each request SHALL emit one completion summary at debug for success and 4xx responses, or warn for 5xx responses. Summaries SHALL include request ID, method, path, status and duration even when debug spans are disabled.

#### Scenario: Unexpected server response at default detail
- **WHEN** a request returns 5xx with debug logging disabled
- **THEN** exactly one middleware completion warning includes its request ID, method, path, status and duration

#### Scenario: Routine client rejection
- **WHEN** a request returns 4xx
- **THEN** its middleware completion summary is debug, not warn or error

### Requirement: Request diagnostic scope
Middleware request duration SHALL measure time until the response is ready, not streaming-body lifetime. Request spans and summaries SHALL include a valid request `x-ship-attachment-id` as `attachment_id`; summaries SHALL include the response's numeric `x-ship-revision` as `revision`. Missing or malformed values SHALL be omitted without changing request handling. Request middleware SHALL NOT record query strings, bodies or headers outside this allowlist and `x-request-id`.

#### Scenario: Sensitive request metadata
- **WHEN** a request includes a query string, body, authorization header or cookies
- **THEN** the request middleware records none of those values

#### Scenario: Explicit diagnostic headers
- **WHEN** a request has a valid attachment ID and its response carries a numeric revision
- **THEN** enabled summaries include `attachment_id` and `revision`, and the request span includes `attachment_id`
- **AND** an unrelated `x-ship-*` header is not recorded
