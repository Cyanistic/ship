# Proposal

This is the planning artifact for a locked plan. Cyan locked the three papers in `design/` on 2026-10-21. The product paper was amended the same day so FR-003 allows a wait to fail when it falls too far behind the server. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

When a shell in a pane exits, the pane stays on screen with `exited (0)` until someone closes it by hand. tmux, Zellij and Herdr all close it, so Ship feels broken here (#16). An agent driving Ship wants the opposite: a pane that stays after its command exits, so it can read the output first. A pane's starting settings are also scattered: the shell and history size are server-wide, and right-click passthrough has no default at all.

## What Changes

- **Exit behavior.** Every pane has an exit behavior: `close` (the default), `keep` or `keep_failed`. It's set at creation or later with `ship pane set`, and saved with the tabs.
- **Waits see the exit before the close.** A `ship pane wait` that's waiting when a program exits still gets the exit status, even when the pane then closes. A wait that has fallen too far behind the server fails saying so.
- **Every-revision feed.** An attach request can ask for `"feed": "every"`, which sends each revision in order and ends with `lost` if the stream falls behind. The default feed still sends only the newest state.
- **BREAKING: one defaults table.** `[server.defaults.pane]` holds `shell`, `scrollback_bytes`, `right_click_passthrough` and `exit_behavior`. `[server] shell` and `[server] scrollback_bytes` are no longer accepted, with no alias.
- **Per-pane settings.** `ship pane create` and `server.pane.create` accept `exit_behavior`, `scrollback_bytes` and `right_click_passthrough`. `--right-click-passthrough` now takes an optional `=false`. `ship pane set` and `server.pane.set` accept `exit_behavior` and `scrollback_bytes`. Lowering a pane's history size drops the oldest history at once.
- **Pane views** report `exitBehavior` and `scrollbackBytes`.
- **BREAKING: protocol 15.** Old clients are refused by the health check.

Not in this change (the product paper lists them all):
- Live inheritance of settings, and tab-level defaults.
- Hooks on exit, and tmux's "keep until a key is pressed".
- Closing a tab when its last pane closes on exit.
- A line limit for history.
- Settings on `ship tab create --starter`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pane-programs`: exited programs close or stay according to the pane's exit behavior.
- `pane-history`: history size is per pane, comes from the defaults table or the create request, and can be changed live.
- `config-file`: the `[server.defaults.pane]` table replaces the server shell and scrollback settings.
- `session-structure`: exit behavior as a pane attribute, new `set` attributes, and passthrough's default from config.
- `session-restore`: restored panes keep their exit behavior and history size.
- `session-observation`: the `feed` field on attach and the every-revision feed.
- `pane-automation`: waiting on a pane that closes on exit, the exit-before-close ordering, and a wait that falls behind.

## Impact

- `ship-core`: `ExitBehavior` and two `Pane` fields, `PaneDefaults` and `defaults.toml`, optional overrides on `PaneInput`, new `SetPane` fields, `FeedMode`, `EndReason::Lost`, `PROTOCOL_VERSION` 15, and the CLI and action fields.
- `ship-server`: settings read the new table, the state actor resolves defaults at creation and commits exit and then close, a new `replicas.rs` holds the latest value plus a ring of 256, attach streams pick a feed, and a pane runtime command changes the history size.
- `vendor/ratatui-ghostty`: one session command that changes a live terminal's history limit.
- `ship-client`: waits use the `every` feed, create and set pass the new fields, and the UI handles `lost`.
- Docs: the guide's server settings, exit behavior and history size sections, the README's config example and "Not built yet" list.
- Estimated at a few hundred lines of implementation Rust. Measured counts go in [tasks.md](tasks.md).
