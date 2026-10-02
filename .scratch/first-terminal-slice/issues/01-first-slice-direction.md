# Choose the first-slice architecture and working rhythm

ID: 01
Parent: [Plan the first client/server terminal slice](../map.md)
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: Cyan
Blocked by: None

## Question

What ownership model, transport direction, dependency preferences and development rhythm should guide the first runnable terminal slice?

## Comments

This is a retrospective record of decisions Cyan made in the preceding live conversation, not an additional decision resolved during documentation update. No prior tracker issue existed. Dependencies were discussed through source inspection and temporary probes, not production implementation.

## Answer

Cyan approved a Cargo workspace containing ship (app binary/CLI/startup), ship-client, ship-server and ship-core. Core holds genuinely shared types/traits/helpers. One executable runs separate client/server modes; initial startup is explicit.

The server owns processes, PTYs, Ghostty and authoritative shared session state. The client owns rendering, input forwarding, its synchronized state copy and client-local presentation. Shared-state changes come from the server; focus is local but can be reported as presence. Concurrent terminal input is allowed.

Use HTTP request/response and SSE events, initially JSON. Loopback binding is the default; config and port customization are deferred. Local token authentication is explicitly deferred with the acknowledged limitation that other local processes/users may reach the control interface. Non-loopback exposure requires a new security decision. SSH port forwarding is the intended later optional connection adapter, not a separate protocol or current implementation.

Use snapshots and structdiff-generated/applied patches, carrying base/new revisions and a server-instance identity. Coordinate snapshot and subscription; fetch fresh state when continuity is lost. Exact stream lifecycle and collection strategy remain open.

Trial the supplied ratatui-ghostty session first. Prefer owned screen/cell types with Ratatui conversions over recreating low-level Ghostty mapping, pending fidelity and access checks. Clap, Tokio, tracing/tracing-subscriber and the the reference error pattern are user preferences. PTY and HTTP library selections remain validation candidates.

First checkpoint: one interactive terminal across the real server/client connection, with resize and distinct close/disconnect/server-stop behavior. Server-owned work survives client disconnection; automatic startup, persistent restart restore and remote attachment are deferred.

Build in fine-grained coherent commits. Zero authored permanent tests initially; add only for encountered flaky behavior with explicit agreement. Verify through build/lint/format and real workflows or temporary probes. Keep production/test/full-authored counts separate.

Later multi-client tab dimensions use minimum available width and height among clients currently viewing that tab. A client on another tab does not constrain it. No-viewer policy is not selected.

Herdr commands are inspiration, not a full imported surface; keep keys/text/paste distinct and target explicit identities. Do not import its special single-writer takeover behavior.

## Evidence

Live choices recorded in [first-slice planning notes](../../../docs/planning/first-terminal-slice.md), including source references and the successful temporary structdiff roundtrip. No Ship runtime implementation exists.

## Uncertain decisions

Screen fidelity, collection strategy, exact request/event schema, terminal creation ownership, subscription coordination, library compatibility and process cleanup need remaining ticket decisions. These have not been selected implicitly.
