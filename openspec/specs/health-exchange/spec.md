# health-exchange Specification

## Purpose

Let users exercise Ship through a real, observable health request with predictable target selection and a bounded local-server lifecycle that survives client exit.

## Requirements

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

### Requirement: Default-local startup and reuse
Without an explicit target, `ship` SHALL use `http://127.0.0.1:43179`, reuse compatible health, and launch a background server only upon positively established connection refusal. It SHALL NOT replace or kill an occupied, incompatible or unhealthy listener.

#### Scenario: Absent default server
- **WHEN** the default connection is refused and the user runs `ship`
- **THEN** Ship launches a local background server, waits for compatible readiness and returns health

#### Scenario: Existing compatible server
- **WHEN** compatible health is already available at the default endpoint
- **THEN** `ship` reuses it without starting another server

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
`ship server [--port PORT]` SHALL run a foreground server bound only to loopback and reject port zero. An explicit `--server-url URL`, including the default URL, SHALL use only that HTTP/HTTPS target without automatic local launch or fallback. Combining that client option with the server subcommand SHALL be rejected.

#### Scenario: Explicit custom-port server
- **WHEN** a user runs `ship server --port 43180` and `ship --server-url http://127.0.0.1:43180`
- **THEN** health succeeds against that server without launching a default server

#### Scenario: Explicit unreachable target
- **WHEN** the user supplies an unreachable URL, including the default URL explicitly
- **THEN** the client fails with safe target context and does not start or switch to a local server

#### Scenario: Invalid command combination
- **WHEN** the user combines the server subcommand with a client target or supplies port zero
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
Server diagnostics SHALL identify startup address, PID and protocol version, and health-request method/path, status and duration. Automatically launched servers SHALL use a unique private mode-0600 temporary log. Launch diagnostics SHALL expose its path and PID; log-creation failures SHALL name the attempted directory without claiming a file exists.

#### Scenario: Observe request outcome
- **WHEN** a health request completes under the default diagnostic filter
- **THEN** server diagnostics show GET `/health`, status and duration without obscuring client stdout or exposing arbitrary bodies/headers

#### Scenario: Private launch log
- **WHEN** automatic launch succeeds
- **THEN** diagnostics identify the daemon PID and unique private log, which remains available after client exit

### Requirement: Normal graceful shutdown
SIGINT and SIGTERM SHALL request graceful server shutdown, bounded by five seconds, after which the process SHALL exit and stop accepting requests.

#### Scenario: Signal shutdown
- **WHEN** a foreground or background server receives SIGINT or SIGTERM
- **THEN** it drains active health requests within the bound, exits and no longer accepts connections
