# pane-automation Specification

## Purpose

Let a script or an agent in one pane drive another: send it input, and wait for its program status to change or its program to exit. Both commands act on **program status**, the records a pane's program reports through OSC 7501 as described by the `program-status` capability. Ship doesn't detect, name or list agents. A wait sees only what reaches a published revision, the same state every client sees; a status that never reaches one didn't happen.

## Requirements

### Requirement: Pane targeting
`ship pane send` and `ship pane wait` SHALL target the pane given by `--pane`, else the pane in `SHIP_PANE_ID`, and SHALL fail when neither is set. They SHALL NOT fall back to any attachment's selection. A malformed ID, an unknown ID, and a pane closed before or during the command SHALL each fail with an error naming the pane.

#### Scenario: Selection changes during a wait
- **WHEN** a user switches tabs and panes in an attached client while `ship pane wait --pane <id>` runs
- **THEN** the wait still acts on `<id>`, and later commands with `--pane <id>` still reach it

#### Scenario: Unknown pane
- **WHEN** a script runs `ship pane send --pane <id> --key enter` for an ID that never existed
- **THEN** it exits 1 with an error naming the pane and nothing is written

### Requirement: Sending text and keys
`ship pane send [TEXT] [--key KEY]...` SHALL deliver TEXT as one paste, delivered the way a client's paste is, then each key in the order given. TEXT `-` SHALL read the text from stdin. Giving neither text nor keys SHALL be a usage error. Input SHALL return a scrolled pane to the bottom.

#### Scenario: Text then keys
- **WHEN** a script runs `ship pane send --pane <id> "hello" --key esc --key enter` against a pane running `cat -v`
- **THEN** `ship pane read --pane <id>` shows `hello`, then Escape, then Enter, in that order

#### Scenario: Multi-line prompt from stdin
- **WHEN** a script pipes three lines to `ship pane send --pane <id> - --key enter`
- **THEN** the three lines arrive as one paste followed by a single Enter

#### Scenario: Interrupt
- **WHEN** a pane runs `sleep 100` and a script runs `ship pane send --pane <id> --key ctrl-c`
- **THEN** `sleep` ends

#### Scenario: Scrolled pane
- **WHEN** a pane is scrolled back into its history and a script sends it input
- **THEN** the pane returns to the bottom

#### Scenario: Nothing to send
- **WHEN** a script runs `ship pane send --pane <id>` with neither text nor keys
- **THEN** it exits 2 with a usage error

### Requirement: Plain send
Without `--wait`, `send` SHALL print nothing on success and SHALL NOT attach to the server. Input to a pane whose program has exited SHALL be accepted and dropped, as interactive input is.

#### Scenario: Exited program
- **WHEN** a pane's program has exited and a script runs `ship pane send --pane <id> "x"`
- **THEN** the command succeeds, prints nothing, and the input is dropped

#### Scenario: No attachment from send
- **WHEN** a script runs `ship pane send` without `--wait`
- **THEN** `ship view list` never shows an attachment for it

### Requirement: Ready states
`ship pane send --if STATE,...` SHALL write nothing unless the pane's root status record reports one of the listed states, with or without `--wait`. `--if` SHALL accept the same values as `--status`. The error SHALL name the pane and its current state, or its blocked kind, or say it has no program status.

#### Scenario: Not the expected state
- **WHEN** a pane's root record is `idle` and a script runs `ship pane send --pane <id> --key enter --if blocked`
- **THEN** nothing is written and the error names the pane and says it is `idle`

#### Scenario: Guarded prompt
- **WHEN** a pane's root record is `blocked` and a script runs `ship pane send --pane <id> "x" --key enter --if idle,done`
- **THEN** nothing is written and the error names the pane and its blocked kind

### Requirement: Key names
`--key` SHALL accept the key names the keymap config accepts, such as `enter`, `esc` and `ctrl-c`. Every key SHALL be parsed before anything is written; an invalid key SHALL be a usage error that sends nothing.

#### Scenario: Invalid key
- **WHEN** a script runs `ship pane send --pane <id> "hi" --key ctl-c`
- **THEN** it exits 2 and the pane receives nothing

### Requirement: Wait condition
`ship pane wait` SHALL succeed when the pane's root status record matches its condition: by default any state other than `working`; with `--status STATE,...`, any listed state. `--status` SHALL accept `idle`, `working`, `done`, `blocked`, `error` and `exited`. When the condition already holds, `wait` SHALL return at once.

#### Scenario: Default condition
- **WHEN** a pane's program reports `working` and later `idle`, and a script runs `ship pane wait --pane <id>` while it is working
- **THEN** the wait returns after `idle` is published and prints the pane with root state `idle`

#### Scenario: Specific state
- **WHEN** a pane's program reports `working` and a script runs `ship pane wait --pane <id> --status blocked`
- **THEN** the wait returns only once the root record reports `blocked`

#### Scenario: Already holds
- **WHEN** a pane's root record is `idle` and a script runs `ship pane wait --pane <id>`
- **THEN** the wait returns at once

### Requirement: Wait result
On success, `wait` SHALL print the pane as `ship pane get` would print it at the revision where the condition matched. A root record that disappears during a wait SHALL match nothing until it returns, the program exits or the pane closes.

#### Scenario: Printed pane
- **WHEN** a wait on a `working` pane returns because the program reported `done`
- **THEN** it prints the pane as JSON in `ship pane get`'s shape, with root state `done`

#### Scenario: Record cleared during a wait
- **WHEN** a program clears its root record while a wait runs, then reports `idle`
- **THEN** the wait keeps waiting through the gap and returns on `idle`

### Requirement: Waiting on exit
`exited` SHALL match when the pane's program exits, and the printed pane SHALL carry its exit status. When the program exits and `exited` isn't in the condition, the wait SHALL fail with an error saying the program exited and giving its exit status.

#### Scenario: Wait for exit
- **WHEN** a pane runs `cargo test` and a script runs `ship pane wait --pane <id> --status exited`
- **THEN** the wait returns when the program exits, and the printed pane shows its exit status

#### Scenario: Either outcome
- **WHEN** a script runs `ship pane wait --pane <id> --status done,exited`
- **THEN** the wait returns on whichever happens first

#### Scenario: Unexpected exit
- **WHEN** a script runs `ship pane wait --pane <id>` and the program exits with code 1 while `working`
- **THEN** the wait exits 1 with an error naming the pane and saying the program exited with code 1

### Requirement: Wait without program status
When a running pane has no root status record, `wait` SHALL keep waiting until the program reports, exits or the pane closes.

#### Scenario: Agent still starting
- **WHEN** a script creates a pane running an agent and at once runs `ship pane wait --pane <id>`
- **THEN** the wait returns once the agent reports `idle`, rather than failing because nothing is reported yet

#### Scenario: Plain shell
- **WHEN** a pane runs a shell that reports no status and a script runs `timeout 2 ship pane wait --pane <id>`
- **THEN** the command exits 124 and the pane is unaffected

### Requirement: Wait lifetime
`wait` SHALL NOT time out on its own. Ending the CLI process SHALL end the wait and leave no effect on the pane or server, including no attachment left in `ship view list`. Running `wait` again after an earlier `wait` or `send --wait` ended early SHALL be safe, and `wait` SHALL NOT send input.

#### Scenario: Bounded by timeout(1)
- **WHEN** a pane's program reports `working` and a script runs `timeout 2 ship pane wait --pane <id>`
- **THEN** the command exits 124, the pane keeps running, and `ship view list` shows no attachment left by the wait

#### Scenario: Waiting again after a timeout
- **WHEN** a `send --wait` is killed by its caller's timeout and the caller runs `ship pane wait --pane <id>`
- **THEN** that wait returns once the program stops working, and the input is not sent again

### Requirement: Send and wait
`ship pane send --wait [--status STATE,...]` SHALL write its input and then wait with the same condition and meaning as `wait`. `--status` on `send` without `--wait` SHALL be a usage error. Once the input is written, the exit, printing and lifetime rules of `wait` SHALL apply.

#### Scenario: Prompt and wait
- **WHEN** a pane's program reports `idle` and a script runs `ship pane send --pane <id> "fix the test" --key enter --wait`
- **THEN** the text arrives as one paste followed by Enter, and the command prints the pane once the program's state has changed from `idle` after the send and then is something other than `working`

#### Scenario: Blocked during the wait
- **WHEN** the program stops at a permission prompt and reports `blocked` while `send --wait` is waiting
- **THEN** the command returns and the printed pane shows `blocked`

#### Scenario: Status without wait
- **WHEN** a script runs `ship pane send --pane <id> "x" --status done`
- **THEN** it exits 2 with a usage error and nothing is written

### Requirement: Checks before sending
Before writing anything, `send --wait` SHALL fail if the pane doesn't exist, if its program has exited, or if its root record doesn't report one of the `--if` states, which default to `idle`, `working`, `done` and `error`. So without `--if` a pane with no root status record or a `blocked` one fails, naming the blocked kind. Each such error SHALL say that nothing was sent. Plain `send` without `--if` SHALL NOT make the status checks.

#### Scenario: Blocked pane
- **WHEN** a pane's root record is `blocked` with kind `permission` and a script runs `send --wait`
- **THEN** nothing is written and the error names the pane and `permission`

#### Scenario: Plain send reaches a dialog
- **WHEN** a pane's root record is `blocked` and a script runs `ship pane send --pane <id> --key esc`
- **THEN** the program receives Escape

#### Scenario: No program status
- **WHEN** a pane runs a shell with no status record and a script runs `send --wait`
- **THEN** nothing is written and the error says the pane has no program status

#### Scenario: Answering a dialog
- **WHEN** a pane's root record is `blocked` and a script runs `ship pane send --pane <id> --key enter --wait --if blocked`
- **THEN** Enter is written, and the command returns only after the root has left `blocked` and then reports something other than `working`

### Requirement: Picking up the input
After writing, `send --wait` SHALL NOT treat a status published at or before the write's revision, the one its input response reports, as satisfying the wait. Unless the root was `working` before the write, it SHALL first see the root's state or blocked kind change from the one before the write, or the program exit, and only then apply its condition. Titles, messages and status that never reaches a published revision SHALL NOT count.

#### Scenario: Short turn
- **WHEN** a program stays `working` for 50 ms after input, then reports `idle`, and a script runs `send --wait` 50 times
- **THEN** all 50 runs return

#### Scenario: Stale idle
- **WHEN** a program reports `idle` before the input and reports nothing new for a moment after it
- **THEN** `send --wait` keeps waiting rather than returning on the earlier `idle`

#### Scenario: Already working
- **WHEN** a program is `working` on an earlier prompt and a script runs `send --wait`
- **THEN** the input is written and the command returns when that turn ends

### Requirement: Stalled pane
If no revision within 5 seconds of the write shows the change that picking up the input requires, `send --wait` SHALL fail with an error naming the pane, saying it stalled, and saying that the input was sent. The stall timer SHALL stop once the program has been seen picking up the input. Any other error after the write SHALL also say that the input was sent.

#### Scenario: Program that never reports
- **WHEN** a pane's root record is `idle`, its program never reports again, and a script runs `send --wait`
- **THEN** the command fails within 6 seconds with an error saying the pane stalled and that the input was sent

### Requirement: Automation errors
Errors from `send` and `wait` SHALL follow the CLI error line in the `health-exchange` capability: one JSON error object as the last line on stderr and exit status 1, with usage errors keeping exit status 2. Each failure SHALL have its own message naming the pane: not found, closed, blocked, in a state `--if` doesn't list, no program status before sending, exited, stalled, the server stopped, and the connection lost. On `send --wait`, the outermost error's `message` SHALL be exactly `nothing was sent` for failures before writing, `the input may have been sent` when writing fails, and `the input was sent` for failures after writing, with the specific failure under `causedBy`.

#### Scenario: Server stops during a wait
- **WHEN** `ship server stop` runs while a script's `wait` is waiting
- **THEN** the wait exits 1 with an error saying the server stopped while waiting on the pane

#### Scenario: Refused before writing
- **WHEN** a script runs `send --wait` on a `blocked` pane without `--if blocked`
- **THEN** the last line on stderr has `.message` equal to `nothing was sent` and the exit status is 1

### Requirement: Input without an attachment
`POST /api/v0/attach/input` SHALL accept a request without the `x-ship-attachment-id` header and route its key and paste frames as it does with one. A mouse frame on such a request SHALL stop reading with 422, as a malformed line does. Without the header, the request SHALL end only when its body or the server ends.

#### Scenario: Headerless keys and paste
- **WHEN** a caller posts a paste frame and a key frame for a pane without the attachment header
- **THEN** the pane receives the paste, then the key, and the response is 204

#### Scenario: Headerless mouse frame
- **WHEN** a caller posts a mouse frame without the attachment header
- **THEN** the response is 422 and reading stops at that line

### Requirement: Input with an attachment
With the `x-ship-attachment-id` header, the input stream SHALL behave as before: an unknown attachment gets 404, the request ends with its attachment, and mouse frames are accepted. A malformed header SHALL get 400.

#### Scenario: Full-screen client's mouse copy
- **WHEN** a user drags a selection in a full-screen client while a second client views the same tab
- **THEN** the copy reaches only the first client's clipboard

#### Scenario: Malformed header
- **WHEN** a caller posts input with `x-ship-attachment-id: nonsense`
- **THEN** the response is 400 and nothing is routed
