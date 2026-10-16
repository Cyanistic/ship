# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Product, architecture and program were locked by Cyan on 2026-10-15, after pane recording was cut from the product paper the same day. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P3, FR-001 to FR-019, success criteria SC-001 to SC-005.
- [`design/architecture.md`](design/architecture.md): shape A, the runner owns the subscriber, and key decisions 1 to 12.
- [`design/program.md`](design/program.md): the planned files, types and signatures, gates G1 to G5, the build order and the deviation log.

## Context

A background server's log goes to a random temp file, the client logs to stderr behind its own screen, and raising the detail means restarting the server and losing the panes that show the problem. This change puts server and client logs at stable paths under `~/.local/state/ship/logs/`, adds `SHIP_LOG` and a live `ship server log` level change, `SHIP_TRACE` timelines for Perfetto, and errors that show the chain of operations they happened in. Session restore depends on the stable log path.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (why the server died), P2 (look closer without a restart) and P3 (where a slow keystroke spent its time).

**Non-goals:** see the product paper's Non-goals. The main ones:

- Pane recording, split out with no use case yet and not filed.
- OpenTelemetry, tokio-console, and joined client and server timelines.
- A log viewer command, a config key for the level, and configurable retention or paths.

## Decisions

- **The runner owns the subscriber** (architecture decision 1). The `ship` binary sets up logging for every role; library crates only emit. `serve` takes one closure that shows, sets and resets the filter, used by the `/api/v0/server/log` route and `/health`.
- **A Ship-owned `RunLog`, written synchronously** (decision 2), so lines survive SIGKILL and aborting panics. It owns start-time names, 5 MiB parts, `current.log` and 200 MB retention per directory.
- **The launcher opens part 1** (decision 3), so output from before setup lands in the run's file. The temp file is gone.
- **`SHIP_LOG` sets Ship's crates and actor detail together; `RUST_LOG` replaces it** (decision 7).
- **`#[traced]` wraps `tracing::instrument`** with debug, `skip_all` and error logging by default (decision 8). Actor messages get short names from `#[named]` (decision 10).
- **Timelines from tracing-chrome** with a fixed debug filter (decisions 5 and 11).
- **Settled program gates:** G1 chrono for local time, G2 local time with offset in lines, G3 `GET`/`PUT /api/v0/server/log` with `{"set": ...}` or `"reset"`, G4 `ship server log` prints JSON, G5 an invalid `SHIP_LOG` fails startup.
- **Protocol version 13.**

## Build order

Program paper, "Build order":

1. Run files and levels.
2. Spans.
3. Live level change.
4. Timelines.
5. Bench, docs, specs and size.

## Risks / Trade-offs

Architecture paper, "Risks and unknowns". The ones to watch:

- Error chains stop at actors: kameo starts each message's span as a new root.
- Synchronous writes at debug on the typing path; only the default level is benched.
- Raw stderr after the first rollover stays in part 1.

Estimated size: about 440 lines of implementation Rust.

## Migration Plan

Old temp-file logs are left where they are. Protocol 12 goes to 13, so server and client are rebuilt as a pair, and mismatched versions fail the health check as before.

## Open Questions

None.
