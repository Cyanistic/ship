# Pane terminals program

Status: Locked on 2026-10-06. Cyan reviewed the first draft in Plannotator, approved every proposed fix in chat ("let's make all of these proposed fixes!"), and had no notes on revision 2 ("i have no notes for this plannotator review!"). After that, the redundant `title` field on `PaneTask` was removed at Cyan's request. Locking turned up a conflict between A1 as first written (no waiting at shutdown) and product FR-013; Cyan chose the `on_stop` wait ("i feel like that's a lot cleaner"), which resolves it. Written from the locked [product](product.md) and [architecture](architecture.md) papers and the current crates; the review added architecture amendments A1 to A6, recorded in the [architecture paper](architecture.md#amendments). Nothing here is created source, and no snippet has been compiled. Approval doesn't start implementation; Cyan requests that separately, slice by slice.

## Rationale

The plan puts each architectural part in one obvious home and changes existing code only where the architecture asks for it.

- **`ship-core`** gains the data both sides serialize:
  - optional names;
  - pane metadata;
  - the `Screen` grid;
  - `InputFrame` (keys serialize through crossterm's own `serde` feature, A5);
  - `ViewInput`, the client's whole view (A4);
  - the new SSE variant.

  It still has no PTYs, Ghostty or HTTP. The relay gains one sink impl.
- **`ship-server`** gains `pane.rs`: the runtime, handle, pane task, capture loop and teardown, all in one file. That's where every live resource lives. Also new: `screens.rs` (the screen table and its sinks) and `input.rs` (the streaming POST). `state.rs` changes in three ways: the actor owns pane runtimes and `commit` drops those whose panes are gone; tab and pane sizes are derived from viewing records after each commit, never stored; and `PaneEvent` and `SetView` arrive as messages. `attach.rs` merges screens into the stream and replaces the selection and session routes with `PUT /api/v0/attach/view`.
- **`ship-client`** gains the full-screen client in a `ui` module (architecture decision 15) and the `input` and `set_view` request methods. The text observer and stdin controls in `ship` are deleted.
- **`vendor/ratatui-ghostty`** is the wrapper copied from `experiments/terminal-transport/vendor/wrapper-compat`, with PROVENANCE. The server depends on it by path.

Three choices do most of the simplifying.

1. **Pane teardown is one async function on one task.** The pane task owns the session, the process group and the exit watcher. When its drop guard cancels it, it runs hangup, grace, kill and reap in order, then publishes the removal notice. Removal, failed creation and tab or session removal all reach it the same way, by dropping a `PaneRuntime`. Server shutdown uses the same teardown and waits for it: the state actor's `on_stop` calls `stop` on every runtime and awaits them all (A1).
2. **One generic sink per server-wide table.** The replica, pane handles and screens each reach readers through one `watch`, filled by one bus subscription made at startup. Streams and the input route only read watches.
3. **The client is one select loop.** It reads the SSE stream, crossterm events, the frame tick and the reconnect timer, and draws after anything that changes what's visible. Key and paste frames go into an unbounded channel that feeds the current streaming POST. With no POST open, frames are dropped. Navigation and resizes change a local view, and the frame tick sends at most one view per frame.

Estimated new implementation Rust: about 2,000 to 2,800 lines, against 7,346 lines today. The vendored wrapper doesn't count.

## Skeleton map

### Added

#### `vendor/ratatui-ghostty/` (slice 1)

The full copy of `experiments/terminal-transport/vendor/wrapper-compat`: `Cargo.toml`, `src/`, `tests/`, `examples/`, `README.md` and the local `LICENSE` notice. `vendor/ratatui-ghostty/PROVENANCE.md` carries over the wrapper sections of `experiments/terminal-transport/vendor/PROVENANCE.md`:

- the published 0.2.0 baseline and its recorded commit;
- the five local changes;
- the MIT notice caveat.

It also records the copy date. The copy isn't a workspace member. Lints and the line budget don't apply to it. The experiment copy stays where it is.

#### `crates/ship-core/src/screen.rs` (slice 3)

```rust
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A pane's visible grid at one moment, row-major. Full snapshot; zstd on the
/// stream supplies the savings between frames.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    pub size: Size,
    pub cells: Vec<Cell>,                 // size.cols * size.rows
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,           // None when hidden
}

/// Default-valued fields are omitted on the wire, so a blank cell is `{}`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    /// Grapheme; " " when omitted. A wide character's trailing cell is "".
    #[serde(default = "space", skip_serializing_if = "is_space")]
    pub symbol: String,
    #[serde(default, skip_serializing_if = "Color::is_default")]
    pub fg: Color,
    #[serde(default, skip_serializing_if = "Color::is_default")]
    pub bg: Color,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attrs: Vec<Attr>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Color {
    #[default]
    Default,
    /// 0 to 15 are the named ANSI colors.
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Attr { Bold, Dim, Italic, Underlined, SlowBlink, RapidBlink, Reversed, Hidden, CrossedOut }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Cursor {
    pub x: u16,
    pub y: u16,
    pub shape: CursorShape,
    pub blinking: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum CursorShape { Block, Underline, Bar }

/// Terminal size in cells. Also the viewer size and tab size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

impl Size {
    pub const FALLBACK: Size = Size { cols: 80, rows: 24 };
}
```

The server converts the wrapper's ratatui `Buffer` into a `Screen` (`pane.rs` `capture`), and the client converts back (`ui/draw.rs` `paint`). That keeps ratatui out of `ship-core`. Named ratatui colors map to `Indexed(0..=15)`. Underline color is dropped (resolved question 7).

#### `crates/ship-server/src/pane.rs` (slice 2; capture in 3; commands in 4)

```rust
use std::{path::PathBuf, sync::Arc, time::Duration};

use kameo::actor::ActorRef;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use ratatui_ghostty::{SessionEvent, SessionHandle, SessionIo};
use ship_core::{id::IdOf, model::*, prelude::*, relay::{Publish, RelayBus}, screen::*};
use tokio::sync::{Notify, mpsc, oneshot};
use tokio_util::sync::{CancellationToken, DropGuard};

pub(crate) const FRAME: Duration = Duration::from_millis(16);
const GRACE: Duration = Duration::from_secs(2);

/// Owned by the state actor. Dropping it cancels the pane task, which runs
/// teardown in the background. `stop` does the same and waits for it.
pub(crate) struct PaneRuntime {
    pub handle: PaneHandle,
    cancel: DropGuard,
    done: oneshot::Receiver<()>,         // the pane task sends () after teardown
}

impl PaneRuntime {
    /// Cancel, then await `done`. An `Err` (task already gone) also counts as done.
    pub async fn stop(self);
}

/// The pane task's command channel, published to the input route. Sending
/// never awaits the PTY; a send after the task ended fails and is ignored.
pub(crate) type PaneHandle = mpsc::UnboundedSender<PaneCommand>;

pub(crate) enum PaneCommand {
    Key(crossterm::event::KeyEvent),     // wrapper encodes against live terminal modes
    Paste(String),                       // wrapper applies bracketed paste when enabled
    Resize(Size),                        // dropped when equal to the pane's current size
}

/// What to start. `command: None` is the login shell from portable-pty's
/// `CommandBuilder::new_default_prog`.
pub(crate) struct Launch {
    pub pane: IdOf<Pane>,
    pub command: Option<Vec<String>>,
    pub cwd: PathBuf,
    pub size: Size,
}

/// Bus publications. One sender per pane, so the bus keeps their order.
#[derive(Clone)]
pub(crate) struct ScreenUpdate { pub pane: IdOf<Pane>, pub seq: u64, pub screen: Arc<Screen> }
#[derive(Clone)]
pub(crate) struct PaneEvent { pub pane: IdOf<Pane>, pub change: PaneChange }
#[derive(Clone)]
pub(crate) enum PaneChange { Title(Option<String>), Exited(ExitStatus) }
/// Last publication of a pane task, after teardown. Prunes the screen table.
#[derive(Clone)]
pub(crate) struct PaneClosed(pub IdOf<Pane>);

/// Synchronous; the state actor calls it just before the commit that inserts
/// the pane (A6):
/// - open the PTY;
/// - spawn the child with `TERM=xterm-256color` and `COLORTERM=truecolor`;
/// - drop the slave;
/// - start the wrapper session and the exit-watcher thread;
/// - spawn the pane task.
///
/// Returns the resolved argv for `Pane::command`. Any failure returns `Err`,
/// and nothing is left running.
pub(crate) fn spawn(launch: Launch, bus: ActorRef<RelayBus>) -> Result<(PaneRuntime, Vec<String>)>;

/// The pane task's state.
struct PaneTask {
    pane: IdOf<Pane>,
    session: SessionHandle,
    pgid: nix::unistd::Pid,            // child pid; it leads its own session and group
    exit: oneshot::Receiver<ExitStatus>, // a std thread blocked in `child.wait()`
    exited: bool,
    size: Size,                          // current PTY size; equal resizes are dropped
    dirty: Arc<Notify>,                  // the wrapper's wake callback calls notify_one
    seq: u64,
    bus: ActorRef<RelayBus>,
}

impl PaneTask {
    /// select! over:
    ///   cancel                  -> teardown
    ///   commands                -> session.send_key / send_paste / send_resize (if the size
    ///                              changed); ignored once exited
    ///   dirty, then the frame interval (missed ticks: Delay) -> publish()
    ///   exit                    -> publish() a final frame, then PaneEvent Exited
    async fn run(self, commands: mpsc::UnboundedReceiver<PaneCommand>, cancel: CancellationToken);

    /// Drain the wrapper's events into a local latest title (the tree keeps
    /// the lasting copy in `Pane.title`), capture, and publish
    /// `ScreenUpdate` plus `PaneEvent::Title` if a title event arrived this frame.
    async fn publish(&mut self);

    /// SIGHUP to the child's group and to the terminal's foreground group
    /// (A1). Wait up to GRACE for the exit watcher, then SIGKILL both
    /// groups. Wait for the reap, drop the session, publish `PaneClosed`, and
    /// send on the runtime's `done` channel.
    async fn teardown(self);
}

/// Wrapper buffer and cursor to an owned `Screen`.
fn capture(session: &SessionHandle) -> Screen;
```

`ExitStatus` lives in `model.rs`. The exit watcher maps portable-pty's `ExitStatus` onto it.

#### `crates/ship-server/src/screens.rs` (slice 3)

```rust
use std::{collections::HashMap, sync::Arc};

use ship_core::{id::IdOf, model::Pane, relay::{RelayBus, Subscribe}, screen::Screen};
use tokio::sync::watch;

use crate::pane::{PaneClosed, ScreenUpdate};

#[derive(Clone)]
pub(crate) struct Shown { pub seq: u64, pub screen: Arc<Screen> }

/// Latest screen per live pane. New streams read current screens from it, so
/// a quiet pane is never blank after attach (architecture decision 6).
pub(crate) type ScreenTable = HashMap<IdOf<Pane>, Shown>;

/// Subscribe two closure sinks that `send_modify` one table: `ScreenUpdate`
/// inserts, `PaneClosed` removes. Returns the receiver for `AppState`.
pub(crate) async fn subscribe(bus: &ActorRef<RelayBus>) -> Result<watch::Receiver<ScreenTable>>;
```

#### `crates/ship-server/src/input.rs` (slice 4)

```rust
use axum::{body::Body, extract::State, http::StatusCode};
use ship_core::{prelude::*, protocol::{InputFrame, INPUT_LINE_MAX}};

use crate::{AppState, attach::AttachmentHeader};

/// `POST /api/v0/attach/input`, NDJSON body of `InputFrame`. Steps:
/// 1. Ask the state actor that the attachment is active (404 if not), before
///    reading any of the body.
/// 2. Read lines through StreamReader and FramedRead<LinesCodec::new_with_max_length(INPUT_LINE_MAX)>.
/// 3. Route each Key or Paste frame to `handles[pane].send(..)`, ignoring a
///    closed channel. Frames for unknown panes are dropped. View changes don't
///    come this way; they use `PUT /api/v0/attach/view` (A4).
///
/// The loop also selects on `replicas.changed()`. It stops when the
/// attachment leaves the replica or the channel closes, so shutdown never
/// waits on an idle input stream.
///
/// Responses:
/// - 204 when the body ends or the attachment ends;
/// - 422 on a malformed or oversized line, after which reading stops.
pub(crate) async fn input(
    State(app): State<AppState>,
    attachment: AttachmentHeader,
    body: Body,
) -> Result<StatusCode>;
```

#### `crates/ship-client/src/ui/` (slices 5 and 6)

```text
crates/ship-client/src/ui/
├── mod.rs        # pub async fn run(client, session) -> Result<()>; the select loop
├── observer.rs   # Observer: attachment, replica, screens; apply, reconcile, remembered
├── keys.rs       # prefix state machine: crossterm Event -> Action
├── draw.rs       # paint Screen, filler, status line, empty state, labels
└── terminal.rs   # TerminalGuard: raw mode, alternate screen, restore, panic hook
```

```rust
// ui/mod.rs
/// `ship attach`. Enters the terminal and opens the attach stream with the
/// terminal size. On every `Attached`, it (re)starts the input POST for that
/// attachment. Navigation and resizes edit a local `ViewInput` and mark it
/// dirty; on the frame tick (`FRAME`, missed ticks: Delay) a dirty view is sent
/// with `set_view`, latest wins. It reconnects with the existing backoff (250 ms doubling to 5 s).
/// It returns Ok after detach or `Ended(SessionRemoved)`, or after a 404 on
/// reattach, printing why after the terminal is restored.
/// SIGTERM and SIGHUP restore the terminal and return.
pub async fn run(client: &Client, session: IdOf<Session>) -> Result<()>;

// ui/observer.rs
pub(super) struct Observer {
    pub attachment: Option<IdOf<Attachment>>,
    pub replica: Option<Arc<Replica>>,
    pub screens: HashMap<IdOf<Pane>, Arc<Screen>>,
    pub connected: bool,
}
impl Observer {
    /// The only writer of server-owned state. `Attached` replaces everything.
    /// `State` applies on the same incarnation and a higher revision. `Screen`
    /// replaces that pane's screen. Then `reconcile`. Returns whether anything visible changed.
    pub fn apply(&mut self, event: SseEvent) -> bool;
    /// Drop screens for panes no longer in the replica.
    fn reconcile(&mut self);
    pub fn record(&self) -> Option<&ViewingRecord>;
    pub fn remembered(&self) -> Option<AttachRequest>;   // carries the current size
}

// ui/keys.rs
pub(super) enum Action {
    Frame(InputFrame),      // Key or Paste; dropped while disconnected
    NextPane, PrevPane,     // C-b n, C-b p (FR-008, FR-018)
    NextSession, PrevSession, // C-b ), C-b (
    Detach,                 // C-b d
    None,                   // prefix pressed, or an unbound key after the prefix
}
#[derive(Default)]
pub(super) struct Keys { prefixed: bool }
impl Keys {
    /// `selected` is the pane frames go to. With no pane selected, keys after
    /// the prefix still act and other keys are dropped. C-b C-b sends one C-b.
    pub fn handle(&mut self, event: crossterm::event::Event, selected: Option<IdOf<Pane>>) -> Action;
}

// ui/draw.rs
/// The selected pane's screen at its own size in the top-left corner, a dim
/// pattern over the rest, and the status line on the last row:
/// `session › tab › pane`, then `exited (code)` or `disconnected, reconnecting`.
/// Shows the FR-018 hint when no pane is selected.
pub(super) fn draw(frame: &mut ratatui::Frame, observer: &Observer);
fn paint(screen: &Screen, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer);
pub(super) fn pane_label(pane: &Pane) -> &str;          // name, else title, else command[0] basename
pub(super) fn tab_label(tab: &Tab, position: usize) -> String; // name, else first pane label, else position + 1

// ui/terminal.rs
/// Raw mode, alternate screen and bracketed paste on construction; restore on
/// drop. `install_panic_hook` restores before the default hook prints.
pub(super) struct TerminalGuard { /* ratatui::DefaultTerminal */ }
impl TerminalGuard {
    pub fn enter() -> Result<Self>;
}
impl Drop for TerminalGuard;
```

Navigation runs in the client against its replica:

- `NextPane` and `PrevPane` cycle through the selected pane's tab, or go to the first or last pane when a tab is selected.
- `NextSession` and `PrevSession` cycle through `replica.sessions` in order and select that session's first pane, else the session itself.
- Each one sets the local view's `selection` and marks it dirty. At most one `PUT /api/v0/attach/view` goes out per frame. The screen changes when the replica arrives, not optimistically.

### Moved

#### `crates/ship/src/observe.rs` `Observer` → `crates/ship-client/src/ui/observer.rs` (slice 5)

`apply`, `record` and `remembered` move and grow as shown above. `render`, `render_tab` and `marker` are deleted.

### Replaced

#### `crates/ship-core/src/model.rs` (slice 1; pane metadata in 2)

```rust
/// Optional tab or pane name. Blank or whitespace-only input means none
/// (FR-019). Missing, `null`, `""` and `"  "` all deserialize to `None`. The
/// only place the rule lives.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, ToSchema)]
#[schema(value_type = Option<String>)]
pub struct OptionalName(Option<String>);
impl<'de> Deserialize<'de> for OptionalName;   // Option<String>, blank -> None
impl OptionalName { pub fn get(&self) -> Option<&str>; }

pub struct Tab {
    pub id: IdOf<Tab>,
    #[serde(default)]
    pub name: OptionalName,                      // was String
    pub tabs: IndexMap<IdOf<Tab>, Tab>,
    pub panes: IndexMap<IdOf<Pane>, Pane>,
}

pub struct Pane {
    pub id: IdOf<Pane>,
    #[serde(default)]
    pub name: OptionalName,                      // was String
    pub command: Vec<String>,                    // resolved argv (slice 2)
    pub cwd: String,                             // starting directory (slice 2)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,                   // live program title (slice 2)
    pub status: PaneStatus,                      // (slice 2)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum PaneStatus {
    Running,
    Exited(ExitStatus),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExitStatus {
    pub code: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,                  // resolved question 4
}

impl Creatable for Session { type Parent = ServerRoot; type Input = CreateSession; }
impl Creatable for Tab     { type Parent = TabParent;  type Input = Named<OptionalName>; }
impl Creatable for Pane    { type Parent = Tab;        type Input = PaneInput; }
```

`Named<N>` keeps its shape. Its default type parameter goes away, and tabs and panes use `Named<OptionalName>` for create and rename. `SessionName` is unchanged (FR-021).

#### `crates/ship-core/src/protocol.rs` (slices 1 to 4)

```rust
pub const PROTOCOL_VERSION: u32 = 2;                         // slice 3
pub const INPUT_LINE_MAX: usize = 64 * 1024;                 // slice 4

/// Pane creation input; also the starter pane on session creation.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaneSpec {
    /// argv; absent means the server's login shell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    /// Absolute directory; absent means the server's home directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// POST /panes input, flattened next to `parent`.
pub struct PaneInput {
    #[serde(default)] pub name: OptionalName,
    #[serde(flatten)] pub spec: PaneSpec,
}

/// POST /sessions body. A starter creates one tab holding one pane in the same
/// commit (FR-005, architecture decision 13). `ship session create` sends none.
pub struct CreateSession {
    pub name: SessionName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter: Option<PaneSpec>,
}

pub struct AttachRequest {
    pub session: IdOf<Session>,
    pub selection: Option<NodeId>,                           // None: first pane, else the session (A2)
    pub size: Size,                                          // slice 4: initial viewer size
}

/// PUT /api/v0/attach/view body (slice 4, A4). The client's whole view; the
/// server replaces the record with it. The session is derived from `selection`.
pub struct ViewInput {
    pub selection: NodeId,
    pub size: Size,                                          // whole client terminal
}

pub struct ViewingRecord {
    pub session: IdOf<Session>,                              // derived from selection
    pub selection: NodeId,
    pub size: Size,                                          // slice 4: whole client terminal
}

pub enum SseEvent {
    Attached(Attached),
    State(Arc<Replica>),
    /// The latest screen of a pane in the attachment's selected tab. Sent when
    /// it changes, and for every such pane when the selected tab changes.
    Screen(PaneScreen),                                      // slice 3
    Ended(Ended),                                            // was Ended { reason }
}

pub struct PaneScreen { pub pane: IdOf<Pane>, pub screen: Arc<Screen> }
pub struct Ended { pub reason: EndReason }

/// One NDJSON line of the input POST (slice 4). Internally tagged, so each
/// line reads `{"type": "key", "pane": ..., "key": ...}`. Keys and pastes
/// only; view changes use `PUT /api/v0/attach/view` (A4).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum InputFrame {
    Key(KeyInput),
    Paste(PasteInput),
}

pub struct KeyInput {
    pub pane: IdOf<Pane>,
    /// crossterm's own serde (A5); modifiers serialize as `"SHIFT | CONTROL"`.
    #[schema(value_type = Object)]
    pub key: crossterm::event::KeyEvent,
}
pub struct PasteInput { pub pane: IdOf<Pane>, pub text: String }

// Deleted in slice 5 with their routes: SelectRequest, SwitchSessionRequest.
```

Every new enum variant wraps a named struct; no variant declares its fields inline. `SseEvent::Ended` moves to the same shape. Its JSON doesn't change. The OpenAPI document describes `KeyInput.key` only as an object; that's the accepted cost of A5.

The hand-written `Create<T>` schema becomes generic over any `T::Input: ToSchema`: an `allOf` of `{parent}` and the input's schema. That covers `Create_Tab` and `Create_Pane`. Session creation documents `CreateSession` directly.

#### `crates/ship-core/src/relay.rs` (slice 2)

Adds one sink. Nothing else changes.

```rust
/// Never `Full`; `Closed` once the receiver is gone. For publications that
/// must not drop, such as exit statuses (architecture decision 5).
impl<T: Send + 'static> Sink<T> for mpsc::UnboundedSender<T>;
```

#### `crates/ship-server/src/state.rs` (slices 1 to 6)

```rust
pub struct ServerState {
    incarnation: Uuid,
    revision: u64,
    sessions: Sessions,
    viewers: Viewers,
    runtimes: HashMap<IdOf<Pane>, PaneRuntime>,  // slice 2
    bus: ActorRef<RelayBus>,
}

impl ServerState {
    pub fn new(bus: ActorRef<RelayBus>) -> Self;

    /// Signature as today: `edit(&mut Sessions, &mut Viewers)`. Plus, on both
    /// the success and the failure path, `retain` only runtimes whose pane is
    /// in the current tree. A runtime started for a failed edit is therefore
    /// dropped and torn down (FR-001, A6). On success: if runtimes changed,
    /// publish `PaneHandles` before the replica; after publishing, `apply_sizes`.
    async fn commit<R>(&mut self, edit: impl FnOnce(&mut Sessions, &mut Viewers) -> Result<R>) -> Result<R>;

    /// Start a pane's program before the commit that inserts it (A6). Spawns at
    /// `tab_size(tab)`, else `Size::FALLBACK` (A3), and puts the runtime in
    /// `runtimes`. Returns the pane, `Running` with the resolved command, for
    /// the edit to insert.
    fn start_pane(&mut self, tab: IdOf<Tab>, input: PaneInput) -> Result<Pane>;

    /// Derived, never stored (A3): the min over the sizes of records viewing
    /// `tab` (selected tab, or the selected pane's tab), one row less for the
    /// status line. `None` when nobody views it.
    fn tab_size(&self, tab: IdOf<Tab>) -> Option<Size>;

    /// For each viewed tab, send `PaneCommand::Resize(tab_size)` to its own
    /// panes. Pane tasks drop resizes that don't change their size. Unviewed
    /// tabs get nothing and keep their sizes (FR-010).
    fn apply_sizes(&self);

    /// Keep a selection still inside the session, as today. A removed pane
    /// goes to the next pane of its old tab, else the previous one, among
    /// panes still in that tab (FR-017). Otherwise walk old ancestors as today.
    fn repair(old: &Sessions, new: &Sessions, record: ViewingRecord) -> Option<ViewingRecord>;

    fn handles(&self) -> PaneHandles;
}

impl Actor for ServerState {
    /// Drain `runtimes` and `join_all` their `stop()`s: every pane in the tree
    /// gets the full hangup, grace and kill before the actor finishes stopping
    /// (FR-013, A1). Worst case about GRACE.
    async fn on_stop(&mut self, ..) -> Result<()>;
}

/// Published to the input route's watch.
pub(crate) type PaneHandles = Arc<HashMap<IdOf<Pane>, PaneHandle>>;

// New and changed messages.
pub struct Rename<T: Identified, N = OptionalName> { pub id: IdOf<T>, pub name: N }  // sessions: N = SessionName
pub struct SetView { pub attachment: IdOf<Attachment>, pub view: ViewInput }         // slice 4, A4
pub struct CheckAttachment(pub IdOf<Attachment>);                                     // slice 4
/// 404 if the attachment is inactive. A selection that isn't in the tree
/// leaves the record unchanged (a race with a removal). Otherwise the record
/// becomes `{session_of(selection), selection, size}` and commits. Replies
/// with the record as stored.
impl Message<SetView> for ServerState { type Reply = Result<ViewingRecord>; }
impl Message<CheckAttachment> for ServerState { type Reply = Result<()>; } // 404 if inactive
/// Title or exit. Ignored for panes no longer in the tree.
impl Message<PaneEvent> for ServerState { type Reply = Result<()>; }
// Deleted in slice 5: Select, SwitchSession.
```

Behavior changes:

- `Create<Pane>` calls `start_pane`, then commits the insert. `Create<Session>` with a starter calls `start_pane` for a new tab ID, then inserts the session, one unnamed tab and the pane in one commit (slice 5).
- `Attach` without a requested selection selects the session's first pane in tree order, else the session (FR-004a, A2, slice 5). Session switching is a client-chosen selection sent as a view (A4). `tree.rs` gains `first_pane(session) -> Option<IdOf<Pane>>` and `session_of(node) -> Option<IdOf<Session>>`.
- Existing edit closures don't change.

`Create<Pane>` resolves `cwd`: the request's directory, else `$HOME`. It must be an existing directory, else `Validation`. A spawn failure (missing command, permission denied) is `Validation` with the OS error as the cause, so the CLI prints why and nothing changes.

#### `crates/ship-server/src/attach.rs` (slices 3 and 4)

`attach` keeps its guard, seed and end rules. The `unfold` loop now selects over `replicas.changed()` and `screens.changed()`. Each wake yields a batch:

- `State` when the replica's revision moved;
- `Screen` for each pane of the record's viewed tab whose `seq` differs from what this stream last sent. That includes every pane of a newly viewed tab.

A `HashMap<IdOf<Pane>, u64>` per stream tracks sent sequence numbers and is pruned to the viewed tab. Batches are flattened, fused and mapped to `Event::json_data` as today. The `Attach` message carries the request's `size` into the record (slice 4). `AttachmentHeader` becomes `pub(crate)` for `input.rs`.

The view route replaces `select` and `switch_session` (A4):

```rust
/// PUT /api/v0/attach/view (slice 4). Replaces the attachment's view.
/// 200 with the record as stored; 404 for an inactive attachment; 422 for a
/// malformed body. A selection removed a moment ago is not an error: the
/// record stays as it was and is returned.
pub(crate) async fn view(
    State(app): State<AppState>,
    AttachmentHeader(attachment): AttachmentHeader,
    Json(view): Json<ViewInput>,
) -> Result<Json<ViewingRecord>>;
```

`PUT /api/v0/attach/selection` and `PUT /api/v0/attach/session` are deleted in slice 5, together with the old controls that call them.

#### `crates/ship-server/src/app.rs` and `lib.rs` (slices 2 to 4)

```rust
#[derive(Clone)]
pub struct AppState {
    pub state: ActorRef<ServerState>,
    pub bus: ActorRef<RelayBus>,
    pub replicas: watch::Receiver<Arc<Replica>>,
    pub screens: watch::Receiver<ScreenTable>,     // slice 3
    pub handles: watch::Receiver<PaneHandles>,      // slice 4
}
```

`serve` changes:

- subscribe the handles watch and the screen table;
- subscribe an `mpsc::UnboundedSender<PaneEvent>` and spawn a forwarder that `tell`s each event to the state actor, awaited;
- set `TCP_NODELAY` with `listener.tap_io(|tcp| { let _ = tcp.set_nodelay(true); })` (slice 4);
- register `input::input` at `POST /api/v0/attach/input` and `attach::view` at `PUT /api/v0/attach/view` (slice 4).

`stop_actors` doesn't change (A1). Stopping the state actor runs its `on_stop`, which waits for every pane in the tree to finish teardown, about 2 s at worst for a program that ignores SIGHUP. That fits within the existing 5 s bound. The one gap: a pane removed less than GRACE before shutdown has already dropped its runtime, so nothing waits for it, and a SIGHUP-ignoring program there can survive. When the process exits, the kernel still hangs up every remaining terminal; a throwaway probe on macOS confirmed that, and Linux is unverified. Cyan accepted the gap on 2026-10-06.

Pane tasks for panes removed just before shutdown stop wherever they are. Their last `PaneClosed` publish may fail, which they ignore. The state actor stops first, so late `PaneEvent`s are dropped by design.

#### `crates/ship-client/src/api.rs` and `lib.rs` (slices 1, 2, 4, 5)

```rust
impl Client {
    pub async fn create_session(&self, body: &CreateSession) -> Result<Session>;           // was name
    pub async fn create_tab(&self, parent: IdOf<TabParent>, name: Option<&str>) -> Result<Tab>;
    pub async fn create_pane(&self, parent: IdOf<Tab>, input: &PaneInput) -> Result<Pane>;
    pub async fn rename<T: Resource>(&self, id: IdOf<T>, name: Option<&str>) -> Result<T>;  // null clears
    /// As today; a create carries `starter`, so a new session lands in a shell.
    pub async fn ensure_session(&self, name: &SessionName, starter: &PaneSpec) -> Result<IdOf<Session>>;
    /// Stream `frames` as NDJSON in one POST until they end or the server
    /// responds. Bypasses the two-second timeout; only the reqwest client's
    /// one-second connect timeout applies. Ok on 204.
    pub async fn input(&self, attachment: IdOf<Attachment>,
        frames: impl Stream<Item = InputFrame> + Send + 'static) -> Result<()>;    // slice 4
    /// PUT /api/v0/attach/view; returns the record as stored (slice 4, A4).
    pub async fn set_view(&self, attachment: IdOf<Attachment>, view: &ViewInput) -> Result<ViewingRecord>;
    // Deleted in slice 5: select, switch_session.
}
```

`lib.rs` gains `pub mod ui;` and a private `send_stream` helper for `input`: `build`, then `Body::wrap_stream` over NDJSON lines, then `open` with no timeout.

#### `crates/ship/src/cli.rs`, `commands.rs`, `main.rs` (slices 1, 2, 5)

```rust
pub struct CreateTabArgs {
    /// Session name or ID, or tab ID
    pub parent: TabParentRef,
    #[arg(long)] pub name: Option<String>,                  // was positional
}
pub struct CreatePaneArgs {
    /// Tab ID
    pub tab: IdOf<Tab>,
    #[arg(long)] pub name: Option<String>,
    /// Starting directory; defaults to the current directory
    #[arg(long)] pub cwd: Option<PathBuf>,
    /// Command and arguments; defaults to your login shell
    #[arg(last = true)] pub command: Vec<String>,
}
pub struct RenameArgs<T> { pub id: IdOf<T>, pub name: Option<String> } // omitted clears
```

`commands::pane` resolves `--cwd` (default `std::env::current_dir()`) to an absolute path and sends `PaneInput`. `main.rs` routes `Attach` to `ship_client::ui::run`, passing the starter spec to `ensure_session`. The `Attach` and `Pane` help text no longer say "text tree" or "metadata-only".

#### Proposed dependency settings (slices 1 to 5)

Not real manifests.

```toml
# workspace
axum = { version = "0.8", features = ["http2"] }                       # slice 4
portable-pty = "=0.9.0"                                                # slice 2; pinned like Herdr
ratatui = "0.30"                                                       # slice 3
crossterm = { version = "0.29", default-features = false, features = ["events", "serde"] }  # slice 4, A5
tokio-util = { version = "0.7", features = ["io", "codec"] }           # slice 2
nix = { version = "0.31", features = ["process", "signal", "term"] }   # slice 2
ratatui-ghostty = { path = "vendor/ratatui-ghostty" }                  # slice 1

# ship-core: + crossterm
# ship-server: + portable-pty, ratatui-ghostty, ratatui, tokio-util, nix (exit is watched on a std thread, so no tokio "process")
# ship-client: + ratatui, crossterm with "event-stream", tokio "signal"
# ship: no new dependencies; futures-util and the tokio "sync" feature may become unused
```

`libghostty-vt` comes in through the wrapper from crates.io and needs Zig plus network access at build time. Vendored wrapper and Ghostty versions stay as in the experiment: `libghostty-vt` 0.2.1, with 0.2.2 available. Bumping it is out of scope.

### Deleted

- `crates/ship/src/observe.rs` (slice 5); its `Observer` moves.
- `crates/ship/src/controls.rs` (slice 5), as its header always said.

### Untouched

- `crates/ship-server/src/health.rs`, `routes.rs` except the body and message types above, and `state/tree.rs` apart from `first_pane` and a pane-ID iterator for `retain`.
- `crates/ship/src/diagnostics.rs` and `local.rs`.
- `crates/ship-macros`.
- `AppError`, `err!`, the compression layer, loopback-only binding and the 5 s drain.
- `experiments/`, `docs/research/`, `.scratch/`.

### Resolved questions

Cyan settled these on 2026-10-06 by approving every recommendation. Items that change the locked architecture are recorded there as amendments.

1. **Hangup is sent, not inferred (A1).** The wrapper's reader thread holds a `dup` of the master fd, blocked in `read`, so dropping the session doesn't close the last master fd and no hangup happens. Teardown on removal therefore:
   - sends SIGHUP to the child's process group and to the terminal's foreground group (`tcgetpgrp`), as a real hangup would;
   - waits the grace period;
   - SIGKILLs both groups.

   The shell forwards SIGHUP to its background jobs, which covers SC-003's `sleep 1000 &`. Jobs that ignore SIGHUP and sit in a third group (`nohup cmd &`) survive; that's the "daemonized descendants" gap the architecture already lists. Server shutdown needs no tracker: the state actor's `on_stop` stops every runtime and awaits each one's `done` signal.
2. **The default program is a login shell.** `CommandBuilder::new_default_prog` starts `$SHELL` (else the passwd shell) as a login shell (`argv[0]` = `-zsh`), as tmux does. That loads `~/.zprofile` and macOS `path_helper`. Explicit commands run as given.
3. **Attach without a selection picks the first pane on the server (A2).** FR-004a: the session's first pane in tree order, else the session. Session switching is no longer a server operation: the client picks the next session's first pane from its replica and sends it as a view (A4).
4. **Exit status shape.** `{code, signal?}`. The status line shows `exited (code)`, or `exited (SIGTERM)` when killed by a signal. portable-pty reports code 1 for signal deaths.
5. **Size stays in the replica.** Viewer sizes ride in `ViewingRecord`, and each view change is one commit and replica publish. The client sends at most one view per frame. Move size out of the replica only if resize drags feel slow (architecture risk).
6. **crossterm in `ship-core`,** with the `events` and `serde` features. That brings mio and signal-hook into core and so into the server; they're small. Unverified: whether `default-features = false, features = ["events", "serde"]` builds on its own.
7. **Cell model.** Own `Color` (`Default`, `Indexed`, `Rgb`) and `Attr` list. Underline color and hyperlinks are dropped for this slice.

Further changes from the review, approved with the rest:

- **No edit candidate (A6).** `commit` keeps today's signature. The program starts before the commit, and `commit` retains only runtimes whose pane is in the tree, on success and failure alike.
- **No stored sizes (A3).** Only the viewer's terminal size is stored, on its viewing record. Tab sizes and pane sizes are derived after each commit. A new pane in a tab nobody views starts at 80x24.
- **One view endpoint (A4).** `PUT /api/v0/attach/view` with `{selection, size}` replaces the selection and session routes and the resize input frame.
- **crossterm's own serde (A5).** The planned `ship-core/src/keys.rs` and its mirrors are gone.
- **`PaneHandle` is the sender itself,** and every new enum variant wraps a named struct.

## Build order

Each slice ends with:

- the workspace building;
- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean;
- debug and release builds;
- the listed real workflow against a foreground `ship server --port <p>`, with `--server-url http://127.0.0.1:<p>` on every client.

Slices that touch routes or protocol types rerun the disposable `openapi()` consumer outside the repo. No permanent tests are added. Every slice is run on macOS. Slices 2, 5 and 7 are also run on Linux, or recorded as unverified there.

### 1. Optional names and CLI spellings

Files:

- `vendor/ratatui-ghostty/` and its PROVENANCE (dependency added, not yet used);
- `model.rs`: `OptionalName`, `Tab` and `Pane` names, `Creatable` inputs;
- `state.rs`: `Rename<T, N>`;
- `routes.rs`: body types;
- client `create_tab`, `rename`;
- `cli.rs`: `CreateTabArgs`, `CreatePaneArgs` (name only), `RenameArgs`;
- `commands.rs`.

Delivers: `ship tab create <parent>` and `ship pane create <tab>` without names. `--name` sets one. `rename <id>` with no name, `""` or `"  "` clears it.

Checks:
- JSON shows `"name": null` for unnamed entities, and named ones are unchanged.
- `curl` with `{"name":"   "}` stores `null`.
- A session create or rename with a blank name fails with 422 and changes nothing (FR-021).
- `ship-server` depends on the vendored wrapper and builds, which proves the Zig toolchain and the `libghostty-vt` download on this machine.
- The OpenAPI consumer shows `name` as nullable.

### 2. Every pane runs a program

Files:

- `pane.rs` without capture or commands: spawn, exit watcher, title publishing (only drains events), teardown;
- `relay.rs` unbounded sink;
- `model.rs`: pane metadata, `PaneStatus`, `ExitStatus`;
- `protocol.rs`: `PaneSpec`, `PaneInput`;
- `state.rs`: `runtimes`, `start_pane`, `retain` in `commit`, `on_stop`, `PaneEvent`;
- `lib.rs`: forwarder;
- `tree.rs`: pane-ID iteration;
- client `create_pane`;
- `cli.rs` and `commands.rs`: `--cwd` and `-- COMMAND`.

Delivers: `ship pane create <tab>` starts a login shell in the CLI's directory. `-- sh -c '...'` runs a command. `pane get` reports command, cwd, title and status (FR-022). Removal and shutdown end everything.

Checks:
- `pgrep -P <server>` shows a child per pane.
- `pane create <tab> -- definitely-not-a-command` fails with a readable message, and `session get` is byte-identical.
- `-- sh -c 'printf "\033]0;hello\007"; sleep 100'`: `pane get` shows `title: "hello"` within a frame.
- `-- sh -c 'exit 3'` shows `exited` with code 3.
- **SC-003:** a pane running `sh -c 'sleep 1000 & wait'`, then `pane rm`, and `ps` shows neither process within 3 s. Also `-- sh -c 'trap "" HUP; sleep 1000'`, which needs SIGKILL.
- Removing a tab and a session with running panes ends them all.
- `kill -TERM` on the server with three running panes exits within 5 s, and no pane process remains. The same with one pane running `sh -c 'trap "" HUP; sleep 1000'`, which needs the SIGKILL from `on_stop`.

### 3. Screens on the attach stream

Files:

- `screen.rs`;
- `pane.rs`: `capture`, the frame loop, `PaneClosed`;
- `screens.rs`;
- `app.rs`: `screens`;
- `attach.rs`: merged stream;
- `protocol.rs`: `SseEvent::Screen`, `PROTOCOL_VERSION`.

The old text observer must keep compiling: its `apply` ignores `Screen` until slice 5 deletes it.

Delivers: an attach stream carries every pane screen of the viewed tab, current on attach and paced while it changes.

Checks:
- `curl -N` attach, then `select` a tab holding `-- top`: about one `screen` event per second, each decoding to a 80x24 grid with `top`'s header.
- A quiet pane's screen arrives right after attach without any output.
- `-- yes`: events arrive at no more than about 60 a second for that pane, and the last event after `pane rm` is not stale. Measure `curl` throughput as evidence of zstd size.
- Another tab's panes produce no events.
- Old protocol clients fail the health check, which shows `protocolVersion` 2.

### 4. Input, view and size

Files:

- `protocol.rs`: `InputFrame`, `KeyInput`, `PasteInput`, `ViewInput`, `INPUT_LINE_MAX`, size fields;
- `pane.rs`: commands, equal-resize drop;
- `input.rs`;
- `state.rs`: `SetView`, `CheckAttachment`, `tab_size`, `apply_sizes`;
- `attach.rs`: size on attach, `view` route;
- `lib.rs`: routes, `TCP_NODELAY`, axum `http2`;
- client `input`, `set_view`.

Delivers: NDJSON frames reach panes in order, a view PUT replaces the attachment's selection and size, and tab sizes follow the smallest viewer.

Checks:
- With an attach stream open, `curl -T - -H 'Content-Type: application/x-ndjson' .../attach/input` receives typed `Paste` and `Key` lines from a FIFO. `echo hi` plus `Enter` shows `hi` in the next screen event.
- 64 KiB plus one byte in a line gives 422.
- An unknown pane is ignored.
- An ended attachment gives 404 before any body is read.
- Two attach streams on one tab with sizes 100x30 and 80x24 give screens of 80x23. Detaching the smaller one gives 100x29. Detaching both keeps 100x29.
- `PUT /attach/view` with a pane in another session moves the record there and returns it. With a just-removed pane it returns the unchanged record and 200.
- `curl --http2-prior-knowledge` on `/health` succeeds, and HTTP/1.1 still works.
- The OpenAPI consumer lists `/api/v0/attach/input` and `/api/v0/attach/view`, with `KeyInput.key` as an object.

### 5. The full-screen client

Files:

- `ship-client/src/ui/` (`mod.rs`, `observer.rs`, `keys.rs` with `Frame` and `Detach` only, `draw.rs`, `terminal.rs`);
- client `ensure_session` starter;
- `state.rs`: starter pane, first-pane selection on attach; delete `Select` and `SwitchSession`;
- `attach.rs`, `protocol.rs`, client: delete the selection and session routes, their bodies, and `select` and `switch_session`;
- `main.rs`, `cli.rs`;
- delete `observe.rs` and `controls.rs`.

Delivers: `ship attach work` lands in a working shell (FR-005), and keys and pastes go to it. Detach, disconnect, exit and resize all behave per P1.

Checks:
- **SC-001 on macOS:** `ls --color`, `nvim` with a multi-line paste in insert mode (no staircase), and Pi, resizing the window during each.
- **SC-002:** `top`, `C-b d`, reattach shows it still running. The same after closing the terminal window.
- **Disconnect:** with the Python relay from the roundtrip slice 6 in front, killing the relay keeps the screen and shows `disconnected, reconnecting`, and typing during the outage does nothing. After the restart, screens are current.
- **SC-005:** `kill -TERM` on the client leaves the outer terminal usable without `reset`. So does a temporary forced panic in a probe build (reverted and recorded).
- **SC-004:** an exited program shows `exited (code)`.
- `ship session create x` still makes an empty session, and attaching to it shows the empty state.

### 6. Navigation, selection rules and labels

Files:

- `ui/keys.rs`: navigation actions;
- `ui/mod.rs`: their handling;
- `ui/draw.rs`: labels and the empty-state hint;
- `state.rs`: `repair` pane rule (FR-017).

Delivers: FR-008, FR-017, FR-018 and FR-020.

Checks:
- Three panes in a tab: `C-b n` and `C-b p` cycle through them with wraparound.
- Removing the selected pane from another terminal selects the next pane, or the previous one when it was last, and removing the rest shows the hint.
- `C-b )` and `C-b (` cycle through two sessions in creation order, landing on each one's first pane with one view PUT each.
- In `cat -v`, `C-b C-b` shows one `^B`.
- **SC-008:** an unnamed pane's label follows `nvim`'s title, and clearing a tab name shows the first pane's label.

### 7. Two clients, feel and platforms

Files: none planned. Fixes found here go into the deviation log.

Delivers: the remaining success criteria with evidence.

Checks:
- **SC-006:** two clients of different sizes on one tab show identical screens at the smaller size, with the dim pattern on the larger. Moving the smaller client to another tab lets the first tab grow.
- **SC-007:** typing in a shell and in `nvim` feels at least as fast as tmux on the same machine, and `cat` of a 50 MB file doesn't make typing in another client feel sluggish. Dragging a window edge sends at most about 60 views a second and doesn't make the other client stutter. Record the observations, and profile if it doesn't.
- **SC-009:** rerun slices 2, 5 and 6's workflows on Linux, or record them as unverified there. Windows is recorded as unverified.

## Deviation log

During implementation, record each surprise found, question asked or assumption made, and which papers it touched.

### Slice 1 (2026-10-06, macOS arm64)

**D1. Ghostty comes from libghostty-rs `master` by git rev, not crates.io.** Touches this paper's `vendor/ratatui-ghostty/` entry and its dependency settings. The experiment's `[patch.crates-io]` pointed `libghostty-vt` at an unpublished local libghostty-rs checkout (`8953a740`, Ghostty `22d13172`, Zig 0.16). The vendored wrapper's manifest now names that same commit directly: `libghostty-vt = { git = "https://github.com/uzaaft/libghostty-rs", rev = "8953a740bc378cec3e07e1f6ca949f0595eab19b" }`, which is the current tip of upstream `master`. All five experiment changes are kept. Beyond the experiment copy, only that dependency line changed, plus rustfmt on two adapted tests so `cargo fmt --all --check` passes. All 80 wrapper tests pass in a temporary copy outside the repo. History: slice 1 first used crates.io 0.2.1 and reverted changes 3 and 5, because no published `libghostty-vt` (0.2.1 or 0.2.2) has `Terminal::new(cols, rows)`. Cyan then chose `master` in chat, since crates.io's 0.2.x line still builds Ghostty `a887df42`, which needs Zig 0.15.2.

**D2. Building needs Zig 0.16.x, not the Zig on this machine.** Ghostty `22d13172` declares `minimum_zig_version = "0.16.0"`, and its `requireZig` needs the same major.minor version, so 0.17 is rejected. This Mac has Homebrew Zig 0.17.0. The builds ran with a downloaded Zig 0.16.0 first on `PATH`, kept in the job's temp directory, not installed. A durable Zig 0.16 on macOS and Linux is still needed before the next slice. Open: whether to document it or pin it somewhere in the repo.

**D3. `Cargo.lock` records the git source at `8953a740`.** The `rev` pins it, so no `--precise` step is needed. Moving to a newer crates.io release later means swapping the git dependency for a version, once one ships Ghostty `22d13172` or later.

**D4. The workspace excludes `vendor/ratatui-ghostty`.** A path dependency under the workspace root becomes a member by default, which would put the wrapper under Ship's Clippy and `--all-targets`. `exclude` keeps it out, as this paper intends.

**D5. `SessionName` changed by one condition.** This paper says `SessionName` is unchanged (FR-021), but it accepted whitespace-only names, which FR-021 and the session-structure delta reject. `from_str` now checks `trim().is_empty()` and says "must not be blank". Stored names aren't trimmed.

**D6. Small additions outside the skeleton.** `OptionalName` gained `From<Option<String>>`, used by its `Deserialize` and by the client to build create bodies, so the blank rule still lives once. The generic `Create<T>` schema planned for later landed now, because tab and pane inputs changed in this slice. The text observer prints `name.get()` so its output is unchanged until slice 5 deletes it.

**D7. Accepted schema imprecision.** `Named_OptionalName` lists `name` as required, though the server also accepts a body without it. `Tab.name` and `Pane.name` are optional and nullable as intended.

Evidence, against a foreground `ship server` with `--server-url` on each client:

- `cargo build -p ship` succeeds with the wrapper as a dependency, compiling `libghostty-vt` from the git rev (Zig 0.16.0, see D2).
- `POST /api/v0/tabs` with `{"name":"   "}` and with no `name` both return `"name":null`.
- Session create with `"   "` or `""` and rename with `"  "` or `null` return 422. `ship session create '  '` and `ship session rename work ' '` fail with "session name must not be blank". `session list` is byte-identical before and after.
- `ship tab create work`, `ship tab create work --name '  '` and `ship pane create <tab>` print `"name":null`. `--name editor` sets the name. `pane rename <id>` with no name, `""` and `"  "` clear it, and so does `tab rename <id>`.
- The same named-entity workflow on `main`'s binary and this branch's prints byte-identical JSON once IDs are normalized.
- The disposable `openapi()` consumer, outside the repo, shows OpenAPI 3.1 with `OptionalName` as `["string","null"]`, `Tab.name` and `Pane.name` not required, `Create_Tab` and `Create_Pane` as `allOf` of `{parent}` and `Named_OptionalName`, tab and pane PATCH bodies as `Named_OptionalName`, and session bodies as `Named_SessionName`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass. The probe reruns on the release binary with the same results.
- Size: implementation Rust 3,809 lines (+58); tests 0.
- Linux: not required for slice 1, unverified.

### Slice 2 (2026-10-06, macOS arm64)

**D8. `Size` landed in slice 2.** `Launch` needs a size, so `ship-core/src/screen.rs` exists now with only `Size` and `Size::FALLBACK`. The rest of the file is still slice 3. Every pane starts at 80x24 until slice 4 derives tab sizes.

**D9. `ship-macros` gained `#[actor(on_stop = path)]`.** This paper lists `ship-macros` as untouched and shows `impl Actor for ServerState` with an `on_stop`. Ship's derive generates the whole kameo `Actor` impl, so a hand-written impl would have to copy its `on_panic` policy. The derive now takes an optional path and forwards kameo's `on_stop` to it with the same arguments and result. `ServerState` uses `#[actor(on_stop = Self::stop_panes)]`, and `stop_panes` has kameo's `on_stop` signature. (Code review replaced a first version that called a bare method name with no arguments.)

**D10. `stop_actors` changed by one call.** This paper says `stop_actors` doesn't change (A1). kameo's `wait_for_shutdown` resolves when the mailbox closes, before `on_stop` runs. The first SIGTERM probe exited in 0.07 s and left `sh -c 'trap "" HUP; sleep 1000'` running. `stop_actors` now awaits `wait_for_shutdown_result` for the state actor, which kameo documents as waiting for `on_stop`. The relay still uses `wait_for_shutdown`.

**D11. The grace period waits for the process groups too.** This paper says teardown waits up to GRACE for the exit watcher, then SIGKILLs both groups. Teardown now waits, under one `timeout(GRACE, ..)`, until the child is reaped and both groups are empty, checked with `killpg(group, None)` every 20 ms. Then it SIGKILLs both groups and waits for the reap. Without this, a background job in the child's group that handles SIGHUP slowly would be SIGKILLed as soon as the shell exited, short of the spec's "about two seconds after hangup". The same path covers leftovers of a program that had already exited.

**D12. Pane task shape for slice 2.** `PaneRuntime` has no `handle`, and `run` takes no command channel. Both arrive with commands in slice 4. `PaneTask` also holds the PTY master as `Arc<Mutex<Box<dyn MasterPty + Send>>>`, shared with the wrapper's resizer, so teardown can read the foreground group (`process_group_leader`, i.e. `tcgetpgrp`). Cyan chose this over a `dup`'d `OwnedFd` with nix `tcgetpgrp`, which would need Ship's first `unsafe` block. A poisoned lock fails the resize and skips the foreground group; nothing recovers it. `spawn` returns a named `Spawned { runtime, command }`. If reading the child's pid or starting the exit-watcher thread fails, `spawn` kills the child and returns `Err`, instead of panicking. `PaneTask::event(&mut self)` takes `&mut`, because `SessionHandle` isn't `Sync`.

**D13. The CLI prefers `$PWD` for the default `--cwd`.** On macOS `std::env::current_dir()` returns `/private/tmp` from `/tmp`. The spec scenario expects `/tmp`, so `commands::pane` uses `$PWD` when it canonicalizes to the same directory, as shells do. `--cwd` is still made absolute with `std::path::absolute`. The server rejects a `cwd` that isn't an existing absolute directory with `Validation`, which is 400 like other validation errors.

**D14. `PaneSpec` is registered in `ApiDoc`.** The hand-written `Create<T>` schema inlines `PaneInput`, whose flattened `PaneSpec` became a dangling `$ref`. `PaneSpec` joins `ViewingRecord` in `ApiDoc`'s components.

**D15. Open: a signal death's `signal` isn't a signal name.** portable-pty fills `ExitStatus::signal()` with `strsignal` text and discards the number: `sh -c 'kill -TERM $$'` reports `{"state":"exited","code":1,"signal":"Terminated: 15"}` on macOS, and Linux's text is `Terminated`. Resolved question 4 expects `exited (SIGTERM)`, which this paper's mapping from portable-pty's status can't produce. Recommendation: have the exit watcher reap with `nix::sys::wait::waitpid` on the child's pid instead of `child.wait()`, and take `Signal::as_str()` from `WaitStatus::Signaled`. That's about the same amount of code, and it needs Cyan's decision before slice 5 shows the status line.

**D16. Code review: discard results with `.ok()`.** Cyan's review asked for `.ok()` over `let _ =` for discarded results. AGENTS.md now says so, and every such site in `crates/` was converted, including existing ones in `attach.rs`, `lib.rs`, `ship-client/src/lib.rs` and `observe.rs`. `attach.rs`'s `let _ = &guard;` stays, because it keeps the guard captured rather than discarding a result.

**D17. Unix-only code sits behind `cfg(unix)`.** Cyan asked that platform-specific code be gated and confined so a later port fails to compile at each boundary, and AGENTS.md now says so. Pane teardown moved into a `program` module in `pane.rs`. On Unix it's the D11 group hang-up and kill. Elsewhere it's portable-pty's `ChildKiller::kill` followed by the reap, which doesn't reach processes the program started. `diagnostics::shutdown` (SIGINT/SIGTERM) and the background server's `setsid` (now `detach` in `main.rs`) are gated with no fallback, and `nix` is a Unix-only dependency of `ship` and `ship-server`. Still bare: the vendored `ratatui-ghostty/src/colors.rs`, which uses `std::os::unix` and `libc::poll` unconditionally, so neither crate compiles on Windows yet. The non-Unix `program` module has not been compiled; no Windows target is installed here, and libghostty-vt's build would need it too.

Evidence, against a foreground `ship server` with `--server-url` on each client, debug binary and then the release binary:

- `ship pane create <tab>` from `/tmp` prints `"command":["/bin/zsh"]`, `"cwd":"/tmp"`, `"status":{"state":"running"}`. `pgrep -lP <server>` lists one child per running pane (`zsh`, `sleep`, `bash` for macOS `sh`).
- `pane create <tab> -- definitely-not-a-command` fails with `cannot start definitely-not-a-command: Unable to spawn definitely-not-a-command because: No viable candidates found in PATH ...`, and `session get` output is byte-identical (`cmp`). `--cwd /nope/nothere` fails with `'/nope/nothere' is not an existing absolute directory`, also byte-identical.
- `-- sh -c 'printf "\033]0;hello\007"; sleep 100'`: `pane get` 50 ms later shows `"title":"hello"`. Setting the title to empty removes `title`.
- `-- sh -c 'exit 3'`: `"status":{"state":"exited","code":3}`.
- SC-003: `sh -c 'sleep 1000 & wait'` then `pane rm`: both processes gone within 0.05 s. `sh -c 'trap "" HUP; sleep 1000'`: still alive at 1.5 s, gone at 2.09 s.
- Removing a tab with five running panes, and a session with three panes across nested tabs (one ignoring SIGHUP), leaves no child of the server after 2.5 s.
- `kill -TERM` on the server with three panes (login shell, `sleep`, one ignoring SIGHUP) exits in 2.14 s (debug) and 2.10 s (release) with no pane process left. Before D10 it exited in 0.07 s and leaked the SIGHUP-ignoring one.
- The disposable `openapi()` consumer, outside the repo, shows OpenAPI 3.1 with `Pane` requiring `command`, `cwd` and `status`, `title` optional, `PaneStatus` as a `oneOf` tagged by `state`, `ExitStatus` with `code` and optional `signal`, `Create_Pane` as `allOf` of `{parent}` and `PaneSpec` plus `name`, and no dangling `$ref`. It needed Zig 0.16.0 on `PATH` (D2), downloaded to the job's temp directory.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.
- Not exercised: an interactive login shell's own background job (`sleep 1000 &` typed at the prompt). That needs input, which arrives in slice 4.
- After the review fixes (D9, D11, D12, D16), the same workflows rerun on debug and release builds give the same results: the HUP-ignoring program is alive at 1.5 s and gone by 2.5 s, removing a tab with nested panes leaves none, and SIGTERM with four children exits in 2.05 s (debug) and 2.06 s (release) with no pane process left.
- After D17, the release build gives the same results: background job gone 0.3 s after `pane rm`, the HUP-ignoring program alive at 1.5 s and gone by 2.5 s, and SIGTERM with four HUP-ignoring panes exits in 2.05 s with no pane process left.
- Size: implementation Rust 4,471 lines (+662); tests 0.
- Linux: unverified. No Linux machine was used for this slice.
