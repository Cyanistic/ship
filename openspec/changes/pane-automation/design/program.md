# Pane automation program

Status: Locked on 2026-10-19, approved by Cyan ("lock it please"). Written the same day from the locked [product](product.md) and [architecture](architecture.md) papers, against the crates at `48f1dcd`. An earlier draft passed Plannotator review ("LGTM"); this version was revised after design conversations with Cyan: the CLI side collapses to one `judge` and one wait loop, with the gate as an optional stall deadline; `--key` values are `KeyCombination`s parsed by clap, with crokey in `ship-core`; the input stream's attachment header becomes optional (architecture decision 4, amended), so plain `send` doesn't attach; the attach area stays an `Option` rather than a client-kind enum. Locking doesn't authorize implementation; each slice runs on Cyan's request.

Gates:

- **G1, U-1 (Enter right after a paste).** Probed at the end of slice 2 with the new `send` command against the agents that already report OSC 7501, before slice 2 goes to review. If one drops or mishandles the Enter, the writer gets a gap rule and the product paper's FR-004 reopens. Launching real agents needs Cyan's OK.
- **G2, U-2 (what agents report around a prompt).** Probed before slice 3 is written, with slice 2's `send` and a `ship pane get` loop. If an agent reports `working` later than 5 seconds or re-reports `idle` first, FR-018 and FR-019 reopen. Same OK needed.
- **G3, protocol 13 to 14.** Recommended. `ViewingRecord.area` becomes optional, so a protocol-13 client would fail to decode any replica that holds an observer. The health check already refuses a version mismatch, so bumping turns that into a clear error.

## Rationale

This is a document-only plan. The skeleton below describes proposed source, not files in the working tree, and none of it has been compiled.

The skeleton follows the architecture's structure box by box:

- **`ship-core`** gets the two CLI-only commands, `pane::SendInput` and `pane::Wait`, and the `WaitState` values `--status` accepts. `--key` values are crokey `KeyCombination`s, so clap rejects an unknown key name as a usage error before anything runs. `AttachRequest.area` and `ViewingRecord.area` become `Option<Size>`, and the protocol version goes to 14 (G3).
- **`ship-server`** changes in three places. `tab_sizes` skips records without an area, and `SetView` stores `Some(area)`. The input route takes an optional attachment header: with one, nothing changes; without one, keys and pastes are routed and a mouse frame stops the stream with 422. `AttachmentHeader` gains the `OptionalFromRequestParts` impl that makes that possible.
- **`ship-client/src/automation.rs`** is new and holds the CLI side: the input, the writer, the observer's replica stream, and the wait (`start`, `judge` and one loop). `execute.rs` resolves the target and calls it. Printing stays in `ship/src/commands.rs`, unchanged, because a matched wait returns `Outcome::Pane` and plain `send` returns `Outcome::Closed`, which prints nothing.
- **`ship-client/src/lib.rs` and `api.rs`**: `Client::input` takes an optional attachment, and the input stream's response revision is recorded as `request` and `read` already do. The writer reads it back through `Client::written`.
- **The full-screen client** wraps its area in `Some` in four places and passes `Some(attachment)` to `input`.

Choices made here, inside what the architecture fixed:

- **One module, `automation.rs`, with one wait loop.** The idea is small: check the snapshot, write, then judge each newer replica. So there's one `judge` used on the snapshot and on every later replica, and one `select!` loop inside `wait`, which `send --wait` reuses by passing its input. No wrapper type around the stream, and no separate precheck or loop function.
- **Plain `send` doesn't attach.** It checks the pane with `GET /panes/{id}` (FR-002), which fails with "pane … not found", then writes on a headerless input request. No commits, nothing to detach. A pane closed between the check and the write drops the frames silently, the same race the full-screen client has.
- **The condition names its default.** `Condition::NotWorking` is the no-`--status` case and `Condition::Any` holds the listed states, rather than an empty list standing for the default (`docs/agents/code.md`, "Say what things mean").
- **The gate is the stall deadline.** FR-018's gate keeps `send --wait` from returning on the state from before the input, such as an `idle` the agent hasn't left yet. It's `stall: Option<Instant>`: `Some(deadline)` while the program hasn't been seen `working` or `blocked` after the write, `None` once it has. That's also exactly when FR-019's 5-second stall applies, so one value covers both. `wait` and an already-`working` `send --wait` start at `None`; otherwise the deadline is set from the write's response.
- **Stdin is read before attaching or checking,** so a slow stdin never holds an attachment open.
- **Exit is checked before the record.** An exited pane has no records (the exit commit clears them), so `send --wait` and `wait` on an exited pane report "exited with ..." rather than "no program status". It still fails before writing, as FR-017 requires.
- **A record that disappears mid-wait matches nothing.** FR-012 fails at once only at the start. If the program clears its root record during a wait, the wait goes on until the record comes back, the program exits or the pane closes.
- **The struct is `SendInput`, not `Send`.** A `Send` struct in `command::pane` would shadow the marker trait inside that module. The subcommand is still `send`, from the variant name.
- **The revision comes from `Client::written`.** `send_stream` records the response's revision the way `request` does, and a CLI's client is fresh per command, so after the write `written` is the input response's revision. The full-screen client's input stream ends only at detach, so recording there changes nothing for it.
- **A headerless mouse frame is a 422, not a silent drop.** Without an attachment its copy has nowhere to go, and failing the stream the way a malformed line does tells the caller instead of losing clicks.
- **Stall and exit errors say whether input was sent.** FR-019 requires it for the stall. The others say it where it's true, so a caller knows whether a retry would send twice.

Estimated change: about 220 lines of implementation Rust, unmeasured, inside the product paper's 200 to 350, near its low end. No test code (`docs/agents/code.md`).

| Area | Lines |
| --- | --- |
| `automation.rs`: input, writer, condition, `start`, `judge`, the wait loop | about +100 |
| `ship-core` `command.rs`: `SendInput`, `Wait`, `WaitState` | about +60 |
| `execute.rs`: two match arms | about +25 |
| `input.rs`, `attach.rs`: optional header | about +20 |
| `protocol.rs`, `state.rs`, `lib.rs`, `api.rs`, `ui/`: optional area, revision, `Option` attachment | about +15 |

## Skeleton map

### Proposed settings

`crokey` becomes a dependency of `ship-core`. It's already a workspace dependency, and the one `ship` binary already links it through `ship-client`.

```diff
 # crates/ship-core/Cargo.toml
 clap = { workspace = true, optional = true }
+crokey.workspace = true
 crossterm.workspace = true
```

No new configuration keys; `send` and `wait` are `#[serde(skip)]`, so the keymap config rejects them.

Protocol version (G3):

```diff
 // crates/ship-core/src/protocol.rs
-pub const PROTOCOL_VERSION: u32 = 13;
+pub const PROTOCOL_VERSION: u32 = 14;
```

An observer's attach body, and its record in `viewers`:

```json
{}
```

```json
"attachment:0193…": {}
```

A headerless input request, as plain `send` makes it:

```text
POST /api/v0/attach/input
content-type: application/x-ndjson

{"type":"paste","pane":"pane:7f2c…","text":"fix the flaky test in saver.rs"}
{"type":"key","pane":"pane:7f2c…","key":{"code":"Enter","modifiers":"","kind":"Press","state":""}}
```

### Added

#### `crates/ship-client/src/automation.rs` (slices 2 and 3)

```rust
//! `ship pane send` and `ship pane wait` (architecture shape A). Input goes
//! out on a headerless input request. Waits read each published replica on
//! an observer attachment with no area and no selection. A status that never
//! reaches a replica didn't happen (FR-018).

use std::{
    io::{self, Read},
    time::Duration,
};

use crokey::KeyCombination;
use crossterm::event::KeyEvent;
use futures_util::{Stream, StreamExt};
use ship_core::{
    command::pane::WaitState,
    id::IdOf,
    model::{Pane, PaneStatus, ReportedState, StatusRecord},
    prelude::*,
    protocol::{AttachRequest, InputFrame, KeyInput, PasteInput, Replica, SseEvent},
    tree,
};
use tokio::time::{Instant, sleep_until};

use crate::Client;

/// FR-019: how long `send --wait` gives the program to report `working` or
/// `blocked` after the write.
const STALL: Duration = Duration::from_secs(5);

// Slice 2

/// What `send` writes. clap has already parsed the keys (FR-005), so only
/// reading stdin can fail.
pub(crate) struct Input {
    text: Option<String>,
    keys: Vec<KeyEvent>,
}

impl Input {
    /// Reads stdin when the text is `-` (FR-003).
    pub(crate) fn new(text: Option<String>, keys: Vec<KeyCombination>) -> Result<Self>;

    /// One paste frame for the text, then one key frame per key, in order
    /// (FR-004).
    fn frames(self, pane: IdOf<Pane>) -> Vec<InputFrame>;
}

impl Client {
    /// `ship pane send`. Without a condition: checks the pane with `GET
    /// /panes/{id}` and writes; `None`. With one: waits as `wait` does, with
    /// the input written after the snapshot is checked.
    pub(crate) async fn send(
        &self,
        pane: IdOf<Pane>,
        input: Input,
        wait: Option<Condition>,
    ) -> Result<Option<Pane>>;

    /// Writes the frames on one headerless input request and returns the
    /// revision its response carried, the line between before and after the
    /// write (architecture decision 5).
    async fn write(&self, frames: Vec<InputFrame>) -> Result<u64>;
}

// Slice 3

/// What ends a wait (FR-008, FR-010).
pub(crate) enum Condition {
    /// No `--status`: any root state other than `working`.
    NotWorking,
    /// `--status`: any listed state; `exited` matches the program's exit.
    Any(Vec<WaitState>),
}

impl Condition {
    pub(crate) fn new(states: Vec<WaitState>) -> Self;
    fn holds(&self, record: &StatusRecord) -> bool;
    fn takes_exit(&self) -> bool;
}

impl Client {
    /// `ship pane wait`, and `send --wait` when `input` is given. Attaches as
    /// an observer; checks the snapshot with `start`, and for `wait` with
    /// `judge`; writes the input, if any; then judges each replica newer than the write in one
    /// `select!` loop, racing `stall` while it's set. Without input it never
    /// writes (FR-015). Dropping the stream on return detaches the observer
    /// through the server's `AttachmentGuard` (FR-014).
    pub(crate) async fn wait(
        &self,
        pane: IdOf<Pane>,
        condition: Condition,
        input: Option<Input>,
    ) -> Result<Pane>;

    /// `POST /attach` with no selection and no area. Yields the `Attached`
    /// replica, then each `state` replica. `ended` is "the server stopped", a
    /// stream cut without it is "lost the connection", and `screen` or
    /// `clipboard` events (which an observer never gets) are skipped.
    async fn replicas(&self) -> Result<impl Stream<Item = Result<Replica>> + Unpin + use<>>;
}

/// The snapshot checks only the start has, in order. For `send --wait`,
/// everything that must fail before writing: the pane gone ("not found"),
/// exited, no root record, or `blocked` (FR-002, FR-017). For `wait`, only a
/// running pane with no root record and no `exited` in the condition
/// (FR-012); `judge` handles the rest. Returns whether the program still has
/// to be seen picking up the input: true for `send --wait` unless the root is
/// already `working` (FR-018), false for `wait`.
fn start(
    snapshot: &Replica,
    pane: IdOf<Pane>,
    condition: &Condition,
    sending: bool,
) -> Result<bool>;

/// One replica's meaning for the wait, used on `wait`'s snapshot and on
/// every later replica, in order: the pane gone fails ("not found" on the
/// snapshot, "closed" after); exited matches when the condition takes exits and
/// otherwise fails with the status (FR-010, FR-011); while `stall` is set,
/// `working` or `blocked` clears it and nothing else counts; once it's clear,
/// a root record that holds the condition matches. `stall` is FR-018's gate
/// and FR-019's deadline at once: `Some(deadline)` while the program hasn't
/// been seen `working` or `blocked` after the write, `None` once it has.
/// `Some(pane)` on a match, `None` to keep reading.
fn judge(
    replica: &Replica,
    pane: IdOf<Pane>,
    condition: &Condition,
    stall: &mut Option<Instant>,
) -> Result<Option<Pane>>;
```

The loop in `wait`, to show the shape (not a body to copy):

```rust
let mut replicas = self.replicas().await?;
let snapshot = next(&mut replicas).await?;
let unseen = start(&snapshot, pane, &condition, input.is_some())?;
let mut stall = None;
let after = match input {
    Some(input) => {                                        // send --wait
        let after = self.write(input.frames(pane)).await?;
        stall = unseen.then(|| Instant::now() + STALL);     // five seconds from the write
        after
    }
    None => match judge(&snapshot, pane, &condition, &mut stall)? {
        Some(pane) => return Ok(pane),                      // FR-009
        None => snapshot.revision,
    },
};
loop {
    tokio::select! {
        replica = next(&mut replicas) => {
            let replica = replica?;
            if replica.revision > after
                && let Some(pane) = judge(&replica, pane, &condition, &mut stall)?
            {
                return Ok(pane);
            }
        }
        () = sleep_until(stall.unwrap()), if stall.is_some() => return Err(stalled(pane)),
    }
}
```

Error messages, each naming the pane (FR-021: stderr, exit 1; an unknown `--key` is clap's usage error, exit 2):

```text
pane pane:… not found                                        before writing, from tree::pane or GET /panes
pane pane:… closed; the input was sent                       during a wait (send --wait); "closed" alone for wait
pane pane:… is blocked on permission; nothing was sent       FR-017, with the blocked kind
pane pane:… has no program status; nothing was sent          FR-017; "has no program status" alone for FR-012
pane pane:… program exited with code 1                       FR-011; "killed by SIGTERM" for a signal
pane pane:… stalled: no working or blocked within 5 seconds; the input was sent    FR-019
the server stopped while waiting on pane pane:…              `ended`
lost the connection while waiting on pane pane:…             stream cut without `ended`
```

### Replaced

#### `crates/ship-core/src/command.rs` (slices 2 and 3)

Two variants on `pane::PaneCommand`, its doc comment updated to name all four CLI-only commands:

```rust
    use crokey::KeyCombination;
    use crate::model::ReportedState;

    /// `get`, `read`, `send` and `wait` are CLI-only, so the config rejects them.
    pub enum PaneCommand {
        // …existing variants…
        /// Send text as one paste, then keys; with --wait, print the pane
        /// once its program has handled the input
        #[serde(skip)]
        Send(SendInput),
        /// Wait for the pane's program status and print the pane as JSON
        #[serde(skip)]
        Wait(Wait),
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    pub struct SendInput {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub pane: PaneTarget,
        /// Text to send as one paste; `-` reads it from stdin
        #[cfg_attr(feature = "clap", arg(required_unless_present = "keys"))]
        pub text: Option<String>,
        /// A key to press after the text, named as in the keymap config
        /// (`enter`, `ctrl-c`); repeat for more, in order
        #[cfg_attr(feature = "clap", arg(long = "key", value_name = "KEY"))]
        pub keys: Vec<KeyCombination>,
        /// Wait until the program has started and finished handling the
        /// input, then print the pane as JSON
        #[cfg_attr(feature = "clap", arg(long))]
        pub wait: bool,
        /// With --wait, the states to wait for; anything but working by default
        #[cfg_attr(feature = "clap", arg(long, value_delimiter = ',', requires = "wait"))]
        pub status: Vec<WaitState>,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    pub struct Wait {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub pane: PaneTarget,
        /// The states to wait for; anything but working by default
        #[cfg_attr(feature = "clap", arg(long, value_delimiter = ','))]
        pub status: Vec<WaitState>,
    }

    /// A state `--status` accepts: the program's reported states, and its exit.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
    #[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
    #[serde(rename_all = "snake_case")]
    pub enum WaitState {
        Idle,
        Working,
        Done,
        Blocked,
        Error,
        Exited,
    }

    impl From<ReportedState> for WaitState { /* one arm per state */ }
```

clap parses `KeyCombination` through its `FromStr`, the same parser the keymap uses, and serde already deserializes it for the keymap.

#### `crates/ship-core/src/protocol.rs` (slice 1)

```rust
pub struct AttachRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
    /// Where the client draws the viewed tab, after its own chrome. None for
    /// an observer, which has no screen and sizes no tab.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<Size>,
}

pub struct ViewingRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
    /// Where the client draws the viewed tab. Tab sizes derive from it; an
    /// observer has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<Size>,
}
```

`ViewInput.area` stays required: an attachment that reports an area is no longer an observer. A client-kind enum was considered and rejected (architecture decision 3).

#### `crates/ship-server/src/state.rs` (slice 1)

```diff
     /// Each viewed tab's size: the smallest of its viewers' areas in each
-    /// dimension. Records without a selection are skipped. Derived, never
-    /// stored.
+    /// dimension. Records without a selection or an area are skipped.
+    /// Derived, never stored.
     fn tab_sizes(&self) -> HashMap<IdOf<Tab>, Size> {
         …
-            let Some(tab) = record
-                .selection
-                .and_then(|selection| tree::viewed_tab(&self.tabs, selection))
-            else {
+            let (Some(tab), Some(area)) = (
+                record
+                    .selection
+                    .and_then(|selection| tree::viewed_tab(&self.tabs, selection)),
+                record.area,
+            ) else {
                 continue;
             };
```

```diff
     // SetView
-            record.area = view.area;
+            record.area = Some(view.area);
```

The `Attach` handler copies `request.area` unchanged; only its type changes.

#### `crates/ship-server/src/attach.rs` (slice 2)

```rust
use axum::extract::OptionalFromRequestParts;

/// No header is `None`; a malformed one still rejects with 400.
impl<S: Send + Sync> OptionalFromRequestParts<S> for AttachmentHeader {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Option<Self>>;
}
```

#### `crates/ship-server/src/input.rs` (slice 2)

```diff
-//! The input stream: keys, pastes and pointers from one attachment, routed to panes.
+//! The input stream: keys, pastes and pointers, routed to panes. Pointers
+//! need the sending attachment, so a mouse selection's copy reaches it.
```

```diff
 #[utoipa::path(
     post,
     path = "/api/v0/attach/input",
     operation_id = "input",
-    params(("x-ship-attachment-id" = String, Header,
-        description = "Attachment ID from the stream's `attached` event, e.g. attachment:3f2a...")),
+    params(("x-ship-attachment-id" = Option<String>, Header,
+        description = "Attachment ID from the stream's `attached` event, e.g. attachment:3f2a.... \
+            Required for mouse frames; without it the request ends only with its body.")),
     …
     responses(
         (status = 204, description = "The body ended, or the attachment or server did"),
-        (status = 400, description = "Missing or malformed attachment header", body = AppError),
+        (status = 400, description = "Malformed attachment header", body = AppError),
         (status = 404, description = "Attachment not found; nothing was read", body = AppError),
-        (status = 422, description = "Malformed or oversized line; reading stopped there", body = AppError),
+        (status = 422, description = "Malformed or oversized line, or a mouse frame without \
+            an attachment; reading stopped there", body = AppError),
```

```rust
pub(crate) async fn input(
    State(app): State<AppState>,
    attachment: Option<AttachmentHeader>,
    body: Body,
) -> Result<Response>;
// With an attachment: CheckAttachment and the end-with-the-attachment branch,
// as today. Without: neither; the loop ends with the body or the server.

/// Errors on a mouse frame without an attachment; the caller stops reading
/// with 422, as for a malformed line.
#[traced(fields(attachment = ?from))]
fn route(live: &LivePanes, frame: InputFrame, from: Option<IdOf<Attachment>>) -> Result<()>;
```

#### `crates/ship-client/src/lib.rs` and `api.rs` (slice 2)

```diff
     /// POST `items` as an NDJSON body, one line each as they arrive, and wait
-    /// for `expected_status`. No timeout: the body lasts as long as `items`.
+    /// for `expected_status`, recording the response's revision. No timeout:
+    /// the body lasts as long as `items`.
     async fn send_stream<T: Serialize + Send + 'static>(
         &self,
         path: &str,
-        attachment: IdOf<Attachment>,
+        attachment: Option<IdOf<Attachment>>,
```

Its body reads `REVISION_HEADER` and calls `self.wrote(revision)` the way `request` does, before dropping the response.

```diff
-    /// Stream `frames` to the attachment's panes in one POST, until `frames`
-    /// ends or the server answers. Only the connect timeout applies.
+    /// Stream `frames` to panes in one POST, until `frames` ends or the server
+    /// answers. Mouse frames need `attachment`. Only the connect timeout applies.
     pub async fn input(
         &self,
-        attachment: IdOf<Attachment>,
+        attachment: Option<IdOf<Attachment>>,
         frames: impl Stream<Item = InputFrame> + Send + 'static,
     ) -> Result<()> {
```

#### `crates/ship-client/src/execute.rs` (slices 2 and 3)

```rust
PaneCommand::Send(pane::SendInput { pane, text, keys, wait, status }) => {
    let id = Self::pane_target(pane, scope)?;
    let input = Input::new(text, keys)?;
    let wait = wait.then(|| Condition::new(status));
    self.send(id, input, wait)
        .await?
        .map_or(Outcome::Closed, Outcome::Pane)
}
PaneCommand::Wait(pane::Wait { pane, status }) => {
    let id = Self::pane_target(pane, scope)?;
    Outcome::Pane(self.wait(id, Condition::new(status), None).await?)
}
```

`pane_target` under `Scope::Cli` never reads a selection (FR-001). Keys can't produce either command, since both are `#[serde(skip)]`.

#### `crates/ship-client/src/ui/mod.rs` and `ui/observer.rs` (slices 1 and 2)

Slice 1:

```diff
-    let request = AttachRequest { selection, area };
+    let request = AttachRequest { selection, area: Some(area) };
…
-                    request.area = view_area(terminal, &local.sidebar);
+                    request.area = Some(view_area(terminal, &local.sidebar));
…
-    (observer.record()?.area != area).then_some((observer.attachment?, ViewInput { area }))
+    (observer.record()?.area != Some(area)).then_some((observer.attachment?, ViewInput { area }))
```

```diff
         Some(AttachRequest {
             selection: record.selection,
-            area,
+            area: Some(area),
         })
```

Slice 2:

```diff
-                        sending = Some(Box::pin(client.input(attached.attachment, UnboundedReceiverStream::new(received))));
+                        sending = Some(Box::pin(client.input(Some(attached.attachment), UnboundedReceiverStream::new(received))));
```

#### Documents (slice 4)

- `docs/guide.md`: a new section after "Scrolling and reading output", "Driving panes from scripts", covering everything FR-023 lists. An "Upgrading" line for protocol 14.
- `docs/product-brief.md`: milestone 4 amended as FR-022 lists.
- `README.md`: line 16's "agents can't drive the workspace yet" goes, "Not built yet" loses "Sending input to a pane, or waiting for another agent to finish, from the CLI", and "What works today" gains a line for `send` and `wait`.
- `openspec/changes/pane-automation/specs/`: a new `pane-automation` capability, which also states the headerless input stream (no spec covers the input route's header today, only its OpenAPI description), and deltas to `session-observation` (observers: attach without an area) and `health-exchange` (protocol 14).

### Moved

None.

### Untouched

- `crates/ship-server/src/routes.rs`, `app.rs`, `pane.rs`: `GET /panes/{id}`, the `stamp` middleware and the pane task already do what the architecture needs. `PaneMouse.from` keeps its type; only mouse frames with a header reach it.
- `crates/ship/src/commands.rs`, `cli.rs`, `main.rs`: `action` prints `Outcome::Pane` and ignores `Outcome::Closed` already, and both commands reach it through `Command::Action`.
- `crates/ship-client/src/keymap.rs`: it keeps its own crokey parsing for chords; `--key` reuses crokey's `FromStr`, not the keymap module.
- Close-on-exit, and the exit-then-close risk it brings (architecture "Risks"), belong to that change.

## Build order

Each slice runs only when Cyan asks for it and stops for review before the next. Every slice passes:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `ship-core` checks with and without `clap`;
- from slice 4, `openspec validate pane-automation --type change --strict`.

Exercises use owned temporary servers on ports 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories. Existing user work is left alone, including the server on 43179. Scripted reporters are temporary probes and aren't committed. Checks run on macOS; Linux is reported unverified unless run there.

1. **Observer attachments.** Files: `protocol.rs`, `state.rs`, `ui/mod.rs`, `ui/observer.rs`. Delivers: `POST /api/v0/attach` with `{}` gets `attached` and then `state` events and no `screen` events, and its record shows in `ship view list` without an area. Checks: with a full-screen client attached at some size, attaching an observer with curl leaves the tab's published `geometry` unchanged and the client draws as before. Reconnecting and resizing the full-screen client still work. `ship server status` reports protocol 14.
2. **Headerless input and `ship pane send`.** Files: `attach.rs`, `input.rs`, `command.rs` (`SendInput` without `--wait` and `--status`), `automation.rs` (slice 2 part), `execute.rs`, `lib.rs`, `api.rs`, `ui/mod.rs`. Delivers P2 against `cat -v` and a shell, read back with `ship pane read`: text then `--key esc --key enter` arrive in that order; a multi-line stdin with `-` arrives as one paste and a single Enter; `--key ctrl-c` interrupts `sleep`; a pane scrolled into history returns to the bottom. Checks: a headerless curl input with a mouse frame gets 422; a malformed header still gets 400; in the full-screen client, typing, pasting and a mouse selection's copy to its own clipboard still work, and a second attached client doesn't receive that copy; `send` with neither text nor keys exits 2; `--key ctl-c` exits 2 and nothing arrives; an unknown or malformed ID fails naming the pane; a pane whose program exited accepts the input silently; `ship view list` shows no attachment from `send`. Then G1: with Cyan's OK, send a prompt with `--key enter` to each agent that reports OSC 7501 and record in the deviation log whether the Enter was taken.
3. **`ship pane wait` and `send --wait`.** Files: `command.rs` (`Wait`, `WaitState`, `--wait` and `--status` on `SendInput`), `automation.rs` (slice 3 part), `execute.rs`. Before writing it, G2 with Cyan's OK. Delivers P1, P3 and P4 against a scripted reporter: a shell script in a pane that reads a line, emits OSC 7501 `working`, sleeps, emits `idle` (or `blocked`, `done`, `error`), or exits. Checks: SC-003, 50 of 50 `send --wait` runs return against a 50 ms `working`; SC-004, each P4 failure prints its own message and a reporter that never reports fails within 6 seconds; SC-005, `timeout 2 ship pane wait` on a `working` pane exits 124 and leaves no observer in `ship view list`; SC-002, switching tabs and panes in a full-screen client throughout changes nothing; `--status done,exited` returns on either; `wait` on a condition that already holds returns at once; `--status` on `send` without `--wait` exits 2; `send --wait` on a pane already `working` returns when that turn ends.
4. **Docs, specs and size.** Files: `docs/guide.md`, `docs/product-brief.md`, `README.md`, `openspec/changes/pane-automation/specs/`. Delivers: the guide's walkthrough runs as written, including `timeout(1)` and the macOS `gtimeout` note. SC-001 by hand from inside a Ship pane with a real agent, with Cyan's OK. Checks: `openspec validate`, and the implementation Rust and test line counts reported separately.

## Deviation log

- Slice 2: Cyan approved using `esc`, with no `escape` alias, after runtime verification showed that crokey 1.5.0 accepts only `esc`. Corrected the examples in the product paper, this paper, the spec delta, tasks and guide. Key parsing still uses crokey unchanged.
- Gate G1: Cyan scoped the real-agent probe to Pi with `openai-codex/gpt-6-luna`. On macOS, three consecutive `ship pane send --pane ID TEXT --key enter` calls each submitted the prompt and produced the exact requested assistant reply, with no extra Enter, inserted delay or fallback submission. The persisted session messages confirmed the provider and model; `ship pane read` showed the replies and the root OSC 7501 record finished at `done`. No paste-to-Enter gap was needed for Pi. Other agents and Linux remain unverified; this does not settle U-1 for those agents. Temporary evidence: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-g1-pi-a_ztwdbb/` (`result.json`, the session JSONL, pane reads and `events.json`). The owned server on port 43335 and its pane were stopped after the probe; no existing server or user work was touched. Gate G2 remains pending and slice 3 has not started.
