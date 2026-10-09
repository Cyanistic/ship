# Pane terminals program

Status: Locked on 2026-10-06. Cyan reviewed the first draft in Plannotator, approved every proposed fix in chat ("let's make all of these proposed fixes!"), and had no notes on revision 2 ("i have no notes for this plannotator review!"). After that, the redundant `title` field on `PaneTask` was removed at Cyan's request. Locking turned up a conflict between A1 as first written (no waiting at shutdown) and product FR-013; Cyan chose the `on_stop` wait ("i feel like that's a lot cleaner"), which resolves it. Written from the locked [product](product.md) and [architecture](architecture.md) papers and the current crates; the review added architecture amendments A1 to A6, recorded in the [architecture paper](architecture.md#amendments). Nothing here is created source, and no snippet has been compiled. Approval doesn't start implementation; Cyan requests that separately, slice by slice. Session text in this paper is superseded by the drop-sessions change (2026-10-08), which replaces sessions with top-level tabs; it is kept as written and marked where a requirement defines sessions.

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

pub(crate) const FRAME: Duration = Duration::from_millis(4); // amended by D45
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
    ///   dirty                   -> mark pending (amended by D45)
    ///   FRAME after the last publish, if pending -> publish()
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
/// It returns Ok after detach, `Ended(SessionRemoved)` or
/// `Ended(ServerShutdown)`, or after a 404 on reattach, printing why after
/// the terminal is restored. (Amended before slice 5, D34.)
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
pub const PROTOCOL_VERSION: u32 = 3;                         // slice 3; 3 after slice 7, D55
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

A `HashMap<IdOf<Pane>, u64>` per stream tracks sent sequence numbers and is pruned to the viewed tab. Batches are flattened, fused and mapped to `Event::json_data` as today (`Event::data` over `serde_json::to_string` after slice 7, D56). The `Attach` message carries the request's `size` into the record (slice 4). `AttachmentHeader` becomes `pub(crate)` for `input.rs`.

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

### Slice 3 (2026-10-06, macOS arm64)

**D18. The wrapper now leaves colors unresolved (vendored change 6).** Touches this paper's `screen.rs` note that named ratatui colors map to `Indexed(0..=15)`. The wrapper resolved every palette color through Ghostty's palette and filled unset foreground and background with Ghostty's defaults, so `capture` would only ever see RGB: clients would show Ghostty's palette instead of their own theme, and a blank cell would serialize as two RGB objects instead of `{}`. Cyan chose to patch the vendored copy in chat. Palette colors now arrive as `Indexed`, unset colors as `Reset` (mapped to `Default`), and a background set by erasing with a color is read from the raw cell. Two wrapper tests were adapted. Recorded in `vendor/ratatui-ghostty/PROVENANCE.md`.

**D19. The wrapper renders during sustained output (vendored change 7).** Touches the architecture risk "the wrapper renders on every output batch". In practice it was the reverse: the session thread drained queued PTY output until the queue was empty and only then rendered and woke the pane task, so `-- yes` published 2 screens in 10 s. It now stops draining after 8 ms, renders and wakes. The risk's own remedy was to patch the vendored copy, so this went ahead without reopening the paper. Recorded in PROVENANCE.

**D20. Open: dev builds compile Ghostty in Zig `Debug` mode.** libghostty-vt-sys picks `Debug` when Cargo sets `DEBUG=true`, which the dev profile does, and Ghostty's page integrity checks then dominate a profile under `yes`. Pacing and size were measured on release builds. Recommendation: set `[profile.dev.package.libghostty-vt-sys] debug = false` in the workspace manifest, so dev builds get `ReleaseFast` Ghostty and feel like release ones. Needs Cyan's decision; nothing changed.

**D21. `tree.rs` gained `viewed_tab`.** This paper lists `tree.rs` as untouched apart from `first_pane` and the pane-ID iterator. The attach stream needs the tab a selection views (the tab, or a pane's tab), and slice 4's `tab_size` needs the same rule, so it lives once in `tree.rs`, and `state::tree` became `pub(crate)`.

**D22. Every pane publishes a screen when its task starts.** This paper doesn't say how a program that never prints gets a screen-table entry. The pane task publishes once before its loop, taking the frame interval's first tick (which completes at once), so the initial screen and the first output still sit a frame apart.

**D23. Small shape changes in `screen.rs` and `AppState`.** `Cell` has no `Default`: the derive would give an empty `symbol`, contradicting the `" "` default, and nothing needs it. A wide character's trailing cell is blank (`" "`, so `{}`), as the emulator reports it, not `""` as this paper's comment says. `AppState.screens` is `pub(crate)`, because `ScreenTable` is crate-private and `AppState` is public.

**D24. Observation: `attach` with an explicit `--server-url` skips the health check.** The slice 2 binary run as `ship --server-url <url>` against this server fails with `expected service ship and protocol 1` and exits 1, as the health-exchange delta requires. But `ship --server-url <url> attach <id>` from that binary attached and printed the tree, because `connect` only checks health for the default local server. The current client has the same gap. Recommendation: have slice 5's `ui::run` path check health for explicit targets too. Not changed in this slice.

**D25. Zig 0.16 was downloaded again.** D2 is still open: this Mac has Zig 0.17, so the protocol-1 build, the OpenAPI consumer and the wrapper tests ran with Zig 0.16.0 from the job's temp directory.

Evidence, against a foreground `ship server` with `--server-url` on each client, release binary unless noted:

- `curl -N` attach on a session, then the old `select` route onto a tab holding `-- top`: one `screen` event at the select and then about one a second, each an 80x24 grid whose first row is `top`'s `Processes: ...` header, with a cursor. The session-selected stream sent no screens before the select. (Debug and release.)
- A quiet pane (`sh -c 'echo quiet-pane; sleep 1000'`) selected in `AttachRequest`: the first event after `attached` is its screen showing `quiet-pane`. The same stream got no screens from `top` running in another tab of the session, over 3 s and again over 5 s.
- `-- yes` for 10 s: 626 screen events on the stream, 630 publishes in the trace log (`RUST_LOG=ship_server=trace`), median gap 16.2 ms and none under 13.4 ms. Before D19: 2 publishes in 10 s.
- Size: `yes` repaints identical screens, 3.9 MB raw against 8.1 KB over zstd in 10 s (about 6.2 KB and 13 B per event). A changing workload (`while :; do ls -la /usr/bin; done`) for 10 s: 626 events, 10.95 MB raw against 205 KB over zstd, about 17.5 KB and 327 B per event, or 20 KB/s.
- Staleness: after `pane rm` on the `yes` pane, the last event is the `state` that removes it, with no screen after it. A pane running `sh -c 'yes | head -n 300000; echo DONE; sleep 1000'` ends with a screen showing `DONE` on its last line.
- Colors on the wire: `\033[31m` gives `{"fg":{"indexed":1}}`, 24-bit gives `{"fg":{"rgb":[1,2,3]}}`, bold underline gives `attrs`, `\033[44m\033[K` gives `{"bg":{"indexed":4}}` across the row, `中` is followed by a `{}` cell, and the program sees `xterm-256color truecolor` and `tput colors` 256.
- `GET /health` reports `"protocolVersion":2`. The slice 2 binary (commit `7aa3342`, protocol 1, built in a temporary worktree) fails the health check and exits 1 (D24 for `attach`).
- `kill -TERM` on the server with two open attach streams and running panes exits in 0.08 s, and both streams end with `ended` `serverShutdown`.
- The disposable `openapi()` consumer, outside the repo, shows OpenAPI 3.1 with `SseEvent` as `attached`, `state`, `screen` and `ended`, `PaneScreen` with `pane` and `screen`, `Ended` with `reason`, `Screen`, `Cell` (no required fields), `Color` as `default`, `indexed` or `rgb`, `Attr`, `Cursor`, `CursorShape` and `Size`, and no dangling `$ref`.
- All 80 vendored wrapper tests pass after D18 and D19 (one ignored, as before), in a copy outside the repo.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass. The probe worktree and temporary target directories were removed.
- Size: implementation Rust 4,827 lines (+356); tests 0.
- Linux: not required for slice 3, unverified.

**D26. Screens reach streams through per-pane watches, not a server-wide table.** Reverses architecture decisions 3 and 6 for screens, and replaces this paper's `screens.rs`, `ScreenUpdate`, `PaneClosed`, `Shown` and per-pane sequence numbers. Cyan chose this in chat after reviewing `Progress`. With one table watch, every frame from any pane woke every attach stream, and each stream kept a per-pane `sent` map of sequence numbers to work out what had changed. A watch already tracks what each receiver has seen, so one watch per pane does that job.

- Each pane task owns a `watch::Sender<Arc<Screen>>`, seeded at spawn with the blank grid. `publish` writes to it, and teardown drops it. That also replaces D22's initial publish: the pane task no longer publishes before its loop. Titles and exits still go over the bus to the state actor.
- `PaneRuntime` carries a `PaneHandle { screen }`. The state actor publishes `LivePanes`, a map from each running pane to its `PaneHandle`, on a watch it writes directly in `retain_runtimes`. That runs before every replica publish and before the `Attached` reply, so a stream always finds every pane of a replica it holds. This brings slice 4's handle map forward, renamed from `PaneHandles`. Slice 4 adds the command sender to `PaneHandle`, and the input route reads `AppState.live`. Publishing it directly rather than through the bus also departs from decision 2 ("one generic sink per server-wide table") for this table.
- `Progress` keeps the replica it sent and a `StreamMap` of the viewed tab's screen watches (`tokio-stream`, new dependency, feature `sync`). Each newer replica re-syncs the map, and a new `WatchStream` yields the current screen first. A wake now comes only from a newer replica or from a watched pane drawing, and each wake yields one event.
- Tasks 3.1 and 3.2 still describe the table shape. Their verification was rerun on this one.

Evidence for D26, release binary against a foreground server with `RUST_LOG=ship_server=trace`:

- A stream attached with a quiet pane (`sh -c 'echo quiet-pane; sleep 1000'`) selected gets its screen showing `quiet-pane` as the first event after `attached`. It gets no other screens over 11 s, while `top`, `yes` and a `yes | head` pane run in other tabs.
- A stream attached at the session root gets no screens. After the old `select` route moves it onto the `top` tab, it gets the `state` and then `top`'s 80x24 screens with the `Processes: ...` header, 8 in about 8 s.
- `-- yes` for 10 s: 628 screen events on the stream, 633 publishes in the trace log, median gap 16.2 ms and none under 13.4 ms. 3.9 MB raw against 8.6 KB over zstd.
- After `pane rm` on the `yes` pane, the last event is the `state` that removes it. The `yes | head -n 300000; echo DONE` pane's last screen ends in `DONE`.
- `kill -TERM` with two open streams exits in 0.04 s, and both streams end with `ended` `serverShutdown`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.
- Size: implementation Rust 4,766 lines (−61 from the table shape); tests 0.
- Linux: unverified.

### Slice 4 (2026-10-06, macOS arm64)

**D27. `CheckAttachment` replies with the current revision, not `()`.** Touches this paper's `state.rs` messages and `input.rs` step 1. The input loop ends when a replica lacks the attachment, but the replica watch lags the state actor: a replica committed before the attach can still be queued on the bus when the input POST arrives, and it lacks the attachment. The reply carries the revision at the check, and the loop only ends on a replica at least that new, as `attach.rs` already does with its own revision.

**D28. A bad input line is answered with an explicit 422.** This paper says 422 on a malformed or oversized line. `ErrorCode` maps only `InvalidStructure` to 422, which means something else, so `input` returns `StatusCode::UNPROCESSABLE_ENTITY` with a `Validation` error body. Every `LinesCodec` error counts, including invalid UTF-8 and a body read error, whose response nobody reads. Blank lines are skipped rather than rejected.

**D29. Sizes come from one `tab_sizes()` map.** This paper names `tab_size(tab)`. `start_pane` and `apply_sizes` both need it, so one pass over the viewing records builds every viewed tab's size. The minimum is taken per dimension, and a size never drops below 1x1, so a client reporting 0 rows can't give a pane a zero-height terminal.

**D30. `tree::session_of` landed in slice 4.** Planned for slice 5, but `SetView` derives the record's session from the selection.

**D31. The size field forced small changes in slice 4.1.** `ViewingRecord.size` and `AttachRequest.size` are required, so `repair`, `Select` and `SwitchSession` carry the record's size, and the old text observer sends `Size::FALLBACK`. Until slice 5 deletes it, a text observer viewing a tab holds it at 80x23 at most. `PaneHandle` (D26) gained `commands`, and the input route routes through `AppState.live`. Pane tasks ignore resizes as well as input once the program has exited.

**D32. Resolved question 6 is verified.** `ship-core` builds on its own with crossterm `default-features = false, features = ["events", "serde"]`. No Ship manifest names bitflags; it arrives through crossterm.

**D33. The OpenAPI consumer reused the built Ghostty.** Built outside the repo with a copy of `Cargo.lock` and `CARGO_TARGET_DIR` set to the workspace's `target`, it reused the existing `libghostty-vt` build and needed no Zig 0.16. D2 is still open.

Evidence, against a foreground `ship server` with `--server-url` on each client, debug binary and then the release binary with the same results:

- A `curl -X POST -T -` input stream fed from a FIFO with a `paste` of `echo hi` and an `Enter` `key` shows `echo hi`, `hi` and a new prompt in the next screen event while the body is still open. Closing the FIFO returns 204.
- A line of exactly 65,536 bytes returns 204. One of 65,537 returns 422 (`max line length exceeded`), and a truncated JSON line returns 422 (`malformed input frame`).
- A frame for an unknown pane is dropped, and the next frame on the same stream reaches its pane.
- An ended attachment gets 404 on the input route within 0.5 s while the request body is still held open, and 404 on `PUT /attach/view`.
- Two attach streams on one tab at 100x30 and 80x24 give 80x23 screens, and the program's SIGWINCH trap prints `23 80`. A pane created in that tab starts at 80x23. Detaching the smaller gives 100x29. After detaching both, a new 100x30 viewer's first screen is still 100x29 with no further SIGWINCH.
- `PUT /attach/view` with a pane in another session returns 200 with the record in that session at the new size. With a pane removed a moment before, it returns 200 with the record unchanged.
- `curl --http2-prior-knowledge` on `/health` answers over HTTP/2, and `--http1.1` still works.
- Through `ship-client`, from a disposable probe outside the repo: `set_view` returns the stored record, and `input` streams a `Paste` and a crossterm `KeyEvent::new(Enter)` that run `echo from-client` in the pane, then returns `Ok` on 204.
- `kill -TERM` on the server with an idle input stream and an attach stream open: the input request completes with 204, the attach stream ends with `serverShutdown`, and the server exits in 0.05 s.
- The disposable `openapi()` consumer, outside the repo, shows OpenAPI 3.1 with `POST /api/v0/attach/input` (`application/x-ndjson` body of `InputFrame`, responses 204, 400, 404, 422, 503) and `PUT /api/v0/attach/view` (`ViewInput`, returning `ViewingRecord`), `KeyInput.key` as a bare `object`, `size` required on `AttachRequest`, `ViewInput` and `ViewingRecord`, and no dangling `$ref`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.
- Size: implementation Rust 5,165 lines (+399); tests 0.
- Linux: not required for slice 4, unverified.

### Before slice 5 (2026-10-06)

Cyan approved four recommendations in chat ("i approve all 4").

**D34. The client exits after `Ended(ServerShutdown)`.** Amends this paper's `ui::run` contract, which listed only detach, session removal and a reattach 404, to match product P1 and the session-observation delta. The client restores the terminal, reports that the server stopped and exits. Task 5.4 implements it.

**D35. Resolves D15: the exit watcher reaps with `waitpid`.** On Unix it takes the signal name from `WaitStatus::Signaled` (`Signal::as_str()`, e.g. `SIGTERM`) instead of portable-pty's `strsignal` text, so the status line can show `exited (SIGTERM)`. Other platforms keep portable-pty's status. Lands in slice 5 with the status line.

**D36. Resolves D20: dev builds compile Ghostty in `ReleaseFast`.** The workspace manifest sets `[profile.dev.package.libghostty-vt-sys] debug = false`, so its build script sees `DEBUG=false`. Release builds were already `ReleaseFast`.

**D37. Resolves D24: `ship attach` checks health for every target.** Slice 5's `ui::run` path runs the health check for an explicit `--server-url` too, so a protocol mismatch is refused instead of misread.

### Slice 5 (2026-10-07, macOS arm64)

**D38. Attach starts the starter pane in the CLI's directory.** `ship attach <name>` for a missing session sends `starter: PaneSpec { command: None, cwd: <current dir> }`, so the shell opens where `ship attach` ran, as tmux and Zellij do. With no `cwd` it would open in the server's home directory.

**D39. The status line shows names only until slice 6.** It reads `session › tab › pane` with a segment only where a name is set, then `exited (code)` or `exited (SIGNAL)` and `disconnected, reconnecting`. Slice 6.3 replaces the missing segments with derived labels.

**D40. SIGINT restores the terminal too.** `ui::run` waits on SIGTERM, SIGHUP and SIGINT. Raw mode turns `C-c` into a key, so SIGINT only arrives from `kill -INT`, and it should restore the terminal like the other two.

**D41. Pane creation checks the tab before starting the program.** The starter pane is started before its tab exists, so `start_pane` no longer looks up the tab. `Create<Pane>` looks it up first and still answers 404 for a missing parent. `Create<Session>` checks name uniqueness before starting the starter, so a conflict starts no program.

**D42. `Attached` keeps screens until the replica says otherwise.** The observer keeps its screens across a reconnect and drops only those whose panes are gone from the new replica, so the last screen stays up through the outage and is replaced as soon as the stream sends the current one.

**D43. The cursor shape goes through crossterm.** The terminal guard sets `SetCursorStyle` from the selected screen's `Cursor` only when shape or blinking changes, and restores `DefaultUserShape` on exit. ratatui draws the cursor position only.

**D44. Zig 0.16.0 is now installed at `/opt/homebrew/bin/zig`.** D2 is still open for the build itself, which no longer needs it here: D33's reuse of the built Ghostty held for this slice's OpenAPI consumer.

Other changes: the `ship` crate drops `futures-util` and tokio's `sync` feature, which only the deleted text observer used. An unbound key after `C-b` is dropped, per this paper.

Evidence, against a foreground debug `ship server` with `--server-url` on each client. A disposable pty harness rendered the client with Python `pyte` in place of a terminal window and checked raw output bytes where pyte doesn't model the alternate screen. Closing the harness's pty stands in for closing the window; it delivers SIGHUP the same way.

- `ship attach work2` from `/tmp` with no `work2` session lands in Cyan's zsh with status ` work2`. `echo hi-$((40+2))` prints `hi-42`, `pwd` prints `/private/tmp`, and the cursor sits at the prompt.
- `C-b d` exits 0 and prints `detached`. The output ends with `\e[?2004l` and `\e[?1049l`, and `stty` afterwards shows `icanon echo`. Reattaching shows `hi-42` again.
- SC-001: `ls -G /` (macOS `ls --color`) shows `Applications` bold in indexed cyan. Resizing the harness to 80x24 makes `stty size` print `23 80`. In `nvim -u NONE -c 'set autoindent'`, a bracketed four-line Python paste in insert mode writes the file byte for byte with no staircase, and a resize gives `&columns &lines` = `110 34`. Pi renders at 100x30, redraws at 70x20, takes typed text and exits cleanly.
- SC-002: `top -s 1` keeps refreshing after `C-b d` and reattach. After the client's pty is closed, the client exits, `top` is still running, and a new attach shows it refreshing.
- Empty state: `ship session create x` then `ship attach x` shows `no tab: ship tab create x`. After `ship tab create x` it shows `no pane: ship pane create tab:…`.
- Disconnect, through a disposable Python TCP relay in the scratchpad: killing it keeps the screen and shows ` work  disconnected, reconnecting`. Text typed during the outage never reaches the pane. Output printed by the shell during the outage appears after the relay restarts, and typing works again.
- SC-004: `sh -c 'echo bye; exit 3'` shows `bye` and ` y  exited (3)`. `kill -TERM $$` shows ` z  exited (SIGTERM)`, and `pane get` returns `{"state":"exited","code":1,"signal":"SIGTERM"}` (D35).
- SC-005: `kill -TERM` on the client exits it with the terminal restored (`icanon echo`, alternate screen and bracketed paste off). A temporary `panic!` on F12 in a probe build restores the terminal before the panic message prints, exits 101 and leaves `icanon echo`. The panic was reverted and the workspace rebuilt.
- 5.4: `kill -TERM` on the server ends the client within 1.3 s with the terminal restored, `server stopped` and exit 0.
- The release binary repeats attach to a new session, typing, detach, reattach and `server stopped`.
- `ship --help` describes attach as a full-screen client; no "text tree" or "metadata-only" remains.
- The disposable `openapi()` consumer, outside the repo, shows OpenAPI 3.1 with only `/api/v0/attach`, `/api/v0/attach/input` and `/api/v0/attach/view` under attach, `POST /api/v0/sessions` taking `CreateSession` (`name` required, `starter` a nullable `PaneSpec`), no `SelectRequest` or `SwitchSessionRequest`, and no dangling `$ref`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.
- Size: implementation Rust 5,491 lines (+326); tests 0.
- Linux: unverified. Slice 5 is one of the slices to run there, and it has not been.

### After slice 5 (2026-10-07)

Cyan approved the fix in chat ("let's go for it! update it!") after a profiling pass on held keys feeling stuttery.

**D45. Panes publish from their own timer arm, at most every 4 ms.** Amends `FRAME` (16 ms) and the pane task's `select!` in this paper. The task used to await the frame tick inside the `dirty` arm, so for up to 16 ms no key or paste reached the program. `lf` redraws twice per key about 15 ms apart, so with a held key some moves landed a frame late and others didn't. Now `dirty` only marks the screen pending, and a separate arm publishes once `FRAME` has passed since the last publish, so commands are never held. A change after a quiet frame publishes at once.

Evidence, macOS arm64, release builds, a pty harness sending `j` to `lf` every 30 ms (Cyan's `KeyRepeat` of 2), two runs each:

- At 120x40 the move interval p95 went from 48 ms to 40 ms, the same as `lf` run directly, and key-to-screen p95 from 20 to 27 ms down to 3 ms.
- At 300x80 the move interval p95 went from 48 ms to 40 ms, and key-to-screen p95 from 32 to 38 ms down to 12 ms. A 1 ms `FRAME` probe build, since discarded, gave the same intervals.
- `yes` for 3 s: the old build showed the program's next output 4.3 s late and used 7.3 s of server CPU at 120x40. The new build is on time with 4.7 s at 120x40 and 4.9 s at 300x80. Client CPU is 0.17 s and 0.72 s. Detach answers in under 10 ms in both builds.
- `sample` on a symbol build at 300x80 shows the rest of the large-terminal cost is whole-screen work: Ghostty's render state and `capture` on the server; `eventsource_stream::parse_event` (about 40% of the client's busy time) and serde's tagged-enum buffering of `SseEvent` on the client. Left alone unless large windows still feel slow.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.


**D46. `ship server stop`.** Cyan asked for it in chat ("can we add that quick stop command somehow? same as the other commands?"). It isn't in the product paper or this one. `POST /api/v0/server/stop` answers 202 and cancels a token that the server's graceful shutdown selects on next to SIGINT and SIGTERM, so a stop runs the same teardown as a signal. `ship server stop` takes `--server-url` like the other client commands but never starts a server, then waits up to ten seconds for the server to refuse connections. `ship server` keeps running a server when given no subcommand, and `--port` conflicts with `stop`. Like every route, it is open to anything that can reach the loopback port, the same trust as removing a session.

Evidence, macOS arm64, release build:

- With a client attached and a pane running `sleep 4242`, `ship --server-url … server stop` exits 0 in 0.06 s with no output. The server logs `stop requested` and exits 0, the client prints `server stopped` and exits 0, and the `sleep` is gone.
- Run again, it prints `ship: no server running at http://127.0.0.1:44140` and exits 1. The default-port case wasn't run, because Cyan's own server was up there; `stop` builds its client without `connect`, so it can't start one.
- A server built before this route answers 404, so `ship server stop` fails with `expected HTTP 202, received HTTP 404` and the server keeps running.
- `ship server --port 5 stop` is refused by clap. `ship server --help` lists `stop`.
- The disposable `openapi()` consumer, outside the repo, lists `POST /api/v0/server/stop` (`stop_server`, 202) and no dangling `$ref`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.

**D47. `SHIP_SERVER_URL`, a global `--server-url`, and one refused-connection message.** Cyan approved the plan in chat ("i like that plan! go for it!"). `--server-url` is `global`, so it also works after the subcommand, and it reads `SHIP_SERVER_URL` when the flag is absent (clap's `env` feature; the flag wins). Either one makes the target explicit for client commands, so they never start or fall back to a local server. `ship server` still refuses the flag but ignores the variable, so a terminal with it exported can still run a server. A refused connection now reads `no server running` for every command, from the client's one error mapping, in place of reqwest's `error sending request`.

Evidence, macOS arm64, debug build, against a foreground server on port 44150:

- `session create a --server-url …` and `tab create a --name t --server-url …` work with the flag last.
- `SHIP_SERVER_URL=…` alone lists sessions. Pointed at an unused port, it fails with `request to http://127.0.0.1:44199/api/v0/sessions failed: no server running` and starts nothing. With both set, the flag wins.
- `ship --server-url … server` and `ship server --server-url …` are both refused. With `SHIP_SERVER_URL` exported, `ship server --port 44151` runs, and `ship server stop` with the variable stops it.
- `pane get` and `server stop` against an unused port both say `no server running`. Bare `ship` still checks the default local server.
- `--help` shows `[env: SHIP_SERVER_URL=]`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.

**D48. Tree lookups moved to `ship-core::tree`.** From Cyan's review of slice 5 ("go for it!"). The client's `observer.rs` had its own recursive tab search because the server's lookups were crate-private. The read-only lookups (`path`, `session_of`, `first_pane`, `tab`, `pane`, `pane_ids`, `pane_owner`, `viewed_tab`, `not_found`) and the `Sessions` and `Tabs` aliases now live in `ship-core/src/tree.rs`. `Replica.sessions` uses `Sessions`. The server's `state/tree.rs` keeps the edits and re-exports the lookups, so its call sites are unchanged. In the same pass, the client's input body uses tokio-stream's `UnboundedReceiverStream` in place of a hand-written `unfold`, `reattach` is now `attach_after`, and a comment says why `next` and `finish` exist. The `Selected` shape is left for slice 6.

Evidence, macOS arm64, debug build, against a foreground server: the no-tab and no-pane hints, typing into a named pane (status ` e › sh`), a relay cut that keeps the screen and drops keys typed meanwhile, typing after the reconnect, and the hint after the selected pane is removed. fmt, Clippy, debug and release builds pass. Size: implementation Rust 5,583 lines; tests 0.

### Slice 6 (2026-10-07, macOS arm64)

**D49. With the session selected, `C-b n` and `C-b p` use the session's first root tab.** FR-018 says they select the first or last pane "of the selected tab", but a selected session has no selected tab. They use the same tab the empty-state hint already names, so the hint and the keys agree. A session whose first root tab has no panes of its own stays put.

**D50. Navigation keeps an unconfirmed target.** This paper says navigation edits the local view's selection. `ui::run` keeps that as `chosen`, the target not yet carried by a view PUT, and computes the next press from it, so two quick presses move twice even before the replica arrives. It's cleared when the PUT that carried it returns, and on disconnect. The screen still follows the replica.

**D51. `(` and `)` are matched with or without Shift.** Terminals differ on whether those keys report Shift, so a key after the prefix counts as bound when its only modifier is Shift. Other modifiers make it unbound, which drops it.

**D52. Known gap: pane titles have no title stack ([#3](https://github.com/Cyanistic/ship/issues/3)).** Ghostty's VT library parses `CSI 22 t` and `CSI 23 t` (push and pop the title) and drops them, with no callback to the wrapper; Ghostty's own app leaves them unimplemented too. tmux and Zellij keep a per-pane title stack. After Neovim quits, an unnamed pane under plain `sh` keeps Neovim's title as its label; Cyan's zsh hides this by setting a title at each prompt. Deferred by Cyan. Options, preferred first: fix it upstream in Ghostty, or scan for the two sequences in the vendored wrapper and keep a stack there.

Evidence, against a foreground debug `ship server` with `--server-url` on each client, then the release binary with the same results. A disposable pty harness rendered the client with Python `pyte`, and view PUTs were counted from the server's request log.

- Three panes in a tab: `C-b n` four times shows `p2`, `p3`, `p1`, `p2`, and `C-b p` four times shows `p1`, `p3`, `p2`, `p1`, each with the pane's own output on screen and one view PUT per press.
- Sessions `a1`, `b1` and `c1` (no tabs), in creation order: `C-b )` goes `b1` (its first pane), `c1` (`no tab: ship tab create c1`, status ` c1`), `a1`, and `C-b (` goes back the other way. Each press sent one view PUT.
- Removing the selected middle pane from another terminal selects the next one (`p2` to `p3`). Removing the selected last pane selects the previous one (`q3` to `q2`). Removing the last remaining pane shows ` a1 › 1` and `no pane: ship pane create tab:…`.
- With the tab selected, `C-b n` selects its first pane. With the session selected, `C-b p` selects the last pane of its first tab, and switching into it with `C-b )` selects the first pane.
- In `cat -v`, `C-b C-b` then Enter shows a single `^B` echoed and a single `^B` printed.
- SC-008: an unnamed `sh` pane in tab `work` shows ` lab › work › sh`. `nvim -u NONE -c 'set title'` changes the label to Neovim's title, and `:e /tmp/…` changes it again. In Cyan's login zsh the label goes `zsh`, then `[No Name] - VIM`, then the shell's `cyan@MacBook-Pro:…` title after `:q`; under plain `sh` it keeps Neovim's title (D52). `tab rename <id>` with no name turns `work` into the first pane's label. Setting the title to `custom` and clearing it switch the label to `custom` and back to `sh`. Naming the pane `ed` and clearing the name with `"  "` switches between `ed` and `sh`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass. No routes or protocol types changed, so the OpenAPI consumer wasn't rerun.
- Size: implementation Rust 5,720 lines (+137); tests 0.
- Linux: not required for slice 6, unverified. Task 7.3 reruns this slice's workflows there.

### Slice 7 (2026-10-07, macOS arm64)

No fixes were needed and no source changed. A disposable pty harness in the session scratchpad rendered each client with Python `pyte`, against a foreground release `ship server` with `--server-url` or `SHIP_SERVER_URL` on each client.

**D53. SC-007's tmux comparison used Zellij.** tmux isn't installed on this Mac. Typing latency was compared against Zellij and against running the program directly. The by-feel judgement in real use is Cyan's and is still open, so task 7.2 stays unchecked. On 2026-10-07, with D57 and D58 in, Cyan checked typing, a flood beside a typing client and window resizes by feel on macOS and reported no issues, so task 7.2 is checked.

**D54. Linux was partly run and is recorded as unverified.** In a Debian trixie aarch64 container under Colima (kernel 6.8, rustc 1.99, Zig 0.16.0), debug and release builds, `cargo fmt --all -- --check` and Clippy with `-D warnings` pass. The scripted slice 2, 5 and 6 workflows passed 43 of 45 checks on the release build. The two failures are both in the relay disconnect check: 1 s after the relay was cut, the status line had no `disconnected, reconnecting`, and typing after the restart didn't show up within 3 s. Whether that's Ship or the harness wasn't investigated, because Cyan said not to pursue Linux in this run. Windows is unverified.

Evidence:

- SC-006: clients at 100x30 and 80x24 on one tab show identical 80x23 regions, and the larger one fills the rest with `·` drawn after `\e[2m`. The smaller client shows no filler. Both typed into the pane (`echo from-a`, `echo from-b`), and `stty size` printed `23 80`. After the smaller client left with `C-b )` (to another session's tab), `stty size` printed `29 100` and the filler was gone. When it came back with `C-b (`, the tab shrank to 80x23 again with identical regions.
- SC-007 typing, key-to-screen over 150 keys 30 ms apart at 120x40, p50 / p95: `sh` directly 3.1 / 3.3 ms, in Ship 3.1 to 5.3 / 3.2 to 5.6 ms across runs, in Zellij 16.6 / 18.6 ms. Neovim insert mode: directly 2.5 / 2.7 ms, in Ship 10.6 / 12.0 ms, in Zellij 16.3 / 18.8 ms. Neovim in Ship is slower than the shell in Ship but faster than Zellij. It wasn't profiled, because nothing was changed.
- SC-007 load: with another client viewing a pane looping `cat` over a 50 MB file of varied base64 lines, typing in a shell in another session stays at p50 3.1 ms, p95 3.2 ms, max 3.3 ms. The server used about 160% CPU while the loop ran, and the viewing client drained 3.9 MB of output in about 5 s using 0.8 s of CPU.
- SC-007 drag: a second client on the typing client's tab, resized every 2 ms for 4.8 s (about 1,660 resizes), sent about 250 view PUTs (about 52 a second). Meanwhile the typing client stayed at p50 3.1 ms, p95 7.9 ms, max 9.3 ms. In one earlier run the max was 48 ms.
- SC-009 on macOS, release build, scripted: slice 2 passes 11 of 11 checks. These cover a login shell in the CLI's directory, one child per pane, an unstartable command and a missing `--cwd` leaving `session get` unchanged, the title `hello`, exit code 3, a background job gone 0.03 s after `pane rm`, a HUP-ignoring program alive at 1.5 s and gone at 1.96 s, tab and nested session removal, and SIGTERM with HUP-ignoring panes exiting in 2.00 s with nothing left. Slice 5 passes 19 of 19, covering attach-or-create, typing, detach and restore (`\e[?1049l`, `\e[?2004l`), reattach, `ls` colors, a resize to `23 80`, a Neovim bracketed paste with `autoindent` written byte for byte, `top` surviving both detach and a closed pty, a relay cut that shows `disconnected, reconnecting` and drops keys typed meanwhile, typing after the reconnect, SIGTERM on the client, `exited (3)`, `SIGTERM` as the signal, both empty-state hints, and server SIGTERM ending the client with `server stopped`. Slice 6 passes 15 of 15, covering pane and session cycling, the removal fallback, the tab-selected `C-b p`, `C-b C-b` in `cat -v`, and the labels. Pi and the forced-panic probe were not rerun.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass. The working tree holds no probes, harnesses or OpenAPI consumers; they lived in the scratchpad. No `#[test]` exists under `crates/`.
- Size: implementation Rust 5,720 lines (unchanged); tests 0.

### After slice 7 (2026-10-07)

Cyan approved the change in chat ("please do it that way so that it has the content field") after a profiling pass on nvim scrolling.

**D55. `SseEvent` is adjacently tagged, and `PROTOCOL_VERSION` goes to 3.** Amends `PROTOCOL_VERSION = 2` in this paper's skeleton; the paper's `SseEvent` doesn't say how it is tagged. `#[serde(tag = "type")]` became `#[serde(tag = "type", content = "data")]`, so each attach event is `{"type": "screen", "data": {"pane": ..., "screen": ...}}`. Internally tagged, serde parsed every event into its generic `Content` tree to find the tag, then deserialized that tree a second time, so each screen cell was allocated, walked and freed twice. Adjacently tagged with `type` first, serde deserializes `data` directly. The server always writes `type` first. A protocol-2 client is refused by the health check. `InputFrame` stays internally tagged; its lines are a few bytes.

Evidence, macOS arm64, release builds:

- Decoding a captured 142 KB nvim screen event, best of 300: 0.80 ms internally tagged, 0.31 ms adjacently tagged.
- nvim scrolling with Ctrl-E at 60 a second, 300 scrolls, CPU per process. `HEAD` (internally tagged, `eventsource-stream`): server 13%, client 36%. This build: server 21 to 25%, client 10 to 11%. Scroll latency p50 is unchanged at about 5.5 ms. Server CPU rises as the client gets faster; the likely cause is that a slow reader let the attach stream skip intermediate screens, so more screens are now encoded and delivered. That wasn't verified.
- `curl -N` on the attach route shows `{"type":"attached","data":{...}}` and `{"type":"screen","data":{...}}`; `/health` reports `protocolVersion` 3; the `HEAD` binary refuses to attach with `expected service ship and protocol 2`. A real client in a 300x80 pty types, draws 400 lines of 299 columns, and shows `disconnected, reconnecting` when the server is killed.
- The disposable OpenAPI consumer, run outside the repo, shows each `SseEvent` variant as an object with required `type` (a one-value enum) and `data` (a `$ref` to `Attached`, `Replica`, `PaneScreen` or `Ended`), with the variant descriptions kept.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.
- Size: implementation Rust 5,730 lines (+10, including the `sse-stream` parser change in `api.rs`, D56); tests 0.

**D56. Attach events are serialized whole, the client parses SSE with `sse-stream`, and the client buffers its frames.** Amends "mapped to `Event::json_data`" in the `attach.rs` section (slices 3 and 4). Approved by Cyan in chat after an A/B run. Three changes, none of them to the wire format:

- The server builds each event with `serde_json::to_string` and `Event::data`. `Event::json_data` hands serde_json axum's `EventDataWriter`, which scans for newlines and copies on every small write serde makes, which is several per screen cell. The whole event was in memory either way. axum has no report of this.
- The client parses the attach stream with `sse-stream` instead of `eventsource-stream`, which rescanned a partial line on every received chunk, so parsing a screen event was quadratic in its size. Blocks without `data` are skipped.
- The client's ratatui backend writes through a 256 KB `BufWriter`, so a frame reaches the terminal in one write when ratatui flushes instead of about every kilobyte. It had no measurable CPU effect in the harness; it's kept because it's small and a frame that arrives whole is less likely to be drawn half-finished, which is unverified.

Evidence, macOS arm64, release builds, nvim scrolling with Ctrl-E at 60 a second, 300 scrolls, three alternating runs each:

- `Event::json_data`: server 20 to 23% CPU, client 9 to 11%. `to_string`: server 11 to 13%, client about 12.5%. The `sse-stream` change is in both and in D55's "this build" figures.
- Adding the `BufWriter`: server 11 to 13%, client 11 to 12%, scroll p95 7.0 to 7.5 ms, against 12 to 13% and 7.0 to 7.6 ms without it.
- Against Herdr and Zellij after these changes, p50 key-to-screen and scroll CPU: scrolling 4.8 to 6.4 ms in Ship, 17.9 ms in Herdr, 19.0 ms in Zellij; CPU 26 to 29% (server plus client), 16% and 4.4%. Typing in `sh` 3.4 to 3.8 ms, 4.8 ms, 13.5 ms; in nvim 5.4 ms, 5.3 ms, 14.5 ms. Herdr ran in its own `shiplat` session.
- Two candidate fixes were tried and dropped: deferring the vendored session thread's render to its drain loop changed nothing measurable, and capturing the screen inside the wrapper's lock raised scroll p95 to 12 to 13 ms.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.
- Size: implementation Rust 5,738 lines (+8); tests 0.

**D57. A pane publishes only a screen rendered since its last capture.** Not in this paper, which has `publish` capture and send on every paced wake. The vendored wrapper gains `SessionHandle::take_if_dirty` (its `PROVENANCE.md` change 8), and `capture` returns `None` when it finds nothing new, so `publish` sends nothing. Approved by Cyan in chat after a throwaway A/B.

Each Neovim keypress in normal mode draws twice, about 1 ms apart: `showcmd` puts the key in the bottom-right corner, then the result erases it. The wrapper renders each frame into its shared buffer and wakes the pane afterward, outside the lock, and the pane read cells and cursor under two more locks. When the pane's read landed after the second render but before its wake, that wake published the same screen 4 ms later. Ghostty already set a dirty flag under the render lock; reading, copying and clearing it under one lock closes the gap. Skipping screens equal to the last one sent was measured too: it also removed the duplicates and doesn't touch the wrapper, but still captured each one. A settle delay before publishing gave one screen per scroll but added about 2.5 ms per keystroke, because tokio's timer rounds up to whole milliseconds.

Evidence, macOS arm64, release builds, nvim scrolling with Ctrl-E at 60 a second, 300 scrolls, alternating runs against `HEAD`:

- Screen events, counted by a second attachment: `HEAD` 600 (2.00 per scroll), 285 of them identical to the one before. This build 300 to 319 (1.00 to 1.06), none identical; the extra few are the `showcmd` frame when the capture lands between the two renders.
- Scroll CPU over seven runs of `HEAD` and eight of this build: `HEAD` server 12.0 to 13.0%, client 11.7 to 12.7%; this build server 7.8 to 9.8%, client 6.7 to 7.7%. Scroll p95 7.2 to 7.6 ms against 7.6 to 8.3 ms.
- Scroll p50: `HEAD` 4.7 to 5.1 ms in every run; this build 5.0 to 5.3 ms in four runs and 6.1 to 6.6 ms in four. The second attachment saw screens sent sooner than before (90% within 3.5 ms of the key), so the slower runs weren't the server holding screens back. Not explained.
- Typing p50, `sh` / Neovim insert: this build 3.1 / 5.1 ms, `HEAD` 3.1 / 5.0 to 5.2 ms. With a 50 MB `cat` loop viewed by another client, typing p50 0.75 ms and server CPU 151% in both.
- A pane running `sleep 30` sends its blank 80x24 screen on attach, and attaching at 80x24 and 50x10 sends 80x23 and 50x9 screens.
- All 80 wrapper tests pass, run in a copy outside the repo (1 ignored, as before).
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.
- Size: implementation Rust 5,741 lines (+3); tests 0.

**D58. The wrapper renders only dirty rows, a pane waits `SETTLE` (1 ms) after a quiet frame, and `capture` keeps its buffer.** Amends D45's "a change after a quiet frame publishes at once", and reverses D57's note that a settle delay was dropped. Approved by Cyan in chat after throwaway A/B builds.

- The vendored wrapper re-rendered every cell on every PTY read. It now renders only the rows Ghostty marks dirty into the buffer it keeps, and every row after a resize or on a frame Ghostty marks fully dirty (its `PROVENANCE.md` change 9).
- With the faster render, Neovim's first frame of a scroll (the `showcmd` change) was published alone and the real frame waited out `FRAME`: 2.00 screens per Ctrl-E, scroll p50 6.1 to 11.2 ms. So the first change after a quiet frame now waits `SETTLE` before publishing. During sustained output `next` is already later, so `SETTLE` changes nothing there. Tokio's timer rounds up to whole milliseconds, so the wait is about 2 ms in practice, and typing pays it. A burst allowance (two publishes back to back, same average rate) also fixed scrolling without the typing cost, but Cyan turned it down for its uneven frame spacing.
- `capture` keeps one `Buffer` in the pane task and replaces it only when the size changes, instead of allocating a grid per capture. Tried alone before `SETTLE`, it caused the same double screens; with it, it doesn't.

Evidence, macOS arm64, release builds, one run per build, so differences of about a point or 1 ms are noise. Columns are the build before the wrapper change, the wrapper change alone, plus `SETTLE`, and plus the kept buffer (this build):

| | before | incremental | + settle | + kept buffer |
|---|---|---|---|---|
| screens per Ctrl-E | 1.00 | 2.00 | 1.00 | 1.00 |
| Ctrl-E scroll p50 | 6.1 ms | 11.2 ms | 8.5 ms | 9.9 ms |
| scroll CPU | ~21% | 29.6% | 20.6% | 16.9% |
| typing p50, `sh` / Neovim insert | 3.9 / 6.4 ms | 3.2 / 4.9 ms | 5.4 / 8.1 ms | 5.7 / 7.7 ms |
| CPU: tick / spinner / matrix / full colour | 14.7 / 9.2 / 13.0 / 28.6% | 11.7 / 7.2 / 11.8 / 26.8% | 11.6 / 7.0 / 11.2 / 19.6% | 11.5 / 7.0 / 11.1 / 19.4% |

- A check build that rendered every row after each incremental render found zero differing cells across about 67,000 renders: the workloads above, Neovim, a `cat` flood and drag-resizes. Workloads and methods are in `docs/research/screen-cost.md`.
- Screens sent while scrolling are byte-for-byte the same size with and without the kept buffer.
- All 80 wrapper tests pass, run in a copy outside the repo (1 ignored, as before).
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds pass.
- Size: implementation Rust 5,755 lines (+14); tests 0.
