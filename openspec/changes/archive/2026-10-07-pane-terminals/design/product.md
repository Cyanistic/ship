# Pane terminals

Status: Locked on 2026-10-06. Cyan approved the first draft in Plannotator with "LGTM" and the revision that resolved the open unknowns with "GO GO GO". This paper defines observable behavior, not architecture or implementation. Decisions come from the 2026-10-06 grilling and product interview. The broader [product brief](../../../../product-brief.md) remains the roadmap, and the locked [session/tab roundtrip](../../archive/2026-10-06-session-tab-roundtrip/design/product.md) remains in force except where this paper amends it.

## Summary

Every pane gets a real program running in a real terminal. `ship attach` becomes a full-screen client that shows the selected pane, sends it keys and pastes, and keeps work running after you detach. Panes and tabs no longer need names. Navigation is a few fixed placeholder keys; real keybindings, splits, a sidebar and scrollback are later slices.

Session switching inside the client is also a placeholder key pair, cycling through sessions.

## Goal

Let Cyan run shells, Pi and Neovim inside Ship panes from an attached client, detach and reattach without interrupting them, and watch the same pane from two clients. This proves the server-owned terminal path (process, screen, input, size, lifecycle) before layouts, keybindings and the sidebar build on it.

## User stories

### P1: Work in a shell inside Ship

**Given** no session named `work`, **when** I run `ship attach work`, **then** a session `work` is created with one tab holding one pane running my `$SHELL`, that pane is selected, and I can type into it immediately.

**Given** an attached client, **when** I type, paste or press special keys (arrows, Ctrl combinations, function keys), **then** the program in the selected pane receives them in the order I pressed them, and its screen updates in my client. A paste arrives as one paste, so editors do not auto-indent it line by line.

**Given** Neovim or Pi running in a pane, **when** it switches to its full-screen view and back, **then** the client shows each screen correctly, including colors, styles, Unicode and the cursor.

**Given** an attached client, **when** I resize my terminal window, **then** the program in the pane sees the new size and redraws to fit.

Independent verification: attach to a new session, run `ls --color`, `nvim`, and Pi, paste a multi-line block into Neovim insert mode, resize the window during each, and exit each program.

### P1: Detach and come back

**Given** a program running in a pane, **when** I press `C-b d`, close the terminal window, or the connection drops, **then** the program keeps running on the server.

**Given** that program still running, **when** I attach again, **then** the session's first pane in tree order is selected, I see its current screen and can keep typing into it.

**Given** a client whose connection drops while attached, **when** the connection is lost, **then** the client keeps showing the last screen, the status line says it is disconnected and reconnecting, and keys I type meanwhile are discarded. When the connection returns, the client shows current screens without any further action from me.

**Given** an attached client, **when** it exits for any reason (detach, session removed, server stopped, crash), **then** my terminal is restored to how it was before attaching.

Independent verification: start `top` or a counting loop in a pane, detach, wait, reattach and confirm it kept running. Kill the client process with a signal and confirm the outer terminal is usable. Interrupt the connection and type during the outage.

### P1: Manage panes from the CLI

**Given** a tab, **when** I run `ship pane create <tab>`, **then** a pane running `$SHELL` in my current directory is created and printed as JSON. With `--cwd DIR` it starts in `DIR`; with `-- COMMAND ARGS...` it runs that command instead of the shell.

**Given** a pane, **when** its program exits, **then** the pane stays, keeps its last screen, and the status line shows `exited (code)`. Input to it is ignored.

**Given** a pane with a running program, **when** I run `ship pane rm <id>`, **then** the pane disappears and its program and every process it started are ended. Programs get a short chance to exit cleanly before being forced.

**Given** a tab or session containing running panes, **when** I remove the tab or session, or stop the server, **then** every program in it ends the same way. Clients attached to a removed session or a stopped server report why they ended, restore the terminal and exit.

Independent verification: create panes with and without `--cwd` and a command, inspect them with `ship pane get`, exit one program and observe `exited (code)`, remove a pane running `sleep 1000 &` from a shell and confirm with `ps` that both processes are gone.

### P1: Move between panes

**Given** a tab with several panes, **when** I press `C-b n` or `C-b p`, **then** the next or previous pane in that tab is selected and shown.

**Given** my selected pane, **when** it is removed, **then** the next pane in the same tab is selected, or the previous one if it was last. Only when the tab has no panes left does selection fall back to the tab.

**Given** a selection that is a tab or session rather than a pane, **when** the client draws, **then** it shows an empty state with a hint such as `no pane: ship pane create <tab-id>`, and `C-b n` or `C-b p` select the first or last pane of the selected tab.

**Given** several sessions, **when** I press `C-b )` or `C-b (`, **then** the client switches to the next or previous session in creation order and selects that session's first pane, or the session itself when it has no panes.

**Given** a program that uses `C-b` itself, **when** I press `C-b C-b`, **then** the pane receives one `C-b`.

Independent verification: create three panes in a tab, cycle through them, remove the selected one from another terminal, and remove the remaining ones until the empty state appears. Create a second session and cycle between sessions.

### P2: Share a pane between two clients

**Given** two clients viewing the same tab, **when** either types, **then** the pane receives input from both, and both clients show the same screen.

**Given** two clients of different sizes viewing the same tab, **when** both are attached, **then** the tab's panes use the smallest of their sizes. The larger client draws the pane at its real size in the top-left corner and fills the leftover area with a dim pattern. A client viewing a different tab does not constrain it. A tab nobody is viewing keeps its last size.

Independent verification: attach two clients of different window sizes to one session, view the same tab, type from both, then move one client to another pane in a different tab and confirm the size grows.

### P2: Names are optional

**Given** a pane without a name, **when** a client shows it, **then** its label is the program's live title, or the command it runs (such as `zsh`) when there is no title.

**Given** a tab without a name, **when** a client shows it, **then** its label is its first pane's label, or its position among its siblings (`1`, `2`, ...) when it has no panes.

**Given** a named tab or pane, **when** I rename it with no name or an empty or whitespace-only name, **then** it becomes unnamed and shows its derived label again.

Independent verification: create unnamed tabs and panes, run `nvim` in a shell pane and watch the label follow its title, rename and clear names.

## Non-goals

- Splits, pane layouts and showing more than one pane at once.
- A sidebar, tab bar or any tab UI. Tabs are created, moved and removed through the CLI as before.
- Configurable keybindings, a configurable prefix, key help, confirmations, and in-client pane create or remove keys. The `C-b` keys in this paper are placeholders the keybindings slice replaces.
- Scrollback. Output that scrolls off the top of the screen cannot be viewed again in this slice.
- Mouse input, text selection and clipboard integration.
- Restarting an exited program in place, or closing a pane automatically when its program exits.
- Reporting a pane's live working directory or foreground program.
- Matching the client's real terminal colors when programs ask for them, and converting colors for terminals without 24-bit color.
- Optional session names. Sessions still require a name.
- Remote attachment, authentication, disk persistence and server restart restoration. Stopping the server ends every program.
- Agent detection, notifications and an automation command catalog (sending text or keys to a pane from the CLI, reading its screen).
- Full Windows support. Windows is best effort and unverified.

## Functional requirements

- **FR-001:** Every pane MUST run exactly one program in its own terminal, started when the pane is created. A pane MUST NOT exist without having started its program; if the program cannot be started, pane creation MUST fail and change nothing.
- **FR-002:** `ship pane create <tab>` MUST start the user's `$SHELL` by default, MUST accept `--cwd DIR` for the starting directory (default: the CLI's current directory), and MUST accept `-- COMMAND ARGS...` to run a command instead. `--name NAME` MUST be optional.
- **FR-003:** Programs in panes MUST see a terminal that advertises 256 colors and 24-bit color.
- **FR-004:** `ship attach <session>` MUST open a full-screen client in the current terminal that shows the selected pane's screen and a one-line status line. The text observer and its stdin `select`/`switch` controls MUST be removed.
- **FR-004a:** Attaching to an existing session MUST select its first pane in tree order when one exists, and the session otherwise. Reconnecting MUST keep the roundtrip's selection retention rules.
- **FR-005:** Attaching by a name that does not exist MUST create the session with one tab containing one `$SHELL` pane, and select that pane. Creating a session with `ship session create` MUST still create an empty session.
- **FR-006:** The client MUST deliver keys, pastes and special keys to the selected pane in the order the user produced them. Pastes MUST arrive as a single paste.
- **FR-007:** The client MUST show screen changes as programs produce them, including colors, styles, Unicode, wide characters, the cursor and full-screen applications, without leaving a stale final screen after output stops.
- **FR-008:** The fixed keys MUST be: `C-b n` next pane in the tab, `C-b p` previous pane in the tab, `C-b )` next session, `C-b (` previous session, `C-b d` detach, `C-b C-b` send one `C-b` to the pane. Sessions cycle in creation order, and switching selects the new session's first pane in tree order, or the session when it has no panes. This replaces in-client switching by name or ID; the HTTP switch route keeps accepting any session. Every other key MUST go to the pane.
- **FR-009:** The status line MUST show the session, tab and pane labels, the pane's exit status once its program has exited, and a disconnected indicator while reconnecting.
- **FR-010:** A tab's panes MUST be sized to the smallest client currently viewing that tab, minus the status line. A tab with no viewers MUST keep its last size. A client larger than the tab MUST draw the pane in its top-left corner and mark the unused area.
- **FR-011:** Multiple clients MUST be able to send input to the same pane at the same time; there is no exclusive writer.
- **FR-012:** When a pane's program exits, the pane MUST remain with its last screen and exit status until removed, and MUST ignore input.
- **FR-013:** Removing a pane, removing a tab or session containing it, or stopping the server MUST end the pane's program and every process it started in its terminal, giving them about two seconds to exit after the terminal closes before forcing them. No process from a removed pane may be left running or unreaped on Linux or macOS.
- **FR-014:** Detaching, closing the client's terminal window, or losing the connection MUST NOT affect running programs.
- **FR-015:** While disconnected, the client MUST keep the last screens, show the disconnected indicator and discard typed keys. After reconnecting it MUST show current state without user action.
- **FR-016:** The client MUST restore the user's terminal on every exit path, including panics and signals it can handle.
- **FR-017:** When a selected pane is removed, selection MUST move to the next pane in the same tab, or the previous one if it was last, and fall back to the tab only when no panes remain. This amends the roundtrip's pane-removal fallback. Other fallback rules are unchanged.
- **FR-018:** When the selection is a tab or session, the client MUST show an empty state with a hint for creating a pane, and `C-b n` or `C-b p` MUST select the first or last pane of the selected tab.
- **FR-019:** Tab and pane names MUST be optional. An empty or whitespace-only name MUST mean no name, on create and rename. Renaming without a name MUST clear the name.
- **FR-020:** An unnamed pane's label MUST be its program's current title, else its command. An unnamed tab's label MUST be its first pane's label, else its 1-based position among its siblings. Labels MUST update as titles change.
- **FR-021:** Session names MUST remain required. An empty or whitespace-only session name MUST be rejected without changing anything.
- **FR-022:** `ship pane get` MUST report the pane's command, starting directory, title and status (running, or exited with its code).

## Success criteria

- **SC-001:** On Linux and macOS, `ship attach <new name>` lands in a working shell, and the P1 shell workflow (colored `ls`, Neovim with a multi-line paste, Pi, resizing during each) works with no visible corruption or reordered keystrokes.
- **SC-002:** A program started before detaching is still running and current after reattaching, after closing the terminal window, and after a dropped connection.
- **SC-003:** After `ship pane rm` on a shell running `sleep 1000 &`, `ps` shows neither process within three seconds. After stopping the server, no pane process remains.
- **SC-004:** An exited program's pane shows its last screen and `exited (code)` in the status line, and `ship pane get` reports the same code.
- **SC-005:** Killing the client with SIGTERM, and forcing a client panic in a temporary probe, both leave the outer terminal usable without `reset`.
- **SC-006:** Two clients of different sizes on the same tab show identical screens at the smaller size; moving the smaller client to another tab lets the first tab grow.
- **SC-007:** Typing into a pane feels at least as responsive as the same program in tmux on the same machine, judged in real use. A sustained `cat` of a large file does not make other panes or typing feel sluggish.
- **SC-008:** Unnamed panes and tabs show derived labels that follow the program's title, and clearing a name restores the derived label.
- **SC-009:** Checks not run on a platform are reported as unverified there; Windows is reported as unverified throughout.

Verification is through real workflows and temporary probes; this paper does not authorize permanent test code.

## Open unknowns

None. Resolved during review on 2026-10-06:

1. Attaching to an existing session selects its first pane in tree order (FR-004a).
2. Session switching inside the client uses placeholder keys `C-b )` and `C-b (` (FR-008), replacing the stdin `switch` control. Switching by name or ID remains available through the HTTP route only.
3. CLI spelling as proposed: `ship pane create <tab> [--name NAME] [--cwd DIR] [-- COMMAND ARGS...]`, `ship tab create <parent> [--name NAME]`, `ship tab|pane rename <id> [NAME]`. The tab name moves from a positional argument to `--name`.
