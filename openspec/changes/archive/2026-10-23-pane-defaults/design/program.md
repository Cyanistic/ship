# Program: pane defaults and exit behavior

Status: Locked on 2026-10-21, approved in Plannotator review ("LGTM"), including the four choices made while drafting. Written on 2026-10-21 from the locked [product](product.md) and [architecture](architecture.md) papers, against `main` at `9c76336`. The `pane-defaults` branch sits at `7ac103e`, 14 commits behind, and is rebased onto `main` before slice 1. Those commits touch `pane.rs`, `lib.rs` and `main.rs` only lightly, and nothing here depends on them. Locking doesn't authorize implementation: each slice runs on Cyan's request.

Summary: five slices. The first adds the two pane fields and the `[server.defaults.pane]` table, which delivers P4 and P6 and the create flags. The second adds the `every` feed, so waits survive a pane closing. The third makes panes close on exit and lets `ship pane set` change the exit behavior (P1 to P3, part of P5). The fourth applies a changed history size to a live pane. The fifth covers docs and specs. Four choices made while drafting are marked **Choice** below and need Cyan's eye. None of them blocks.

## Rationale

This is a document-only plan. The skeleton below names real paths and shows declarations, but no source file has been created or changed for it, and nothing here has been compiled.

The skeleton follows the architecture's structure box by box:

- **ship-core model and defaults** (`model.rs`, `defaults.toml`, `config.rs`). They own `ExitBehavior`, the two new `Pane` fields, the rule for whether an exit closes the pane, and the built-in pane defaults.
- **ship-core protocol** (`protocol.rs`). It owns the wire changes: optional overrides on `PaneInput`, two new `SetPane` fields, `AttachRequest.feed`, `EndReason::Lost` and `PROTOCOL_VERSION` 15.
- **ship-core command** (`command.rs`). It owns the CLI flags and key-binding action fields for `pane create` and `pane set`. Both read the same structs.
- **ship-server settings** (`settings.rs`). It reads `[server.defaults.pane]` and rejects the old `[server] shell` and `[server] scrollback_bytes`.
- **ship-server state** (`state.rs`). It resolves a new pane's settings against the defaults when the pane is created, commits exit and then close, and applies `pane set`.
- **ship-server pane runtime** (`pane.rs`). It takes the shell and history size from `Launch` instead of reading the file itself, and forwards a history change to the session.
- **ship-server replicas** (new `replicas.rs`). It owns the `watch` plus ring pair, and `ReplicaSink`. `attach.rs` picks a `Feed` per attachment.
- **vendored session** (`vendor/ratatui-ghostty/src/session.rs`). One new command changes the history limit of a live terminal.
- **ship-client** (`automation.rs`, `execute.rs`, `ui/mod.rs`). Waits ask for `every` and read `Lost`. Create and set pass the new fields through.

## Skeleton map

### Added

**`crates/ship-server/src/replicas.rs`, slice 2.** Architecture decisions 1, 3 and 6.

```rust
//! Published replicas: the latest for most readers, and a ring of recent ones
//! for streams that must see every revision.

use std::{ops::ControlFlow, sync::Arc};

use ship_core::{protocol::Replica, relay::{SendResult, Sink}};
use tokio::sync::{broadcast, watch};

/// Replicas an `every` stream may fall behind before it ends with `lost`.
pub(crate) const RING: usize = 256;

/// In `AppState`. Cloning is cheap.
#[derive(Clone)]
pub struct Replicas {
    latest: watch::Receiver<Arc<Replica>>,
    ring: broadcast::Sender<Arc<Replica>>,
}

impl Replicas {
    /// The handle and the relay bus's sink for it.
    pub(crate) fn new(first: Arc<Replica>) -> (Self, ReplicaSink);

    /// Saver, input routing, `latest` attach streams, startup baseline.
    pub fn latest(&self) -> watch::Receiver<Arc<Replica>>;

    /// An `every` stream. Subscribe before the attach commit.
    pub(crate) fn every(&self) -> broadcast::Receiver<Arc<Replica>>;
}

/// Subscribed on the relay bus in place of the bare `watch::Sender`.
pub(crate) struct ReplicaSink {
    latest: watch::Sender<Arc<Replica>>,
    ring: broadcast::Sender<Arc<Replica>>,
}

impl Sink<Arc<Replica>> for ReplicaSink {
    /// Ring first, ignoring its no-receiver error. Then `latest`, whose result
    /// alone decides `Closed`.
    fn try_send(&mut self, value: Arc<Replica>) -> ControlFlow<SendResult, SendResult>;
}
```

**OpenSpec artifacts, when this paper locks.** These are documents, not implementation: `proposal.md`, `design.md` (a summary that points at the three papers), the delta specs listed in slice 5, and `tasks.md` (these slices).

### Moved

None.

### Replaced

**`crates/ship-core/src/model.rs`, slice 1, with `closes` used in slice 3.**

```rust
/// What happens to a pane when its program exits. Shared state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum ExitBehavior {
    #[default]
    Close,
    Keep,
    KeepFailed,
}

impl ExitBehavior {
    /// `Close` always. `KeepFailed` only for code 0 with no signal. `Keep` never.
    pub fn closes(self, status: &ExitStatus) -> bool;
}

pub struct Pane {
    // existing fields unchanged, then after right_click_passthrough:
    /// Set at creation from `[server.defaults.pane]` unless given. Saved files
    /// from before this change get the built-in default.
    #[serde(default)]
    pub exit_behavior: ExitBehavior,         // "exitBehavior"
    #[serde(default = "config::builtin_scrollback_bytes")]
    pub scrollback_bytes: usize,             // "scrollbackBytes"
}
```

**`crates/ship-core/src/config.rs`, slice 1.** `PaneDefaults` lives here, not in ship-server, because `Pane`'s serde default needs the built-in history size (see Choice 2).

```rust
/// `[server.defaults.pane]`: what a new pane starts with unless its create
/// request says otherwise.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneDefaults {
    /// Absent: the login shell. Relative paths resolve from the config file's folder.
    pub shell: Option<PathBuf>,
    pub scrollback_bytes: usize,
    pub right_click_passthrough: bool,
    pub exit_behavior: ExitBehavior,
}

/// `defaults.toml`'s pane history size, for saved panes that lack one.
pub fn builtin_scrollback_bytes() -> usize;
```

**`crates/ship-core/src/defaults.toml`, slice 1.** The `[server]` table becomes:

```toml
[server.defaults.pane]
# shell = "/bin/zsh"                 # path, relative to the config file; unset: your login shell
scrollback_bytes = 10_000_000        # bytes of history; panes created afterwards start with this
right_click_passthrough = false      # bool; give right-clicks to programs that track the mouse
exit_behavior = "close"              # "close" | "keep" | "keep_failed"
```

**`crates/ship-server/src/settings.rs`, slice 1.** It keeps its role and loses the two old fields.

```rust
/// `[server]`. Read whenever a pane is created, never cached.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSettings {
    pub defaults: ServerDefaults,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerDefaults {
    pub pane: PaneDefaults,
}

impl ServerSettings {
    /// Unchanged contract. A relative `defaults.pane.shell` resolves from the file's folder.
    pub fn load(path: &Path) -> Result<Self>;
}
```

`deny_unknown_fields` on `ServerSettings` is what makes `ship config check` report a leftover `[server] shell` with its line and column (P4). toml's error already carries both.

**`crates/ship-core/src/protocol.rs`, slices 1 to 3.**

```rust
pub const PROTOCOL_VERSION: u32 = 15;                       // slice 1

/// POST /panes input. Absent fields take `[server.defaults.pane]`.
pub struct PaneInput {                                      // slice 1
    pub name: OptionalName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_click_passthrough: Option<bool>,              // was bool
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_behavior: Option<ExitBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrollback_bytes: Option<usize>,
    #[serde(flatten)]
    pub spec: PaneSpec,
}

pub struct SetPane {                                        // slices 3 and 4
    pub name: Option<OptionalName>,
    pub right_click_passthrough: Option<Switch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_behavior: Option<ExitBehavior>,                // slice 3
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrollback_bytes: Option<usize>,                    // slice 4
}

pub struct AttachRequest {                                  // slice 2
    pub selection: Option<NodeId>,
    pub area: Option<Size>,
    #[serde(default, skip_serializing_if = "FeedMode::is_latest")]
    pub feed: FeedMode,
}

/// Which replicas an attach stream sends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeedMode {
    /// The newest replica only. Revisions in between may be skipped.
    #[default]
    Latest,
    /// Every revision after the seed, in order. Ends with `lost` when the
    /// stream falls more than the ring's capacity behind.
    Every,
}

pub enum EndReason {                                        // slice 2
    ServerShutdown,
    /// An `every` stream fell behind the server's ring.
    Lost,
}
```

**`crates/ship-core/src/command.rs`, slices 1, 3 and 4.** These structs are both the CLI arguments and the `server.pane.*` key-binding actions, so one change covers FR-006 and FR-007.

```rust
pub struct Create {                                         // slice 1
    // tab, direction, name, cwd, command, select, attachment unchanged
    /// Give right-clicks on contents to a program that tracks the mouse;
    /// defaults to the config's setting
    #[cfg_attr(feature = "clap", arg(long, num_args = 0..=1, require_equals = true,
        default_missing_value = "true", value_name = "true|false"))]
    pub right_click_passthrough: Option<bool>,              // was bool, see Choice 1
    /// What happens when the program exits; defaults to the config's setting
    #[cfg_attr(feature = "clap", arg(long, value_name = "close|keep|keep_failed"))]
    pub exit_behavior: Option<ExitBehavior>,
    /// Bytes of history to keep; defaults to the config's setting
    #[cfg_attr(feature = "clap", arg(long, value_name = "BYTES"))]
    pub scrollback_bytes: Option<usize>,
}

pub struct Set {
    // pane, name, right_click_passthrough unchanged
    /// What happens when the program exits. On an exited pane, only stored
    #[cfg_attr(feature = "clap", arg(long, value_name = "close|keep|keep_failed"))]
    pub exit_behavior: Option<ExitBehavior>,                // slice 3
    /// Bytes of history to keep. Raising keeps history; lowering drops the
    /// oldest beyond the limit at once
    #[cfg_attr(feature = "clap", arg(long, value_name = "BYTES"))]
    pub scrollback_bytes: Option<usize>,                    // slice 4
}
```

**`crates/ship-server/src/pane.rs`, slices 1 and 4.** `spawn` stops reading the config file. The state actor reads it once per create and passes the resolved values in.

```rust
pub(crate) struct Launch {                                  // slice 1
    pub pane: IdOf<Pane>,
    pub command: Option<Vec<String>>,
    pub cwd: PathBuf,
    pub size: Size,
    /// From `[server.defaults.pane]`; only used when `command` is `None`.
    pub shell: Option<PathBuf>,
    pub scrollback_bytes: usize,
}

impl PaneEnv {
    /// Was private and called from `spawn`. A bad file gives the built-in
    /// defaults and logs the error (FR-005).
    pub(crate) fn settings(&self) -> ServerSettings;        // slice 1
}

pub(crate) enum PaneCommand {
    // existing variants unchanged
    /// New history limit; the session prunes at once when it's lower.
    Scrollback(usize),                                      // slice 4
}
```

**`crates/ship-server/src/state.rs`, slices 1, 3 and 4.**

```rust
impl ServerState {
    /// Reads `pane_env.settings()` once and fills each absent `PaneInput`
    /// field from `defaults.pane`. The resolved shell and history size go
    /// into `Launch`. The resolved passthrough, exit behavior and history
    /// size go into the returned `Pane`.
    fn start_pane(&mut self, id: IdOf<Pane>, size: Size, input: PaneInput) -> Result<Pane>; // slice 1

    /// Restore passes the saved pane's three settings as `Some`, so a
    /// restored pane keeps them (FR-010).
    fn restore_tab(&mut self, tab: Tab) -> Tab;             // slice 1
}

impl Message<PaneEvent> for ServerState {
    /// `Exited(status)`: commit the status with program records cleared, as
    /// today. Then, in the same handler, if `pane.exit_behavior.closes(&status)`,
    /// commit `layout::close` exactly as `Remove<Pane>` does. Two revisions,
    /// exit first (architecture decision 5).
    async fn handle(..);                                    // slice 3
}

impl Message<Set<Pane, SetPane>> for ServerState {
    /// Also applies `exit_behavior` (stored only, also on an exited pane) and
    /// `scrollback_bytes` (stored, and sent to the runtime as
    /// `PaneCommand::Scrollback` when it changed). Commits only on a change, as today.
    async fn handle(..);                                    // slices 3 and 4
}
```

**`crates/ship-server/src/app.rs`, `lib.rs`, `attach.rs`, `input.rs`, slice 2.**

```rust
// app.rs
pub struct AppState {
    pub replicas: Replicas,                                 // was watch::Receiver<Arc<Replica>>
    // other fields unchanged
}

// lib.rs: `Replicas::new(state.replica())` replaces `watch::channel`. The bus
// subscribes the `ReplicaSink`. The baseline reads `replicas.latest().borrow()`,
// and the saver runs on `replicas.latest()`.

// input.rs: `app.replicas.latest()` in place of `app.replicas.clone()`.

// attach.rs
struct Progress {
    replicas: Feed,                                         // was watch::Receiver<Arc<Replica>>
    // other fields unchanged
}

enum Feed {
    Latest(watch::Receiver<Arc<Replica>>),
    Every(broadcast::Receiver<Arc<Replica>>),
}

// attach(): the Feed is built from `request.feed` before `state.ask(Attach ..)`.
// Progress::next, Every arm: Ok(r) newer than the seed => State(r); older or
// equal => skip; Lagged => Err(EndReason::Lost); Closed => Err(ServerShutdown).
```

**`vendor/ratatui-ghostty/src/session.rs`, slice 4.** Architecture decision 8.

```rust
enum SessionCommand {
    // existing variants unchanged
    ScrollbackBytes(usize),
}

impl SessionHandle {
    /// Calls `Terminal::set_scrollback_max_bytes(Some(bytes))` on the session
    /// thread. Lowering prunes the oldest history at once. A failure arrives
    /// as `SessionEvent::Error`, like other terminal errors.
    pub fn set_scrollback_bytes(&self, bytes: usize);
}
```

**`crates/ship-client/src/automation.rs`, slice 2.**

```rust
/// An observer on the `every` feed: no selection, no area.
async fn observe(&self) -> Result<impl Stream<Item = Result<SseEvent>> + use<>>;

/// `Ended(Lost)` fails with "fell behind the server while waiting on pane P".
/// `Ended(ServerShutdown)` keeps today's message.
async fn next(..) -> Result<Arc<Replica>>;
```

`wait`, `watch`, `send_and_wait` and `Condition` don't change. On a pane that closes on exit, the waiter now sees the `Exited` revision first. `Condition::matches` already returns the pane for `--status exited`, and fails with the exit status otherwise.

**`crates/ship-client/src/execute.rs`, slices 1, 3 and 4.** Pass the new `Create` and `Set` fields into `PaneInput` and `SetPane` as they are. No defaults are applied on the client.

**`crates/ship-client/src/ui/mod.rs`, slice 2.** The UI never asks for `every`, so it never receives `Lost`. The exhaustive match at the `Ended` arm gets `EndReason::Lost`, handled like a dropped stream: reattach with backoff (see Choice 4).

**`README.md`, `docs/guide.md`, slice 5.** README: remove "Closing a pane automatically when its program exits." from "Not built yet", and move the config example's `shell = ...` from `[server]` to `[server.defaults.pane]`. Guide: rewrite "Server settings" around the defaults table, describe exit behavior with P2's agent workflow under "Scripting Ship", and document per-pane history size and what lowering it does.

### Untouched

- `crates/ship-server/src/saver.rs`. Saved files serialize `Pane` whole, so the new fields are saved without changes. It takes `replicas.latest()` from `lib.rs`.
- `crates/ship-client/src/ui` apart from the one match arm. Mouse handling still reads `pane.right_click_passthrough`, and the UI attach stays on `latest`.
- `crates/ship-server/src/views.rs` and the tab listing. They serialize `Pane`, so `exitBehavior` and `scrollbackBytes` show up in every view (FR-009) for free.
- `CreateTab` and `--starter`. The starter pane's `PaneInput` has every field absent, so it takes the defaults (product non-goal).
- Selection repair. A close on exit goes through `commit()` like `ship pane close`, so the neighbor and selection behave the same (P1).

### Proposed settings

The `[server.defaults.pane]` table shown under `defaults.toml`. No other configuration.

### Choices made while drafting

1. **`pane create --right-click-passthrough` takes an optional value.** It used to be a bare flag that could only turn passthrough on. Now that the default comes from config and can be `true`, a create needs a way to say `false`. The plan is `--right-click-passthrough` (meaning true) or `--right-click-passthrough=false`. The key-binding action takes `true` or `false`. Alternative: a separate `--no-right-click-passthrough` flag.
2. **Old saved files get the built-in defaults, not the config file's.** Architecture decision 9 says serde defaults fill the missing fields. Serde can't read the config file, so a pane saved before this change restores with `close` and 10,000,000 bytes even if the config says otherwise. This only affects the first restart after upgrading. The alternative is optional fields in the saved format, which would leak into the `Pane` wire type.
3. **`PaneDefaults` lives in ship-core's `config.rs`.** `Pane`'s serde default needs the built-in history size, and `Pane` is in ship-core. Keeping the struct next to `defaults.toml` avoids writing 10,000,000 in two places.
4. **The UI treats `Lost` as a dropped stream.** It can't receive one, but the match needs an arm. Reattaching is the safe handling if it ever does.

### Blocking unknowns

None.

## Build order

Every slice runs these checks:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- debug and release builds
- `openspec validate pane-defaults --type change --strict`

Use owned temporary servers on ports 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories. Don't touch the server on 43179. The checks run on macOS. Linux counts only when it's run there, and is otherwise reported unverified.

1. **Pane fields and the defaults table** (`model.rs`, `config.rs`, `defaults.toml`, `protocol.rs` `PaneInput` and `PROTOCOL_VERSION`, `command.rs` `Create`, `settings.rs`, `pane.rs` `Launch` and `settings`, `state.rs` `start_pane` and `restore_tab`, `execute.rs` create). Input: a config file and `ship pane create`. Output: panes carry `exitBehavior` and `scrollbackBytes` taken from the table, or from the create flags. Exit behavior has no effect yet.
   Checks:
   - `ship pane get` shows `"exitBehavior":"close"` and `"scrollbackBytes":10000000` on a new pane, next to `rightClickPassthrough`.
   - With `[server.defaults.pane] exit_behavior = "keep"` saved on a running server, a new pane shows `keep` and an older one still shows `close` (P4).
   - `ship pane create --exit-behavior keep_failed --scrollback-bytes 50000000 --right-click-passthrough=false` overrides all three. The same fields work in a `server.pane.create` key binding.
   - `[server.defaults.pane] shell = "/bin/bash"` starts new shell panes in bash. A relative path resolves from the config file's folder.
   - `ship config check` on a file with `[server] shell` names the field, line and column (SC-003). `ship config defaults` prints the new table.
   - A broken config file: new panes use the built-in defaults and the server log shows the error.
   - Restart with saved tabs: the restored panes keep `keep` and 50,000,000 (P6, SC-004). A saved file from before this change restores with `close` and 10,000,000.
   - A protocol-14 client is refused by the health check.

2. **The `every` feed** (`replicas.rs`, `app.rs`, `lib.rs`, `attach.rs`, `input.rs`, `protocol.rs` `AttachRequest`, `FeedMode`, `EndReason`, `automation.rs`, `ui/mod.rs`). Input: `POST /api/v0/attach {"feed":"every"}`. Output: one `state` event per revision, in order, and `ended: lost` on overflow.
   Checks:
   - `curl -N` attaching with `{"feed":"every"}` while a script renames a pane 20 times shows 20 `state` events with consecutive revisions. The same with no `feed` shows coalesced revisions, as today.
   - Overflow, as a temporary probe: build with `RING` set to 2, attach with `every` from a reader that has stopped reading (`curl` under SIGSTOP behind a small socket buffer, or a test client that sleeps), make well over 2 commits, then resume. The stream ends with `{"type":"ended","reason":"lost"}`, and `ship pane wait` on that path prints "fell behind the server". Revert `RING` afterwards.
   - `ship pane wait`, `send --wait` and the UI behave as before. `ship view list` shows no leftover attachment after a wait ends.
   - The UI attaches, renders and reconnects as before.

3. **Close on exit** (`model.rs` `closes`, `state.rs` `PaneEvent` and `Set`, `protocol.rs` and `command.rs` `exit_behavior` on set, `execute.rs` set). Input: a program exits. Output: the pane closes or stays according to its exit behavior, and a waiter sees the exit first.
   Checks:
   - `exit` in a default shell pane closes it. Its neighbor takes the space and the selection moves there (P1, SC-001). In a lone pane, the tab stays, empty.
   - `create --exit-behavior keep -- sh -c 'echo hi; exit 3'`: the pane stays, `ship pane read` prints `hi`, `get` shows code 3 (P2, SC-002).
   - `ship pane wait --status exited` on a `close` pane running `sh -c 'sleep 1; exit 4'` prints the pane with code 4, and the pane is gone afterwards. A plain `ship pane wait` on the same setup fails with "program exited with code 4", not "pane closed" (FR-003).
   - `keep_failed`: `sh -c 'exit 0'` closes, `sh -c 'exit 3'` stays, and a `sleep` killed with `kill -TERM` stays (P3).
   - `ship pane set --exit-behavior keep` on a running `close` pane keeps it on exit. On an already exited pane it stores the value, and the pane stays (FR-007). The same works from a `server.pane.set` key binding.

4. **History size on a live pane** (`session.rs`, `pane.rs` `PaneCommand::Scrollback`, `state.rs` `Set`, `protocol.rs` and `command.rs` `scrollback_bytes` on set, `execute.rs` set). Input: `ship pane set --scrollback-bytes N`. Output: the pane's history limit changes at once.
   Checks:
   - In a pane that has printed about 5 MB (`seq 1 700000`), `set --scrollback-bytes 1000000` drops the oldest history right away: `ship pane read --lines 700000` returns far fewer lines, and its first line is a later number than before.
   - Raising the limit keeps what's there and lets it grow: after `set --scrollback-bytes 50000000` and more output, the earlier lines are still readable.
   - `ship pane set --help` says what raising and lowering do (FR-008).

5. **Docs and specs.** Input: the locked papers and the real output from slices 1 to 4. Output: the contract is written down.
   - Delta specs (drafted when this paper locks, then checked against real output here):
     - `pane-programs`, MODIFIED "Exited programs": the pane closes or stays according to its exit behavior. A kept pane behaves as today.
     - `pane-history`, MODIFIED "History size": per-pane, from the defaults table or the create request, changeable with `set`, and what raising and lowering it does.
     - `config-file`, REMOVED "Server shell setting" and "Server scrollback setting". ADDED "Pane defaults": the table, its four settings and their defaults, read at creation, and a bad file giving the built-in defaults.
     - `session-structure`, MODIFIED "Set attributes" and "Right-click passthrough setting". ADDED "Exit behavior": the three values and when each closes.
     - `session-restore`, MODIFIED "Restore on startup": restored panes keep their exit behavior and history size.
     - `session-observation`, MODIFIED "Observer attachments": the `feed` field. ADDED "Every-revision feed": order, the seed rule, `lost` on overflow.
     - `pane-automation`, MODIFIED "Waiting on exit": a pane that closes on exit still reports its exit. ADDED "Exit before close": the ordering promise from architecture decision 5, and a wait that falls behind failing with "fell behind".
   - README and guide as listed under Replaced.
   Checks:
   - `openspec validate pane-defaults --type change --strict` passes.
   - The guide's examples match output captured in slices 1 to 4.

6. **Size.** Report the change's implementation Rust and test line counts separately.

## Deviation log

- Slice 4: Cyan approved comparing the fully updated `Pane` with its original value instead of maintaining field-by-field change checks, and forwarding `PaneCommand::Scrollback` whenever the request supplies `scrollback_bytes`, even if unchanged. The state revision still changes only when the pane changes. This refines the `Set<Pane>` skeleton above without changing the shared-state no-op rule.
- Slice 5: checking the guide against real output also required fixing `docs/site/README.md` and the landing page's CLI demo. A short-lived `seq 100` pane now uses `--exit-behavior keep` and waits for exit before reading. The landing page's `--lines 5` capture is `97` through `100`, not `96` through `100`, because the final blank terminal row counts toward the requested lines. The displayed commands and copy-button payload were checked against each other and run verbatim. Existing page scopes are unchanged.
- Slice 5: the session-observation delta's stalled-reader scenario incorrectly promised to drain retained revisions before `lost`. It now matches the locked architecture and `Feed::next`: `Lagged` ends the stream with `lost`, without draining the ring. This is a spec correction, not a runtime change or a claim that the overflow probe passed; task 2.4 remains incomplete.

- Slice 2: `Replicas.ring` is a `broadcast::WeakSender`, with the strong sender owned only by `ReplicaSink`. Cyan approved this adjustment in chat after reviewing code diffs. Keeping a strong sender in `Replicas` would keep the ring open during shutdown while the server waits for attach streams to finish. `every()` now returns `Option<Receiver>`; an attach after publication stops returns unavailable.

- Slice 1: ~~`config.rs` also has `PaneDefaults::builtin()`, which reads the table out of `defaults.toml`. `builtin_scrollback_bytes` calls it, so 10,000,000 is still written only in `defaults.toml`. The skeleton showed only `builtin_scrollback_bytes`.~~ Superseded by the next entry.
- Slice 1: `ServerSettings`, `ServerDefaults` and their `load` moved from `ship-server/src/settings.rs` into `ship-core/src/config.rs`, and `settings.rs` is gone. Cyan approved the move in chat on 2026-10-23 ("give it a go! i like this better"). The file map's `settings.rs` entries (lines 16, 139 and 403) now refer to that code in `config.rs`. `Pane`'s serde default lives in core and couldn't reach the server's typed defaults before the move. `builtin_scrollback_bytes` now reads `ServerSettings::default()` instead of indexing `defaults.toml` by string. `ship config check` imports it from `ship_core`. The `[client]` half stays in `ship-client`.
- Slice 5 targets: after the rebase onto main, `docs/guide.md` is split into pages under `docs/site/`, the README no longer has a config example, and the site's landing page repeats the README's "Not built yet" list and a `[server]` config example. The `README.md`/`docs/guide.md` entry above (Replaced) and its slice 5 checks now refer to the `docs/site/` pages and the landing page listed in tasks 5.1 and 5.2. FR-011's "guide" refers to those pages.
- Slice 1: the `ExitBehavior` variants carry doc comments, so `ship pane create --help` lists what each value does. `closes()` waits for slice 3 as planned.
- Slice 1: the `server.pane.create` key-binding fields were checked with `ship config check`, which parses them with the same struct, including a rejected bad value. A key press in the full-screen client was not tried.
- Slice 1: the commented `shell` example in `defaults.toml` is `/bin/sh` instead of `/bin/zsh`, at Cyan's request, since every Unix system has it. It is only a comment: an unset shell is still the login shell.
