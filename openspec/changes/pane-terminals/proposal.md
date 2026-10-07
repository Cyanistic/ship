# Proposal

## Why

Ship can hold sessions, tabs and panes, but a pane is only a name, so nothing useful runs inside it yet. This change gives every pane a real program in a real terminal and replaces the text observer with a full-screen client, proving the server-owned terminal path (process, screen, input, size, lifecycle) before layouts, keybindings and the sidebar build on it.

## What Changes

- Every pane runs exactly one program in its own terminal, started when the pane is created. `ship pane create <tab>` starts a login shell in the CLI's current directory, takes `--cwd DIR`, and runs `-- COMMAND ARGS...` instead when given. A program that can't start fails creation and changes nothing.
- An exited program's pane stays with its last screen and exit status and ignores input. `ship pane get` reports the command, starting directory, live title and status.
- Removing a pane, its tab or session, or stopping the server ends the program and the processes it started in its terminal, with about two seconds to exit before they're killed.
- **BREAKING:** `ship attach <session>` opens a full-screen client in the current terminal instead of printing a text tree. It shows the selected pane's screen and a status line, sends keys and pastes to that pane, and restores the terminal on every exit path. The stdin `select` and `switch` controls are removed.
- Attaching selects the session's first pane. Attaching by a new name creates the session with one tab holding one shell pane. `ship session create` still creates an empty session.
- Fixed placeholder keys: `C-b n` and `C-b p` cycle panes in the tab, `C-b )` and `C-b (` cycle sessions, `C-b d` detaches, `C-b C-b` sends one `C-b`.
- When a selected pane is removed, the selection moves to the next pane in its tab, else the previous one, and falls back to the tab only when no panes remain.
- A tab's panes take the size of the smallest client viewing that tab. A larger client draws the pane in its top-left corner and marks the rest.
- Several clients can view and type into the same pane at once.
- While disconnected, the client keeps the last screen, shows that it's reconnecting and discards keys.
- **BREAKING:** Tab and pane names become optional. A blank name means no name, renaming without a name clears it, and the tab name moves from a positional argument to `--name`. Unnamed panes and tabs show derived labels. Session names stay required.
- **BREAKING:** `PUT /api/v0/attach/view` replaces `PUT /api/v0/attach/selection` and `PUT /api/v0/attach/session`. A streaming `POST /api/v0/attach/input` carries keys and pastes. The attach stream also carries pane screens. The health `protocolVersion` becomes 2, so older clients refuse the server instead of misreading it.

## Capabilities

### New Capabilities

- `pane-programs`: The program behind each pane: starting it, its terminal, its exit status and title, and ending it with every process it started.
- `terminal-client`: The full-screen attached client: drawing the selected pane, sending input, the fixed keys, sizing, the status line, labels and terminal restoration.

### Modified Capabilities

- `session-structure`: Panes are no longer metadata only, tab and pane names become optional, and the CLI spellings change.
- `session-observation`: Attaching opens the full-screen client, a new name creates a starter pane, the selection starts at the first pane, the pane-removal fallback changes, session switching uses keys, and a stopped server ends the client.
- `observer-recovery`: A reconnecting client keeps its last screen, shows the outage in its status line and discards keys typed meanwhile.
- `health-exchange`: The protocol version becomes 2.
- `client-maintenance`: The OpenAPI document describes the attached key event only as an object.

## Impact

All four crates change, and a terminal wrapper is vendored:

- `ship-core`: optional names, pane metadata, screens, input and view types.
- `ship-server`: pane runtimes and teardown, screens on the attach stream, the input and view routes, sizes derived from viewers.
- `ship-client`: the full-screen client and the new request methods.
- `ship`: CLI spellings; the text observer and stdin controls are deleted.
- `vendor/ratatui-ghostty`: the terminal wrapper, copied from `experiments/` with provenance.

New dependencies are portable-pty, ratatui, crossterm (with `serde`), nix and tokio-util, plus axum's `http2` feature. Ghostty's VT library comes in through the wrapper and needs Zig and network access at build time.

The three locked papers in [design/](design/), indexed by [design.md](design.md), are the authoritative design. Implementation follows the program paper's seven slices, one at a time, and starts only when Cyan requests it. Linux and macOS are verified deliberately; Windows stays best effort and unverified. No splits, layouts, sidebar, configurable keybindings, scrollback, mouse, persistence or permanent authored tests are included.
