# Proposal

This is the planning artifact for a locked plan. Cyan locked the product, architecture and program papers in `design/` on 2026-10-15. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

A background server logs to a random temp file nobody sees, the client logs to stderr behind its own screen, and raising the detail means restarting the server and losing the panes that show the problem. Session restore also needs a stable log path to point its failures at.

## What Changes

- **Run files.** A server writes its log under `~/.local/state/ship/logs/<port>/`, or `$XDG_STATE_HOME/ship/logs/<port>/`, one file per run named for its start time, such as `2026-10-15T18-40-07.log`.
  - `current.log` in the same directory links to the running server's file.
  - A run continues in a numbered part, such as `2026-10-15T18-40-07.2.log`, after 50 MB.
  - Each directory is pruned, oldest first, to about 200 MB, never deleting the current part.
  - Files are 0600 and directories 0700.
- **Launch output.** The launcher opens the run's first part, so anything printed before logging starts, and any panic, lands in that run's file. A failed launch names the file. The random temp file is gone.
- **Foreground servers** write the same file and also print to their terminal.
- **Client files.** The full-screen client writes `logs/client/<start time>-<pid>.log` under the same rules and logs nothing to stderr while drawing. One-shot commands keep logging to stderr.
- **Levels.** Ship's crates log at info and dependencies at warn by default.
  - `SHIP_LOG=<level>` sets Ship's level, and at debug or trace also shows actor message handling.
  - `RUST_LOG` replaces the whole filter.
  - An invalid `SHIP_LOG` fails startup and lists the accepted levels.
- **`ship server log`** prints the running server's filter as JSON.
  - `ship server log <level>` applies a level as `SHIP_LOG` would.
  - `--filter <filter>` applies a filter as `RUST_LOG` would.
  - `--reset` returns to the startup filter.
  - A change lasts until the server stops. An invalid one fails and changes nothing.
- **`ship server status`** includes the current log part's path.
- **Error chains.** An error logged inside traced operations shows the chain of operations it happened in, including work inside actors.
- **Timelines.** `SHIP_TRACE=<dir>` makes a server or client write `<dir>/<role>-<pid>.json` on a clean exit, openable in ui.perfetto.dev. It records Ship at debug detail whatever the log level. Without `SHIP_TRACE`, nothing is recorded.
- **Privacy.** Logs and timelines never contain pane output, typed or pasted input, clipboard contents, program arguments or environment values.
- `GET /api/v0/server/log` returns the filter. `PUT /api/v0/server/log` takes `{"set": "<filter>"}` or `"reset"` and answers 422 on an invalid filter.
- **BREAKING**: protocol version 12 goes to 13, and the health response gains `log`. Server and client upgrade together.

Not in this change (the product paper lists them all):
- Pane recording.
- OpenTelemetry and tokio-console.
- Joined client and server timelines.
- A log viewer command.
- Configurable retention, paths or formats.
- A config key for the level.
- Changing a running client's level or a running server's timeline.

## Capabilities

### New Capabilities

- `diagnostics`:
  - Where server and client logs go, their parts, link, retention and permissions.
  - `SHIP_LOG` and `RUST_LOG`.
  - Changing a running server's filter with `ship server log` and its route.
  - Error chains.
  - Timelines.
  - What logs and timelines never contain.

### Modified Capabilities

- `health-exchange`:
  - Protocol version 13 and the `log` field in health.
  - The launch log moves from a temp file to the run file.
- `local-foundation`: log discovery is documented in the guide, not the README.

## Impact

- `ship`:
  - A new `runlog` module owns run files, parts, `current.log` and retention.
  - `diagnostics` is rewritten to build the subscriber per role, hold the reloadable filter, log panics and add the timeline.
  - The launcher opens part 1 and passes it with a hidden `--log-file`.
  - The CLI gains `ship server log`.
- `ship-server`: `serve` takes a closure that shows, sets and resets the filter. The `/api/v0/server/log` routes and `health` use it. Long-lived tasks get named root spans and actor messages get short names.
- `ship-client`: a tracing dependency, spans on the keystroke path, and the `server_log` API call.
- `ship-core`: protocol version 13, `LogChange`, `LogFilter`, `HealthResponse.log`, and `spawn_traced`.
- `ship-macros`: `#[traced]` and `#[named]`.
- Dependencies: `chrono` and `tracing-chrome` in, `tempfile` out of `ship`.
- Docs: a guide section on logs, levels and timelines. The README only if its lists change.
- Estimated at about 440 lines of implementation Rust. Measured counts go in [tasks.md](tasks.md).
