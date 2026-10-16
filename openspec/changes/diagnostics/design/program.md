# Diagnostics program

Status: Locked on 2026-10-15, approved by Cyan in Plannotator review ("LGTM"), which accepts G1 to G5 as recommended, with G1 changed to chrono in review. Written on 2026-10-15 from the locked [product](product.md) and [architecture](architecture.md) papers, against the crates at `fc2b5a7`. Nothing here authorizes implementation.

Settled gates, each accepted as recommended:

- **G1, local time.** File names use local time (FR-001), and std can't read the local offset. Recommendation: `chrono`, through tracing-subscriber's `chrono` feature, whose `ChronoLocal` timer formats lines with no Ship code; file names use `chrono::Local` directly. Rejected: `jiff`, which would need a hand-written timer; and `time`, whose local offset lookup fails on purpose in multithreaded Unix processes. Neither `chrono` nor `jiff` is built today; both appear in `Cargo.lock` only as serde_with's unused optional dependencies.
- **G2, time inside lines.** Recommendation: lines also use local time with its offset through `ChronoLocal::rfc_3339()`, such as `2026-10-15T18:40:07.123456+02:00`, so a line's time matches its file's name. The alternative is fmt's default UTC, which costs nothing but makes lines and file names disagree by the offset.
- **G3, the route's shape (protocol).** Recommendation: `GET /api/v0/server/log` returns `LogFilter`, and `PUT /api/v0/server/log` takes a `LogChange`, written `{"set": "warn,ship=debug"}` or `"reset"`, and returns the new `LogFilter`. A failed `PUT` is 422 with the filter's parse error and changes nothing (FR-015).
- **G4, what `ship server log` prints.** Recommendation: the `LogFilter` as JSON, like `ship server status` prints health. The alternative is just the active filter as plain text.
- **G5, an invalid `SHIP_LOG` at startup.** Recommendation: fail with a configuration error that lists the accepted levels, as an invalid `RUST_LOG` does today. A background server that fails this way names its run file. The alternative is to warn and run at info.

## Rationale

This is a document-only plan. Nothing below exists as source yet, and none of it has been compiled.

The skeleton follows architecture shape A: the `ship` binary builds the subscriber for each role, and the library crates only emit. Per crate:

- **`ship`:** a rewritten `diagnostics` module that builds the subscriber for each role and holds the server's log closure. A new `runlog` module for the files. The launcher opens part 1. The CLI gains `ship server log` and the hidden `--log-file`.
- **`ship-server`:** `serve` takes the log closure, keeps it in `AppState`, and serves `/api/v0/server/log` and the log path in `/health`. Functions on the keystroke path get `#[traced]`, long-lived tasks get named root spans, and actor messages get short names.
- **`ship-client`:** gains a tracing dependency, `#[traced]` on its keystroke path, and an API call for the log route.
- **`ship-core`:** the protocol types, version 13, and `spawn_traced`.
- **`ship-macros`:** `#[traced]` and `#[named]`.

Applying `docs/agents/code.md`:

- **Earn every layer:** `RunLog` is the only new type that owns behavior. It locks itself and implements `Write` for `&RunLog`, so tracing-subscriber's `MakeWriter` impl for `Arc<W>` takes `Arc<RunLog>` directly, with no wrapper type. The closure that `serve` takes is a plain `Fn`, stored behind one type alias, with no struct or trait around it.
- **Check what's already guaranteed:** fmt's layer formats each event into one buffer and writes it with one `write_all`, so `RunLog` can roll to a new part between events without splitting a line. `reload::Handle::reload` rebuilds tracing's interest cache itself, so a raised level reaches callsites that were cached as disabled.
- **Platform-specific code:** std has portable calls for neither permission modes nor symlinks; both live only in `std::os::unix` (and `std::os::windows` for links). So the 0600 and 0700 modes and the `current.log` link sit in one `#[cfg(unix)]` module in `runlog.rs`, and other platforms get a no-op fallback, which FR-002 allows for the link and Windows' best-effort status allows for the modes.
- **One source of truth and named shapes:** a background server is the one with a launcher's `log_file`, so `Role::Server` has no separate foreground flag, and the timeline's layer is built inside `init`, so only its guard is kept.

Estimated change: about 440 lines of implementation Rust, up from today's 8,707 (cloc over `crates/`, inline tests included). By area:

| Area | Lines |
| --- | --- |
| `runlog` | about 90 |
| `diagnostics`: roles, levels, panic hook, timeline, closure | about 75 |
| Launcher and CLI | about 35 |
| Protocol, route, health, client API | about 65 |
| `#[traced]` and `#[named]` | about 135 |
| `spawn_traced`, root spans and annotations | about 45 |

## Skeleton map

### Proposed settings

Workspace dependencies (slice 1 adds `chrono`; slice 4 adds `tracing-chrome`):

```toml
# Cargo.toml [workspace.dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock"] }   # G1
tracing-chrome = "0.7"
tracing-subscriber = { version = "0.3", features = ["env-filter", "chrono"] }   # G2; defaults include fmt and registry
```

Crate manifests:

```toml
# crates/ship/Cargo.toml
chrono.workspace = true      # slice 1
tracing-chrome.workspace = true   # slice 4
# tempfile is removed in slice 1; local.rs was its only user

# crates/ship-client/Cargo.toml
tracing.workspace = true     # slice 2
ship-macros.workspace = true # slice 2

# crates/ship-core/Cargo.toml
tokio = { version = "1", features = ["rt"] }   # slice 2, for spawn_traced; kameo enables tokio/sync
```

The fixed values live as constants in `runlog.rs` and `diagnostics.rs`: 5 MiB per part, 200 MB per directory, and the timeline filter `warn,ship=debug,ratatui_ghostty=debug,kameo=info`. None of them are settings (product non-goal).

`PROTOCOL_VERSION` goes from 12 to 13 in slice 3.

### Added

#### `crates/ship/src/runlog.rs` (slice 1)

```rust
//! A run's log file: start-time names, numbered parts, `current.log` and
//! per-directory retention. Written synchronously so lines survive SIGKILL
//! and aborting panics (architecture decision 2).

use std::{fs::File, io, path::{Path, PathBuf}, sync::Mutex};
use ship_core::prelude::*;

const PART_BYTES: u64 = 5 * 1024 * 1024;     // FR-003
const DIR_BYTES: u64 = 200 * 1024 * 1024;     // FR-004
const CURRENT: &str = "current.log";

/// `ship/logs` under etcetera's `choose_base_strategy()` `state_dir()`
/// (`$XDG_STATE_HOME`, default `~/.local/state`), or under its `data_dir()`
/// where there is no state directory (Windows). Same strategy as the config path in `cli.rs`.
pub fn logs_dir() -> Result<PathBuf>;

/// Shared by the fmt layer and the server's log closure as `Arc<RunLog>`.
pub struct RunLog {
    dir: PathBuf,
    /// `2026-10-15T18-40-07`, or `2026-10-15T18-40-09-48812` for a client.
    stem: String,
    part: Mutex<Part>,
}

/// The part being written.
struct Part {
    number: u32,
    file: File,
    written: u64,
}

impl RunLog {
    /// Creates `dir` (0700) and the first part (0600), points `current.log` at it,
    /// then prunes `dir` to `DIR_BYTES`, never deleting the new part.
    pub fn start(dir: &Path, stem: String) -> Result<Self>;
    /// Continues a part 1 the launcher already started, counting what is in it.
    pub fn resume(path: &Path) -> Result<Self>;
    /// The part being written.
    pub fn path(&self) -> PathBuf;
}

/// Locks the part. Before a write that would cross `PART_BYTES`: opens
/// `<stem>.<n>.log`, moves `current.log`, prunes. A failed rollover keeps
/// writing the full part. With this, `Arc<RunLog>` is a `MakeWriter`.
impl io::Write for &RunLog { /* write, flush */ }

/// Start-time stem in local time (G1).
pub fn stem(now: &chrono::DateTime<chrono::Local>) -> String;

/// Owner-only modes and the `current.log` link: std has these only in `std::os::unix`.
#[cfg(unix)]
mod unix {
    /// 0700 on directories.
    pub fn private_dir(builder: &mut std::fs::DirBuilder);
    /// 0600 on files.
    pub fn private_file(options: &mut std::fs::OpenOptions);
    /// Symlinks a temporary name to `target`, then renames it over `current.log`.
    pub fn link_current(dir: &Path, target: &Path) -> io::Result<()>;
}
// Elsewhere: modes are left as the platform makes them and no link is made (FR-002).
```

Retention sorts the directory's `*.log` files by modification time, skips `current.log` and the open part, and deletes the oldest until the total is at most `DIR_BYTES`.

### Replaced

#### `crates/ship/src/diagnostics.rs`: `init()` (slices 1, 3, 4)

Today: `init()` builds `fmt()` with `RUST_LOG` or `info`, on stderr, for every command. Proposed:

```rust
use tracing_subscriber::{EnvFilter, Registry, reload};
use ship_core::{LogChange, LogFilter, prelude::*};

/// What this process is, which decides where its log goes.
pub enum Role {
    /// `ship server`. A background child has the launcher's part 1 in `log_file`
    /// and writes only the file; a foreground server has none and also writes stderr.
    Server { port: u16, log_file: Option<PathBuf> },
    /// The full-screen client: a file in `logs/client/`, nothing on stderr (FR-006).
    Client,
    /// One-shot commands: stderr only, as today.
    Command,
}

/// Kept alive by `main` until it returns.
pub struct Diagnostics {
    /// Present for `Role::Server` and `Role::Client`.
    log: Option<Arc<RunLog>>,
    filter: reload::Handle<EnvFilter, Registry>,
    startup: String,
    /// With `SHIP_TRACE=<dir>`, `init` adds a chrome layer writing
    /// `<dir>/<role>-<pid>.json` (threaded style, args included) and keeps its
    /// guard here. Dropping it at the end of `main` flushes the writer thread's
    /// buffer and closes the JSON (slice 4).
    timeline: Option<tracing_chrome::FlushGuard>,
}

/// Builds the registry: `file.and_then(stderr).with_filter(reload)` directly on
/// `Registry`, then the timeline with its own fixed filter. Installs the panic hook.
pub fn init(role: Role) -> Result<Diagnostics>;

impl Diagnostics {
    /// The closure `serve` takes (architecture decision 1). `None` shows.
    pub fn server_log(&self) -> impl Fn(Option<LogChange>) -> Result<LogFilter> + Send + Sync + 'static;
}

/// `RUST_LOG` as is when set; else `level_filter(SHIP_LOG or "info")` (FR-011, G5).
fn startup_filter() -> Result<String>;

/// `warn,ship=L,ratatui_ghostty=L,kameo=X`, X = info at debug or trace, else warn.
/// Also used by `ship server log <level>`.
pub fn level_filter(level: tracing::Level) -> String;

/// Logs the panic and its location as an error event, then calls the previous hook.
fn panic_hook();


/// Unchanged.
pub fn shutdown() -> Result<impl Future<Output = Result<()>> + Send + 'static>;
```

Both outputs use `ChronoLocal::rfc_3339()` as their timer (G2). `diagnostics.rs` stays unchanged otherwise. The client's existing panic hook in `ship-client/src/ui/terminal.rs` is installed later, takes this one as its previous hook, and so restores the terminal first and then logs the panic.

#### `crates/ship/src/main.rs`: `run` and `dispatch` (slices 1 and 3)

```rust
fn run(cli: Cli, source: Option<ValueSource>) -> Result<()>;
// detach() as today, then `let diagnostics = diagnostics::init(role(&cli))?;`,
// kept alive until the runtime returns.

/// Server, client or command, from the parsed command line.
fn role(cli: &Cli) -> Role;

async fn dispatch(cli: Cli, source: Option<ValueSource>, diagnostics: &Diagnostics) -> Result<()>;
// `ship server`: ship_server::serve(address, starter, config, shutdown, diagnostics.server_log())
// `ship server log ...`: commands::server_log (slice 3)
```

#### `crates/ship/src/local.rs`: the temp file in `default_health` (slice 1)

The `tempfile` block is replaced with `RunLog::start(&logs_dir()?.join(port), stem)`. Its file gets cloned for the child's stdout and stderr, and its path goes in `--log-file`. A failed launch names that path, as it names the temp file today.

#### `crates/ship/src/cli.rs` (slices 1 and 3)

```rust
pub struct ServerArgs {
    // ...as today, plus:
    /// The launcher's part 1; set only with --background-child.
    #[arg(long, hide = true, requires = "background_child")]
    pub log_file: Option<PathBuf>,
}

pub enum ServerCommand {
    Stop,
    /// Print the server's health as JSON, including its log file; never starts a server
    Status,
    /// Print the running server's log filter, or change it until the server stops
    Log(LogArgs),
}

#[derive(Args)]
#[group(multiple = false)]
pub struct LogArgs {
    /// Ship's level, as SHIP_LOG sets it: error, warn, info, debug or trace
    pub level: Option<tracing::Level>,
    /// A full filter, as RUST_LOG sets it
    #[arg(long)]
    pub filter: Option<String>,
    /// Return to the filter the server started with
    #[arg(long)]
    pub reset: bool,
}
```

#### `crates/ship/src/commands.rs` (slice 3)

```rust
/// Shows or changes the filter and prints the result as JSON (G4).
pub async fn server_log(client: &Client, args: LogArgs) -> Result<()>;
```

#### `crates/ship-core/src/protocol.rs` (slice 3)

```rust
pub const PROTOCOL_VERSION: u32 = 13;

pub struct HealthResponse {
    pub service: String,
    pub protocol_version: u32,
    pub version: String,
    /// The log part the server is writing (FR-008).
    pub log: String,
}

/// PUT /api/v0/server/log body: `{"set": "warn,ship=debug"}` or `"reset"` (G3).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum LogChange {
    Set(String),
    Reset,
}

/// GET and PUT /api/v0/server/log response.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogFilter {
    pub active: String,
    pub startup: String,
    /// Same as `HealthResponse.log`.
    pub file: String,
}
```

#### `crates/ship-server/src/lib.rs` and `app.rs` (slices 2 and 3)

```rust
/// The runner's control over the log filter (architecture decision 1).
pub type ServerLog = Arc<dyn Fn(Option<LogChange>) -> Result<LogFilter> + Send + Sync>;

pub async fn serve(
    address: SocketAddr,
    starter: bool,
    config: PathBuf,
    shutdown: impl Future<Output = Result<()>> + Send + 'static,
    log: impl Fn(Option<LogChange>) -> Result<LogFilter> + Send + Sync + 'static,
) -> Result<()>;

pub struct AppState {
    // ...as today, plus:
    pub(crate) log: ServerLog,
}
```

`forward_pane_events` wraps its task in `info_span!(parent: None, "pane_events")` (slice 2).

#### `crates/ship-server/src/health.rs` (slice 3)

```rust
/// Now reads the log file through `app.log(None)`.
pub(crate) async fn health(State(app): App) -> Result<Json<HealthResponse>>;
```

#### `crates/ship-server/src/routes.rs` (slice 3)

```rust
#[utoipa::path(get, path = "/api/v0/server/log", operation_id = "get_server_log", ...)]
pub(crate) async fn get_server_log(State(app): App) -> Result<Json<LogFilter>>;

#[utoipa::path(put, path = "/api/v0/server/log", operation_id = "set_server_log",
    request_body = LogChange, ...)]   // 422 on an invalid filter (FR-015)
pub(crate) async fn set_server_log(State(app): App, Json(change): Json<LogChange>) -> Result<Json<LogFilter>>;
```

#### `crates/ship-client/src/api.rs` (slice 3)

```rust
impl Client {
    /// GET when `change` is `None`, PUT otherwise.
    pub async fn server_log(&self, change: Option<&LogChange>) -> Result<LogFilter>;
}
```

#### `crates/ship-macros/src/lib.rs` (slice 2)

```rust
/// `tracing::instrument` with Ship's defaults (architecture decision 8):
/// `level = "debug"`, `skip_all`, and on functions returning `Result`,
/// `err(level = "warn", Debug)`. Handles `no_err` and the default level;
/// all remaining options pass through unchanged to `tracing::instrument`.
/// The using crate must depend on `tracing`.
#[proc_macro_attribute]
pub fn traced(attr: TokenStream, item: TokenStream) -> TokenStream;

/// On `impl Message<X> for A`: adds `fn name() -> &'static str` returning
/// `X` as written, such as `"Get<Tab>"` (architecture decision 10).
#[proc_macro_attribute]
pub fn named(attr: TokenStream, item: TokenStream) -> TokenStream;
```

`#[traced]` adds `err` only when the return type's last path segment is `Result`, since `instrument(err)` doesn't compile on other returns.

#### `crates/ship-core/src/lib.rs` (slice 2)

```rust
/// Spawns `future` inside the caller's span, for short tasks tied to their
/// caller. Long-lived tasks get their own root spans instead (decision 9).
pub fn spawn_traced<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where F: Future + Send + 'static, F::Output: Send + 'static;
```

#### Span annotations (slice 2)

| Where | What |
| --- | --- |
| `ship-server/src/state.rs` | `#[named]` on all 21 `impl Message<X> for ServerState` |
| `ship-server/src/pane.rs` | the pane task in `info_span!(parent: None, "pane", pane = %id)`; the `pane-exit` thread in `debug_span!(parent: None, "pane_exit", pane = %id)`; `#[traced]` on `spawn`, `PaneTask::command`, `publish`, `teardown` |
| `ship-server/src/input.rs` | `#[traced]` on `route` |
| `ship-server/src/attach.rs` | `#[traced]` on `attach`'s stream `next`; the detach in `AttachmentGuard::drop` through `spawn_traced` |
| `ship-client/src/ui/mod.rs` | `#[traced]` on `turn`, `send`, `redraw` |
| `ship-client/src/ui/observer.rs` | `#[traced]` on `Observer::apply` |
| Other non-trivial functions | `#[traced]` where an error or a step is worth seeing, at `trace` for anything per cell or per row, such as `capture` and `row` |

Fields name IDs, sizes and counts only; arguments are skipped by default, with no automatic recording of pane output, input, clipboard text, full argument lists or environments (FR-010). Explicit errors may retain useful program names and dependency details, as Cyan approved during slice 2.

### Moved

None.

### Untouched

- `vendor/ratatui-ghostty`: no wrapper changes.
- The config file and `config.rs`: no log setting (product non-goal).
- `ship-core/src/error.rs`: `ResultExt` logging stays as is; `#[traced]` matches its Debug format.
- The keymap and every bindable action.

## Build order

1. **Run files and levels.** Files: `runlog.rs` (new), `diagnostics.rs`, `main.rs`, `local.rs`, `cli.rs` (`--log-file`), manifests (`chrono` in, `tempfile` out). Delivers FR-001 to FR-007, FR-009 and FR-011: a server, background or foreground, writes `logs/<port>/<stem>.log` with `current.log`, a client writes `logs/client/`, one-shot commands keep stderr, and `SHIP_LOG` and `RUST_LOG` set the filter. Checks:
   - Bare `ship` launches a background server, whose run file holds the launch line, and `current.log` points at it. Modes are 0600 and 0700.
   - SIGKILL the server, start another: two files, and the killed run's last line is in its own (SC-002).
   - A temporary probe with `PART_BYTES` and `DIR_BYTES` lowered shows a `.2.log` part, `current.log` following it, and the oldest file pruned. The probe isn't committed.
   - A forced panic in a debug build lands in the run file.
   - `SHIP_LOG=debug` shows Ship's debug lines and kameo's info spans; `SHIP_LOG=nope` fails naming the levels (G5).
   - The client draws with nothing on stderr.
2. **Spans.** Files: `ship-macros/src/lib.rs`, `ship-core/src/lib.rs` and its manifest, `ship-client`'s manifest, and the annotations listed above. Delivers FR-010 and FR-016. Checks:
   - `ship pane create -- /nonexistent` logs one warn line whose span chain runs from `CreatePane` to `spawn` when debug spans are enabled. Kameo's message span is a new root, as the architecture paper specifies; the sending HTTP request is not part of that chain.
   - With `SHIP_LOG=debug`, actor spans read `CreatePane` and `Resize`, not type paths.
   - Type a marker string into a pane and paste another at `SHIP_LOG=trace`; neither appears in any log file.
   - `cargo expand` of one `#[traced]` function shows the expected `instrument` arguments.
3. **Live level change.** Files: `protocol.rs`, `lib.rs` and `app.rs` (`serve`, `AppState`), `health.rs`, `routes.rs`, `api.rs`, `cli.rs` (`ServerCommand::Log`), `commands.rs`, `main.rs`, `diagnostics.rs` (`server_log`). Delivers FR-008 and FR-012 to FR-015. Checks:
   - `ship server log` prints the filter; `ship server log debug` makes debug lines appear within a second while a pane keeps running (SC-003); `--reset` returns to the startup filter; `--filter 'ship=['` fails and `ship server log` shows nothing changed.
   - `ship server status` prints the current part's path, also after a rollover in the slice 1 probe.
   - The OpenAPI document lists both routes.
4. **Timelines.** Files: `diagnostics.rs` (`init`), manifests (`tracing-chrome`). Delivers FR-017 to FR-019. Checks:
   - `SHIP_TRACE=<dir>` on a server and a client, a few keystrokes, clean exits: `<dir>/server-<pid>.json` and `<dir>/client-<pid>.json` exist and parse.
   - Both files hold the keystroke path's spans by name: `turn`, `send`, `route`, `command`, `publish`, `apply`, `redraw` (SC-004). Cyan opens one in ui.perfetto.dev to confirm the view.
   - At default `SHIP_LOG`, the timeline still holds debug spans (FR-018). Without `SHIP_TRACE`, no file appears (FR-019).
5. **Bench, docs, specs and size.** Files: `docs/guide.md`, `README.md` if its lists changed, the spec deltas named in the product paper. Checks:
   - The bench on ports 43311 to 43317 at default settings shows typing latency and flood throughput within noise of `fc2b5a7` (SC-001).
   - Implementation Rust and tests reported separately (SC-005).
   - Linux reported unverified.

## Deviation log

### Approved retention adjustment after slice 3

Cyan approved reducing the part limit from 50 MiB to 5 MiB after slice 3. The 200 MiB per-directory retention budget is unchanged. Runs still use one timestamp with numbered parts; smaller parts allow finer-grained pruning. Historical slice evidence below records the limits used at the time.

### Slice 1: run files and levels

- `etcetera::choose_base_strategy()` selects Apple's strategy on macOS, whose `state_dir()` is `None` and whose data directory is `~/Library/Application Support`. The proposed call therefore would not satisfy FR-001. `runlog::logs_dir` uses etcetera's `Xdg` strategy explicitly outside Windows, retaining the planned Windows fallback.
- Run stems include nanoseconds to distinguish starts within the same second. File creation is exclusive and never appends to an earlier run.
- Both formatters emit new-span events so enabled kameo message spans are visible even when their bodies emit no events.
- The `Diagnostics` holder and reload handle are left for slice 3, when they gain their consumer. In slice 1 the installed subscriber owns the run writer; there is no exit-flush guard until slice 4.
- Auto mode initially blocked temporary probe edits. Cyan explicitly authorized their path scopes before verification resumed. The bench default port, reduced limits and forced panic sites were all restored after the probes; no permanent test or probe code was added.

### Slice 2: spans

- Cyan requested that `spawn_traced` be available without `kameo`, since it has no actor dependency. Tokio’s `rt` feature is unconditional; `kameo` enables `tokio/sync` for the relay.
- Cyan requested that `traced` forward options it does not consume to `tracing::instrument`, rather than keep an allowlist. `no_err` is consumed and `level` suppresses the default; the inner macro validates remaining options. A temporary compile/expansion probe verified `target`, `parent` and `follows_from` forwarding while preserving defaults. The probe was removed. Upstream tracing-attributes 0.1.31 rejects `parent`/`follows_from` after `target` (its parser checks the target field); the probe placed `target` last, without adding a Ship workaround.

- Cyan approved keeping useful failure diagnostics rather than treating FR-010 as a promise that no sensitive information can reach logs. FR-010 and its summaries/delta now prohibit automatic recording of terminal payloads, full argument lists and environments, but permit useful program names and dependency error details in explicit failures. Existing detailed spawn errors remain intact.
- The checklist and spec delta claimed a request-through-actor chain, contradicting the controlling architecture's explicit actor-boundary limit. They now describe the implemented `CreatePane -> spawn` chain with debug spans enabled. No cross-actor context propagation was added.
- `Progress::next` uses `no_err`: its Result reports normal stream shutdown, not an operation failure. This avoids a warning on normal detach/shutdown.
- Temporary helper probes and cargo expansion verified macro behavior, generic message names and short-task span inheritance. Real debug/release PTY workflows verified the annotations and payload boundary. No permanent tests were added.
