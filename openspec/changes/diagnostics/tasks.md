# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` on 2026-10-15, and they are the contract. `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `ship-core` checks with and without `clap`;
- `openspec validate diagnostics --strict`.

Use owned temporary servers on ports 43311 to 43317, with `XDG_STATE_HOME` pointed at a temporary directory so real logs are untouched. Don't restart existing user work, including the server on 43179, and clean up only what the exercise created. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

Each slice documents its own user-visible behavior in `docs/guide.md`, in one section on logs, levels and timelines that grows slice by slice.

## 1. Run files and levels

- [x] 1.1 Add `runlog.rs` and the launcher's part 1:
  - `RunLog` with start-time names, 5 MiB parts, `current.log`, 200 MB retention per directory, and the `#[cfg(unix)]` module for modes and the link.
  - `local.rs` opens part 1 with `RunLog::start`, hands it to the child as stdout and stderr, and passes its path with the hidden `--log-file`. `tempfile` leaves the manifest.

  Verify with `XDG_STATE_HOME` set to a temporary directory and bare `ship` on a bench port:
  - The run file holds the launch line, and `logs/<port>/current.log` resolves to it.
  - The file is 0600 and its directories 0700.
  - A launch made to fail names the run file.
- [x] 1.2 Rewrite `diagnostics::init` for `Role::Server`, `Role::Client` and `Role::Command`, with `startup_filter`, `level_filter`, the `ChronoLocal::rfc_3339()` timer and the panic hook. Wire `role` in `main.rs`. Verify:
  - SIGKILL the server and start another on the same port: two files, the killed run's last line in its own file (SC-002).
  - A foreground `ship server --port <bench port>` prints to the terminal and writes a file.
  - The client draws with nothing on stderr and writes `logs/client/<start>-<pid>.log`. `ship tab list` logs to stderr and writes no file.
  - A forced panic in a temporary debug build lands in the run file, after the client restores the terminal.
  - Lines carry local time with the offset.
- [x] 1.3 Verify levels and retention:
  - `SHIP_LOG=debug` shows Ship's debug lines and kameo's actor spans. `SHIP_LOG=nope` exits 1 listing the levels. `RUST_LOG` replaces the filter.
  - A temporary probe with `PART_BYTES` and `DIR_BYTES` lowered shows a `.2.log` part, `current.log` following it, and the oldest file pruned while the current part stays. The probe isn't committed.
  - The guide says where server and client logs live, how `current.log` and parts work, and what `SHIP_LOG` and `RUST_LOG` do.

### Slice 1 evidence

Verified on macOS with temporary `XDG_STATE_HOME` directories and owned servers on ports 43311 to 43317. Existing server work on 43179 was not touched.

- Restored-source gates passed: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build`, `cargo build --release`, `cargo check -p ship-core --no-default-features`, `cargo check -p ship-core --no-default-features --features clap`, and `openspec validate diagnostics --strict`.
- A temporary Python PTY exercise (`/tmp/ship-diagnostics-check.py`) verified foreground file/stderr output, 0600 files and 0700 newly created directories, local timestamps with offsets, SIGKILL preserving the killed run's last bytes, distinct restart files and the current link. The full-screen client created its PID-named file with no stderr while drawing; its existing `detached` notice appeared after terminal restoration. A one-shot command created no client file and dependency tracing reached stderr.
- `SHIP_LOG=debug` produced Ship debug events and `actor.handle_message` spans. Invalid `SHIP_LOG` and `RUST_LOG` exited 1 with accepted-level guidance. `RUST_LOG=error` overrode even an invalid `SHIP_LOG`; `RUST_LOG=ship=info` suppressed debug despite `SHIP_LOG=debug`.
- A temporary debug binary (`/tmp/ship-diagnostics-probe`) used default port 43315, part limit 2048 bytes and directory limit 4096 bytes. Bare `ship` launched its background server and the run file held `server listening`. A forced child panic made launch fail with the retained run path; that private file contained the panic.
- The reduced-limit probe produced `.2.log`, moved `current.log`, preserved complete newline-terminated events and pruned the oldest file while retaining the current part. A separate startup with oversized old history also pruned it and retained the new part.
- Forced server and client panics logged their message and source location. The client emitted the alternate-screen restoration sequence and restored termios settings (ignoring macOS's kernel-managed PENDIN flag). Its existing hook restores the terminal before invoking the diagnostics hook.
- All probe source changes were removed, default port 43179 and 50/200 MiB limits restored, and the normal PTY exercise rerun successfully. Owned probe servers were stopped. No permanent authored tests were added. Linux and Windows remain unverified.

## 2. Spans

- [x] 2.1 Add `#[traced]` and `#[named]` to `ship-macros`, and `spawn_traced` to `ship-core` with tokio's `rt` feature. Give `ship-client` its tracing and `ship-macros` dependencies. Verify:
  - `cargo expand` of one `#[traced]` function returning `Result` shows `level = "debug"`, `skip_all` and `err(level = "warn", Debug)`, and one with `no_err` shows no `err`.
  - A `#[named]` message's `name()` returns its type as written, such as `"CreatePane"`.
- [x] 2.2 Apply the annotations in the program paper's span table: `#[named]` on all 21 `ServerState` messages, root spans for the pane task, `pane_exit` and `pane_events`, the detach through `spawn_traced`, and `#[traced]` on the keystroke path in server and client. Verify:
  - `ship pane create -- /nonexistent` logs one warn line whose chain runs from the actor's `CreatePane` message to `spawn`. Kameo starts message spans as roots; the HTTP request is not part of that chain (architecture decision 9 and its actor-boundary risk).
  - With `SHIP_LOG=debug`, actor spans read `CreatePane` and `Resize`, not type paths.
  - At `SHIP_LOG=trace`, a marker string typed into a pane and another pasted appear in no log file.
  - The guide describes reading an error's chain.

### Slice 2 evidence

Verified on macOS with debug and release binaries, isolated state directories and owned servers on ports 43311 and 43312. The existing server on 43179 was not touched.

- A temporary `ship-server` example exercised `#[traced]` on sync/async functions and a non-Debug argument: default debug spans, skipped arguments, one warn event using Debug, no error event with `no_err`, and forwarding of `level`, `name`, `fields` and `ret`. A generic named message returned `Ping<u8>`. A `spawn_traced` child yielded before emitting an event, which still appeared inside its caller's span.
- `cargo expand` output in `/tmp/ship-diagnostics-helper-expanded.rs` confirmed those defaults, no argument fields and no error event for `no_err`. Expanded state code in `/tmp/ship-diagnostics-state-expanded.rs` contained all 21 short message names, including `CreatePane`, `Resize` and `Get<Tab>`. The temporary example and its temporary dev dependency were removed afterward.
- `python3 /tmp/ship-diagnostics-slice2.py target/debug/ship` and the same exercise with `target/release/ship` passed. A real CLI launch of `/nonexistent` produced exactly one warning with `CreatePane -> spawn`, including the useful program/dependency failure details. Resize messages used the short name; pane and pane-exit roots had no actor/spawn parents.
- The exercise drove a real PTY client: selected a pane, typed a marker, pasted another using bracketed paste, and verified both in the server's screen through `pane read`. A program emitted output and an OSC 52 clipboard marker; the client forwarded the clipboard payload to its outer terminal. No input, output, clipboard, full-argument or environment marker appeared in any process's log at trace. The client/server logs contained `turn`, `send`, `apply`, `redraw`, `route`, `command`, `publish`, `capture`, `row` and `pane_events`. Detach removed the viewing record. Client stderr stayed quiet while drawing.
- Gates passed: formatting, workspace/all-target Clippy with warnings denied, debug/release builds, core without optional features and with `clap` or `kameo`, strict OpenSpec validation and `git diff --check`. The normal slice-1 PTY exercise was rerun successfully.
- All owned probe processes were stopped. No permanent authored tests, temporary source probes or extra dev dependencies remain. Linux and Windows remain unverified; performance comparison remains slice 5.

## 3. Live level change

- [x] 3.1 Add the protocol and server side:
  - `ship-core`: protocol 13, `LogChange`, `LogFilter`, `HealthResponse.log`.
  - `ship-server`: the `ServerLog` alias, `serve`'s closure parameter, `AppState.log`, `health` reading the file, and the `get_server_log` and `set_server_log` routes, 422 on an invalid filter.
  - `ship`: `Diagnostics::server_log`, passed to `serve` in `dispatch`.

  Verify against a bench server:
  - `curl` on `GET /api/v0/server/log` returns `active`, `startup` and `file`.
  - A `PUT` with `{"set": "ship=["}` answers 422 and a following `GET` shows the filter unchanged.
  - `ship server status` reports protocol 13 and the current part in `log`, also after a rollover with the slice 1 probe.
  - The OpenAPI document lists both routes.
- [x] 3.2 Add `ship server log` to the CLI, `commands.rs` and the API client. Verify:
  - `ship server log` prints the filter as one JSON line and starts no server when none is running.
  - `ship server log debug` makes debug lines appear within a second while a pane keeps printing, unaffected (SC-003).
  - `--reset` brings `active` back to `startup`. `--filter 'ship=['` fails and `ship server log` shows nothing changed.
  - `ship server log debug --reset` exits 2.
  - The guide documents `ship server log` and its options.

### Slice 3 evidence

Verified on macOS with isolated state directories and owned servers on ports 43311 and 43312. Existing work on 43179 was not touched. All owned probe servers were stopped.

- `python3 /tmp/ship-diagnostics-slice3.py target/debug/ship` and the release equivalent passed. Real CLI and HTTP requests verified protocol 13, current-part paths in status and filter responses, one-line JSON, all five level mappings, custom filters, CLI and HTTP reset, HTTP 422 with the parse error for invalid filters, and unchanged active/startup filters after rejection. Conflicting options and invalid levels exited 2. Querying a stopped server failed without starting one or creating logs.
- Debug output appeared within one second of the CLI level change while the same pane kept printing. Its pane ID and running server remained unchanged, and later screen reads showed advancing output.
- With Cyan's explicit authorization, `PART_BYTES` was temporarily reduced to 2048 and a debug probe copied to `/tmp/ship-diagnostics-slice3-rollover`. The exercise produced numbered parts through `.21.log`; after rollover, health, the filter response and `current.log` identified the same current part. Logging was lowered before the final comparison to avoid rotating between observations. Probe assertions were corrected to compare active/startup independently of the moving file and to resolve both sides of macOS's `/var` versus `/private/var` alias. The production 50 MiB limit was restored; `git diff --exit-code -- crates/ship/src/runlog.rs` passed.
- A temporary Rust consumer called `ship_server::openapi()` and wrote `/tmp/ship-diagnostics-slice3-openapi.json`. It verified GET/PUT operation IDs, the 422 response, LogChange/LogFilter/HealthResponse schemas, and required log/filter fields in the generated document. No repository example or permanent test was added.
- After restoration, formatting, workspace/all-target Clippy with warnings denied, debug/release builds, core checks without default features and with clap, strict OpenSpec validation and whitespace checks passed. Both normal-limit slice-3 workflows were rerun successfully. The slice-2 debug PTY exercise also passed, including typing/paste, span names and the payload boundary.
- No coding subagents or permanent authored tests were used. Linux and Windows remain unverified. Slice 4 was not started.

## 4. Timelines

- [ ] 4.1 Add `tracing-chrome` and the chrome layer in `init` with its fixed filter, keeping the `FlushGuard` in `Diagnostics`. Verify:
  - `SHIP_TRACE=<dir>` on a bench server and a client, a few keystrokes and clean exits leave `<dir>/server-<pid>.json` and `<dir>/client-<pid>.json`, and both parse as JSON.
  - Between them they hold `turn`, `send`, `route`, `command`, `publish`, `apply` and `redraw` (SC-004). Cyan opens one in ui.perfetto.dev to confirm the view.
  - At default `SHIP_LOG` the timeline still holds debug spans. Without `SHIP_TRACE` no file appears.
  - The guide documents `SHIP_TRACE` and opening a timeline in Perfetto.

## 5. Bench and size

- [ ] 5.1 Run the bench on ports 43311 to 43317 at default settings against `fc2b5a7`. Verify typing latency and flood throughput are within run-to-run noise (SC-001), and record the numbers below.
- [ ] 5.2 Finish the docs. Verify:
  - The guide's section reads end to end, and following it finds a background server's current log, raises its level and records a timeline.
  - The README changes only if "What works today", "Not built yet", keys or install steps changed.
- [ ] 5.3 Measure size with `cloc` against `fc2b5a7`. Verify implementation Rust and tests are reported as separate counts (SC-005), against the 440-line estimate.

## Workflow follow-up

- File tracking issues for the deferred items when the pull request lands: tokio-console behind a cargo feature, and joined client and server timelines.
- The session-restore paper's U-1 can point at `logs/<port>/current.log`. It's edited from that change, not this one.
- Archive the change after merge, which applies the deltas to `openspec/specs/`.
