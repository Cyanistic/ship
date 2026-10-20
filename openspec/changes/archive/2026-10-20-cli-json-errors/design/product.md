# CLI errors as JSON

Status: Locked on 2026-10-20. Written from a design conversation with Cyan, who settled the four questions in #37 in chat ("lgtm! go for it!"), and approved in Plannotator review ("LGTM"). Revised the same day at Cyan's direction to drop the `data.sent` field ("just drop it for now. we don't have to worry about it in 99% of cases."), since the outer message already says whether input was sent. Revised again the same day at Cyan's direction ("yes please!") to rename `caused_by` to `causedBy`, matching the API's camelCase keys, and once more ("lgtm!") to spell error kinds in snake_case, matching the API's other lowercase values.

## Goal

Successful `ship` commands print JSON, but failures print a line of prose like `ship: the input was sent: pane pane:5157… stalled: ...`. Scripts and agents parse JSON when a command works and pick apart sentences when it doesn't. The prose also leaves information out: the error's category isn't printed, and because causes are joined with `: `, a cause whose own message contains a colon can't be told apart from the next one.

The case that matters most is `ship pane send`. Before retrying, a caller has to know whether its input already reached the pane, and today it can only find out by picking the words "nothing was sent" or "the input was sent" out of a longer line. This change prints failures as one JSON line on stderr, in the same shape the HTTP API already returns, so that answer becomes the error's whole `message` field.

```text
today    ship: the input was sent: pane pane:5157… stalled: its program status did not change within 5 seconds
after    {"source":"internal","message":"the input was sent","type":{"kind":"conflict"},"causedBy":{"source":"internal","message":"pane pane:5157… stalled: its program status did not change within 5 seconds","type":{"kind":"conflict"}}}
```

## User stories

### P1: A script reads a failed command's error

- **Given** a command that fails, such as `ship pane get` for a pane that doesn't exist, **when** it exits, **then** its exit status is 1, stdout is empty, and the last line on stderr is one JSON object naming the category (`not_found`) and the message.
- **Given** an error with causes, **then** each cause is its own nested object, so the chain can be walked without splitting text.
- **Given** a person running the same command by hand, **then** they see the same JSON line. There's no terminal detection and no flag.

Tested independently by running failing commands and parsing stderr's last line with `jq`.

### P2: A caller decides whether to resend after `ship pane send` fails

- **Given** a pane that is `blocked` and `send --wait` without `--if blocked`, **when** it refuses, **then** the error's `message` is exactly `nothing was sent`.
- **Given** a pane whose program never changes its status after the input, **when** `send --wait` gives up after 5 seconds, **then** the error's `message` is exactly `the input was sent`, and the caller waits with `ship pane wait` instead of sending again.

Tested independently with a scripted OSC 7501 reporter in one pane, driving `send` from another.

### P3: A mistyped command still explains itself

- **Given** `ship pane read --pain ID`, **then** clap's usual text is printed on stderr, with its suggestion and usage line, and the exit status is 2, as today.

Tested independently by running commands with bad flags and values.

### P4: Checking the config file

- **Given** a config file with mistakes, **when** the user runs `ship config check`, **then** each mistake is printed as text with its line and column, the exit status is 1, and no further error line follows.
- **Given** a valid config file, **then** it prints nothing and exits 0, as today.

Tested independently with a broken and a valid config file.

## Non-goals

- Errors shown in the full-screen client's status line. They stay as they are.
- Changing clap's usage errors, `--help` or `--version`.
- Turning stderr log lines into JSON. One-shot commands keep logging as text, as the diagnostics spec requires.
- A separate, flatter error shape for the CLI. The CLI prints exactly what the HTTP API returns.
- New exit codes. Failures stay 1 and usage errors stay 2.
- A separate field saying whether `ship pane send`'s input went out. The outer message already says it, and the case is rare.
- Panics. A panic still prints Rust's panic message.

## Functional requirements

- **FR-001** When a command other than `ship config check` fails, `ship` MUST print the error as one line of JSON on stderr and exit with status 1. The line MUST be the last thing written to stderr. Log lines MAY come before it.
- **FR-002** The JSON MUST be the same serialization of the error that the HTTP API returns as an error body, including its category, data and chain of causes.
- **FR-003** Error keys MUST follow the camelCase used by the rest of the API, so the cause key is `causedBy`, not `caused_by`. Error kinds MUST be spelled in snake_case like the API's other values, such as `not_found` and `upstream_http_status`, not `NotFound`. This changes HTTP error bodies as well as the CLI line.
- **FR-004** A failed command MUST write nothing to stdout.
- **FR-005** The JSON MUST be printed whether or not stderr is a terminal. There MUST be no flag or setting to change it.
- **FR-006** Usage errors that clap reports MUST keep clap's text and its exit status 2. `--help` and `--version` MUST behave as today.
- **FR-007** The messages "nothing was sent", "the input was sent" and "the input may have been sent" MUST remain the outermost message on the `send --wait` errors that carry them today.
- **FR-008** `ship config check` MUST print each mistake it finds as text on stderr and exit with status 1 when there is at least one, without printing any further error line. With no mistakes it MUST print nothing and exit 0.
- **FR-009** The guide MUST describe the error line, how to tell from a `send --wait` error whether the input went out, and the meaning of exit statuses 1 and 2.

## Success criteria

- **SC-001** For every failing command in the guide's examples, `tail -n 1` of stderr parses with `jq` and stdout is empty.
- **SC-002** A caller can decide whether to resend after a `send --wait` failure by comparing the outer `message` to a fixed string, without splitting text.
- **SC-003** A command run with a misspelled flag prints the same text as before this change and exits 2.
- **SC-004** The pane automation spec's error convention requirement is updated to match.

## Open unknowns

None.
