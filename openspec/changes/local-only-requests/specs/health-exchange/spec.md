# Spec Delta

## ADDED Requirements

### Requirement: Local requests only
The server SHALL answer a request only when its host, from the HTTP/2 authority or else the HTTP/1 `Host` header, is `localhost`, an IPv4 address in `127.0.0.0/8` or `[::1]`, with or without a port, and it carries no `Origin` header. Any other request, including one with no host, SHALL get status 403 with an error body whose `.type.kind` is `forbidden`, and SHALL NOT reach a route. This applies to every route, `/health` included.

#### Scenario: Ship client
- **WHEN** a user runs `ship server status` against the default server
- **THEN** health succeeds as before

#### Scenario: DNS rebinding
- **WHEN** a request to the server carries `Host: attacker.example:43179`
- **THEN** the server responds 403 with `.type.kind` equal to `forbidden` and the route doesn't run

#### Scenario: Browser request to the loopback address
- **WHEN** a request to `/api/v0/server/stop` carries `Host: 127.0.0.1:43179` and `Origin: https://attacker.example`
- **THEN** the server responds 403 and keeps running

#### Scenario: SSH tunnel
- **WHEN** a client on another machine reaches the server through `ssh -L 43180:127.0.0.1:43179` with `--server-url http://127.0.0.1:43180`
- **THEN** its requests carry a loopback host and succeed
