# Server-held selection and ordered input program

Status: Locked on 2026-10-13, approved by Cyan in chat ("perfect! go for it!") after Plannotator review, which accepts G1 to G4 as recommended. Revision 2 followed that review: `Request` and `Local` replace `Pending` and the nine-parameter turn, and bindings name their `action` and may set a `hint`, following the same-day amendment to the [product](product.md) and [architecture](architecture.md) papers. Written on 2026-10-13 from those papers, against the crates at `87cd38b`. Nothing here authorizes implementation.

Settled gates, each accepted as recommended:

- **G1, a press on a pane while the lane is busy.** Today a press selects its pane when that pane isn't selected, and otherwise goes to the program. That check uses the selection at press, which can be stale behind a pending create. Recommendation: while the lane has items waiting, a press on any pane is a select; when the lane is idle, today's rule applies. This satisfies P1's "the click wins" in one line. The cost: a click meant for a program, made while a create is pending, selects instead of reaching the program. Alternative: decide at the turn, which moves the drag-tracking state (`Mouse.held`) into the lane. Settled as recommended: "the others sound good to me."
- **G2, how `ship view select` says "nothing".** Recommendation: `ship view select --attachment ID NODE`, or `--none` instead of `NODE`. Omitting both is an error, so a typo can't clear a selection. Alternative: omitting `NODE` clears. Settled as recommended: "perfect! go for it!"
- **G3, a lane select that loses a race.** A navigation or click resolves on fresh state, but another client or a script can remove the target before the request lands, and the server answers 404. Recommendation: the lane treats a 404 from its own select as "nothing to do" and shows no error, matching FR-014. Scripts still get the 404. Alternative: show it like any other failure. Settled as recommended: "the others sound good to me."
- **G4, the delaying proxy for SC-001.** Recommendation: a throwaway TCP proxy of about 40 lines in `experiments/latency-proxy/`, outside the workspace and the budget, delaying each chunk 500 ms each way. Remove it once its evidence is recorded, as with `scrollback-probe`. Alternatives: `toxiproxy`, which isn't installed; or `tc netem` on loopback, which is Linux-only and needs root. Settled as recommended: "the others sound good to me."

## Rationale

This is a document-only plan. Nothing below exists as source yet, and none of it has been compiled.

The skeleton follows the architecture's structure. The lane is a new client module that holds items and the gate. `drive` keeps the `select!` loop and does one turn at a time through it. Navigation, memory and folds stay where they are and are called at the turn. The server gains one route module for views and a `select` field on both creates, both handled inside `commit`.

Applying `docs/agents/code.md`:

- **Rent the mechanism:** `strum` names the action variants for the fallback hint, so serde can derive `Binding` and the raw-TOML hint reader goes.
- **Combine only what needs the result:** `select` is a create parameter because it needs the new ID and a half-done state is unwanted. Unzoom on select is an invariant repair inside the select commit, not a parameter.
- **Name your shapes:** `Item` and `Turn` name what the lane holds and what running one item did, instead of tuples or flags. `Local` names the client-local state `drive` now keeps in loose variables, so the turn is a method on it rather than a function of nine parameters. `Request` names an in-flight request; the existing `Pending` alias, an `Option` around it, goes, so no type admits "waiting on nothing".
- **Earn every layer:** the lane owns only order and the gate. Resolution stays in `Local::turn`, which has the observer, memory, sidebar and input sender in hand (architecture shape C was rejected for this reason). The lane is drained after every event rather than polled as a `Stream`: the gate opens only after something `drive` just handled (a reply, a replica update, a new item), and a stream would need its own copy of the replica's revision to wake itself. Agreed in chat: "i'd keep your design then."

One correction to the architecture's size list: `Scope::Keys` keeps `tabs`, because `tab_target` and `current_tab` still look up a pane's owner in it. Nothing else in that list changes.

- **`ship-core`:** `SelectView`, `ViewInput` without `selection`, `select` on `CreateTab` and `CreatePane`, `select` and `attachment` on the two create commands, protocol version 10.
- **`ship-server`:** `views.rs` with three routes, the `ListViews`, `GetView` and `Select` messages, the create handlers' selection, and `SetView` reduced to area.
- **`ship-client`:** single-action bindings, `ui/lane.rs`, `Keys::handle` without pane stamping, `Local` and its turn, area-only view PUTs, three `Client` methods.
- **`ship`:** `ship view list | selected | select`.

Estimated change: within 150 lines either way of today's implementation Rust, unmeasured. By area:

| Area | Lines |
| --- | --- |
| Bindings: derived `Binding`, `Action::hint` replacing `action_hint` and the hand-written deserializer | about −25 |
| Views: wire types, routes, actor messages, API, CLI | about +110 |
| Create `select`: commands, bodies, handlers, `execute` | about +40 |
| Lane, `Local` and its turn, minus `chosen`, `putting`'s selection, `creates` and per-binding cancellation; most of `Local` is moved from `drive` | about +40 |

## Skeleton map

### Proposed settings

The default keys become tables with one named `action` (FR-018), and the four creates select (FR-011):

```toml
[client.modes.normal.keys]
"alt-n"       = { action = { server.tab.create = { starter = "shell", select = true } } }
"alt-shift-n" = { action = { server.tab.create = { starter = "shell", child = true, select = true } } }
"alt--"       = { action = { server.pane.create = { direction = "down", select = true } } }
"alt-|"       = { action = { server.pane.create = { direction = "right", select = true } } }
"alt-x"       = { action = { server.tab.close = {} } }
"alt-left"    = { action = { client.tab.prev = {} } }
# …every other binding: brackets dropped, the action wrapped in `action = { … }`
```

The defaults set no `hint`; each falls back to `Action::hint`, which gives the same words as today. A user file sets one with `hint = "new tab"` beside `action`, and unbinds with `{}`: `"alt-q" = {}`. A key other than `action` and `hint` fails `ship config check` with its line and column. Cyan waived strict list rejection during slice 1: leave Serde's incidental positional forms alone, without a parser guard.

New dependency: `strum` with the `derive` feature, in the workspace and in `ship-core` and `ship-client`, for `IntoStaticStr` on the action enums. Not optional in `ship-core`, since the client needs the names from `Command`.

`PROTOCOL_VERSION` goes from 9 to 10 in slice 2. Slices 3 and 4 extend version 10 on the same branch.

### Added

#### `crates/ship-client/src/ui/lane.rs` (slice 4)

```rust
//! The lane: everything the user does, in order, one turn at a time.

use std::collections::VecDeque;

use crossterm::event::KeyEvent;
use ship_core::{id::{IdOf, Attachment}, model::{NodeId, Tab}, protocol::InputFrame};

use super::{Request, observer::Observer};
use crate::keymap::Action;

/// One thing the user did. Carries what was done, not a target that depends
/// on the selection; that is resolved at the item's turn (FR-013).
pub(super) enum Item {
    /// A bound action, including `client.send` and `client.detach`.
    Action(Action),
    /// A key for whatever pane is selected at its turn, or a tab's landing pane (FR-005).
    Key(KeyEvent),
    Paste(String),
    /// A pane or sidebar row that was clicked. Does nothing if gone by its turn (FR-014).
    Select(NodeId),
    Fold(IdOf<Tab>),
    /// A mouse frame for a program; names its pane already.
    Mouse(InputFrame),
}

/// What running one item did, which tells the lane what happens next.
pub(super) enum Turn<'a> {
    /// Ran on the client or wrote a frame; the next item may run now.
    Done,
    /// Started a request; the gate stays shut until it replies and the
    /// replica catches up.
    Wait(Request<'a>),
    /// Nothing ran yet; run these first, in order, then this item again.
    /// Zoomed navigation puts an unzoom ahead of itself (architecture
    /// decision 6), and swap puts its resolved swap ahead.
    First(Vec<Item>),
    Detach,
}

#[derive(Default)]
pub(super) struct Lane<'a> {
    items: VecDeque<Item>,
    running: Option<Request<'a>>,
}

impl<'a> Lane<'a> {
    pub fn push(&mut self, item: Item);
    /// Put `items` at the front, in order (`Turn::First`).
    pub fn first(&mut self, items: Vec<Item>);
    /// Whether anything is waiting or running; G1 reads it at press.
    pub fn busy(&self) -> bool;
    /// The next item when nothing is running and the replica has caught up
    /// to `written` (architecture decision 1). Takes it off the lane.
    pub fn next(&mut self, observer: &Observer, written: u64) -> Option<Item>;
    pub fn wait(&mut self, request: Request<'a>);
    /// The running request's result, or never while none runs. Clears it.
    pub async fn finished(&mut self) -> Result<()>;
    /// On disconnect: drop every item and the request in flight (decision 8).
    pub fn clear(&mut self);
}
```

#### `crates/ship-server/src/views.rs` (slice 2)

```rust
//! Each attachment's view, readable and selectable by anyone holding its ID.

use axum::{Json, extract::{Path, State}, response::Response};
use ship_core::{Result, id::{Attachment, IdOf}, protocol::{SelectView, ViewingRecord}};

use crate::{AppState, state::{GetView, ListViews, Select}};

/// GET /api/v0/views: every attachment's record, keyed by ID, in attach order.
/// 200 `IndexMap<String, ViewingRecord>`; 503.
pub(crate) async fn list_views(State(app): State<AppState>) -> Result<Response>;

/// GET /api/v0/views/{attachment}. 200 `ViewingRecord`; 400 malformed ID; 404; 503.
pub(crate) async fn get_view(
    State(app): State<AppState>,
    Path(attachment): Path<IdOf<Attachment>>,
) -> Result<Response>;

/// PUT /api/v0/views/{attachment}/selection, body `SelectView`. 200 the record
/// as stored; 404 for an unknown attachment or node, changing nothing (FR-002);
/// 422 malformed body; 503.
pub(crate) async fn select(
    State(app): State<AppState>,
    Path(attachment): Path<IdOf<Attachment>>,
    Json(body): Json<SelectView>,
) -> Result<Response>;
```

#### `crates/ship/src/cli.rs`, `ship view` (slice 2)

```rust
#[derive(Subcommand)]
pub enum Command {
    Server(ServerArgs),
    #[command(flatten)]
    Action(ServerAction),
    /// List clients' views, or read or set one client's selection
    #[command(subcommand)]
    View(ViewCommand),
    #[command(subcommand)]
    Config(ConfigCommand),
}

/// CLI-only: a binding acts on its own client through navigation instead.
#[derive(Subcommand)]
pub enum ViewCommand {
    /// Print every attached client's view as JSON, keyed by attachment ID
    List,
    /// Print a client's selection as JSON: a tab or pane ID, or null
    Selected(ViewTarget),
    /// Select a tab or pane for a client, or nothing with --none (G2)
    Select {
        #[command(flatten)]
        view: ViewTarget,
        #[arg(required_unless_present = "none", conflicts_with = "none")]
        node: Option<NodeId>,
        #[arg(long)]
        none: bool,
    },
}

#[derive(Args)]
pub struct ViewTarget {
    /// Attachment ID, from `ship view list`
    #[arg(long, env = "SHIP_ATTACHMENT_ID", value_name = "ID")]
    pub attachment: IdOf<Attachment>,
}
```

`commands.rs` gains `pub async fn view(client: &Client, command: ViewCommand) -> Result<()>`, printing JSON as the other commands do.

### Moved

None.

### Replaced

#### `crates/ship-client/src/keymap.rs`, one action per binding (slice 1)

```rust
/// One action and its status-line hint. `{}` sets neither and unbinds the
/// chord, so the key reaches the program. Another key is a config error,
/// from serde itself; strict list rejection was waived during slice 1.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    #[serde(default)]
    pub action: Option<Action>,          // was `actions: Vec<Action>`
    /// The file's hint. Completed on load to `chord hint`, falling back to
    /// `Action::hint`, as `normalized` does today with the derived one.
    #[serde(default)]
    hint: Option<String>,
}

impl Binding {
    pub fn enters_mode(&self) -> bool;   // matches `Some(Action::Client(ClientAction::Mode(_)))`
}

// Each action enum gains `strum::IntoStaticStr` with
// `#[strum(serialize_all = "snake_case")]`, matching its serde names:
// `ClientTab`, `ClientPane`, `Sidebar`, `ConfigAction`, and in `ship-core`
// `TabCommand`, `PaneCommand` and `Direction`.

impl Action {
    /// A word for the status line when the binding sets no hint: the mode's
    /// name, the direction, the key sent, else the verb. The same words
    /// `action_hint` read from the TOML.
    pub fn hint(&self) -> Cow<'static, str>;
}
```

Removed: the hand-written `Deserialize for Binding` and `action_hint`. `normalized` completes `hint` from the file's or the action's, and `Mode::hints` skips unbound chords. `From<ModeFile> for Mode` keeps only bindings with an action. `over_defaults` doesn't change.

#### `crates/ship-core/src/command.rs`, create selects (slice 3)

```rust
pub mod tab {
    pub struct Create {
        // …as today
        /// Select the new tab, or its starter pane, for --attachment
        #[cfg_attr(feature = "clap", arg(long))]
        pub select: bool,
        /// Attachment to select for; set by the client on keys
        #[cfg_attr(feature = "clap", arg(long, env = "SHIP_ATTACHMENT_ID", value_name = "ID"))]
        #[serde(skip)]
        pub attachment: Option<IdOf<Attachment>>,
    }
}

pub mod pane {
    pub struct Create {
        // …as today, plus the same `select` and `attachment`
    }
}
```

The config writes `select = true`; `attachment` can't be set from the config.

#### `crates/ship-core/src/protocol.rs` (slices 2 and 3)

```rust
pub const PROTOCOL_VERSION: u32 = 10;               // slice 2

/// PUT /views/{attachment}/selection body, e.g. `{"selection": "pane:…"}`;
/// `null` selects nothing.
pub struct SelectView {
    pub selection: Option<NodeId>,
}

/// PUT /attach/view body: the client's area only. Selection moved to
/// `SelectView` (architecture decision 3).
pub struct ViewInput {
    pub area: Size,                                   // `selection` removed in slice 4
}

pub struct CreateTab {
    // …as today
    /// Attachment whose selection moves to the new tab, or its starter pane,
    /// in the same commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<IdOf<Attachment>>,             // slice 3
}

pub struct CreatePane {
    // …as today, plus the same `select`               // slice 3
}
```

#### `crates/ship-server/src/state.rs` (slices 2 to 4)

```rust
/// GET /views.
pub struct ListViews;
pub struct GetView(pub IdOf<Attachment>);
pub struct Select {
    pub attachment: IdOf<Attachment>,
    pub selection: Option<NodeId>,
}

impl Message<ListViews> for ServerState { type Reply = Viewers; }
impl Message<GetView> for ServerState { type Reply = Result<ViewingRecord>; }

impl Message<Select> for ServerState {
    type Reply = Result<ViewingRecord>;
    /// 404 for an unknown attachment or node. A pane hidden by its tab's zoom
    /// unzooms that tab in the same commit (FR-004), so `repair` keeps it.
    async fn handle(&mut self, select: Select, _: &mut Context<Self, Self::Reply>) -> Self::Reply;
}

/// `CreateTab` and `CreatePane`: when `select` is set, the commit fails with
/// 404 before placing anything if the record is missing (FR-009), and sets
/// its selection to the new pane, or the tab when it has none (FR-007).
fn select_created(viewers: &mut Viewers, attachment: IdOf<Attachment>, node: NodeId) -> Result<()>;

/// `SetView` keeps only the area (slice 4). The "selection no longer in the
/// tree leaves the record" rule goes with the field.
```

#### `crates/ship-client/src/api.rs` (slices 2 and 4)

```rust
impl Client {
    pub async fn views(&self) -> Result<IndexMap<IdOf<Attachment>, ViewingRecord>>;
    pub async fn view(&self, attachment: IdOf<Attachment>) -> Result<ViewingRecord>;
    pub async fn select(&self, attachment: IdOf<Attachment>, body: &SelectView) -> Result<ViewingRecord>;
    /// Area only after slice 4.
    pub async fn set_view(&self, attachment: IdOf<Attachment>, view: &ViewInput) -> Result<ViewingRecord>;
}
```

#### `crates/ship-client/src/execute.rs` (slice 3)

```rust
/// The client's selection, the tabs it was made in, and its own attachment,
/// which `select = true` creates select for.
pub struct KeyScope {
    pub selection: Option<NodeId>,
    pub tabs: Tabs,
    pub attachment: Option<IdOf<Attachment>>,
}
```

`execute` turns `select` into the body's attachment: the command's `--attachment`, else the key scope's, else "pass --attachment or set SHIP_ATTACHMENT_ID" before any request (FR-008).

#### `crates/ship-client/src/ui/keys.rs` (slice 4)

```rust
pub(super) enum Handled {
    /// For the lane: a bound action, or an unbound key or paste with no pane attached.
    Queue(Item),
    /// Applies at press (FR-015): the sidebar toggle or a config reload.
    /// A mode change applies inside `handle` and returns `None`.
    Now(ClientAction),
    None,
}

impl Keys {
    /// No longer takes the selected pane: an unbound key in normal mode is
    /// `Queue(Item::Key)` whatever is selected now.
    pub fn handle(&mut self, event: Event) -> Handled;
}
```

#### `crates/ship-client/src/ui/mouse.rs` (slice 4)

`Mouse::route` gains `busy: bool` (G1). While busy, a press on any pane is `Clicked::Select`. `Clicked` maps one-to-one onto `Item::Select`, `Item::Fold` and `Item::Mouse`.

#### `crates/ship-client/src/ui/mod.rs` (slices 1, 3 and 4)

```rust
// Removed in slice 4: `chosen`, `putting`'s selection, `queued`, `running`,
// `moved`, the `creates` special case (slice 3). Kept: `navigate`, `Step`,
// `zoomed_pane`, `attach_after`, `next`, `finish`, `view_area`.

/// An in-flight request, polled by `drive`'s `select!`. Replaces `Pending`;
/// the slots that held one (`reconnect`, `sending`, `putting`) become
/// `Option<Request<…>>`.
type Request<'a, T = ()> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

/// What the client knows and keeps for itself between turns. `drive` owns
/// one and its `select!` arms update the fields that were loose variables.
struct Local {
    observer: Observer,
    memory: Memory,
    sidebar: Sidebar,
    keys: Keys,
    mouse: Mouse,
    /// Frames for the current input POST; `None` while disconnected.
    input: Option<mpsc::UnboundedSender<InputFrame>>,
    message: Option<String>,
}

impl Local {
    /// Run one item: resolve it against the current replica, memory and
    /// folds, then do it. A failure sets `message` and returns `Done` (FR-016).
    fn turn<'a>(&mut self, client: &'a Client, item: Item) -> Turn<'a>;
    /// The selected pane, or the landing pane of a selected tab (FR-005).
    fn typed(&mut self, key: KeyEvent) -> Turn<'static>;
    fn pasted(&mut self, text: String) -> Turn<'static>;
    fn send(&mut self, frame: InputFrame) -> Turn<'static>;
    /// `client.select` on this attachment; a 404 is `Done` (G3).
    fn select<'a>(&mut self, client: &'a Client, node: NodeId) -> Turn<'a>;
    fn fold(&mut self, tab: IdOf<Tab>) -> Turn<'static>;
    /// Client actions, navigation (a select, or `First` with an unzoom when
    /// zoomed) and server commands through `execute`.
    fn act<'a>(&mut self, client: &'a Client, action: Action) -> Turn<'a>;
}

/// The pane typing goes to: the selected pane, else the landing pane of the
/// selected tab, else none.
fn typing_pane(observer: &Observer, memory: &Memory) -> Option<IdOf<Pane>>;

/// The area PUT, when the record's area differs. Selection is never sent.
fn unsent(observer: &Observer, area: Size) -> Option<(IdOf<Attachment>, ViewInput)>;
```

In `drive`: terminal events become lane items or `Now` actions; a cut connection calls `lane.clear()`; `lane.finished()` is the `select!` arm for the running request. After each event, `while let Some(item) = lane.next(..)` runs `local.turn`, handing `Wait` to `lane.wait`, `First` to `lane.first` and returning on `Detach`, until the gate shuts.

### Untouched

- `ui/memory.rs`, `ui/sidebar.rs`, `ui/draw.rs`, `ui/observer.rs`: called at the turn instead of at press. `Observer::remembered` still seeds reconnects (FR-006).
- `input.rs` on the server: frames keep naming their pane.
- The repair rules in `state.rs` (`repair`, `repair_removed`).
- The vendored wrapper.

## Build order

1. **Bindings hold one named action.** Files: workspace `Cargo.toml` and the `ship-core` and `ship-client` manifests (`strum`), `command.rs` (`IntoStaticStr` on `TabCommand`, `PaneCommand`, `Direction`), `keymap.rs`, `keymap/defaults.toml`, `ui/keys.rs` (`enters_mode`), `ui/mod.rs` (`queued` holds actions, not lists). Delivers FR-018. Checks: `ship config check` accepts `"alt-t" = { action = { server.tab.create = {} } }`, `"alt-t" = { action = { … }, hint = "new" }` and `"alt-q" = {}`, and rejects `"alt-t" = { server.tab.create = {} }` and `"alt-t" = { action = { … }, hnt = "x" }` with line and column. Strict list rejection is waived: record the incidental acceptance of `"alt-t" = [{ server.tab.create = {} }]` without guarding it. Every default key still works in a running client, `alt-q` unbound with `{}` reaches the shell, and the `resize` mode hints read `h left  j down  k up  l right  esc normal` as before. A set `hint` shows in place of the derived word.
2. **Views and `ship view`.** Files: `protocol.rs` (`SelectView`, version 10), `state.rs` (`ListViews`, `GetView`, `Select`), `views.rs`, `lib.rs` (routes), `api.rs`, `cli.rs`, `commands.rs`, `main.rs`. Delivers FR-001 to FR-004, P3 and U-1. The client still sends its selection on the view PUT during this slice. Checks (SC-003): with a client attached, `ship view list` shows it. `ship view select` to a pane, a tab and `--none` each move the client within a frame, and `ship view selected` prints each. Selecting a pane hidden by zoom unzooms the tab for a second client too. An unknown attachment or node exits non-zero and changes nothing. `ship server status` reports protocol 10.
3. **Creates select.** Files: `command.rs`, `protocol.rs` (`select` on both bodies), `state.rs` (`select_created`), `execute.rs` (`KeyScope.attachment`), `defaults.toml` (`select = true`), `ui/mod.rs` (the `creates` path removed). Delivers FR-007 to FR-011, P2. Checks (SC-004): `ship pane create --tab T -- sleep 9` moves no client. With `--select --attachment A` the client lands on the new pane, and a second client never sees it unselected (watch `ship view list` in a loop). `--select` without an attachment fails before creating, and an unknown attachment creates nothing. `alt-n`, `alt-shift-n`, `alt--` and `alt-|` still land on what they made.
4. **The lane.** Files: `ui/lane.rs`, `ui/keys.rs`, `ui/mouse.rs`, `ui/mod.rs`, `protocol.rs` (`ViewInput` without `selection`), `state.rs` (`SetView` area only), `api.rs`. Throwaway: `experiments/latency-proxy/` (G4). Delivers FR-005, FR-012 to FR-017, P1. Checks, through the proxy at 500 ms each way (SC-001): ten runs each of `alt-n` + `echo hi` + Enter and `alt-|` + `alt-f`, all landing right. A click on another pane during a pending create ends there. A failed action shows its error, and keys typed after it still arrive. Killing the proxy mid-burst drops what wasn't sent and replays nothing. `alt-q` after typing detaches after the typing lands. A script's `ship view select` survives a terminal resize. Against a local server (SC-002), typing, navigation and key repeat feel unchanged in daily use.
5. **Docs, specs and size.** Files: this change's `proposal.md`, `tasks.md` and spec deltas (`keymap`, `session-observation`, `mouse-input`, `config-file`, `terminal-client`'s "Custom binding hint"); `docs/guide.md` (keys, `--select`, `ship view`); `README.md`'s config example; `GLOSSARY.md` "Selection"; `AGENTS.md` "Client/server boundary"; an amendment line at the top of the archived keys-config product paper. Also bring `handoff.md` and the deferred issues to Cyan. Checks: `openspec validate`, every link resolves, and the guide's examples run as written. Report implementation Rust and tests separately against today's count (SC-005), and remove `experiments/latency-proxy/`.

## Deviation log

- **Slice 1, strict list rejection waived by Cyan.** Derived Serde struct deserialization also accepts positional sequences, so `[{ server.tab.create = {} }]` fills `action` and defaults `hint`. `deny_unknown_fields` rejects unknown map keys, not sequences. After this was demonstrated with `ship config check`, Cyan explicitly approved: "Waive strict list rejection and update the slice-1 planning documents accordingly." No custom parser guard was added. Tables remain the documented syntax. FR-018, architecture decision 7, the proposal, design summary, task 1.2 and the keymap delta reflect the waiver.
- **Slice 2, actor reply correction.** The planned `ListViews::Reply = Viewers` does not compile because Kameo does not implement `Reply` for `IndexMap`. Implemented `Result<Viewers>`, matching the existing `ListTabs` handler, without a wrapper or wire change.
- **Slice 2, view docs moved forward.** Added `ship view` to `docs/guide.md` with the runnable slice. Auto mode blocked the attempted one-line README scriptable-CLI update; README was not modified. The remaining slice-5 docs work is still pending.
- **Slice 1, binding docs moved forward.** Updated the guide's binding syntax and hints and the README config example with the runnable slice, rather than leaving invalid examples until slice 5. The remaining selection docs and other slice-5 work are still pending.

### Slice-1 verification

On macOS, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release workspace builds, and `ship-core` checks with and without `clap` passed. Temporary `ship config check` files verified the supported table forms and unbinding, bare actions and `hnt` rejected with line and column, and incidental positional list acceptance. All TOML examples in the README and guide passed config checking. A temporary Rust probe verified derived action/direction names, action hint fallbacks and ordered resize hints. A real client in a PTY against an owned temporary server exercised every default key, exact resize hints, live reload with `hint = "fill"`, and unbound `alt-q` delivered to the shell as `ESC q` without detaching. The default `alt-q` detached without stopping the server. Probe logs remain at `/tmp/ship-slice1.5G3EVE/runtime.log` for this session; the client and temporary server were stopped. No permanent test code was added. Linux remains unverified.

### Slice-2 verification

On macOS, formatting, Clippy with `-D warnings`, debug and release workspace builds, and `ship-core` checks with `--no-default-features` and `--features clap` passed. Against an owned temporary server, curl verified list/get, attach ordering, pane/tab/null selection, unknown attachment and node 404s leaving records, tabs and revision unchanged, malformed attachment 400 and invalid selection type 422. `ship server status` reported protocol 10.

Two real clients ran in PTYs, with their terminal output decoded by a temporary pyte probe. `ship view` verified list, selected and select, `SHIP_ATTACHMENT_ID`, explicit attachment override, missing node/none and conflicting arguments exiting 2, and unknown IDs exiting 1. Both clients rendered independent thick-border pane highlights. Selecting a tab removed pane highlights, selecting nothing showed the nothing-selected hint, and selecting an empty tab showed its no-pane hint. Selecting a zoom-hidden pane unzoomed the shared tab and both clients rendered the split, while preserving the second client's selection. Actual typing reached only the externally selected pane. The existing `Observer::apply` returns visible on a new replica and `drive` redraws immediately after that event, establishing the next-frame path alongside these rendered checks; no wall-clock frame deadline was benchmarked.

The guide's new shell example ran verbatim with clients attached. Evidence remains at `/tmp/ship-slice2.yVDXBF/runtime.log`, `guide-example.log` and the decoded client screens for this session. Both clients detached normally and the owned server stopped. No permanent test code or coding subagents were used. Linux remains unverified. As planned, the client still sends selection with area on its old view PUT; separating those fields remains slice 4.
