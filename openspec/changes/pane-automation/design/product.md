# Pane automation

Status: Locked on 2026-10-19, approved by Cyan after Plannotator review ("LGTM") and a check of the revision that removed `--enter` ("i checked the new version already"). Written on 2026-10-19 from a design conversation with Cyan. U-1 and U-2 stay open and are probed before the requirements they block are implemented. The approval doesn't authorize implementation; Cyan requests that separately.

Amended on 2026-10-19 during architecture at Cyan's direction: the published revision is the source of truth, so status that never reaches a revision doesn't count, and a pane already `working` before the send counts as having picked up the input, as in Herdr. FR-018, SC-003, the second P1 story and FR-023 changed. Cyan approved the amendment without a further review ("please update it then").

## Goal

Milestone 4 in the product brief asks that an agent running in one pane can drive another: start a sibling, give it work, wait for the result and read it. Most of that already exists. `ship pane create` starts a program and prints the new pane with its ID, `ship tab list` and `ship pane get` show every pane with its program status, and `ship pane read` prints output and history. Two pieces are missing: sending input to a pane, and waiting for a pane's status to change. This change adds `ship pane send` and `ship pane wait`.

Both work on **program status**, the OSC 7501 records a pane's program reports, not on agents. Ship doesn't detect agents, name them or keep a list of them. Whatever a pane's program reports is what Ship acts on, so the commands work the same for Claude Code, a build script or anything else that reports. Herdr infers status by matching each agent's screen against a maintained manifest; Ship relies on programs reporting it themselves.

The loop the brief asks for:

```bash
id=$(ship pane create --name worker -- claude | jq -r .id)
ship pane send --pane "$id" "fix the flaky test in saver.rs" --key enter --wait
ship pane read --pane "$id" --lines 80
```

## User stories

### P1: Give a sibling work and wait for it to finish

An orchestrating agent starts a worker in a new pane, sends it a prompt and blocks until the worker stops working.

- **Given** a pane whose program reports `idle`, **when** the caller runs `ship pane send --pane ID "TEXT" --key enter --wait`, **then** the text arrives as one paste followed by Enter, the command waits until the program has reported `working` or `blocked` after the send and then something other than `working`, and it prints the pane as JSON.
- **Given** the program reports `working` and then `idle` within a fraction of a second, **when** `send --wait` is waiting, **then** it returns as long as `working` reached a published revision. A `working` that never reached one didn't happen.
- **Given** the program is already `working` on an earlier prompt, **when** the caller runs `send --wait`, **then** the input is written and the command returns when that turn ends.
- **Given** the worker stops at a permission prompt and reports `blocked`, **when** `send --wait` is waiting, **then** it returns, and the printed JSON shows `blocked`, so the caller can tell "needs a human" from "finished".
- **Given** the caller's own tool timed out during `send --wait`, **when** it runs `ship pane wait --pane ID`, **then** that wait returns once the worker stops working, and the input is not sent again.
- **Given** the user selects a different pane in the client while a wait is running, **then** the wait and any later command still act on the pane given by `--pane`.

Tested independently with a script in a pane that emits OSC 7501 reports on input, driving `send --wait` and `wait` from another pane.

### P2: Send input without waiting

- **Given** any pane, **when** the caller runs `ship pane send --pane ID --key ctrl-c`, **then** the program receives Ctrl+C.
- **Given** a multi-line prompt on stdin, **when** the caller runs `ship pane send --pane ID - --key enter`, **then** the whole text arrives as one paste and a single Enter follows it.
- **Given** `--key esc --key enter` with text, **then** the text arrives first, then Escape, then Enter.
- **Given** a pane scrolled back into its history, **when** input is sent, **then** the pane returns to the bottom, as interactive input does.

Tested independently by sending to `cat -v` or a shell and reading the pane.

### P3: Wait for a particular status, or for the program to end

- **Given** a worker reporting `working`, **when** the caller runs `ship pane wait --pane ID --status blocked`, **then** it returns only once the root record reports `blocked`.
- **Given** a pane running `cargo test`, **when** the caller runs `ship pane wait --pane ID --status exited`, **then** it returns when the program exits, and the printed JSON carries its exit status, even when the pane closes on exit.
- **Given** `--status done,exited`, **then** the wait returns on whichever happens first.
- **Given** the condition already holds, **when** the caller runs `wait`, **then** it returns at once.

Tested independently with a scripted reporter and with short-lived commands.

### P4: Clear failures

- **Given** a pane showing a permission dialog and reporting `blocked`, **when** the caller runs `send --wait`, **then** nothing is written and the error names the pane and its blocked kind. A plain `send --key esc` still reaches the dialog.
- **Given** a plain shell pane with no status record, **when** the caller runs `send --wait`, **then** nothing is written and the error says the pane has no program status.
- **Given** a program that doesn't report OSC 7501, or one that never starts working, **when** `send --wait` has sent its input, **then** after 5 seconds without `working` or `blocked` it fails with an error saying the pane stalled.
- **Given** a wait without `exited` in its condition, **when** the program exits, **then** the wait fails with an error saying the program exited, including its exit status.
- **Given** a pane that is closed, or an ID that never existed, **then** `send` and `wait` fail with an error naming the pane.

Tested independently by provoking each case from a script.

## Non-goals

- Agent detection, agent names, agent kinds, or an `agent` command group. Panes are targeted by ID only.
- Waiting for text to appear in a pane's output. Program status covers agents; a later change can add output waits if a workflow needs them.
- An event stream for CLI callers. `wait`, `pane get` and `tab list` cover the brief's "receiving state/event updates".
- A `--timeout` flag. Callers use `timeout(1)` or their tool's own limit; the guide shows how.
- Guarding plain `send`. Without `--wait`, `send` is raw input to whatever is running.
- Detecting an agent that exited back to a shell and left a `done` or `idle` record behind. Ship reports the record as it is; agents can prevent this by sending `clear` before they exit.
- Interleaving text and keys in one `send`.
- Separate exit codes per error.
- Waiting on non-root records, or on several panes at once.

## Functional requirements

### Targeting

- **FR-001** `send` and `wait` MUST target the pane given by `--pane`, else the pane in `SHIP_PANE_ID`, and MUST fail when neither is set. They MUST NOT fall back to any attachment's selection.
- **FR-002** A malformed ID, an unknown ID, and a pane closed before or during the command MUST each fail with an error naming the pane.

### `ship pane send`

- **FR-003** `send` MUST accept an optional text argument, where `-` means read the text from stdin, then any number of `--key KEY` options. It MUST fail when given neither text nor keys.
- **FR-004** The text MUST be delivered as one paste, the same way a client's paste is delivered. The keys MUST follow it in the order given.
- **FR-005** `--key` MUST accept the key names the keymap config accepts. All keys MUST be validated before anything is written, so an invalid key sends nothing.
- **FR-006** Input MUST return a scrolled pane to the bottom, as `pane-history` already requires of input.
- **FR-007** Without `--wait`, `send` MUST print nothing on success. Input to a pane whose program has exited MUST be accepted and dropped, as interactive input is.

### `ship pane wait`

- **FR-008** `wait` MUST succeed when the pane's root status record matches the condition: by default, any state other than `working`; with `--status STATE,...`, any listed state. `--status` MUST accept `idle`, `working`, `done`, `blocked`, `error` and `exited`.
- **FR-009** When the condition already holds, `wait` MUST return at once.
- **FR-010** `exited` MUST match when the pane's program exits. When a pane closes because its program exited, a waiter MUST still receive the exit status.
- **FR-011** When the program exits and `exited` isn't in the condition, `wait` MUST fail with an error saying the program exited and giving its exit status.
- **FR-012** When the pane has no root record and `exited` isn't in the condition, `wait` MUST fail at once with an error saying the pane has no program status.
- **FR-013** On success, `wait` MUST print the pane as `ship pane get` would print it at the moment the condition matched.
- **FR-014** `wait` MUST NOT time out on its own. Ending the CLI process MUST end the wait and leave no effect on the pane or server.
- **FR-015** Running `wait` again after an earlier `wait` or `send --wait` ended early MUST be safe. It MUST NOT send input.

### `send --wait`

- **FR-016** `send --wait` MUST accept the same `--status` option as `wait`, with the same meaning. `--status` on `send` without `--wait` MUST be a usage error.
- **FR-017** Before writing anything, `send --wait` MUST fail if the pane's root record is `blocked`, naming the blocked kind, or if the pane has no root record.
- **FR-018** After writing, `send --wait` MUST NOT treat a status published before the write as satisfying the wait. It MUST first see the root record `working` or `blocked` in a revision published after the write, or have seen it `working` already before the write, then apply the condition from FR-008. Status that never reaches a published revision doesn't count.
- **FR-019** If neither `working` nor `blocked` is reported within 5 seconds of the write, `send --wait` MUST fail with an error saying the pane stalled. The error MUST make clear that the input was sent.
- **FR-020** FR-010 through FR-014 MUST apply to `send --wait` once its input is written.

### Errors

- **FR-021** Errors MUST follow the CLI's existing convention: a message on stderr and exit status 1, with usage errors keeping clap's exit status 2.

### Documents

- **FR-022** The product brief's milestone 4 MUST be amended:
  - "sessions, tabs, panes and agents" becomes "tabs, panes and their program status";
  - "Prompting a named agent" becomes "Sending a prompt to a pane and waiting for it to be handled";
  - "Waiting for output or an agent-state transition" becomes "Waiting for a program-status transition or exit";
  - "Receiving state/event updates" is met by `wait` and the existing listing commands;
  - the creation-response contract is settled by `ship pane create` printing the new pane.
- **FR-023** The guide MUST show the orchestration loop, how to bound a wait with `timeout(1)` (including `gtimeout` from coreutils on macOS), that a failed or timed-out `send --wait` must not be resent blindly, that `send --wait` on a pane that is already working returns when the current turn ends, and the stale-record limitation from Non-goals.

## Success criteria

- **SC-001** From inside a Ship pane, a script starts a sibling running an agent that reports OSC 7501, sends it a prompt with `send --wait`, and reads its answer with `pane read`, using only IDs from command output. Verified on Linux and macOS.
- **SC-002** Switching tabs and panes in an attached client throughout SC-001 doesn't change which pane receives input or which pane is waited on.
- **SC-003** Against a reporter that stays `working` for 50 ms after input, then reports `idle`, `send --wait` returns in 50 out of 50 tries.
- **SC-004** Each failure in P4 produces its own error message, and `send --wait` against a pane that never reports fails within 6 seconds.
- **SC-005** `timeout 2 ship pane wait --pane ID` against a `working` pane exits with status 124, and the pane and server are unaffected.

## Open unknowns

- **U-1: Enter right after a paste.** Herdr had to add a paste boundary before Enter for Codex on Windows. Whether any agent that reports OSC 7501 drops or mishandles an Enter that arrives immediately after a paste is unprobed. If one does, FR-004 needs a rule for the gap. Blocks FR-004.
- **U-2: What agents report around a prompt.** The order of OSC 7501 reports the agents that already support the protocol emit when a prompt arrives is unprobed: whether `working` comes promptly and within FR-019's 5 seconds, and whether any re-reports `idle` first. Blocks FR-018 and FR-019.
