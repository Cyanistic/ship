# Spec Delta

## Purpose

Let users find out why Ship misbehaved without restarting the server: read the log of the current or a crashed run, change a running server's detail level, and record where time went.

## ADDED Requirements

### Requirement: Server run files
A server SHALL write its log under `$XDG_STATE_HOME/ship/logs/<port>/`, defaulting to `~/.local/state/ship/logs/<port>/` on Linux and macOS. Each run SHALL start a new file named for its start time in local time, and SHALL never append to an earlier run's file. Lines SHALL carry local time with its offset. Background and foreground servers SHALL both write the file; a foreground server SHALL also print to its terminal. Other platforms' location is best effort.

#### Scenario: Two runs
- **WHEN** a server on port 43179 starts, is killed with SIGKILL, and another starts on the same port
- **THEN** `logs/43179/` holds two files named like `2026-10-15T18-40-07.log`, and the killed run's last lines are in its own file

#### Scenario: Foreground server
- **WHEN** a user runs `ship server --port 43180`
- **THEN** its lines print to the terminal and are also written to a new file in `logs/43180/`

#### Scenario: Line time
- **WHEN** a user reads a line of a run file
- **THEN** its time is local with an offset, such as `2026-10-15T18:40:07.123456+02:00`, matching the file name's clock

### Requirement: Current link
Each port's directory SHALL hold `current.log`, a link to the file the running server is writing, moved when a server starts and when it moves to a new part. Where links can't be created, it SHALL be skipped without failing the run.

#### Scenario: Follow the running server
- **WHEN** a server starts on port 43179
- **THEN** `logs/43179/current.log` resolves to that run's file, and `tail -f` on it shows the server's lines

### Requirement: Parts and retention
When a run's file reaches 50 MB, the run SHALL continue in a numbered next part, such as `2026-10-15T18-40-07.2.log`, without splitting a line. At start and at each new part, the oldest files in that directory SHALL be deleted until it totals at most 200 MB. The part being written SHALL never be deleted.

#### Scenario: Rollover
- **WHEN** a run's file passes 50 MB
- **THEN** later lines go to `<start>.2.log` and `current.log` follows it

#### Scenario: Long debug run
- **WHEN** a server runs for weeks at debug detail
- **THEN** its port's directory stays at about 200 MB, with the oldest files gone and the current part intact

### Requirement: Launch output in the run file
When `ship` launches a background server, the run's first part SHALL exist before the server starts, and anything the server prints before its logging is set up SHALL land in it. A failed launch SHALL name that file. No temporary log file SHALL be created.

#### Scenario: Failed launch
- **WHEN** a launched server fails before it becomes ready
- **THEN** `ship` reports the failure and names the run file holding the server's output

### Requirement: Panics in the log
A panic in a server or client SHALL be logged as an error, with its message and location, in that process's log. The client SHALL restore the terminal before the panic is logged.

#### Scenario: Server panic
- **WHEN** a background server panics
- **THEN** the panic's message and location are in that run's file

### Requirement: Client files
A full-screen client SHALL write its own file in `logs/client/`, named for its start time and process ID, such as `2026-10-15T18-40-09-48812.log`, with parts, retention and a `current.log` pointing at the newest client's file under the same rules as server files and its own 200 MB budget. It SHALL NOT log to stderr while drawing. One-shot commands SHALL log to stderr and write no file.

#### Scenario: Client draws cleanly
- **WHEN** a user opens the client and works in it
- **THEN** nothing is printed to stderr, and the client's lines are in `logs/client/current.log`

#### Scenario: One-shot command
- **WHEN** a user runs `ship tab list`
- **THEN** any log lines go to stderr and no file is created in `logs/client/`

### Requirement: Private files
Log files SHALL be readable only by their owner, and log directories SHALL be accessible only by their owner, on platforms with permission modes.

#### Scenario: Modes
- **WHEN** a server creates a run file on Linux or macOS
- **THEN** the file's mode is 0600 and its directories' modes are 0700

### Requirement: No automatic payload recording
Ship SHALL NOT automatically record pane output, typed or pasted input, clipboard contents, full program argument lists or environment values. Explicit failure diagnostics MAY include useful program names and dependency error details; logs are not guaranteed to contain no sensitive information. This boundary applies to logs and timelines at every level.

#### Scenario: Helpful failure details
- **WHEN** a pane's program fails to start
- **THEN** the failure log may name the program and include the dependency's error details, without automatically dumping the full argument list or environment

#### Scenario: Typed and pasted text at trace
- **WHEN** a server and client run with `SHIP_LOG=trace` and a user types one marker string into a pane and pastes another
- **THEN** neither marker appears in any log file

### Requirement: Startup levels
By default Ship's crates SHALL log at info and dependencies at warn. `SHIP_LOG=<level>`, one of `error`, `warn`, `info`, `debug` or `trace`, SHALL set Ship's crates to that level, and at `debug` or `trace` SHALL also show actor message handling. `RUST_LOG`, when set, SHALL replace the whole filter. A server launched by `ship` SHALL inherit both. An invalid `SHIP_LOG` or `RUST_LOG` SHALL fail startup naming what is accepted.

#### Scenario: Debug detail
- **WHEN** a server starts with `SHIP_LOG=debug` and a user creates a pane
- **THEN** its log shows Ship's debug lines and the actor message that created the pane

#### Scenario: Invalid level
- **WHEN** a user runs `SHIP_LOG=nope ship server`
- **THEN** it exits 1 with an error listing the accepted levels and serves nothing

### Requirement: Show the server's filter
`ship server log` SHALL print the running server's log filter as one JSON line with `active`, `startup` and `file`, the path of the part being written. `GET /api/v0/server/log` SHALL return the same JSON. Like `ship server status`, it SHALL never start a server.

#### Scenario: Show
- **WHEN** a user runs `ship server log` against a server started with default settings
- **THEN** it prints JSON whose `active` and `startup` are the default filter and whose `file` is the run's current part

### Requirement: Change the server's filter
`ship server log <level>` SHALL apply that level as `SHIP_LOG` would, and `ship server log --filter <filter>` SHALL apply a filter as `RUST_LOG` would, to the running server at once, without restarting it or affecting any pane. `ship server log --reset` SHALL restore the startup filter. Each SHALL print the resulting filter as `ship server log` does. A change SHALL last until the server stops.

#### Scenario: Raise and reset
- **WHEN** a user runs `ship server log debug` while a pane's program keeps printing, then `ship server log --reset`
- **THEN** debug lines appear in the log within a second, the pane is unaffected, and after the reset `active` equals `startup` again

#### Scenario: Only one change at a time
- **WHEN** a user combines a level, `--filter` or `--reset` in one command
- **THEN** the command is rejected before contacting the server

### Requirement: Invalid filter changes nothing
An invalid level or filter SHALL fail and leave the active filter unchanged. `PUT /api/v0/server/log` SHALL take `{"set": "<filter>"}` or `"reset"`, return the resulting filter, and answer 422 with the filter's error when the filter is invalid.

#### Scenario: Bad filter
- **WHEN** a user runs `ship server log --filter 'ship=['`
- **THEN** the command fails with the filter's error, and `ship server log` shows the same `active` as before

### Requirement: Error chains
An error logged from inside traced operations SHALL show the chain of operations it happened in, within each task or actor message to where it failed. Actor messages start new root spans and do not include the sending HTTP request's chain.

#### Scenario: Failed pane program
- **WHEN** a user runs `ship pane create -- /nonexistent`
- **THEN** the server log has one warning line whose chain runs from the `CreatePane` actor message to `spawn`, when debug spans are enabled

### Requirement: Timelines
When a server or client starts with `SHIP_TRACE=<dir>`, it SHALL write a timeline of its traced operations to `<dir>/<role>-<pid>.json`, where role is `server` or `client`, when it exits cleanly. A timeline SHALL open in ui.perfetto.dev, SHALL record Ship's operations at debug detail and actor message handling whatever the log level, and SHALL be available in normal builds.

#### Scenario: Record a keystroke
- **WHEN** a server and client run with `SHIP_TRACE=/tmp/ship-trace`, a user types a few keys, and both exit cleanly
- **THEN** `/tmp/ship-trace` holds `server-<pid>.json` and `client-<pid>.json`, which show the keystroke's client and server steps by name

#### Scenario: Default level
- **WHEN** a server runs with `SHIP_TRACE` and no `SHIP_LOG`
- **THEN** its timeline still holds debug-level operations

### Requirement: No timeline without SHIP_TRACE
Without `SHIP_TRACE`, a server or client SHALL record no timeline and write no timeline file.

#### Scenario: Unset
- **WHEN** a server and client run and exit without `SHIP_TRACE`
- **THEN** no timeline file is written anywhere
