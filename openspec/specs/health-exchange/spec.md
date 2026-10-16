# health-exchange Specification

## Purpose

Let users exercise Ship through a real, observable health request with predictable target selection and a bounded local-server lifecycle that survives client exit.

## Requirements

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

### Requirement: Persistent daemon and bounded launch cleanup
An automatically launched ready server SHALL remain reachable after client exit and launching-terminal closure. Readiness SHALL have a five-second absolute deadline; ordinary TCP connect and health requests SHALL be bounded by one and two seconds respectively. Failed launch cleanup SHALL affect only that invocation's child. Concurrent invocations SHALL leave at most one serving daemon and no surviving losing children.

#### Scenario: Client and terminal exit
- **WHEN** health succeeds after automatic launch and the client exits and its launching terminal closes
- **THEN** the daemon remains reachable by a later invocation

#### Scenario: Failed startup
- **WHEN** log creation, child initialization or readiness fails
- **THEN** Ship reports actionable context, cleans up its own still-live launch attempt, and does not claim readiness or kill another server

#### Scenario: Concurrent launch race
- **WHEN** ten clients run concurrently with the default endpoint initially absent
- **THEN** at most one compatible daemon remains, losing children exit, and each client succeeds or fails with bounded contextual diagnostics

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

### Requirement: Target integrity and diagnostic privacy
Health requests SHALL refuse redirects and SHALL NOT disable TLS verification. Credential-bearing URLs SHALL be redacted in diagnostics. Base paths, queries and fragments SHALL NOT change the root-relative health route. Results SHALL remain separate from diagnostics.

#### Scenario: Redirect response
- **WHEN** a target redirects a health request
- **THEN** the client reports request failure without following the redirect

#### Scenario: Credential-bearing target failure
- **WHEN** a target URL contains userinfo and parsing or a request fails
- **THEN** diagnostics identify the target without exposing those credentials

#### Scenario: Target with path and query
- **WHEN** the client is supplied an absolute target containing a base path, query or fragment
- **THEN** the request addresses the target's root-relative `/health`, not the supplied path/query

### Requirement: Observable startup and private daemon logs
Server diagnostics SHALL identify startup address, PID and protocol version, and, at debug detail, health-request method/path, status and duration. Automatically launched servers SHALL write to that run's private mode-0600 file under the port's log directory, as the `diagnostics` capability describes, and no temporary log SHALL be created. Launch diagnostics SHALL expose the run file's path and PID; log-creation failures SHALL name the attempted directory without claiming a file exists.

#### Scenario: Observe request outcome
- **WHEN** a health request completes with Ship debug logging enabled
- **THEN** server diagnostics show GET `/health`, status and duration without obscuring client stdout or exposing arbitrary bodies/headers

#### Scenario: Private launch log
- **WHEN** automatic launch succeeds
- **THEN** diagnostics identify the daemon PID and its run file under `logs/<port>/`, which remains available after client exit and is linked from `current.log`

### Requirement: Normal graceful shutdown
SIGINT and SIGTERM SHALL request graceful server shutdown, bounded by five seconds, after which the process SHALL exit and stop accepting requests.

#### Scenario: Signal shutdown
- **WHEN** a foreground or background server receives SIGINT or SIGTERM
- **THEN** it drains active health requests within the bound, exits and no longer accepts connections

### Requirement: Server starter
`ship server --starter` SHALL create one top-level tab holding one pane running the configured shell, else the login shell, in the server user's home directory before it accepts any connection. `ship server` without it SHALL start with no tabs. A server SHALL create tabs on its own only at startup. If the starter's program cannot start, the server SHALL exit with that error instead of serving.

#### Scenario: With and without the starter
- **WHEN** a user runs `ship server`, and separately `ship server --starter`
- **THEN** `ship tab list` prints `{}` for the first and one tab with one pane for the second

#### Scenario: Broken starter
- **WHEN** a user runs `HOME=/nonexistent ship server --starter`
- **THEN** the server exits 1 with an error naming what failed and never accepts a connection

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
