# Proposal

## Why

Ship has a working local server and health exchange, but it holds no work yet. Before real terminals arrive, it needs server-owned sessions, recursive tabs and logical panes, plus clients that observe that structure live and recover from dropped connections. Proving this metadata-only structure first keeps terminal and layout work from also carrying the ownership, replication and reconnection design.

## What Changes

- Add server-owned sessions containing recursive ordered tabs, with metadata-only logical panes owned by tabs. Entities have stable kind-prefixed IDs such as `session:3f2a...`.
- Add CLI commands to create, inspect, rename and remove sessions, tabs and panes, and to move and reorder tabs within and across sessions. Creation prints the created entity as JSON.
- Accept a session name wherever the CLI accepts a session ID. Session names are unique and may not contain `:`. Tab and pane names may repeat and remain ID-only.
- Add `ship attach <session>`: a text observer that prints the live structure, its own selection and its attachment ID. Attaching by name creates a missing session.
- Give each observer an independent selection of a session, tab or pane, with fallback to the nearest surviving parent when the selected entity is removed or moves to another session.
- Removing a session kicks its observers out; they report the removal and exit.
- Reconnect a running observer automatically after a dropped connection, restoring current state and keeping its selection when it is still valid.
- Serve the new HTTP API under `/api/v0`, with compressed SSE for observation. `/health` stays unprefixed and unchanged.
- Describe every new route in the existing OpenAPI document.

No existing supported behavior is removed.

## Capabilities

### New Capabilities

- `session-structure`: Sessions, recursive tabs and logical panes; their IDs, naming rules, CLI create/inspect/rename/remove, tab movement, recursive removal and atomic rejection of invalid operations.
- `session-observation`: Attaching text observers, attach-or-create by name, live replicated structure, independent per-observer selection and fallback, session switching, kick-and-exit on session removal, slow-observer catch-up and shutdown of open streams.
- `observer-recovery`: Automatic reconnection of a running observer, state replacement without replayed mutations, and selection retention or fallback after reconnecting.

### Modified Capabilities

- `client-maintenance`: Schema inspection extends from the health operation to every session, tab, pane and attachment route and its shared request and response types.

## Impact

Implementation affects all four crates: `ship-core` (IDs, entity model, protocol types, an optional `axum` feature for error responses), `ship-server` (state actor, ported relay, routes, SSE attach), `ship-client` (entity and attach methods, session name resolution) and `ship` (CLI subcommands and the text observer). New runtime dependencies are Kameo, IndexMap, UUID, serde_with, futures-util, tokio-stream and eventsource-stream, plus zstd/gzip/stream features on Reqwest and Tower HTTP.

The authoritative design is the three locked papers in [design/](design/), indexed by [design.md](design.md). Implementation follows the program paper's seven slices, one at a time. All state is in memory and the server remains loopback-only and unauthenticated. No PTYs, pane layouts, disk persistence, graphical UI or permanent authored tests are included. Writing these artifacts does not start implementation.
