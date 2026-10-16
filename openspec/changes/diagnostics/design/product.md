# Diagnostics

Status: Locked again on 2026-10-15, approved by Cyan after Plannotator review ("LGTM") of the cut of pane recording, which Cyan moved out of this change ("i don't even have a use case for it yet"). Previously locked on 2026-10-15, approved by Cyan after Plannotator review ("LGTM"), which accepted every item under "Needs attention in review" as recommended. Written on 2026-10-15 from a design conversation with Cyan, which started from a handoff about giving the server log a stable path. Decisions Cyan stated are quoted below. The approval doesn't authorize implementation; Cyan requests that separately.

## Summary

Today a background server's log goes to a random temp file nobody sees, the client logs to stderr behind its own screen, and when something misbehaves there is no way to look closer without restarting the server and losing the sessions that show the problem.

This change gives Ship a set of debugging tools:

- Server and client logs at stable paths under `~/.local/state/ship/logs/`, with run history and bounded size.
- One knob, `SHIP_LOG`, for Ship's detail level, and `ship server log` to change a running server's level without a restart.
- `SHIP_TRACE`, which writes a timeline of where time went in a server or client, viewable in Perfetto.
- Errors in the log that show the chain of operations they happened in.

Session restore depends on the stable log path: its open unknown U-1 points restore problems at this log.

Needs attention in review:

- Pane recording is cut: the old P3, FR-017 to FR-020 and SC-004 are gone, and the requirements after them are renumbered. Nothing else changed.

- The `current.log` link, local time in names and the retention numbers (FR-002 to FR-004). Cyan chose `logs/`, timestamps and a directory per port; the rest are recommendations.
- The client writes its own per-process log (FR-006). Recommended, not discussed beyond the problem.
- `ship server status` gains the log path (FR-008). This adds a field to the health response, a protocol change.
- `ship server log` is a new server command (FR-012 to FR-015), also a protocol addition. Cyan wants it ("worthwhile and extremely useful in the right scenario").
- A timeline records at debug detail whatever the log level is (FR-018). Recommended so a trace is never empty.
- `SHIP_TRACE` names a directory, not a file (FR-017), because a background server inherits the variable from the `ship` that launches it.

## Goal

When Ship misbehaves, Cyan can find out what happened and why without restarting the server: read the log of the current or a crashed run, raise the detail on the live server, and see where the time went in a slow keystroke. Default runs must stay as fast as they are today.

## User stories

### P1: Find out why the server misbehaved or died

Given the background server crashed or acted oddly, when Cyan opens `~/.local/state/ship/logs/`, then the port's directory `43179/` holds one file per run, named by when it started, and `43179/current.log` points at the running one. A panic or a startup failure is in the file of the run it happened in. An error line shows the chain of operations it happened in, not only where it was reported.

- Given `ship` failed to launch the server, then the message names the log file.
- Given the server has run for weeks at debug detail, then the logs for its port take no more than about 200 MB.

Tested independently: start a server, kill it with SIGKILL, start another, and check both runs' files and `current.log`. Force an error inside a pane action and check its log line.

### P2: Look closer without a restart

Given a running server with sessions Cyan cares about, when Cyan runs `ship server log debug`, then debug lines start appearing in the log at once, including actor messages. `ship server log --reset` returns to the level the server started with. Nothing restarts and no session is touched.

Tested independently: change the level on a live server, check the log, reset, check again.

### P3: See where a slow keystroke spent its time

Given typing feels slow, when Cyan starts the server and a client with `SHIP_TRACE=/tmp/ship-trace`, types, and exits, then the directory holds one timeline per process, and opening one in ui.perfetto.dev shows each step of the keystroke's path on its thread.

Tested independently: record a short session, open both files in Perfetto, and find the input and render steps.

## Non-goals

- Recording a pane's output for replay. Split out of this change with no use case yet; not filed.
- OpenTelemetry export. The one use, a trace spanning client and server, needs trace context in Ship's messages.
- tokio-console.
- Joining client and server timelines into one view automatically.
- A command to view or follow logs. `tail -f` on `current.log` covers it.
- Configurable retention, log paths or formats.
- Changing a client's level while it runs, or a running server's timeline.
- A config-file key for the log level.
- A timeline that survives a crash.

## Functional requirements

### Log files

- FR-001: A server SHALL write its log under `$XDG_STATE_HOME/ship/logs/<port>/`, defaulting to `~/.local/state/ship/logs/<port>/`, on Linux and macOS, starting a new file each run named for its start time in local time, such as `logs/43179/2026-10-15T18-40-07.log`. A run SHALL never append to an earlier run's file. A Windows fallback location is best effort.
- FR-002: Each port's directory SHALL hold `current.log`, a link to the running server's file and not a log of its own, moved at each start. Where links can't be created, it SHALL be skipped.
- FR-003: When a run's file reaches 50 MB, the run SHALL continue in a numbered next part, such as `2026-10-15T18-40-07.2.log`, and `current.log` SHALL follow.
- FR-004: At start and at each new part, the oldest files in that port's directory SHALL be deleted until it totals at most 200 MB. The current part SHALL never be deleted.
- FR-005: Both a background server and a foreground `ship server` SHALL write the file. A foreground server SHALL also print to its terminal.
- FR-006: A client drawing the screen SHALL write its own file in `logs/client/`, named for its start time and process ID, such as `2026-10-15T18-40-09-48812.log`, under the same rules, with `current.log` pointing at the newest client's file and the directory's own 200 MB budget. It SHALL NOT log to stderr while drawing. One-shot CLI commands SHALL log to stderr.
- FR-007: Anything a launched server prints before its log is set up, and any panic, SHALL land in that run's file. A failed launch SHALL name the file. The random temp file SHALL no longer be created.
- FR-008: `ship server status` SHALL print the server's current log file path.
- FR-009: Log files and directories SHALL be readable only by their owner.
- FR-010: Logs and timelines SHALL NOT contain pane output, typed or pasted input, clipboard contents, program arguments or environment values.

### Levels

- FR-011: By default Ship's crates SHALL log at info and dependencies at warn. `SHIP_LOG=<level>` SHALL set Ship's crates to that level, and at debug or finer SHALL also show actor message handling. `RUST_LOG`, when set, SHALL replace the whole filter. A server launched by `ship` SHALL inherit both from the launching environment.
- FR-012: `ship server log` SHALL print the running server's active filter.
- FR-013: `ship server log <level>` SHALL apply that level as `SHIP_LOG` would, and `ship server log --filter <filter>` as `RUST_LOG` would, to the running server at once, without restarting it or affecting any pane.
- FR-014: `ship server log --reset` SHALL restore the filter the server started with.
- FR-015: A level set with `ship server log` SHALL last until the server stops. An invalid level or filter SHALL fail and change nothing.

### Errors

- FR-016: An error logged from inside traced operations SHALL show the chain of operations it happened in, including work done inside actors.

### Timelines

- FR-017: When a server or client starts with `SHIP_TRACE=<dir>`, it SHALL write a timeline of its traced operations to `<dir>/<role>-<pid>.json` when it exits cleanly, openable in ui.perfetto.dev. It SHALL be available in normal builds.
- FR-018: A timeline SHALL record Ship's operations at debug detail and actor message handling, whatever the log level.
- FR-019: Without `SHIP_TRACE`, nothing SHALL be recorded.

## Success criteria

- SC-001: With default settings, the bench's typing latency and flood throughput show no regression against `fc2b5a7`, within run-to-run noise.
- SC-002: After a SIGKILL and a restart, the killed run's last lines are in its own file, found by listing `logs/`.
- SC-003: `ship server log debug` makes debug lines appear within a second, and no pane notices.
- SC-004: A timeline of one keystroke shows the client and server steps of its path by name.
- SC-005: Implementation Rust and tests are reported separately. The conversation estimated 200 to 300 implementation lines after cutting recording; architecture refines it.

## Decisions

Settled in conversation on 2026-10-15:

| Decision | Why |
| --- | --- |
| Logs under the XDG state directory, in `logs/`, named per run with a timestamp | Stable place, run history by listing it. "i'd nest it under a logs/ dir as well" |
| A directory per port, and one for clients | File names are just timestamps, so no delimiter between port and time, and retention is per directory. "oh that's actually a lot cleaner" |
| Per-port files | Session restore's saved file is per port, and two servers sharing a file would interleave |
| Keep run history by size, not truncate at start | Truncating destroys the crash log when it's needed |
| `SHIP_LOG` tunes Ship and actor detail together; `RUST_LOG` overrides | `RUST_LOG=warn,ship=debug` silently drops kameo's actor spans, which are at info. "you have to remember that" |
| Live level change on the server | Restarting to get debug detail kills the sessions showing the bug. "worthwhile and extremely useful in the right scenario" |
| Timelines from tracing-chrome, viewed in Perfetto | "i feel like tracing-chrome is actually probably super useful" |
| Traced functions log their errors by default | Logged while their spans are open, so the chain comes free. "i want it to err by default and use no_err to not do the error thing" |
| Pane recording left out | No use case yet, and it shares no mechanism with the rest. "i'm wondering if this should be a separate change now"; no issue filed for now |
| No OpenTelemetry | No user for it without cross-process trace context in the protocol |

### Changes to other documents

- `docs/guide.md`: a new section on logs, levels and timelines.
- `README.md`: only if "What works today" lists debugging; checked at implementation.
- Specs: `health-exchange` (log path in status), `local-foundation` (launch log) and a new requirement for the server log command.
- The session-restore paper's U-1 can point at `logs/<port>/current.log` once this locks. Not edited from this change.

### Deferred, to file when the pull request lands

- tokio-console behind a cargo feature.
- Joined client and server timelines.

## Open unknowns

None. The items under "Needs attention in review" were accepted at lock.
