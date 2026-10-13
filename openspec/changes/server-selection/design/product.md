# Server-held selection and ordered input

Status: Locked on 2026-10-13, approved by Cyan after Plannotator review ("LGTM! onward!"), which accepts U-1 and the `{}` unbind as recommended. Written on 2026-10-12 from a two-day design conversation with Cyan; the decisions below were settled in that conversation and are quoted where Cyan said them. It replaces [handoff.md](../handoff.md), whose direction (a "selected" target form, navigation and folds on the server) was dropped.

Amended on 2026-10-13 during program review: FR-018's binding syntax gains an `action` field and an optional `hint`. Approved by Cyan in chat ("yeah strum is exactly what i was thinking! let's update the papers please!"), with re-review waived ("don't worry about me checking them again. i'm sure they'll be correct").

## Summary

Each client's selection already lives on the server, one per attachment, but only the client can change it, by sending what it chose with its view. This change makes that selection something other callers can read and set by naming the attachment, and lets tab and pane creates select what they made in the same edit. Navigation, folds, selection memory, modes and the keymap stay on the client.

It also fixes the key-ordering race (#20). Everything a client does to the server, including typed keys and clicks, takes effect in the order it happened, and anything that depends on the selection is resolved only once everything before it has taken effect. On a slow link, `alt-n` then typing lands the typing in the new pane, at the cost of waiting a round trip.

Every key binding becomes a single action.

Needs attention in review:

- Bindings drop lists and name their action: `"alt-n" = { action = { server.tab.create = … } }`, not `[{ … }]`. This amends the locked keys-config paper (FR-018).
- A binding may set its own status-line hint with `hint = "…"`; unset, the hint comes from the action as today (FR-018, amended).
- Unbinding is written `{}` (FR-018). Before A-6 it was `"none"`.
- A CLI user finds an attachment ID with `ship view list` (U-1, settled).
- A server action can still take effect before keys typed just before it (Non-goals). The fix is deferred with Pipeline.

## Goal

Let a key act on what the previous key produced, however slow the link, so type-ahead is correct. Let scripts and agents read and steer a specific client's selection, which later run actions build on. Do both without moving navigation or any client logic to the server, and without targets that mean "whatever is selected".

## User stories

### P1: Type right after creating

Given a remote server with a slow link, when Cyan presses `alt-n` and types `ls` and Enter straight away, then `ls` runs in the new tab's shell, not the pane Cyan was in. When Cyan presses `alt-|` then `alt-f`, the new pane is the one zoomed.

- Given the lane is idle, when Cyan types, then each key is sent at once, as today.
- Given Cyan clicks another pane while a create is still pending, then the click wins: the create finishes first and Cyan ends up on the clicked pane.

Tested independently: put a delaying proxy with 500 ms each way between client and server, then repeat `alt-n` + `echo hi` + Enter and `alt-|` + `alt-f` ten times each.

### P2: Create without stealing focus

Given an agent in one pane runs `ship pane create --tab <id> -- cargo test`, then Cyan's selection doesn't move. Given a script runs the same with `--select --attachment <id>`, then that client is on the new pane, and no client ever sees the new pane unselected first.

Tested independently: create from the CLI with and without `--select` while a client watches, and check the selection each time.

### P3: Steer a client from a script

Given a client's attachment ID, when a script runs `ship view selected --attachment <id>`, then it prints that client's selection. When it runs `ship view select --attachment <id> <pane-id>`, then the client moves there on its next frame.

Tested independently: set a client's selection from a second terminal to a pane, a tab and nothing.

## Non-goals

- Ordering a server action after keys typed just before it. The keys and the action travel separately and the action can arrive first, so typing then `alt-pageup` can end at the bottom. Pipeline fixes it and is deferred.
- Sending a key's action before the previous one's result arrives. Each dependent step waits a round trip. Deferred with Pipeline, client-minted IDs and server-side resolution.
- A "selected" target on any endpoint. Every request names its targets by ID.
- Moving navigation, folds, selection memory, sidebar state or modes to the server.
- `server.run` and `client.run`.
- Several servers in one client.
- An action plus a mode change in one binding. Press the action, then `esc`.
- A pending indicator while the lane waits.
- Keeping an attachment's selection across a server restart.
- Ordering between different clients. When another client closes the pane a waiting key was headed for, the key goes nowhere, as typing into a closed pane does today.

## Functional requirements

### Selection

- FR-001: Each attachment SHALL keep one selection on the server: a tab, a pane, or nothing. It SHALL change only by its own client's navigation and clicks, `ship view select`, `--select` on a create, and the existing repair rules (session-observation).
- FR-002: `ship view selected --attachment ID` SHALL print that attachment's selection. `ship view select --attachment ID` SHALL set it to a given tab or pane, or to nothing. An unknown attachment or node SHALL fail and change nothing.
- FR-003: A client SHALL show a selection set by someone else as it shows its own, within a frame of the server publishing it.
- FR-004: Selecting a pane hidden by its tab's zoom SHALL unzoom that tab for every client and then select the pane.
- FR-005: When an attachment's selection is a tab that has panes, its client SHALL send typing to the pane it would land on for that tab (the zoomed pane, else its last selected pane there, else the first in layout order). The selection SHALL stay on the tab.
- FR-006: Reconnecting SHALL keep the previous selection as observer-recovery describes.

### Creates

- FR-007: `ship tab create` and `ship pane create` SHALL accept `--select`, and `server.tab.create` and `server.pane.create` SHALL accept `select = true`. The create SHALL select the new pane, or the new tab when it has no pane, for one attachment, in the same edit that creates it: no client SHALL see the new tab or pane before that attachment selects it.
- FR-008: On the CLI that attachment SHALL come from `--attachment ID`, defaulting to `SHIP_ATTACHMENT_ID`. In a binding it SHALL be the binding's own client. With `--select` and no attachment, the command SHALL fail before creating anything.
- FR-009: A create naming an attachment that doesn't exist SHALL fail and create nothing.
- FR-010: A create without `--select` SHALL NOT change any selection.
- FR-011: The defaults SHALL set `select = true` on `alt-n`, `alt-shift-n`, `alt--` and `alt-|`.

### Order

- FR-012: A client's bound actions, typed keys, pastes, `client.send`, clicks, wheel turns, drags, fold-marker clicks and mouse input to programs SHALL take effect on the server in the order they happened, apart from the Non-goals' exception.
- FR-013: Anything that depends on the selection, the layout or the folds, such as typed keys, navigation, swap and keyboard folds, SHALL be resolved against the state after everything before it has taken effect, not the state when it was pressed.
- FR-014: Anything that names its own target, such as a click on a pane, row or fold marker, SHALL act on what was clicked. When that target is gone by its turn, it SHALL do nothing.
- FR-015: Mode changes, the sidebar toggle and config reloads SHALL apply when pressed.
- FR-016: A failed action SHALL show its error. Later input SHALL still run.
- FR-017: When the connection drops, input not yet confirmed SHALL be dropped and SHALL NOT be sent after reconnecting. Detaching SHALL take effect after all earlier input.

### Bindings

- FR-018: A binding SHALL be a table with an optional `action`, holding exactly one action, and an optional `hint`: `"alt-r" = { action = { client.mode = "resize" } }`, `"alt-n" = { action = { … }, hint = "new tab" }`. `{}` SHALL unbind the chord. A list, or any other key in the table, SHALL be a config error. The status line SHALL show `hint` when set, else a word taken from the action as it does today (the mode's name, the direction, the key sent, or the verb). A file binding SHALL still replace the default on its chord whole. This amends keys-config FR-012 and FR-030 and terminal-client's "Custom binding hint" scenario.

## Success criteria

- SC-001: With 500 ms added each way, `alt-n` then `echo hi` + Enter runs in the new tab 10 times in 10, and `alt-|` then `alt-f` zooms the new pane 10 times in 10.
- SC-002: Against a local server, typing and navigation feel no slower than before the change.
- SC-003: A script steers a client to a pane, a tab and nothing with `ship view select`, and the client follows each within a frame.
- SC-004: A CLI create without `--select` never moves any client's selection.
- SC-005: Implementation Rust and tests are reported separately. The change is expected to remove client code (the `chosen` path, the create special case, nested binding lists), but no estimate is made before architecture.

## Decisions

Settled in conversation on 2026-10-11 and 2026-10-12:

| Decision | Why |
| --- | --- |
| Selection stays per attachment on the server, steerable by naming the attachment | Scripts and agents can steer one client; Herdr's per-client focus isn't addressable from outside |
| Every endpoint takes IDs only; no "selected" target | No "anything resolves to an ID" ambiguity, no pull toward server-side logic |
| Input resolves when its turn comes, on the client | Gives in-sequence results on any link without server resolution; costs a round trip per dependent step. "i'm happy with that. later we can worry about pipelining and server side resolution." |
| Clicks wait their turn but keep their own target | A click is still an event in time; skipping ahead would let a pending create undo it |
| Creates select only when asked, in one edit | Agents don't steal focus; no half-done "tab exists, not on it" state. Follows tmux (`-d`) with the opposite default for scripts. "yeah this should be an option on both tabs and panes" |
| Combine only what needs the result | Added to `docs/agents/code.md`; decides which effects become parameters |
| Selecting behind a zoom unzooms for everyone | Today's behavior; Cyan agreed |
| A tab selection sends typing to the landing pane | Memory is client-side, so the server can't pick the pane; Cyan agreed |
| One action per binding | Settles error handling and client/server mixing; `--starter` and `--select` cover the lists in use. "just keep it as client.mode for now" |
| A binding names its `action` and may set a `hint` | The old hint was read from raw TOML, which forced a hand-written deserializer; a named field lets serde derive it, and the fallback comes from the action itself. Bindings are being rewritten anyway, so users rewrite once. "yeah strum is exactly what i was thinking! let's update the papers please!" |
| Pipeline, client-minted IDs and server resolution deferred | A speed upgrade, not a correctness one; trigger is lane waits feeling slow |

### Changes to locked papers and other documents

- keys-config product paper: FR-012 and FR-030 amended by FR-018; A-5's wait between list items is retired with lists.
- `terminal-client` spec: the "Custom binding hint" scenario's binding becomes `"f" = { action = { server.pane.zoom = {} } }`, and a scenario for a set `hint` is added.
- Specs: `keymap` (one action, order of input), `session-observation` (who may change a selection, the zoom rule, create selection), `observer-recovery` unchanged.
- `AGENTS.md`, "Client/server boundary": selection is per-client state held by the server per attachment; folds, memory and sidebar state stay client-local. Edited only after this paper locks.
- `GLOSSARY.md`: "Selection" gains its owner and who may set it.
- `docs/guide.md`: keys section, `--select`, `ship view`.

### Deferred, to file when the pull request lands

Nothing is filed now; Cyan: "let's not file anything now and have them all for the end".

- Close #20.
- Pipeline with client-minted IDs and server-side resolution, including the action-overtakes-keys gap.
- `server.run` and `client.run`: argv, no `~` expansion, `SHIP_ATTACHMENT_ID` and `SHIP_SERVER_URL`, blocking the lane until exit, non-zero exit cancels.
- Several servers in one client: a lane per server, navigation that never crosses servers plus explicit server actions, `area: null` for a server not being drawn, upgrading every server together.
- A pending indicator.

## Open unknowns

- U-1 (settled at lock): a CLI user gets an attachment ID from `ship view list`, which prints each attachment with its selection and also serves SC-003. Run actions will later set `SHIP_ATTACHMENT_ID` as well. The rejected alternative deferred `ship view` and CLI `--select` to the run-actions change.
