# Pane defaults and exit behavior

Status: Locked on 2026-10-21, approved in Plannotator review ("LGTM"). Written from a design conversation with Cyan on 2026-10-21. Cyan agreed in chat to: closing on exit by default, with `keep` for agents that want the output; moving `server.shell` with no alias, since nothing is released; per-pane `scrollback_bytes` on create and set; saving both settings per pane; and the six recommendations in the interview round ("i agree with your defaults"). Cyan chose the name `exit_behavior`.

Amended on 2026-10-21, approved by Cyan in chat without review ("update the architecture paper and the product one! i don't have to review the product one"). FR-003 now allows a wait to fail when it falls too far behind the server, which the chosen architecture requires. FR-003 previously read: "A `ship pane wait` that is waiting when a program exits MUST see the exit, with its exit status, before the pane is closed." That wording is superseded.

## Goal

When a shell in a pane exits, the pane stays on screen with `exited (0)` until it's closed by hand. tmux, Zellij and Herdr all close it, so Ship feels broken here (#16). An agent driving Ship wants the opposite: a pane that stays after its command exits, so it can read the output first.

This change gives every pane an exit behavior, `close` by default, which can be set when the pane is created or later. It also gathers the settings a new pane starts with under one config table, so the shell, history size, right-click handling and exit behavior are all set in one place and can each be overridden per pane.

```toml
[server.defaults.pane]
# shell = "/bin/zsh"
scrollback_bytes = 10_000_000
right_click_passthrough = false
exit_behavior = "close"            # "close" | "keep" | "keep_failed"
```
```console
$ ship pane create --exit-behavior keep -- cargo test
$ ship pane set --pane P --exit-behavior keep_failed --scrollback-bytes 50000000
```

## User stories

### P1: A shell's pane closes when the shell exits

- **Given** a pane running a shell with the default exit behavior, **when** the user types `exit`, **then** the pane closes as `ship pane close` would: its neighbor takes the space and the selection moves there.
- **Given** that pane is the only one in its tab, **then** the tab stays, now empty, as it does after a manual close.

Tested independently by exiting shells in split and lone panes with a client attached.

### P2: An agent reads a command's output after it exits

- **Given** a script runs `ship pane create --exit-behavior keep -- cargo test`, **when** the program exits, **then** the pane stays with its last screen and exit status, `ship pane read` returns its output, and the script removes it with `ship pane close`.
- **Given** a script waits with `ship pane wait --status exited` on a pane whose exit behavior is `close`, **when** the program exits, **then** the wait still succeeds and prints the pane with its exit status, even though the pane then closes. A wait that has fallen far behind the server fails saying so instead of guessing.

Tested independently with scripted `create`, `wait`, `read` and `close` calls.

### P3: Failures stay on screen, successes go away

- **Given** a pane with `keep_failed`, **when** its program exits with code 0, **then** the pane closes. **When** it exits with a non-zero code or is killed by a signal, **then** the pane stays.

Tested independently with `sh -c 'exit 0'`, `sh -c 'exit 3'` and a killed `sleep`.

### P4: Defaults for new panes live in one table

- **Given** a config file sets `[server.defaults.pane] exit_behavior = "keep"` and is saved on a running server, **when** a pane is created afterwards, **then** it starts with `keep`. Panes created before keep their values.
- **Given** a config file still has `[server] shell`, **when** the user runs `ship config check`, **then** it reports the unknown field with its line and column.
- **Given** `ship config defaults`, **then** the output shows the new table with what each setting accepts.

Tested independently with config edits on a running server and `ship config check`.

### P5: Changing a live pane

- **Given** a running pane, **when** a script or a key binding sets `exit_behavior`, **then** the new value applies when the program exits.
- **Given** a pane whose program has already exited, **when** its exit behavior is set, **then** the value is stored and nothing else happens.
- **Given** a pane with history, **when** `scrollback_bytes` is raised, **then** its history is kept and can grow further. **When** it's lowered, **then** the oldest history beyond the new limit is dropped right away.

Tested independently with `ship pane set` and a `server.pane.set` key binding.

### P6: Settings survive a restart

- **Given** a pane with `exit_behavior = "keep"` and `scrollback_bytes = 50000000`, **when** the server restarts, **then** the restored pane, a fresh shell, has the same two values.

Tested independently by stopping and restarting a server with saved tabs.

## Non-goals

- Panes or tabs inheriting settings live from a parent, tmux-style. A pane takes its values when it's created and keeps them.
- Tab-level defaults.
- Running a command or hook when a program exits.
- tmux's `key` mode, which keeps the pane until a key is pressed.
- Closing a tab when its last pane closes on exit.
- A line limit for history. The limit stays in bytes.
- An alias or migration for `[server] shell`.
- Exit behavior or other settings as flags on `ship tab create --starter`. A starter pane uses the defaults and can be changed with `ship pane set`.
- Rerunning a pane's earlier program on restore. Restored panes still start fresh shells.

## Functional requirements

- **FR-001** Every pane MUST have an exit behavior of `close`, `keep` or `keep_failed`. It is shared state, the same for every client.
- **FR-002** When a pane's program exits, the server MUST close the pane exactly as `ship pane close` would if the exit behavior is `close`, or if it is `keep_failed` and the program exited with code 0. Otherwise the pane MUST stay as exited panes do today.
- **FR-003** A `ship pane wait` that is waiting when a program exits MUST see the exit, with its exit status, before the pane is closed, unless it has fallen more than a fixed number of revisions behind the server. In that case it MUST fail with an error saying it fell behind, never with a report that the pane closed.
- **FR-004** `[server.defaults.pane]` MUST hold `shell`, `scrollback_bytes`, `right_click_passthrough` and `exit_behavior`, with the defaults: the login shell, 10,000,000, false and `close`. `[server] shell` and `[server] scrollback_bytes` MUST no longer be accepted.
- **FR-005** The server MUST read `[server.defaults.pane]` when it creates a pane, so a saved change applies to panes created afterwards and not to existing ones. When the file is bad, the server MUST use the built-in defaults and log the error.
- **FR-006** `ship pane create` and the `server.pane.create` action MUST accept `exit_behavior`, `scrollback_bytes` and `right_click_passthrough`. A value given there MUST override the default for that pane.
- **FR-007** `ship pane set` and the `server.pane.set` action MUST accept `exit_behavior` and `scrollback_bytes`. Setting the exit behavior of a pane whose program has exited MUST store the value and do nothing else.
- **FR-008** Raising a pane's `scrollback_bytes` MUST keep its history. Lowering it MUST drop the oldest history beyond the new limit right away. The help for `ship pane set` and the guide MUST say so.
- **FR-009** `ship pane get`, `ship tab list` and the other views of a pane MUST report `exitBehavior` and `scrollbackBytes` next to `rightClickPassthrough`.
- **FR-010** Saved tabs MUST store each pane's exit behavior and `scrollback_bytes`, and restore MUST give the restored pane those values.
- **FR-011** The guide MUST describe the defaults table, exit behavior with the agent workflow from P2, and per-pane history size. The README's "Not built yet" line about closing panes on exit MUST be removed.

## Success criteria

- **SC-001** Typing `exit` in a default shell pane closes it, with no manual close.
- **SC-002** A script can create a pane, wait for its program to exit, read its output and close it, using only `ship` commands.
- **SC-003** `ship config check` on a file with `[server] shell` names the field, line and column.
- **SC-004** After a restart, `ship tab list` shows the same `exitBehavior` and `scrollbackBytes` for each restored pane as before.
- **SC-005** The specs for exited programs, pane history size, the shell setting, set attributes and restore are updated to match.

## Open unknowns

None.
