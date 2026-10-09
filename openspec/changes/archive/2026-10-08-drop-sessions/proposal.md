# Proposal

## Why

A session is a tab that can't hold panes but must have a unique name, and a client attaches to one at a time. The planned overview shows everything on a server at once, which makes choosing a session to attach to pointless and leaves the server with two container types where one would do. This change makes a server one tree of tabs and panes that a client opens as a whole.

## What Changes

- **BREAKING:** The server holds an ordered list of top-level tabs instead of sessions. Every pane still belongs to a tab. Session names, their uniqueness rule and name lookup are gone; every tab and pane is targeted by ID.
- **BREAKING:** Bare `ship` opens the full-screen client on the whole server with nothing selected, instead of printing health. `ship attach` and the `ship session` commands are removed. `ship server status` prints the health JSON bare `ship` used to print, and never starts a server.
- When `ship` starts the local server itself, the server begins with one top-level tab holding a shell in the server user's home directory, and the client opens on that tab's pane. Opening a server that was already running never creates a tab, even when it has none.
- `ship server --starter` starts with that one tab; `ship server` alone starts empty. A server with no tabs is valid and keeps running.
- With nothing selected, the client shows the number of top-level tabs and the key that opens one, or `no tabs · ship tab create`. The status line no longer shows a session.
- `C-b )` and `C-b (` cycle top-level tabs, landing on a tab's first pane, and open the first or last top-level tab from nothing selected.
- An attached client stays attached when its tabs are removed, falling back to nothing selected. It ends only on detach or server shutdown.
- `ship tab create` and `ship tab move` accept an omitted parent, meaning the top level. `ship tab list` prints the top-level tabs and their descendants as JSON.
- **BREAKING:** A tab move names exactly one destination: a parent (or the top level), or a sibling to go before or after. `--before` and `--after` no longer take a parent beside them.
- **BREAKING:** `GET /api/v0/tabs` replaces the five session routes. Tab create takes an optional parent; the move body is `{"parent": …}`, `{"before": …}` or `{"after": …}`; attach and view take an optional selection and no session. The health `protocolVersion` becomes 5.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-structure`: Top-level tabs replace sessions; tab parents, movement, naming, CLI targeting and routes change.
- `session-observation`: Bare `ship` opens the whole server; selection is optional and starts empty; attach-or-create, session removal and session cycling are replaced by top-level tab behavior.
- `terminal-client`: The client opens from bare `ship`, the empty state gains the nothing-selected hint, and the status line drops the session.
- `observer-recovery`: Reconnecting keeps a selection that still exists anywhere on the server, else nothing; a client is never ended by its session disappearing.
- `health-exchange`: Protocol 5, bare `ship` opens the client, `ship server status` and `ship server --starter`.
- `local-foundation`: Help describes bare `ship` opening the client instead of printing health.
- `pane-programs`: Removing a tab, not a session, ends the programs inside it.
- `client-maintenance`: The OpenAPI description covers tab, pane and attachment operations, without sessions.

The `session-structure` and `session-observation` capability paths keep their names, because OpenSpec has no capability rename; their content stops describing sessions.

## Impact

All four crates change, mostly by deletion. No dependency changes.

- `ship-core`: `Session`, `SessionName`, `TabParent`, `Creatable`, `Create<T>`, `ServerRoot` and `UntaggedEither` go away; every level of the tree is an ordered map of `Arc<Tab>`; selection becomes optional.
- `ship-server`: session handlers and routes removed, `GET /api/v0/tabs` added, startup seeding between bind and accept.
- `ship-client`: session resolution removed; the UI opens on the whole server.
- `ship`: session and attach commands removed; `ship server status` and `--starter` added.

The three locked papers in [design/](design/), indexed by [design.md](design.md), are the authoritative design. Implementation follows the program paper's four slices, one at a time, and starts only when Cyan requests it. Linux and macOS are verified deliberately; Windows stays best effort and unverified. No sidebar, overview, several machines, name lookup or permanent authored tests are included.
