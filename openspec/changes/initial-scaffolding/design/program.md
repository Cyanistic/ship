# Initial scaffolding program

Status: revised draft, not locked. Audience: mixed. Product and architecture are settled; this paper defines their implementation shape. This revision incorporates Cyan's program review. It does not authorize implementation.

## Rationale

Ship starts as one executable composing four crates. Running `ship` performs the health exchange, starting a local server if none is listening. Running `ship server` starts a foreground server explicitly. The client methods are agent-authored, reviewed Rust using shared DTOs, not generated during builds.

The skeleton separates data, requests, transport policy and process startup. Build it in three runnable slices: foundation, health/lifecycle, then schema agreement and the client-update workflow.

This is a document-only plan. Declarations below describe proposed files, not compilable implementation. Leave the abandoned partial skeleton, its untracked files, and its branch untouched. A later implementation request must reconcile those files against the accepted plan.

## Skeleton map

### Files and responsibilities

| File | Slice | Responsibility |
| --- | --- | --- |
| `Cargo.toml` | 1 | Four-crate workspace, shared dependency constraints, lints and build profiles. |
| `Cargo.lock` | 1, updated later | Cargo-owned resolved dependencies. Never edit manually. |
| `.gitignore` | 1 | Ignore root `/target/`. |
| `README.md` | 1, extended later | Explain Ship and its user-facing commands, startup, logs and shutdown. |
| `crates/ship/Cargo.toml` | 1 | One executable; depends on the three library crates and application dependencies. |
| `crates/ship/src/main.rs` | 1, extended in 2 | Parse CLI, initialize the process, dispatch, print results and choose exit status. |
| `crates/ship/src/cli.rs` | 1, operations in 2 | Root client options and optional `server` subcommand. |
| `crates/ship/src/diagnostics.rs` | 2 | Tracing initialization and signal-driven shutdown. |
| `crates/ship/src/local.rs` | 2 | Probe, launch and wait for the default-local server. |
| `crates/ship-core/Cargo.toml` | 1 | Shared Serde/error dependencies; Utoipa enters in slice 3. |
| `crates/ship-core/src/lib.rs` | 1, extended later | Reexport shared errors and protocol data. |
| `crates/ship-core/src/error.rs` | 1 | Contextual error types, result extensions and error construction. |
| `crates/ship-core/src/protocol.rs` | 2 | Plain health DTO and shared wire/default constants. Schema derive enters in 3. |
| `crates/ship-client/Cargo.toml` | 1, extended in 2 | Shared DTOs, Reqwest, URL, Tokio and Serde. No server dependency or build script. |
| `crates/ship-client/src/lib.rs` | 1, exports in 2 | Public `ServerTarget`, `HealthClient`, `LocalProbe`; private implementation modules. |
| `crates/ship-client/src/target.rs` | 2 | Parse, resolve and safely display the chosen server URL. |
| `crates/ship-client/src/transport.rs` | 2 | HTTP client, redirects, bounds, status checks and error translation. |
| `crates/ship-client/src/api.rs` | 2 | Thin agent-authored endpoint methods using shared DTOs directly. |
| `crates/ship-client/src/health.rs` | 2 | Stable facade, health identity validation and local absence detection. |
| `crates/ship-server/Cargo.toml` | 1, extended later | Core, Axum, Tokio, tracing and tower-http; Utoipa enters in 3. |
| `crates/ship-server/src/lib.rs` | 1, runtime in 2 | Router, loopback listener and graceful shutdown; schema export in 3. |
| `crates/ship-server/src/health.rs` | 2 | Construct and return the health DTO; document its route in 3. |
| `crates/ship-server/src/schema.rs` | 3 | Pure Utoipa document construction, without starting a server. |

These are additions to the accepted baseline, not necessarily absent from the current working tree. Nothing is moved. The old codegen plan was never implemented and requires no source migration.

This revision changes only `openspec/changes/initial-scaffolding/design/program.md`. Do not modify product, architecture, implementation files, experiments, research, unrelated planning, branches or commits. No proposal, OpenSpec design artifact, specs or tasks are created. Attachment to OpenSpec remains a separate decision.

### Workspace and dependencies

Proposed `Cargo.toml`:

```toml
[workspace]
members = ["crates/ship", "crates/ship-core", "crates/ship-server", "crates/ship-client"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"

[workspace.lints.clippy]
all = "warn"

[profile.dev]
opt-level = 0
debug = "line-tables-only"
incremental = true

[profile.dev.build-override]
opt-level = 3

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"

[profile.test]
inherits = "dev"
incremental = true
codegen-units = 256
```

Unsafe code is allowed: do not add a workspace deny/forbid rule. This skeleton does not require unsafe process-launch code; permitting it is not a reason to introduce it.

All four manifests inherit workspace version, edition and lints. Centralize dependencies under `[workspace.dependencies]`; members inherit only what they use:

| Dependency | Version constraint | Features |
| --- | --- | --- |
| Clap | `4` | `derive` |
| Tokio | `1` | `rt-multi-thread`, `macros`, `net`, `signal`, `time` |
| tracing | `0.1` | Defaults |
| tracing-subscriber | `0.3` | `env-filter` |
| Serde | `1` | `derive` |
| serde_json | `1` | Defaults |
| Axum | `0.8` | Defaults |
| tower-http | `0.6` | `trace` |
| URL | `2` | Defaults |
| nix | `0.31` | `process` |
| tempfile | `3` | Defaults |
| Utoipa | `6.0` | Defaults |
| Reqwest | `0.13.4` | Defaults disabled; `json`, `rustls` |

These are ordinary Cargo compatibility requirements, not exact `=` pins. Cargo.lock fixes the versions actually selected for the application. Compatibility is established by resolution, builds and the real exchange, not by this table.

Application dependencies: the three Ship libraries, Clap, Tokio, tracing, tracing-subscriber and serde_json; add nix/tempfile with local startup. Core uses Serde, serde_json and tracing, then Utoipa. Client uses core, Reqwest, URL, Tokio and Serde. Server uses core, Axum, Tokio, tower-http and tracing, then Utoipa. Client and server do not depend on each other.

No Progenitor, generator runtime, AI SDK, Java tooling, experiment patches, task runner, configuration framework or second error framework. No `ship-client/build.rs`, generated inclusion module or `OUT_DIR` client source.

### CLI and application entry

```sh
ship                              # Health exchange; auto-start default-local if absent
ship --server-url https://…        # Connect only; never launch a local server
ship server                       # Explicit foreground loopback server
ship server --port 45000
```

The bare command is the client workflow. There is no `client` subcommand, redundant `health` subcommand, TUI, server-management command or shutdown endpoint in this checkpoint.

Proposed parser in `crates/ship/src/cli.rs`:

```rust
use clap::{Parser, Subcommand};
use ship_core::DEFAULT_PORT;

#[derive(Parser)]
pub struct Cli {
    #[arg(long, conflicts_with = "command")]
    pub server_url: Option<String>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    Server {
        #[arg(long, default_value_t = DEFAULT_PORT)]
        port: u16,
        #[arg(long, hide = true)]
        background_child: bool,
    },
}
```

This declaration is illustrative; verify Clap's exact option/subcommand conflict behavior when implementing. Reject combining a custom client URL with `server`. An explicitly supplied URL, even the default URL, always means connect-only. Reject port zero. Bind only `127.0.0.1`; no host flag. Help must explain that automatic startup leaves a background server running.

Keep `main` synchronous. A child with the hidden launch marker establishes its independent Unix session before Tokio/tracing initialization. Then create the runtime and dispatch either foreground serving or one health request. The hidden marker is internal launch plumbing, not a supported manual command.

Successful client invocation prints one JSON line containing `HealthResponse`. Diagnostics go to stderr or the daemon log, not stdout. Help/version and successful operations exit 0; operational failures exit 1; Clap usage errors retain exit 2. Include safe target context on failure and do not print success output.

`diagnostics.rs` initializes tracing once per process. Default to `info`, accept `RUST_LOG`, reject an invalid filter, and disable ANSI when stderr is not a terminal. SIGINT/SIGTERM request graceful server shutdown. Drain active health requests within five seconds, then exit. Signal-registration failure is an error.

### Shared errors

Keep the contextual AppError direction. External errors retain a diagnostic string rather than a second framework or guessed blanket conversions.

Proposed declarations in `crates/ship-core/src/error.rs`:

```rust
use std::borrow::Cow;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum ErrorCode {
    Validation, NotFound, Conflict, Configuration, Network, RateLimited,
    UpstreamHttpStatus(u16), Internal, Serialization, Unauthorized, Io,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum AppError { Internal(InternalError), External(ExternalError) }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InternalError {
    pub message: Cow<'static, str>,
    #[serde(rename = "type")]
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caused_by: Option<Box<AppError>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalError {
    pub code: ErrorCode,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl AppError {
    pub fn internal(code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self;
    pub fn external(code: ErrorCode, error: impl std::fmt::Display) -> Self;
    pub fn code(&self) -> &ErrorCode;
    pub fn with_data(self, data: Value) -> Self;
    pub fn with_cause(self, cause: AppError) -> Self;
    pub fn context(self, message: impl Into<Cow<'static, str>>) -> Self;
}

impl std::fmt::Display for AppError;
impl std::error::Error for AppError;

pub trait ResultExt<T>: Sized {
    fn context(self, message: impl Into<Cow<'static, str>>) -> Result<T>;
    #[track_caller] fn error(self) -> Result<T>;
    #[track_caller] fn warn(self) -> Result<T>;
    #[track_caller] fn debug(self) -> Result<T>;
    #[track_caller] fn trace(self) -> Result<T>;
}
```

ResultExt applies to shared Result. Each logging method returns the unchanged result and records caller file/line; report a failure once at its chosen boundary. Context preserves the underlying category. Optional fields omit None; deserialized messages are owned. Display renders the context/cause chain; an internal cause is exposed through Error::source. Library conversions explicitly supply a category.

Include an exported `err!` macro in the error module. **Its call syntax and behavior still need Cyan's example or existing definition.** Do not treat the earlier speculative macro forms in the superseded paper as approved. Reuse that intended contract with hygienic `$crate` paths rather than inventing a new one; confirm before implementing it.

Retain the proposed HTTP-status helper without an HTTP-crate dependency: validation 400, not found 404, conflict 409, configuration/internal/serialization/I/O 500, network 502, rate limited 429, upstream HTTP status 502, unauthorized 401. Preserve upstream status in diagnostic data, not by blindly returning it. Server controls any HTTP error body; this health route exposes only its success DTO, not AppError internals.

Core's `lib.rs` reexports these error types/extensions and the protocol declarations below.

### Shared protocol: data, not behavior

`crates/ship-core/src/protocol.rs` contains the shared wire contract. Constants live here because server routes, client requests and local target selection must agree on them. They are not fields in the response:

- `DEFAULT_PORT: u16 = 43179` and `DEFAULT_SERVER_URL: &str = "http://127.0.0.1:43179"`: shared default endpoint.
- `HEALTH_PATH: &str = "/health"`: shared route path.
- `PROTOCOL_VERSION: u32 = 1`: shared compatibility expectation.
- `HEALTH_OPERATION_ID: &str = "health"`: schema metadata added in slice 3.

The DTO is plain data, without `current()` or `validate_identity()` methods:

```rust
use serde::{Deserialize, Serialize};
use utoipa::ToSchema; // Added with the derive in slice 3.

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub service: String,
    pub protocol_version: u32,
    pub version: String,
}
```

The server constructs it with service `ship`, shared protocol version 1 and package version, initially `0.1.0`. The client validates service and protocol version after deserialization; package version is informational. GET /health returns HTTP 200 with this flat JSON object. TCP success or an unrelated HTTP 200 is not compatible health. This detects accidental mismatches, not authenticated server identity.

### Client: method authoring versus transport policy

Public facade:

```rust
pub struct ServerTarget { /* Private URL. */ }
impl ServerTarget {
    pub fn default_local() -> Self;
    pub fn parse(input: &str) -> ship_core::Result<Self>;
    pub fn base_url(&self) -> &url::Url;
    pub fn health_url(&self) -> ship_core::Result<url::Url>;
    pub fn display_safe(&self) -> String;
}

pub struct HealthClient { /* Owns private Transport. */ }
pub enum LocalProbe { Absent, Ready(ship_core::HealthResponse) }
impl HealthClient {
    pub fn new(target: ServerTarget) -> ship_core::Result<Self>;
    pub fn target(&self) -> &ServerTarget;
    pub async fn health(&self) -> ship_core::Result<ship_core::HealthResponse>;
    pub async fn probe_default(&self, remaining: std::time::Duration)
        -> ship_core::Result<LocalProbe>;
}
```

`target.rs` accepts absolute HTTP/HTTPS URLs with a host and optional port. Normalize to the root path and remove query/fragment; a supplied base path is not a health-route prefix. Clone is supported; Display/Debug redact userinfo. Parsing and foreign HTTP errors must not leak credential-bearing URLs. No insecure TLS bypass.

`transport.rs` owns the private Reqwest client and selected target. Its internal `get_json<T: DeserializeOwned>(path, expected_status, budget)` resolves the URL, refuses redirects, checks exact status and bounds the complete request/body deserialization. Keep health bounds per call rather than defining a global timeout for future streaming. Translate structured failures to contextual errors. No retries or speculative middleware framework.

`api.rs` starts as one file with an internal `health(&Transport, Duration) -> Result<HealthResponse>` function. It specifies the shared path, HTTP 200 and the shared response type, then delegates transport work. No copied DTOs, alternative serializers, body-builder assumptions, startup code or transport-policy changes.

`health.rs` calls that method, then validates service/protocol identity through private client-owned logic. Its default-local probe uses TCP `ErrorKind::ConnectionRefused` to establish absence; other failures are not absence. After a successful TCP connection, perform and validate the actual health request. The probe accepts only the default-local target and fits both connect and request waits within the supplied remaining budget.

Accepted api.rs is ordinary checked-in Rust. People and agents can edit it. Do not mark it auto-generated/do-not-edit; no generator can reproduce it. Normal builds and runtime invoke neither a model nor a schema-to-client generator. Splitting the file later is cheap; file length alone does not trigger deterministic generation.

### Local startup: probe, launch, wait

`crates/ship/src/local.rs` has one application-facing operation:

```rust
pub async fn default_health(client: &ship_client::HealthClient)
    -> ship_core::Result<ship_core::HealthResponse>;
```

Its flow is:

1. **Probe the default endpoint.** If it returns compatible health, use it. If the TCP connection is refused, the server is absent. Any other error fails without replacing a listener.
2. **Launch this executable as a server.** Resolve current_exe and spawn it directly with `server --port 43179 --background-child`. No shell, cargo command or second daemon binary.
3. **Wait for health within five seconds.** Poll at 100 ms intervals. Accept only compatible health, not simply a live child or open TCP port.
4. **Return the health response.** Leave a ready daemon running when the client exits. If startup fails, clean up only the child this invocation launched and report the failure with its log path.

The module retains the Child handle and log path while startup is unresolved. That temporary state is for checking early exit and cleaning up a failed attempt, not a PID registry or supervisor. Keep those implementation details private; no public LaunchAttempt abstraction is needed.

**Launch details.** Redirect stdin from null and both output handles to a unique mode-0600 tempfile with prefix `ship-server-` and suffix `.log` in the OS temporary directory. Keep the log after launch. Never reuse a predictable writable filename or loosen permissions. Report daemon PID/log location on successful launch and log location on failure. If log creation fails, report the attempted directory and say no file was created. Retention follows OS/manual temporary-file cleanup; no rotation or app-directory framework.

The marked child calls safe `nix::unistd::setsid()` synchronously before Tokio/tracing/listening. Do not set its process group beforehand, use pre_exec, or fork a multithreaded runtime. Session setup failure exits before listening. This mechanism targets Linux/macOS; only the environment actually exercised can be claimed verified.

**Bounds and cleanup.** Initial TCP connect is bounded by one second; a complete health request by two seconds. Readiness uses an absolute five-second deadline, with every attempt capped by its remaining time. Refusal after launch means not ready yet; other health errors fail. Inspect try_wait between attempts. If the child exits, make one bounded final health probe to permit a concurrent compatible winner; otherwise report exit status and log path. On failure, terminate/reap only this attempt's still-live child. Never kill an existing listener or another winning server. Losing bind attempts must exit rather than persist idle.

Custom targets bypass this entire launch flow. Foreground servers stop normally on SIGINT/SIGTERM; failed-launch child cleanup is a different path.

### Server and schema

Proposed public declarations in `crates/ship-server/src/lib.rs`:

```rust
pub fn loopback_addr(port: u16) -> ship_core::Result<std::net::SocketAddr>;
pub fn router() -> axum::Router;
pub async fn serve(
    address: std::net::SocketAddr,
    shutdown: impl std::future::Future<Output = ship_core::Result<()>> + Send + 'static,
) -> ship_core::Result<()>;
pub fn openapi() -> utoipa::openapi::OpenApi; // Slice 3.
```

Reject non-loopback addresses at serve, not only at the CLI. Classify bind failures through structured I/O; AddrInUse maps to conflict. Log address, process ID and protocol version at startup. Configure tower-http TraceLayer so method/path and response status/duration appear at the default info filter; do not log arbitrary bodies or headers.

`health.rs` constructs the plain shared DTO and returns `axum::Json<HealthResponse>`. Router uses HEALTH_PATH. Slice 3 adds Utoipa descriptions for GET /health, operation ID health and response 200/application/json. If attributes require literals, put them near the handler and manually compare them to the shared constants; do not assume const expressions are accepted.

`schema.rs` derives OpenApi from that path and HealthResponse and exposes pure document construction through openapi(). No listener, tracing initialization, tasks or async runtime is required. Keep Utoipa 6's default OpenAPI 3.1 output, not a relabeled 3.0 document. No schema HTTP endpoint, installed export command or client build dependency.

### Documentation and client updates

README is for end users: what Ship does at this checkpoint, how to build/run it, command examples, automatic local startup, custom-target connect-only behavior, daemon logs and manual signal-based shutdown. Do not put internal API-update instructions, implementation slices or verification logs in it.

Keep development instructions and evidence in this program paper:

- Read the corresponding server handler/route, shared DTOs/Serde attributes, exported schema and existing client conventions.
- Change only the affected endpoint methods and separately approved API changes. Reuse shared DTOs directly.
- Preserve transport, startup and facade contracts. Ask before changing public interfaces or security policy.
- Review method/path/query/header/body/status agreement and actual Serde behavior.
- Run local checks and make a real request against the corresponding rebuilt server.
- Record commands, observed results and unverified gaps under this paper's Deviation log. Do not present manual reconciliation as automatic synchronization.

Compilation catches incompatible Rust uses, not all wire drift. No model/provider/prompt replay is a build prerequisite. Deterministic-generation candidates and their type-mapping/builder caveats remain in architecture.md; investigating them is not an implementation task here.

### Remaining decisions and evidence

The `err!` contract is unresolved and must be confirmed before implementing that macro. The other changes above record Cyan's accepted program direction. Dependency compatibility, exact Clap parsing behavior and runtime process behavior remain future verification work, not proof already collected.

## Build order

### 1. Foundation and local checks

Create/reconcile workspace/manifests, Cargo-produced lockfile, ignore file, end-user README, application main/CLI shell, minimum library roots and shared error support. Add only foundation dependencies. Confirm the err! contract before implementing it. Do not ship todo/panic placeholders for future behavior.

Deliver working help/version and a buildable four-crate executable; the health/server operations enter slice 2. Use the available Rust/Cargo environment, initially 1.98.0, without asserting an MSRV or untested platform parity.

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace` and a release build. Exercise help/version without a server. Inspect serialization, error handling, macro hygiene and dependency direction. Record environment, commands, outcomes and gaps in this paper before slice 2. No authored tests, CI files or terminal/SSE/compression behavior.

### 2. Health and process lifecycle

Add protocol, local startup/diagnostics, client target/transport/api/health and server health. Extend roots, CLI/dispatch, manifests, lockfile and end-user README. Endpoint methods are already accepted source; no later generator swap.

After local checks and debug/release builds:

- Exercise `ship --help` and `ship server --help`. Verify root --server-url/server mutual exclusion.
- With the default port free, run `ship`, observe health JSON and daemon PID/log, then run it again after client exit. Verify the same daemon is reused.
- Launch from a separate terminal and close it. Verify the server survives; inspect its independent Unix session and private log mode.
- Inspect request diagnostics for GET /health and HTTP 200.
- SIGTERM the recorded daemon; verify exit and refusal on the former port. Run foreground `ship server`, connect using `ship`, then SIGINT and verify shutdown.
- Run `ship server --port 43180` and `ship --server-url http://127.0.0.1:43180`. Verify success without launching a default server.
- With servers stopped, explicitly request an unreachable custom URL and the default URL. Both fail with context and neither auto-starts.
- Occupy the default port with a controlled non-Ship listener. Verify bad status/JSON/identity fails without launch or replacement.
- As the ordinary development user, use a new temporary TMPDIR without write permission. Verify actionable log-creation failure, no claimed file and no daemon; restore permissions and remove the temporary directory.
- Exercise readiness failure by pausing the just-spawned child before listening with a debugger/signal. Verify bounded failure, owned-child cleanup and retained log. If this cannot be done reliably, record the gap.
- Start ten clients concurrently against a free default port. Verify at most one serving daemon, no surviving losing children and only success or bounded contextual failure. Inspect processes as well as exit codes.

Normally stop the remaining server, restore the environment, repeat local checks and record actual outcomes before slice 3.

### 3. Schema agreement and client-update workflow

Add schema.rs, server export/route annotations, core schema derives and metadata, dependencies/lockfile changes, and any relevant user documentation. No build script or persistent generation/verification tool.

Run local checks and debug/release builds. Exercise health against explicit and auto-started servers. Inspect direct HealthResponse reuse and absence of copied models.

Use a disposable inspection harness outside the production tree to call ship_server::openapi() and serialize to a temporary file. Inspect route, operation identifier, status and field/wire-type agreement with the server and Serde. Record the invocation/output examined, not an automatic freshness guarantee.

Temporarily change a shared DTO field/type and reconcile the server/schema. Leave an incompatible consumer use to observe compiler feedback, then update affected source according to the development instructions. Rebuild the corresponding server and make the real request. This proves that particular incompatible use, not that every DTO change causes a compile error.

Separately change the health route while deliberately keeping the old path in api.rs for this controlled exercise. Observe request failure despite compilation succeeding. Reconcile the method with the server/shared path, rebuild and verify success. Sharing HEALTH_PATH normally avoids that literal drift; the exercise demonstrates why compilation alone is insufficient.

Restore the baseline, discard temporary inspection files, reinspect schema, repeat affected health/lifecycle checks and rerun local checks. Record reviewed patches and observed results here. No permanent test suite, new shipping command, copied model or model API call.

## Deviation log

### Foundation checkpoint: tasks 1.1–1.5

Implemented and verified on 2026-10-04 under the explicit foundation-only apply request. Help/version and local checks pass. No health, lifecycle or schema work was started; stop here before slice 2. The planning-status prose and superseded snippets elsewhere in this paper are historical, not evidence of implementation.

#### Reconciliation and reused mechanisms

- The initial tree had untracked root/crate manifests, `.gitignore`, core `lib.rs` and `protocol.rs`, but no accepted build. The core root referenced missing `error.rs`; the manifests still selected generator/framework dependencies and denied unsafe. Reconciled only foundation manifests and active roots against design.md.
- Preserved the existing `/target/` ignore, four-crate ownership, package version/edition inheritance, restrained Clippy lint and all proposed dev/build-override/release/test profiles. Removed Progenitor, color-eyre, directories, serde_with, the client build dependency/dormant build declaration, unsafe denial and exact version constraints. Root requirements now include compatible Reqwest `0.13.4` with defaults disabled and json/rustls, Utoipa `6.0`, Tokio macros and tempfile. Only foundation dependencies are inherited by members at this checkpoint.
- Left the pre-existing untracked `crates/ship-core/src/protocol.rs` unchanged and unwired. It includes historical schema metadata and belongs to later reconciliation, not this checkpoint. Core now exports only the implemented errors; client/server roots contain ownership documentation, not future-feature function/panic stubs. No experiment integration, build script, generated client, permanent authored tests or CI was added.
- Reused Serde/serde_json for the approved tagged representation, tracing for reporting and Clap for help/version. Cargo alone created the application lockfile. Added the smaller string-backed AppError/context/category/status implementation and ResultExt; did not retain LossyError or another report framework.
- design.md resolves the historical err! question through the reference error module. Reused its grammar and imported category namespace with `$crate` paths; adapted constructors, source/external modifiers and JSON helpers to Ship. Internal source modifiers retain the supplied cause; external modifiers wrap foreign text under the outer category; later cause modifiers replace earlier ones as in the reference. Fallible expression/object conversion omits data. Object fields borrow their values, matching the reference JSON shorthand. No foreign adapters, message-based category inference, HTTP response implementation, bail! or ensure! were added.
- README contains only supported behavior, prerequisites, build/run commands and local checks. Bare invocation currently displays help and exits 0; there are no server/client-operation flags advertised before slice 2.

#### Environment

- macOS 26.6.2, build 25G83, arm64; Rust host `aarch64-apple-darwin`.
- `rustc --version --verbose`: 1.98.0 (`88d9e12ae`, 2026-08-18), LLVM 22.1.8. `cargo --version`: 1.98.0 (`797e8a9bc`, 2026-08-05).
- `rustup show active-toolchain`: `stable-aarch64-apple-darwin (default)`. `rustup component list --installed` confirmed rustfmt and Clippy.
- Cargo successfully accessed crates.io. The application lockfile resolved 34 registry packages; representative versions: Clap 4.6.7, Serde 1.0.229, serde_json 1.0.151 and tracing 0.1.44. No MSRV or other-platform support claim is made.

#### Commands and observed results

- `cargo metadata --format-version 1 --no-deps` plus disposable Python assertions: exactly four members; all member manifests inherit version, Rust 2024 and workspace lints; no build dependencies. Initial `cargo build --workspace` exited 0.
- `cargo metadata --format-version 1` plus resolved-node inspection: app directly depends on Clap and the three libraries; client/server directly depend on core only; core directly depends on Serde, serde_json and tracing. No client/server implementation dependency in either direction and no generator/model dependency. Root manifest inspection confirmed compatible constraints, retained profiles and absence of unsafe denial/superseded frameworks.
- Disposable external consumer at `/tmp/ship-foundation-proof.R6cSoH`: `CARGO_TARGET_DIR=/Users/cyan/Documents/projects/ship/target/foundation-proof cargo run --manifest-path /tmp/ship-foundation-proof.R6cSoH/Cargo.toml` exited 0. First inspected errors directly; then removed its direct serde_json requirement and reran with macro consumers. A nested macro-calling module had no JSON import or category-variant import. Its manifest directly depended only on ship-core, serde, tracing and tracing-subscriber on the final run.
- Consumer assertions passed for exact internal/external JSON tags and omitted absent fields, optional data, upstream status payload, owned deserialized messages, serialization round-trip, nested Display/Error::source context, preserved context category and all eleven category/status mappings. Logging retained an error's serialized representation and the identity of an Ok boxed value; successful results emitted no events.
- Macro assertions passed for literal/owned-expression/formatted messages, variant/qualified/variable/function code expressions, expression/object/empty data, source/external/pure-external forms, trailing commas supported by the reference, chained modifier orders, cause replacement and single evaluation of a code expression. Deliberately failing Serialize implementations omitted data for expression, object and pure-external forms without panic. Owned object-field values remained usable afterward. Pure external construction produced ExternalError, and all forms produced AppError values rather than Result or an early return.
- Final captured reporting events were exactly ERROR, WARN, DEBUG and TRACE, each with `caller.file="src/main.rs"` and caller lines 138, 141, 143 and 145 respectively. Assertions compared each logged location with its invocation's `file!()`/`line!()`, not the library's reporting line.
- `./target/debug/ship --help`, `./target/debug/ship --version` and `./target/debug/ship` each exited 0 with empty stderr. Help showed only `-h/--help` and `-V/--version`; version was `ship 0.1.0`. A disposable Python socket check observed positive connection refusal at `127.0.0.1:43179` before and after these commands. No server was required or started.
- README commands `cargo run -p ship -- --help` and `cargo run -p ship -- --version` exited 0. Release `./target/release/ship --help` and `./target/release/ship --version` also exited 0 with the same output.
- Ran `cargo fmt --all` to format the authored source, then `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace` and `cargo build --workspace --release`: all exited 0. Repeated after the final macro field-borrow adjustment and consumer run; all passed with no Clippy warnings.
- Inspected production files for future todo/unimplemented/panic stubs, authored test attributes/directories, persistent harnesses, build.rs and OUT_DIR integration: none in active foundation source. `git diff --name-only`/`git status --short --untracked-files=all` showed only authorized planning/foundation paths and the preserved remnants, with no experiment/CI edits.
- Removed the external consumer directory, its Cargo-owned lockfile, dedicated `target/foundation-proof` build output, and temporary metadata/path files. No permanent inspection tooling remains. Standard workspace debug/release build artifacts remain ignored under `/target/`. No commits, pushes or releases were performed.

#### Gaps and stop boundary

All specified foundation exercises were available and passed. Only macOS arm64 with the recorded toolchain was exercised. Centralized requirements for unused later-slice dependencies, including Reqwest/Utoipa, have not been resolved or compiled by this foundation and are not claimed compatible yet. Runtime transport, detachment, daemon cleanup, health DTO/route behavior and schema agreement remain deliberately unverified pending their own slices. No new consequential decision or scope deviation was needed.

### Prior program review record

Program review incorporated:
- Bare `ship` now performs the client workflow; explicit `server` remains.
- Removed the unsafe-code denial and exact dependency pins.
- README is end-user-facing; development instructions/evidence remain here.
- Result extensions use error/warn/debug/trace without a report_ prefix.
- HealthResponse is plain data; server construction and client validation own its behavior.
- Startup is explained as probe, launch, wait and cleanup rather than unexplained public helpers.
- err! is required, but its intended contract still needs confirmation.

The agent-authored client supersedes the build-time Progenitor trial. Future generator options remain documented without being adopted. During implementation, record each surprise, changed assumption, approval question, verification command/outcome and unavailable check here. Do not silently expand the settled product/architecture.

The complete earlier codegen draft is preserved below as historical planning, not current implementation instructions.

<details>
<summary>Superseded program draft: build-time codegen trial</summary>

# Initial scaffolding program

Status: draft, not approved. Audience: mixed. [Product](product.md) and [architecture](architecture.md) are locked; this paper proposes their file-level realization. Approval does not authorize implementation.

## Rationale

One executable composes four crates. Core owns wire identity and contextual errors; server owns the listener and schema; client owns requests; the application owns CLI, output and local-process startup. First establish local checks, then prove the handwritten health exchange, then replace only its request mechanism with generated methods. Startup behavior stays outside generated code.

This is a document-only plan. The code blocks are virtual files containing declarations, not created source, executable placeholders or compilation-verified code. Existing untracked manifests and two core source files are incomplete remnants of an abandoned workflow, not approved interfaces or verified work. Leave them and `design/initial-scaffolding-skeleton` untouched during this review. A later implementation request must reconcile those files against the approved paper, not assume they are finished.

## Skeleton map

**Planned added:**

- Foundation, slice 1: `Cargo.toml`, `Cargo.lock`, `.gitignore`, `README.md`; four crate manifests; `crates/ship/src/main.rs`, `crates/ship/src/cli.rs`, and each library's `src/lib.rs`; `crates/ship-core/src/error.rs`.
- Health, slice 2: `crates/ship-core/src/protocol.rs`; `crates/ship/src/local.rs`, `crates/ship/src/diagnostics.rs`; `crates/ship-client/src/target.rs`, `crates/ship-client/src/health.rs`; `crates/ship-server/src/health.rs`. Extend the foundation files for the running exchange.
- Codegen, slice 3: `crates/ship-server/src/schema.rs`, `crates/ship-client/build.rs`, `crates/ship-client/src/generated.rs`. Extend core's schema derives, server route annotations, client request implementation, manifests and documentation. Cargo creates generated outputs; authors do not create or edit those outputs.

“Added” means absent from the accepted production baseline, not necessarily absent from today's working tree. Every production file change below belongs to one of these three slices. Documentation updates include the evidence for that slice before proceeding.

**Planned moved:** none.

**Planned replaced:** no accepted production implementation. In slice 3 replace the handwritten request inside `crates/ship-client/src/health.rs` with the generated call; do not retain a handwritten fallback or add a generation-skipping feature.

**Planned untouched:** `experiments/terminal-transport/**`, including its independent manifests, patches and vendored code; `.scratch/**`; existing research and unrelated planning; locked `design/product.md` and `design/architecture.md`. This stage writes only this paper. It does not create `proposal.md`, `design.md`, specs, tasks, branches, real configuration or source files, or commits. Attachment to OpenSpec remains a separate post-approval decision because `proposal.md` is missing.

**Virtual configuration files**

`Cargo.toml` (slice 1; health dependencies activated in slice 2; generation dependencies in slice 3):

```toml
[workspace]
members = ["crates/ship", "crates/ship-core", "crates/ship-server", "crates/ship-client"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"

[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
all = "warn"

[profile.dev]
opt-level = 0
debug = "line-tables-only"
incremental = true

[profile.dev.build-override]
opt-level = 3

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"

[profile.test]
inherits = "dev"
incremental = true
codegen-units = 256
```

Dependencies are centralized under `[workspace.dependencies]` and inherited by members. Proposed version constraints: Clap `4` with `derive`; Tokio `1` with `rt-multi-thread`, `macros`, `net`, `signal`, `time`; tracing `0.1`; tracing-subscriber `0.3` with `env-filter`; Serde `1` with `derive`; serde_json `1`; Axum `0.8`; tower-http `0.6` with `trace`; URL `2`; nix `0.31` with `process`; tempfile `3`; Utoipa `5`; Progenitor `=0.15.0` with defaults disabled; Reqwest `=0.13.4` with defaults disabled and `json`, `rustls`. Final generator support dependencies and replacement settings are a blocking inspection item below until primary-source checking is complete.

These are proposed constraints, not a resolved compatible set. Commit the Cargo-produced application lockfile during future implementation. Build and lint verification, not manifest inspection, establishes compatibility. No experiment dependencies, optimization overrides, alternate release profiles, directories/configuration framework, or color-eyre dependency are needed for this plan. The architecture permits retaining foreign errors as strings; choose that smaller representation here.

`Cargo.lock` (slice 1; updated by Cargo in slices 2 and 3):

```text
Cargo-generated application dependency resolution; never manually edited.
```

`.gitignore` (slice 1):

```gitignore
/target/
```

`README.md` (slice 1, extended in slices 2 and 3):

```text
Prerequisites and exercised environment
Local checks
CLI and health workflow
Automatic startup, daemon log location and manual shutdown
Generated-client rebuild and temporary API-change verification
Slice verification record: commands, observed outcomes, unavailable checks
```

Use the available Rust/Cargo 1.98.0 environment as the initial exercise target, with rustfmt and Clippy installed. This is not an established minimum supported Rust version or a cross-platform claim. Record the actual OS/toolchain and resolved versions during implementation. Local checks are `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace`. Also build the release binary at each slice that adds runtime/build behavior. No authored tests, CI files or task-runner dependency.

`crates/ship/Cargo.toml` (slice 1; extended in slice 2):

```toml
[package]
name = "ship"
version.workspace = true
edition.workspace = true

[lints]
workspace = true

# Runtime dependencies inherited from workspace:
# ship-core, ship-client, ship-server, clap, tokio, tracing,
# tracing-subscriber, serde_json; slice 2 adds nix and tempfile.
# One binary, src/main.rs. No separate daemon executable.
```

`crates/ship-core/Cargo.toml` (slice 1; extended in slice 3):

```toml
[package]
name = "ship-core"
version.workspace = true
edition.workspace = true

[lints]
workspace = true

# serde, serde_json, tracing; slice 3 adds utoipa.
```

`crates/ship-server/Cargo.toml` (slice 1; extended in slices 2 and 3):

```toml
[package]
name = "ship-server"
version.workspace = true
edition.workspace = true

[lints]
workspace = true

# ship-core; slice 2 adds axum, tokio, tower-http, tracing.
# Slice 3 adds utoipa. No ship-client or progenitor dependency.
```

`crates/ship-client/Cargo.toml` (slice 1; extended in slices 2 and 3):

```toml
[package]
name = "ship-client"
version.workspace = true
edition.workspace = true

[lints]
workspace = true

# ship-core; slice 2 adds reqwest, url, tokio.
# Slice 3 adds generated-code runtime support, with exact list inspected below.

# Slice 3 only:
[build-dependencies]
ship-server.workspace = true
progenitor.workspace = true
serde_json.workspace = true
```

The client has no runtime dependency on server. Its build dependency invokes server schema construction in the host process, not a listener. Build dependencies do not imply installed-binary startup work. Foundation creates real minimum crate modules only; the full declarations below enter with the named behavior slice, not as todo/panic stubs.

**Virtual shared files**

`crates/ship-core/src/lib.rs` (slice 1; protocol exports in slice 2):

```rust
pub mod error;
pub mod protocol; // Slice 2.

pub use error::{AppError, ErrorCode, ExternalError, InternalError, Result, ResultExt};
pub use protocol::{DEFAULT_PORT, DEFAULT_SERVER_URL, HEALTH_PATH,
                   HEALTH_OPERATION_ID, PROTOCOL_VERSION, HealthResponse};
```

`crates/ship-core/src/error.rs` (slice 1):

```rust
use std::borrow::Cow;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum ErrorCode {
    Validation, NotFound, Conflict, Configuration, Network, RateLimited,
    UpstreamHttpStatus(u16), Internal, Serialization, Unauthorized, Io,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum AppError { Internal(InternalError), External(ExternalError) }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InternalError {
    pub message: Cow<'static, str>,
    #[serde(rename = "type")]
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caused_by: Option<Box<AppError>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalError {
    pub code: ErrorCode,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl AppError {
    pub fn internal(code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self;
    pub fn external(code: ErrorCode, error: impl std::fmt::Display) -> Self;
    pub fn code(&self) -> &ErrorCode;
    pub fn with_data(self, data: Value) -> Self;
    pub fn with_cause(self, cause: AppError) -> Self;
    pub fn context(self, message: impl Into<Cow<'static, str>>) -> Self;
}

impl std::fmt::Display for AppError; // Human-readable context/cause chain.
impl std::error::Error for AppError; // Internal cause exposed as source.

pub trait ResultExt<T>: Sized {
    fn context(self, message: impl Into<Cow<'static, str>>) -> Result<T>;
    #[track_caller] fn report_error(self) -> Result<T>;
    #[track_caller] fn report_warn(self) -> Result<T>;
    #[track_caller] fn report_debug(self) -> Result<T>;
    #[track_caller] fn report_trace(self) -> Result<T>;
}
```

`ResultExt` applies to shared `Result<T>`. Library boundary conversions remain explicit, supplying the category before adding context. Reporting returns the unchanged result and records caller file/line; callers report a failure once at the chosen boundary. Context copies the underlying category into its internal wrapper. All optional fields omit `None`; deserialization owns messages where needed. External text is a diagnostic snapshot, not an HTTP response contract. `ErrorCode::http_status(&self) -> u16` supplies the explicit mapping below without depending on an HTTP crate.

Export macro call forms, defined here in slice 1: `err!(code, "message {}", value)`, `err!(code, data = json_value, "message {}", value)`, `err!(code, cause = app_error, "message {}", value)`, and a combined `data = ..., cause = ...` form in that order. `bail!` accepts the same forms and returns `Err`; `ensure!(condition, code, ...)` uses the same construction arguments when false. Use `$crate` paths for hygiene. Do not add a second error framework or blanket foreign-error conversions that guess categories.

Explicit category-to-HTTP-status helper: validation 400, not found 404, conflict 409, configuration/internal/serialization/I/O 500, network 502, rate limited 429, upstream HTTP status 502, unauthorized 401. Upstream status is preserved in diagnostic data, not blindly returned. Server owns any HTTP error body and whether to use this mapping; this health route returns only its success DTO. Do not expose AppError internals as a new public API.

`crates/ship-core/src/protocol.rs` (slice 2; `ToSchema` added in slice 3):

```rust
use serde::{Deserialize, Serialize};
use utoipa::ToSchema; // Slice 3 only.

pub const DEFAULT_PORT: u16 = 43179;
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:43179";
pub const HEALTH_PATH: &str = "/health";
pub const HEALTH_OPERATION_ID: &str = "health"; // Slice 3.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub service: String,
    pub protocol_version: u32,
    pub version: String,
}

impl HealthResponse {
    pub fn current() -> Self;
    pub fn validate_identity(&self) -> crate::Result<()>;
}
```

`current` uses service `ship`, protocol version 1, and the shared package version, initially `0.1.0`. Compatibility requires exactly service `ship` and protocol version 1; package version is informational. GET `/health` returns HTTP 200 with this flat JSON object. Neither TCP success nor other HTTP 200 responses count as compatible health. Identity is accidental-mismatch detection, not authentication.

**Virtual application files**

`crates/ship/src/cli.rs` (slice 1 parser/help shell; supported operations enter in slice 2):

```rust
use clap::{Parser, Subcommand};
use ship_core::DEFAULT_PORT;

#[derive(Parser)]
pub struct Cli {
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Server {
        #[arg(long, default_value_t = DEFAULT_PORT)]
        port: u16,
        #[arg(long, hide = true)]
        background_child: bool,
    },
    Client {
        #[arg(long)]
        server_url: Option<String>,
    },
}
```

Public commands are `ship server [--port PORT]` and `ship client [--server-url URL]`, plus standard help/version. The client command performs the single health operation; no redundant `health` subcommand. An omitted URL means default-local startup; an explicitly supplied URL, even the default URL, means connect-only. Port zero is rejected rather than selecting an ephemeral listener. Server always binds `127.0.0.1`, with no host flag. Help must say that automatic startup leaves a background server running. `--background-child` is an internal launch marker, not a management command.

Slice 1 provides working top-level help/version and minimum crate composition. Slice 2 adds these operational command variants and their help before its acceptance exercises. No invocation claims an exchange works during foundation. The final documented help exposes both commands without requiring a running server.

`crates/ship/src/main.rs` (slice 1; dispatch in slice 2):

```rust
mod cli;
mod diagnostics; // Slice 2.
mod local;       // Slice 2.

use std::process::ExitCode;
use ship_core::Result;

fn main() -> ExitCode;
async fn dispatch(cli: cli::Cli) -> Result<()>;
```

Keep `main` synchronous so a marked child can establish its session before constructing a multithreaded Tokio runtime. Then initialize tracing once and dispatch. Foreground server supplies a signal-wait future to server `serve`. Client resolves its mode from the presence of `--server-url`, obtains health once through the selected flow, and prints the serialized `HealthResponse` as one JSON line on stdout. Diagnostics go to stderr or the child log. Help/version exit 0; successful commands exit 0; operational failures exit 1; Clap's usage errors retain its exit 2. Errors include safe target context; no success output on failure.

`crates/ship/src/diagnostics.rs` (slice 2):

```rust
use ship_core::Result;

pub fn init() -> Result<()>;
pub async fn shutdown_signal() -> Result<()>;
```

Tracing-subscriber writes stderr, defaults to `info`, accepts `RUST_LOG`, and disables ANSI when stderr is not a terminal. Reject an invalid filter with configuration context instead of silently ignoring it. SIGINT and SIGTERM both request graceful shutdown. A signal-registration failure is an error, not silent indefinite operation. Server drains active health requests on shutdown, bounded by five seconds, then closes/exits. No shutdown endpoint, PID registry or supervisor.

`crates/ship/src/local.rs` (slice 2):

```rust
use std::path::PathBuf;
use std::process::Child;
use ship_client::HealthClient;
use ship_core::{HealthResponse, Result};

pub async fn default_health(client: &HealthClient) -> Result<HealthResponse>;
pub fn detach_child_session() -> Result<()>;

struct LaunchAttempt {
    child: Child,
    log_path: PathBuf,
}

fn launch_server() -> Result<LaunchAttempt>;
async fn await_ready(client: &HealthClient, attempt: &mut LaunchAttempt)
    -> Result<HealthResponse>;
```

Resolve `std::env::current_exe()` and launch that path directly with arguments `server --port 43179 --background-child`; no shell and no `cargo run` child. Redirect stdin from null and both output handles to the same private log file before spawn. In the child, call safe `nix::unistd::setsid()` at synchronous entry, before Tokio/tracing/listener startup. Do not set the child's process group beforehand, use `pre_exec`, fork a multithreaded runtime, or introduce unsafe blocks. A normal spawned child initially shares its parent's group; it is not that group's leader and can create its own session. A session-establishment failure exits before listening. Manual direct use of the hidden marker is not a supported launch workflow.

Use tempfile to create a unique mode-0600 file in the OS temporary directory, named with prefix `ship-server-` and suffix `.log`, and keep it after startup. This avoids a shared predictable writable log name or a new application-directory policy. Require the file to remain private; do not reuse an existing file or loosen permissions. Child output uses cloned handles referring to that file. Report the absolute path and child PID when launch succeeds, and the path with startup failures; if private-log creation itself fails, report the attempted temporary directory and say that no log was created. Log retention follows OS temporary-file cleanup and manual developer cleanup. No PID file or log rotation. Installed startup never invokes a generator.

Default-local algorithm: first probe absence/health. Compatible health returns immediately without spawning. Only a structured `ConnectionRefused` from the default loopback TCP connect qualifies as absent. A successful TCP connect must be followed by health validation; timeout, other connect errors, bad status/JSON, wrong identity/version and occupied/unhealthy listener are errors. The preliminary TCP connection is dropped, not interpreted as server identity.

Use a one-second TCP-connect bound and a two-second total health-request bound, covering connection and body read. After launch, use an absolute five-second readiness deadline and a 100 ms interval. Each attempt is capped by the remaining deadline; do not let the last request extend the deadline. Within this readiness window, connection refusal means not ready yet. Other health failures stop with context. Check `Child::try_wait` between attempts; if the child exits, make one bounded final health probe so a concurrent compatible winner can be accepted, otherwise report status and log path. A losing bind attempt must exit; it must not persist as an idle daemon.

On deadline/failure, terminate and reap only this attempt's still-live child, then report failure. Do not kill any pre-existing listener or a concurrent winning server. Killing the owned child is cleanup of a failed launch, not the normal server shutdown path. On compatible readiness, leave a still-running child detached and do not kill it when the client exits. Ten simultaneous starts may produce bounded errors for losing attempts, but must leave at most one compatible listener and no surviving losing children.

**Virtual client files**

`crates/ship-client/src/lib.rs` (slice 1; exports in slice 2):

```rust
mod target;
mod health;
mod generated; // Slice 3 only, private generated implementation boundary.

pub use target::ServerTarget;
pub use health::{HealthClient, LocalProbe};
```

`crates/ship-client/src/target.rs` (slice 2):

```rust
use url::Url;
use ship_core::Result;

#[derive(Clone, Debug)]
pub struct ServerTarget { /* Private URL storage; Debug redacts credentials. */ }

impl ServerTarget {
    pub fn default_local() -> Self;
    pub fn parse(input: &str) -> Result<Self>;
    pub fn base_url(&self) -> &Url;
    pub fn health_url(&self) -> Result<Url>;
    pub fn display_safe(&self) -> String;
}
```

Accept absolute HTTP/HTTPS URLs with a host and optional port. Normalize the stored base URL to its root path and remove query/fragment before either request implementation uses it. Resolve the root-relative `HEALTH_PATH`, so a supplied base path is not a health-route prefix. Give the generator this same normalized root base, without a trailing slash when its constructor requires that spelling. Do not interpolate credentials into diagnostics; sanitize userinfo before formatting any target. Parsing errors must not echo raw credential-bearing input. Foreign HTTP errors must have embedded URLs removed/redacted before storing their text in AppError; a separately attached target is sanitized. No insecure TLS option or custom certificate bypass. Custom request failure never calls the local-launch module.

`crates/ship-client/src/health.rs` (slice 2; request internals replaced in slice 3):

```rust
use ship_core::{HealthResponse, Result};
use crate::ServerTarget;

pub struct HealthClient { /* Private Reqwest client and selected target. */ }

pub enum LocalProbe {
    Absent,
    Ready(HealthResponse),
}

impl HealthClient {
    pub fn new(target: ServerTarget) -> Result<Self>;
    pub fn target(&self) -> &ServerTarget;
    pub async fn health(&self) -> Result<HealthResponse>;
    pub async fn probe_default(&self, remaining: std::time::Duration)
        -> Result<LocalProbe>;
}
```

`probe_default` accepts only the default-local target; other targets return a configuration error. It uses Tokio TCP connect to classify absence through `std::io::ErrorKind::ConnectionRefused`, not message strings or a Reqwest error guess. Once connected, request and validate health. Cap TCP and health waits by the supplied remaining budget. All outcomes other than positive absence or compatible health are errors.

Reqwest redirect policy is `none`; any redirect is a request failure, not a new target. Check exactly HTTP 200, deserialize the shared DTO, then validate identity. Foreign conversions distinguish network/timeout, upstream status and serialization through structured library information. The handwritten slice performs GET directly. The generated slice uses the same configured Reqwest client and preserves bounds, status and identity checks around the generated method. No global future-streaming timeout/retry policy, middleware additions or handwritten final-client branch.

**Virtual server files**

`crates/ship-server/src/lib.rs` (slice 1; runtime in slice 2; schema export in slice 3):

```rust
mod health;
mod schema; // Slice 3.

use std::future::Future;
use std::net::SocketAddr;
use ship_core::Result;

pub fn loopback_addr(port: u16) -> Result<SocketAddr>;
pub fn router() -> axum::Router;
pub async fn serve(
    address: SocketAddr,
    shutdown: impl Future<Output = Result<()>> + Send + 'static,
) -> Result<()>;
pub fn openapi() -> utoipa::openapi::OpenApi; // Slice 3.
```

Reject non-loopback addresses at the exported `serve` boundary as well as restricting CLI construction. Bind failures use structured I/O information; `AddrInUse` maps to conflict. Startup diagnostics identify the bound address, process ID and interface version. Tower HTTP TraceLayer logs request method/path and response status/duration at useful default levels; do not rely on its debug-only defaults while using an info filter. No generic request bodies or headers in logs. Schema construction does not initialize tracing, bind sockets, spawn tasks or require an async runtime.

`crates/ship-server/src/health.rs` (slice 2; route annotation in slice 3):

```rust
use axum::Json;
use ship_core::HealthResponse;

// Slice 3 annotation: GET /health, operation_id = "health",
// response 200 body = HealthResponse, application/json.
pub(crate) async fn health() -> Json<HealthResponse>;
```

The router uses the shared health path. If Utoipa requires literal attribute values, keep those literals adjacent to the route declaration and inspect their agreement with shared constants during slice 3. Do not assume an attribute accepts const expressions. No listener metadata endpoint or OpenAPI HTTP route.

**Virtual generation files**

`crates/ship-server/src/schema.rs` (slice 3):

```rust
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(crate::health::health), components(schemas(ship_core::HealthResponse)))]
struct Api;

pub(crate) fn document() -> utoipa::openapi::OpenApi;
```

`crates/ship-client/build.rs` (slice 3):

```rust
fn main() -> Result<(), Box<dyn std::error::Error>>;
```

Build-script sequence, not executable code in this paper: call `ship_server::openapi()`, serialize its OpenAPI document, parse the schema type required by Progenitor, configure explicit replacement of `HealthResponse` with `ship_core::HealthResponse`, generate request methods, and write `openapi.json` plus `client.rs` under `OUT_DIR`. Do not read a checked-in schema or call a running server. A failed export/generation/write is a failed Cargo build with useful context, not a warning followed by an alternate client.

`crates/ship-client/src/generated.rs` (slice 3):

```rust
// Include OUT_DIR/client.rs inside this private module.
// Expected generated seam: Client with configured Reqwest client;
// health() returning a response wrapper over ship_core::HealthResponse.
```

Do not claim that this expected seam is a verified generated signature yet. Scope any necessary generated-code lint allowance to this private module; handwritten crates still use workspace lints and warnings-as-errors. Start with no blanket lint exceptions. If generated output emits warnings, inspect their precise lint names and use only those allowances. Do not manually modify output or apply crate-wide `allow(warnings)`.

**Decisions and blocking unknowns**

Endpoint/CLI spelling, health fields, fixed bounds, direct same-executable launch, safe child-entry detachment, private temporary log handling, error text retention, profiles and local checks are proposals within the locked architecture. No additional product capability is intended. Linux/macOS are the intended Unix mechanism; only the environment actually exercised can be claimed verified. A non-Unix implementation is not included or silently invented.

Before paper approval, resolve by primary-source inspection: Progenitor 0.15.0 replacement settings, generated method naming/signatures, runtime support dependencies and input-tracking behavior. Version pins and snippets remain inspection-only until future slice checks prove a compatible resolution. If any source fact invalidates the selected architecture, stop for Cyan rather than substitute a tool or generation path.

Before implementation completion, runtime/build verification must establish detachment, file privacy, child cleanup, concurrent-start behavior and generated shared-type correctness. These are future acceptance gates, not evidence already obtained during program design.

## Build order

1. **Foundation and local checks.** Create/reconcile the workspace and four manifests, root ignore file, Cargo-produced lockfile, README, application `main.rs`/`cli.rs`, minimum library roots and core `error.rs`. Add only foundation dependencies initially. Deliver a buildable four-crate executable with genuine help/version and shared contextual error construction/reporting; operational command behavior enters slice 2. Run formatting, restrained Clippy, workspace debug and release builds; exercise `ship --help` and `ship --version` without a server. Inspect serialization attributes, macro hygiene, dependency direction and absence of test/CI/terminal code. Record commands, environment and outcomes in README before starting slice 2. This slice does not claim a working HTTP operation or codegen.

2. **Handwritten health and process lifecycle.** Add core protocol, application local/diagnostics, client target/health and server health files; extend library roots, CLI/dispatch, manifests, lockfile and README. No schema generation/build script yet. Deliver explicit loopback serving, default-local automatic launch/reuse with server survival, custom-target connect-only health, safe diagnostic context, request traces and signal-driven shutdown.

   Run all local checks and build the real binary first. Exercise `ship server --help` and `ship client --help`. With the default port free, run `ship client`, observe health JSON plus daemon PID/log, let the client exit, and run it again: same listener/PID, no second launch. Inspect daemon diagnostics for GET `/health` and HTTP 200. From a separate launching terminal, start via client and close that terminal; confirm server survival from another terminal. Check log mode 0600 and independent Unix session using platform-appropriate process inspection.

   Send SIGTERM to the recorded daemon PID and verify clean exit plus refusal on its former port. Run explicit `ship server`, check its stderr startup/request diagnostics and health via the client, then SIGINT and verify shutdown. Run explicit `ship server --port 43180` and `ship client --server-url http://127.0.0.1:43180`; verify success without launching the default server. With both servers stopped, explicitly supply an unreachable custom port, then explicitly supply the default URL: both fail with target context and neither auto-starts. Restore a free default port.

   Occupy the default port with a controlled non-Ship listener; verify incompatible/malformed health fails without launch or replacement. With no default listener, invoke the client with `TMPDIR` pointing at a newly created empty directory whose write permission has been removed for this exercise. Confirm an actionable startup/log-creation error with the attempted directory, no claimed log file and no daemon; restore directory permissions and remove that temporary directory afterward. Run as the ordinary development user, not root. This verifies automatic-startup failure without altering the installed binary or adding a production failure-injection flag. Also exercise a bounded readiness failure by pausing the just-spawned child before listener readiness with a debugger or signal, checking owned-child cleanup and preserved log. If the platform cannot reliably perform that pause, record the gap rather than claim it passed.

   With the default port free, invoke ten clients concurrently. Observe at most one serving daemon, no surviving losing children, and only success or bounded contextual errors. Collect stdout/stderr/PIDs, then normally stop the remaining server. Do not infer convergence solely from ten successful exit codes. Restore the environment and repeat local checks. Record all outcomes before slice 3; unavailable checks stay explicitly unverified.

3. **Build-time codegen trial.** Add server schema, client build script and private generated inclusion module; update route/schema annotations, core derives, client request internals, dependency manifests, lockfile and README. Deliver the same health behavior through Progenitor, exact shared DTO identity, no runtime server dependency in the client, no separate tool and no checked-in generated files.

   Run the normal workspace build, inspect OUT_DIR schema/methods and shared-type replacement, then call health against both explicit and auto-started servers through the real binary. Recheck custom-target no-fallback, redirect refusal, bounded failures and baseline lifecycle behavior after the request swap. Run formatting, Clippy and debug/release builds. Evidence must identify the generated call site and the actual returned core type, not just the existence of generated files.

   Temporarily change a health DTO field/type and update the server's construction while leaving a deliberate incompatible handwritten caller use. On a normal build, inspect the exported schema for the new wire shape and observe compiler feedback at the caller. Separately rename the health operation identifier to `health_changed` while keeping the old generated-method call. Observe a rerun of the client build script, a changed generated method interface, and compilation failure at the old method call. Capture output content and Cargo's rebuild diagnostics; a shared-type compile failure or output timestamp alone does not prove regeneration.

   Reconcile both callers, build and perform the real generated request against the correspondingly rebuilt server. Stop the old daemon before expecting it to serve the changed schema. Restore all authored files to the intended baseline, rebuild normally, inspect the restored schema/method interface, run the final health exchange and local checks, and record evidence. Never edit generated output. A failed generator/shared-type/rebuild scenario stops for Cyan with failure evidence; slice 2 remains a previous working checkpoint, not fulfillment of the final product goal.

## Deviation log

Empty. Later implementation records each surprise, the question asked or assumption made, and the papers affected. Changes to locked product/architecture reopen downstream papers under the ripple rule; do not silently edit them or continue implementation through an unresolved decision.

</details>
