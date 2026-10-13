# Proposal

Planning artifact for a locked plan. The product, architecture and program papers in `design/` were locked by Cyan on 2026-10-13, after an amendment the same day gave bindings a named `action` and an optional `hint`. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

On a slow link, keys typed right after `alt-n` land in the pane the user was in, not the new tab's shell (#20). The client stamps typing with the selection at press and sends it at once, while the create is still in flight. Separately, each client's selection already lives on the server per attachment, but nothing outside that client can read or set it, so scripts and agents can't steer a client, and a CLI create can't put a client on what it made.

## What Changes

- Everything a client does goes through one ordered lane: bound actions, typed keys, pastes, `client.send`, clicks, fold clicks, wheel turns, drags and mouse input to programs. Anything that depends on the selection, the layout or the folds is resolved when its turn comes, not at press. A server action, including the select a navigation becomes, waits for its reply and the client's catch-up before the next item runs. Typed input doesn't wait.
- A click keeps its own target and does nothing if that target is gone by its turn. While the lane is busy, a press on any pane selects it.
- Mode changes, the sidebar toggle and config reloads apply at press. A failed action shows its error and later input still runs. Disconnecting drops input not yet run, and detaching happens after all earlier input.
- `GET /api/v0/views`, `GET /api/v0/views/{attachment}` and `PUT /api/v0/views/{attachment}/selection` read and set any attachment's selection. `ship view list`, `ship view selected --attachment ID` and `ship view select --attachment ID NODE|--none` drive them from the CLI. Selecting a pane hidden by zoom unzooms its tab for everyone.
- `ship tab create` and `ship pane create` accept `--select` with `--attachment ID` (default `SHIP_ATTACHMENT_ID`), and `server.tab.create` and `server.pane.create` accept `select = true`. The create selects what it made for that attachment in the same edit, so no client sees it unselected first. Without `--select`, a create moves no selection. The four default create keys set `select = true`.
- A typed key with a tab selected goes to that tab's landing pane, and the selection stays on the tab.
- **BREAKING**: a binding is one action in a table, `"alt-r" = { action = { client.mode = "resize" } }`, with an optional `hint = "…"` for the status line. `{}` unbinds. The bare `{ server.… }` form and any other key in the table are config errors with line and column. Cyan waived strict list rejection during slice 1: positional forms Serde happens to accept are not guarded or documented as binding syntax.
- **BREAKING**: protocol version 9 to 10. `PUT /api/v0/attach/view` keeps only `area`, and creates gain `select`. Server and client upgrade together.

Not in this change: ordering a server action after keys typed just before it, pipelined requests, a "selected" target on any endpoint, server-side navigation, `server.run` and `client.run`, several servers in one client, and a pending indicator. These are listed in the product paper and filed as issues when the pull request lands.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-observation`: who may change an attachment's selection, the views routes and `ship view`, and selecting behind a zoom.
- `session-structure`: `--select` and `--attachment` on tab and pane creates, selecting in the same edit.
- `keymap`: one named action per binding with an optional hint, `{}` to unbind, the end of action lists, `select = true` on the default creates, and key actions running in the client's input order.
- `terminal-client`: one ordered input lane resolved at each item's turn, typing into a selected tab's landing pane, and status-line hints from `hint` or the action.
- `mouse-input`: clicks keep their target and wait their turn, and a press on a pane while input is pending selects it.
- `config-file`: the reload scenario's binding syntax.
- `health-exchange`: protocol version 10.

## Impact

- `ship-core`: `SelectView`, `ViewInput` without `selection`, `select` on `CreateTab` and `CreatePane`, `select` and `attachment` on both create commands, `strum::IntoStaticStr` on the command enums, protocol version 10.
- `ship-server`: a `views` route module, the `ListViews`, `GetView` and `Select` actor messages, selection inside the create commits, and `SetView` reduced to area. The repair rules are unchanged.
- `ship-client`: derived single-action bindings with `Action::hint`, the lane and the `Local` state it turns on, `Keys::handle` without pane stamping, area-only view PUTs, and three view methods on `Client`.
- `ship`: `ship view list | selected | select`.
- New dependency: `strum` with `derive`, in `ship-core` and `ship-client`.
- Throwaway: `experiments/latency-proxy/`, for SC-001 only, removed once its evidence is recorded.
- Implementation Rust within about 150 lines either way of today, unmeasured.
