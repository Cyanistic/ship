# Drop sessions program

Status: Locked on 2026-10-08. Cyan approved it in Plannotator with "LTGM", after the review changes recorded below and architecture amendment A1. Written from the locked [product](product.md) and [architecture](architecture.md) papers, including product amendment A-1 and architecture amendment A1, and the crates at 37b8ed6. Nothing here is created source, and no snippet has been compiled. Approval doesn't start implementation; Cyan requests that separately, slice by slice. Reopened on 2026-10-08 for amendment P1 (see the Deviation log) and locked again the same day; Cyan re-approved it in Plannotator with "LGTM".

Decided in chat on 2026-10-08, when Cyan agreed with all three calls ("yeah generic struct doesn't make sense anymore. i think i agree with your calls."):

- **U-1, resolved:** `ship server status` never starts a server, like `ship server stop`.
- **Named create bodies.** The generic `Create<T>` and the `Creatable` trait are replaced by `CreateTab` and `CreatePane`, because a tab's parent becomes optional and a pane's doesn't. The architecture doesn't mention this; it's a program-level choice.
- **What "select the first tab" means.** Both `C-b )` and the client's opening view select the tab's first pane, falling back to the tab itself when it has no panes, the same way session cycling lands today. Selecting the bare tab would show the "no pane" hint over a running shell.
- **One move destination** (architecture amendment A1, from Cyan's Plannotator comment "is there a cleaner way to do that before/after thing?"). `MoveTab` names a parent or a sibling, never both. Cyan considered and dropped making it a clap subcommand shared with the protocol ("i see how it's not clean now").

## Rationale

This change is almost all replacement and deletion. No new file is added to the crates. Each architecture decision maps to one place:

- **Top-level map (decision 1)** and **`Arc` at every level (decision 2)** happen in `ship-core`:
  - `tree.rs` drops `Sessions`, and `Tabs` becomes `IndexMap<IdOf<Tab>, Arc<Tab>>`, which serves as both `Replica.tabs` and `Tab.tabs`;
  - `model.rs` loses `Session`, `SessionName`, `TabParent` and `Creatable`, and `NodeId` loses its `Session` variant;
  - `id.rs` loses `ServerRoot` and `UntaggedEither`.
- **The server's edit helpers** in `state/tree.rs` take `&mut Tabs` and an `Option<IdOf<Tab>>` parent, where `None` means the top level.
- **Optional selection (decisions 3 and 4).** `ViewingRecord`, `ViewInput` and `AttachRequest` carry `selection: Option<NodeId>` and nothing else besides the size. `ServerState::repair` stops returning `Option`, because a record survives every edit. `attach.rs` loses its session-removed ending.
- **Startup seeding (decisions 6 and 7)** is one new state message, `Starter`, which takes over what `Create<Session>`'s starter did. `ship_server::serve` sends it between bind and accept, and returns its error.
- **The opening view (decision 5).** `local::default_health` reports whether it launched a server. The UI loop takes an `open_first` flag. On the first `Attached` it queues the view `C-b )` would pick, through the existing frame-tick PUT, and holds drawing until the record has a selection.
- **Routes (decision 8).** The five session handlers are deleted and `list_tabs` is added. `PROTOCOL_VERSION` becomes 5.
- **Docs (decision 9)** are their own slice.

Two choices keep it small:

1. **Named create bodies.** `Create<T>` exists so that one generic shape covers three parent kinds, and it needs about 35 lines of hand-written `ComposeSchema`. With sessions gone, two kinds are left and they disagree about optionality. A serde `default` on a projected `IdOf<T::Parent>` field would need bounds the pane case can't meet. Two derived structs are shorter than the generic plus a workaround, and their schemas come from the derive.
2. **No new client request for the opening view.** The client marks the view dirty with a chosen selection, exactly as a key press does. The frame tick sends the PUT.

Estimated change: implementation Rust shrinks by roughly 350 to 500 lines from today's 5,824 (`git ls-files 'crates/*.rs' | xargs wc -l`). This is an estimate from the deleted declarations; slice 4 reports the measured number. Tests stay at zero.

## Skeleton map

### Added

None in the crates. The only new file is `openspec/changes/drop-sessions/design.md`, the spec index, written after this paper locks (see the program-design attach step). It isn't part of any slice.

### Moved

None.

### Replaced

#### `crates/ship-core/src/model.rs` (slice 2; `Tab.tabs` and `Session.tabs` types in slice 1)

Removed:
- `Session` and its `Prefixed` and `Identified` impls;
- `SessionName` with its `FromStr` and `Display`;
- `TabParent`;
- `Creatable` and its three impls;
- `From<IdOf<TabParent>> for NodeId`;
- the `serde_with` and `ServerRoot`/`UntaggedEither` imports.

```rust
use crate::{AppError, err, id::{Id, IdOf, Identified, Prefixed}, tree::Tabs};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: IdOf<Tab>,
    #[serde(default)]
    pub name: OptionalName,
    #[schema(schema_with = tabs_schema)]
    pub tabs: Tabs,                                   // was IndexMap<IdOf<Tab>, Tab> (slice 1)
    #[schema(schema_with = panes_schema)]
    pub panes: IndexMap<IdOf<Pane>, Pane>,
}

/// Also used by `Replica::tabs`.
pub(crate) fn tabs_schema() -> Object;

/// Rename input; tabs and panes use `OptionalName`.
pub struct Named<N> { pub name: N }

/// Any selectable entity. Untagged; the prefix decides the variant.
pub enum NodeId {
    Tab(IdOf<Tab>),
    Pane(IdOf<Pane>),
}
// FromStr: "expected tab or pane ID, got '…'"
```

`Pane`, `PaneStatus`, `ExitStatus`, `OptionalName` and `panes_schema` are unchanged.

#### `crates/ship-core/src/id.rs` (slice 2)

The following are removed: `ServerRoot`; `UntaggedEither` with its `Identified`, `FromStr`, `Display` and `ComposeSchema` impls; and the `OneOfBuilder` import. `Id<T>`, `Prefixed`, `Identified`, `IdOf` and `Attachment` stay.

#### `crates/ship-core/src/tree.rs` (slice 1; top level in slice 2)

```rust
/// Child tabs in order, at every level including the top. `Arc` so a commit
/// copies only the tabs on an edited path.
pub type Tabs = IndexMap<IdOf<Tab>, Arc<Tab>>;

/// Ancestry of `node`, top-level tab first and `node` last. `None` if absent.
pub fn path(tabs: &Tabs, node: NodeId) -> Option<Vec<NodeId>>;
/// `tab`'s first pane in tree order: its own panes before its child tabs'.
pub fn first_pane(tab: &Tab) -> Option<IdOf<Pane>>;
pub fn tab(tabs: &Tabs, id: IdOf<Tab>) -> Result<&Tab>;
pub fn pane(tabs: &Tabs, id: IdOf<Pane>) -> Result<&Pane>;
pub fn pane_ids(tabs: &Tabs) -> impl Iterator<Item = IdOf<Pane>> + '_;
pub fn pane_owner(tabs: &Tabs, id: IdOf<Pane>) -> Result<IdOf<Tab>>;
/// The tab itself, or a pane's tab. `None` for a node not in the tree.
pub fn viewed_tab(tabs: &Tabs, selection: NodeId) -> Option<IdOf<Tab>>;
/// The tabs `tab` sits among: its parent's children, or `tabs` at the top.
pub fn siblings(tabs: &Tabs, tab: IdOf<Tab>) -> Option<&Tabs>;
pub fn not_found(node: NodeId) -> AppError;
```

Slice 1 only changes `Tabs` to hold `Arc<Tab>` and keeps the session-rooted signatures. Slice 2 removes `Sessions` and `session_of`, roots every walk at `&Tabs`, and changes `first_pane` from taking a session to taking a tab. `siblings` replaces the private sibling lookup in the client's `draw::position`~~, and the server's `take_tab` uses it too~~. Superseded by P1: the server's edits find a tab's map through `holder_mut` instead.

#### `crates/ship-core/src/protocol.rs` (slice 2)

Removed:
- `Create<T>` with its `ComposeSchema` and `ToSchema` impls;
- `CreateSession`;
- `Placement`, folded into `MoveTab`;
- `sessions_schema`;
- `EndReason::SessionRemoved`.

```rust
pub const PROTOCOL_VERSION: u32 = 5;

/// POST /tabs body. No parent means the top level.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTab {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<IdOf<Tab>>,
    #[serde(default)]
    pub name: OptionalName,
}

/// POST /panes body.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePane {
    pub parent: IdOf<Tab>,
    #[serde(flatten)]
    pub input: PaneInput,
}

/// What a pane runs. Pane creation input; also the `--starter` pane.
pub struct PaneSpec { /* unchanged */ }

/// POST /tabs/{id}/move body: exactly one destination (architecture A1).
/// `{"parent": "tab:…"}`, `{"parent": null}` for the top level,
/// `{"before": "tab:…"}` or `{"after": "tab:…"}`. Replaces `Placement`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum MoveTab {
    /// Append under the parent, or to the top level when `null`.
    Parent(Option<IdOf<Tab>>),
    /// Insert before the sibling, under its parent.
    Before(IdOf<Tab>),
    /// Insert after the sibling, under its parent.
    After(IdOf<Tab>),
}

/// POST /attach body. The selection is what a reconnecting client last had;
/// kept if it still exists, else nothing is selected.
pub struct AttachRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
    pub size: Size,
}

/// PUT /attach/view body: the client's whole view.
pub struct ViewInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
    pub size: Size,
}

/// Server-owned view of one attachment. Deleted only on detach.
pub struct ViewingRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
    pub size: Size,
}

pub struct Replica {
    pub incarnation: Uuid,
    pub revision: u64,
    #[schema(schema_with = tabs_schema)]
    pub tabs: Tabs,                                    // was `sessions`
    #[schema(schema_with = viewers_schema)]
    pub viewers: IndexMap<IdOf<Attachment>, ViewingRecord>,
}

pub enum EndReason {
    /// The server is shutting down. Reattaching may succeed later.
    ServerShutdown,
}
```

#### `crates/ship-core/src/error.rs` (slice 2)

The `InvalidStructure` doc changes from "session/tab/pane structure" to "tab/pane structure". No other change.

#### `crates/ship-server/src/state/tree.rs` (slice 1; top level in slice 2)

```rust
pub(crate) use ship_core::tree::{Tabs, first_pane, not_found, pane, pane_ids, pane_owner, path, siblings, tab, viewed_tab};

/// The map that holds tab `id`, `Arc::make_mut` on each tab on the way down
/// and no other (P1).
fn holder_mut(tabs: &mut Tabs, id: IdOf<Tab>) -> Option<&mut Tabs>;
/// `Arc::make_mut` on each tab along the path, top level first.
pub(crate) fn tab_mut(tabs: &mut Tabs, id: IdOf<Tab>) -> Result<&mut Tab>;
/// `None` is the top level.
pub(crate) fn children_mut(tabs: &mut Tabs, parent: Option<IdOf<Tab>>) -> Result<&mut Tabs>;
/// Detach a tab with its subtree and panes from its parent.
pub(crate) fn take_tab(tabs: &mut Tabs, id: IdOf<Tab>) -> Result<Arc<Tab>>;
/// Insert at the destination: appended under a parent, or next to a sibling
/// under the sibling's parent (`siblings`). A missing parent or sibling is 404.
pub(crate) fn place(tabs: &mut Tabs, tab: Arc<Tab>, to: MoveTab) -> Result<()>;
```

In slice 1 these keep their session-rooted parents (`&mut Sessions`, `IdOf<TabParent>`) and today's `place(children, tab, placement)`. The only change there is that each step down calls `Arc::make_mut`. Slice 2 swaps the roots and parents and gives `place` the `MoveTab` destination, as shown above. `sibling_index` and its "not a child of the destination" error go away. Per P1, `tab_mut`, `take_tab` and `place`'s sibling destinations go through `holder_mut`, so `path` and its `expect`s and `unreachable!`s leave this file.

#### `crates/ship-server/src/state.rs` (slice 2; starter in slice 3)

Removed:
- `ListSessions`;
- `Create<Session>`, `Get<Session>`, `Rename<Session, SessionName>` and `Remove<Session>`, along with their handlers;
- the `session` and `inside` helpers;
- `ensure_unique`.

```rust
pub struct ServerState {
    incarnation: Uuid,
    revision: u64,
    tabs: Tabs,                                        // was `sessions`
    viewers: Viewers,
    runtimes: HashMap<IdOf<Pane>, PaneRuntime>,
    live: watch::Sender<LivePanes>,
    pane_env: PaneEnv,
    bus: ActorRef<RelayBus>,
}

impl ServerState {
    async fn commit<R>(&mut self, edit: impl FnOnce(&mut Tabs, &mut Viewers) -> Result<R>) -> Result<R>;
    /// Keep a selection still in `new`; a removed pane goes to its tab's
    /// neighbor; else the nearest ancestor still in `new`; else `None`.
    fn repair(old: &Tabs, new: &Tabs, record: ViewingRecord) -> ViewingRecord;
    /// Records without a selection are skipped.
    fn tab_sizes(&self) -> HashMap<IdOf<Tab>, Size>;
    fn start_pane(&mut self, tab: IdOf<Tab>, input: PaneInput) -> Result<Pane>;  // unchanged
}

/// GET /tabs.
pub struct ListTabs;                                   // Reply = Result<Tabs>
/// Startup only: one top-level tab holding one pane, in one commit (slice 3).
pub struct Starter(pub PaneSpec);                      // Reply = Result<Tab>

impl Message<CreateTab> for ServerState { type Reply = Result<Tab>; }   // children_mut(tabs, create.parent)
impl Message<CreatePane> for ServerState { type Reply = Result<Pane>; }
impl Message<Move> for ServerState { type Reply = Result<Tab>; }        // rejects a destination parent or sibling inside the moving tab
impl Message<Attach> for ServerState { type Reply = Result<Attached>; } // selection kept if it exists, else None
impl Message<SetView> for ServerState { type Reply = Result<ViewingRecord>; } // None always accepted; Some(absent) leaves the record
```

`Get`, `Rename`, `Remove`, `Detach`, `CheckAttachment` and `PaneEvent` are unchanged apart from operating on `tabs`. `Remove<Tab>` on a top-level tab is the same `take_tab`.

#### `crates/ship-server/src/routes.rs` (slice 2)

The five session handlers are removed. The new handler:

```rust
#[utoipa::path(get, path = "/api/v0/tabs", operation_id = "list_tabs",
    responses(
        (status = 200, description = "Top-level tabs keyed by ID, in order, with their descendants", body = IndexMap<String, Tab>),
        (status = 503, body = AppError),
    ))]
pub(crate) async fn list_tabs(State(app): App) -> Result<Response>;
```

`create_tab` takes `Json<CreateTab>` and `create_pane` takes `Json<CreatePane>`. On `move_tab`, the 404 reads "Tab, parent or sibling not found" and the 422 reads "Move into itself or a descendant, or a malformed body".

#### `crates/ship-server/src/attach.rs` (slice 2)

The handler doc and responses drop the session: the 404 response is removed, and `ended` is sent only on server shutdown. `Progress::next` loses the `viewers.contains_key` check, because a record now outlives every edit and is removed only by this stream's own guard. `Progress::view` reads `replica.tabs` and finds nothing to watch when the selection is `None`. `view` is unchanged.

#### `crates/ship-server/src/lib.rs` (slice 2; starter in slice 3)

```rust
//! Loopback HTTP listener, health route, tab and pane API and attach stream.

/// Binds, spawns the actors, creates the starter tab when `starter` is set,
/// then serves. A failed starter stops the actors and returns its error
/// before any connection is accepted (decision 7).
pub async fn serve(
    address: SocketAddr,
    starter: bool,
    shutdown: impl Future<Output = Result<()>> + Send + 'static,
) -> Result<()>;
```

In slice 2, `api_router` loses the five session routes and gains `routes::list_tabs`. In slice 3, `serve` sends `state.ask(Starter(PaneSpec::default()))` after `forward_pane_events` and before `axum::serve`. While that runs, the listener is bound but not yet accepting, so a health probe that connects in that window waits in the backlog rather than being refused.

#### `crates/ship-client/src/api.rs` and `lib.rs` (slice 2)

Removed:
- `SessionRef` and `TabParentRef`;
- `Resource for Session`;
- `sessions`, `create_session`, `resolve_session`, `resolve_parent` and `ensure_session`;
- the `SessionRef` and `TabParentRef` re-exports.

```rust
impl Client {
    pub async fn tabs(&self) -> Result<Tabs>;                                       // GET /tabs
    pub async fn create_tab(&self, parent: Option<IdOf<Tab>>, name: Option<&str>) -> Result<Tab>;
    pub async fn create_pane(&self, parent: IdOf<Tab>, input: &PaneInput) -> Result<Pane>;  // body is CreatePane
    // get, rename, remove, move_tab, attach, input, set_view: unchanged signatures
}
```

#### `crates/ship-client/src/ui/` (slice 2; opening view in slice 3)

`mod.rs`:

```rust
//! The full-screen client, opened by bare `ship`.

enum Exit { Detached, ServerStopped, Signaled }        // SessionRemoved removed

/// `open_first`: this `ship` launched the server, so on the first `Attached`
/// queue the view `C-b )` would pick and draw nothing until the record has a
/// selection or the PUT fails (slice 3).
pub async fn run(client: &Client, open_first: bool) -> Result<()>;

async fn drive(client: &Client, guard: &mut TerminalGuard, request: AttachRequest,
    events: Events, signaled: impl Future<Output = ()>, open_first: bool) -> Result<Exit>;

/// Where an action moves the selection `from`. Top-level tabs cycle in order
/// and land on the tab's first pane, else the tab; from nothing selected they
/// go to the first or last. Panes cycle within their tab; from a tab, to its
/// first or last pane. `None` when there is nowhere to go.
fn navigate(tabs: &Tabs, from: Option<NodeId>, action: &Action) -> Option<NodeId>;
```

The reattach `NotFound` arm goes away, because attach no longer names anything that can be missing. A view PUT sends `chosen.or(record.selection)`.

`keys.rs`: `Action::NextSession` and `PrevSession` become `NextTab` and `PrevTab`. The doc comment `C-b )`, `C-b (` and the bindings stay the same.

`observer.rs`:

```rust
/// The record's selection resolved against the replica. `tab` was optional
/// only because a session could be selected; every selection now has a tab.
pub(super) struct Selected<'a> {
    /// The selected tab, or the selected pane's tab.
    pub tab: &'a Tab,
    pub pane: Option<&'a Pane>,
}

impl Observer {
    /// Reattach with the last record's selection at the current size.
    pub fn remembered(&self, size: Size) -> Option<AttachRequest>;
    /// `None` when nothing is selected or before the first `Attached`.
    pub fn selected(&self) -> Option<Selected<'_>>;
}
```

`draw.rs`:

```rust
/// "connecting" while the observer has no record; with nothing selected,
/// `3 tabs · C-b ) to open one` or `no tabs · ship tab create`; with a tab
/// and no pane, `no pane: ship pane create <tab-id>`.
fn hint(observer: &Observer, selected: Option<&Selected>) -> String;
/// `tab › pane` as far as the selection goes, then exit status and connection.
fn status_line(observer: &Observer, selected: Option<&Selected>) -> String;
fn position(tabs: &Tabs, tab: IdOf<Tab>) -> usize;       // via tree::siblings
```

#### `crates/ship/src/cli.rs` (slice 2; `--starter` in slice 3)

Removed:
- `Command::Session` and `Command::Attach`;
- `SessionCommand`;
- `NameArgs`, `SessionArgs`, `RenameSessionArgs` and `AttachArgs`;
- the `ship_client` and `SessionName` imports.

```rust
#[command(
    version,
    about = "Open the Ship client, run a loopback server or script tabs and panes",
    long_about = "Open the Ship client, run a loopback server or script tabs and panes.\n\nBare ship and tab and pane commands reuse the default-local server or start a missing one in the background.\nThat server stays running after the client and launching terminal exit."
)]
pub struct Cli { /* unchanged */ }

pub enum TabCommand {
    /// List top-level tabs and their descendants as JSON, keyed by ID
    List,
    /// Append a tab to a tab, or to the top level without one, and print it as JSON
    Create(CreateTabArgs),
    Get(IdArgs<Tab>),
    Rename(RenameArgs<Tab>),
    Rm(IdArgs<Tab>),
    /// Append under PARENT or to the top level, or place before or after a sibling
    Move(MoveTabArgs),
}

pub struct ServerArgs {
    pub command: Option<ServerCommand>,
    pub port: u16,
    /// Start with one tab holding a shell in your home directory (slice 3)
    #[arg(long)]
    pub starter: bool,
    #[arg(long, hide = true)]
    pub background_child: bool,
}

pub enum ServerCommand {
    /// Stop the server, ending every program in it, and wait until it exits
    Stop,
    /// Print the server's health as JSON
    Status,
}

pub struct CreateTabArgs {
    /// Parent tab ID; omitted means the top level
    pub parent: Option<IdOf<Tab>>,
    #[arg(long)]
    pub name: Option<String>,
}

pub struct MoveTabArgs {
    pub id: IdOf<Tab>,
    #[command(flatten)]
    pub to: Destination,
}

/// At most one; none means the top level.
#[derive(Args)]
#[group(multiple = false)]
pub struct Destination {
    /// Parent tab ID; omitted means the top level
    pub parent: Option<IdOf<Tab>>,
    /// Insert before this sibling, under its parent
    #[arg(long)]
    pub before: Option<IdOf<Tab>>,
    /// Insert after this sibling, under its parent
    #[arg(long)]
    pub after: Option<IdOf<Tab>>,
}

impl From<Destination> for MoveTab;   // before, else after, else Parent(parent)
```

#### `crates/ship/src/main.rs`, `commands.rs`, `local.rs` (slice 2; launch flag in slice 3)

```rust
// main.rs dispatch
Some(Command::Server(ServerArgs { command: Some(ServerCommand::Status), .. }))
    => commands::server_status(&client(&cli.server_url)?).await,   // never starts a server (U-1)
Some(Command::Server(args)) => ship_server::serve(loopback_addr(args.port)?, args.starter, diagnostics::shutdown()?).await,
None => {
    // explicit target: health check, open_first = false
    // otherwise: local::default_health, open_first = launched
    ship_client::ui::run(&client, open_first).await
}

// commands.rs
pub async fn server_status(client: &Client) -> Result<()>;  // crate::health, then print
// `session` removed; `tab` gains `List`, passes create's `args.parent` straight
// through, and sends `MoveTab::from(args.to)` for a move

// local.rs
/// Whether the default server was already running or this call launched one.
/// `launched` stays true when a concurrent launcher's server won the bind.
pub struct Local { pub health: HealthResponse, pub launched: bool }
pub async fn default_health(client: &Client) -> Result<Local>;
// spawn args gain "--starter" (slice 3)
```

`connect` keeps its current shape and drops the `Local` it gets back. Only bare `ship` reads `launched`.

#### Docs (slice 4)

- `AGENTS.md` lines 3, 5 and 15: the server owns recursive tabs and panes, and top-level tabs are the top-level container.
- `GLOSSARY.md`:
  - the intro line;
  - **Session** is removed (it moves under _Avoid_ in **Tab**);
  - **Tab** reads "a container at the server's top level or within another tab";
  - **Selection** reads "tab or pane, or nothing".
- `README.md` lines 27, 28, 38, 66 and 90: tabs instead of sessions; `ship` opens the client.
- Archived papers under the ripple rule: in `2026-10-06-session-tab-roundtrip/design/{product,architecture,program}.md` and `2026-10-07-pane-terminals/design/{product,architecture,program}.md`, each status line gains a note that session text is superseded by `drop-sessions`, and each requirement that defines sessions gets a "Superseded by drop-sessions" mark. No text is rewritten.

### Untouched

- `crates/ship-server/src/pane.rs`, `input.rs`, `health.rs`, `app.rs`: the "session" in `pane.rs` is the ghostty wrapper's `SessionHandle`, a different thing that stays.
- `crates/ship-core/src/screen.rs`, `relay.rs`; `crates/ship-macros`; `crates/ship-client/src/ui/terminal.rs`; `crates/ship/src/diagnostics.rs`.
- `Cargo.toml` files: no dependency changes. `serde_with` stays because `id.rs` uses it.
- `openspec/specs/*`: these are assembled views and are updated when this change is archived, not by hand. Until then, SC-002 holds for code and hand-written docs only.
- The layout-sidebar and keys-config papers: they're revised in their own changes.

## Build order

Each slice ends with all of the following:

- the workspace building;
- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean;
- the listed real workflow against a foreground `ship server --port <p>`, with `--server-url http://127.0.0.1:<p>` on every client, unless the slice says otherwise.

No permanent tests are added. Every slice is run on macOS. Slices 2 and 3 are also run on Linux, or recorded as unverified there. Windows is unverified throughout.

### 1. `Arc` at every level

Files: `tree.rs` (`Tabs` type), `model.rs` (`Tab.tabs` and `Session.tabs`), `state/tree.rs` (`tab_mut`, `take_tab` and `place` with `Arc::make_mut`), and any call site that changes from `Tab` to `Arc<Tab>`.

Delivers: no behavior change. The tree has one map type at every level below the session.

Checks:
- `ship attach work` still works, and so do creating nested tabs, moving a nested tab between two sessions and back with its shell still running, and renaming a pane title deep in the tree.
- `git diff --stat` for the slice is recorded in the deviation log as the measured cost of decision 2. If it's clearly larger than the top-level-only alternative, the log reopens decision 2 (architecture risk "Wider `Arc` use").

### 2. Top-level tabs replace sessions

Files: every "slice 2" entry above. This is the largest diff, and it's mostly deletion.

Delivers:
- bare `ship` opens the client with nothing selected (there's no starter yet, so the starting state is empty);
- the hint;
- `C-b )` and `C-b (` across top-level tabs;
- attachments surviving the removal of every tab;
- `ship tab list`, plus `ship tab create` and `ship tab move` without a parent;
- `ship server status`;
- the session commands, the session routes and `ship attach` are gone;
- protocol 5.

Checks:
- P1 "Open the server": create three top-level tabs with the CLI, run `ship`, read the hint, and cycle through all three tabs in both directions.
- P1 "Script tabs by ID": build a two-level tree, move a nested tab to the top level and back, and confirm its program keeps running. Confirm that `ship tab list` shows the tree.
- Move with `--before` and `--after` a sibling at another level, and confirm the tab lands under that sibling's parent. Confirm that `ship tab move <id> <parent> --before <sib>` is rejected by clap, that `curl` with `{}` gets 422, and that moving a tab before its own child gets 422.
- P2: remove the viewed tab while a sibling exists, and confirm the client falls back. Remove every tab, and confirm the client stays attached with `no tabs · ship tab create`. Then detach.
- A protocol-4 client against this server fails the health check.
- `ship server status` prints the health JSON.
- The OpenAPI document has no session paths or schemas, and `list_tabs` appears.

### 3. The starter

Files: `ServerArgs.starter`, `serve(…, starter, …)`, `Starter` in `state.rs`, `local.rs` (`Local`, `--starter`), `main.rs` (`open_first`), and `ui/mod.rs` (`open_first`, the hold on drawing).

Delivers: the A-1 stories.

Checks:
- Stop the server, then run `ship` from a project directory. Confirm the client opens on a shell whose `pwd` is home, with no hint frame visible.
- Run `ship server` and confirm `ship tab list` prints `{}`. Run `ship server --starter` and confirm it lists one tab with one pane.
- Remove every tab, then run `ship`. Confirm the hint shows and `ship tab list` reads the same before and after (SC-003 A-1).
- Start two `ship` processes together on a stopped server. Confirm that one tab exists and both clients select it.
- Point the login shell at a missing binary (`SHELL=/nonexistent ship server --starter`). Confirm the server exits with the error and bare `ship` reports it with the retained log.
- Judge the auto-start delay by feel against slice 2 (architecture risk "Starter timing").

### 4. Docs and the size report

Files: the Docs entries above.

Delivers: FR-013 and SC-002 for hand-written docs, plus SC-005 and SC-006.

Checks:
- `rg -i session AGENTS.md GLOSSARY.md README.md crates/` matches only the ghostty `SessionHandle` lines in `pane.rs`.
- Every archived paper that defines sessions carries a superseded mark.
- The size report gives implementation Rust before and after, and test lines (zero), separately.
- The platform report lists which checks were run on Linux and macOS, with Windows marked unverified.
- SC-004: opening, cycling and detaching feel as quick as `ship attach` did in slice 1.

## Deviation log

### P1 (2026-10-08, from slice 1 review): one walk finds and copies a tab's map

Cyan asked in review of slice 1 whether the `expect` calls in `state/tree.rs` could go. They exist because `tab_mut` and `take_tab` compute `path` first and then walk it again with `get_mut`, asserting that the second pass agrees with the first. Slice 2 replaces that with one private helper that finds the map holding a tab while it walks:

```rust
/// The map that holds tab `id`, copying each tab on the way down.
fn holder_mut(tabs: &mut Tabs, id: IdOf<Tab>) -> Option<&mut Tabs> {
    if tabs.contains_key(&id) {
        return Some(tabs);
    }
    let next = tabs.values().position(|tab| tree::tab(&tab.tabs, id).is_ok())?;
    holder_mut(&mut Arc::make_mut(&mut tabs[next]).tabs, id)
}

pub(crate) fn tab_mut(tabs: &mut Tabs, id: IdOf<Tab>) -> Result<&mut Tab> {
    holder_mut(tabs, id)
        .and_then(|tabs| tabs.get_mut(&id))
        .map(Arc::make_mut)
        .ok_or_else(|| not_found(NodeId::Tab(id)))
}

pub(crate) fn take_tab(tabs: &mut Tabs, id: IdOf<Tab>) -> Result<Arc<Tab>> {
    holder_mut(tabs, id)
        .and_then(|tabs| tabs.shift_remove(&id))
        .ok_or_else(|| not_found(NodeId::Tab(id)))
}
```

`place` with a `before` or `after` destination inserts into `holder_mut(tabs, sibling)`, which is the sibling's parent's map, copied for writing. The `position` check is read-only, so tabs off the route are never copied, and its cost matches the `path` walk it replaces. Public signatures and behavior don't change. Deferred to slice 2 because the helper is written against `&Tabs` roots; at slice 1's session roots it would need a session step and a local lookup that slice 2 deletes.

### Slice 2 (2026-10-08): top-level tabs replace sessions

Slice 2 follows the skeleton with one deviation that needs Cyan's review. Implementation Rust went from 5,834 to 5,189 lines (−645), already beyond the whole change's −350 to −500 estimate. Tests stay at zero.

**Deviation: `move_tab` reads the body as a JSON value first.** A derived `Json<MoveTab>` answers `{}` and `{"before": …, "after": …}` with 400, not the 422 the spec requires. serde_json reports a map without exactly one key, deserialized as an externally tagged enum, as a syntax error, and axum maps syntax errors to 400. The handler now extracts `Json<serde_json::Value>`, so a body that isn't JSON still gets 400. It then runs `serde_json::from_value::<MoveTab>` and maps a failure to `InvalidStructure` (422) with the message "malformed move body". `MoveTab` stays a derived enum, and the wire format and OpenAPI schema are unchanged. This adds about four lines and a doc comment in `routes.rs`.

Smaller choices, none of which changes the contract:

- `state/tree.rs` re-exports only the lookups the server uses. It doesn't re-export `first_pane` or `siblings`, because unused re-exports warn. `holder_mut` is as written in P1.
- `commands::server_status` isn't added; it would only forward. The `ServerCommand::Status` arm in `main.rs` calls `commands::print(&health(…).await?)`, and `print` becomes `pub(crate)` (from Cyan's review of slice 2).
- `Rename<T, N = OptionalName>` loses its name parameter, because only sessions used `SessionName`.
- With exactly one top-level tab, the hint reads `1 tab · C-b ) to open one`.
- With nothing selected, the status line is blank, apart from the disconnected marker.
- A move whose sibling is the moving tab itself fails with 422 as "into itself", not 404.

Evidence, on macOS against a foreground `ship server --port 43311`:

- curl: `GET /api/v0/tabs` returned `{}`. `POST /api/v0/tabs` with `{}` returned 201 with a top-level tab. A move with `{}`, with both `before` and `after`, or with a pane ID as parent returned 422; `not json` returned 400; `{"parent": null}` returned 200 and the tab moved to the top level.
- CLI: built top › nested (child tab, running `sleep` pane) and other. Moved nested to the top level and back, and its program's PID stayed alive both times. `other --before child` landed under nested, and `--after top` landed at the top level. `tab move top --before nested` and `tab move top child` exited 1 with "into itself or its own descendant", and `tab list` was byte-identical before and after. A missing `--after` sibling exited 1 with not found. `tab move <id> <parent> --before <sib>` and `tab get notes` exited 2. `ship --help` contains no "session".
- Client, driven in a 30×100 pty and rendered with pyte (temporary probe): opened on `3 tabs · C-b ) to open one`. `C-b )` went one › a, two › b, three › c, one › a, and `C-b (` went three, two, one, each showing that pane's output. `C-b (` from nothing selected went to three › c. Cycling to a paneless tab showed `no pane: ship pane create <id>`. `C-b )` into a tab whose only pane is in a child tab selected `nested › deep`. Removing that nested tab fell back to its parent, and removing the parent, a top-level tab with siblings, selected nothing. Removing every tab showed `no tabs · ship tab create` while attached. Two `ship tab create` calls then showed `2 tabs · C-b ) to open one` without reattaching. `C-b d` printed `detached` and exited 0.
- A d79868e (protocol 4) build against this server: bare `ship` and `ship attach work` exited 1 with "incompatible health response … expected service ship and protocol 4".
- `ship server status` printed `{"service":"ship","protocolVersion":5,"version":"0.1.0"}` and exited 0. Against a free port it exited 1 with "no server running", and nothing was listening there afterward. The default port was occupied by Cyan's own server, so the default-URL path wasn't exercised; it runs the same code without `local::default_health`.
- A disposable `openapi()` consumer crate outside the repo found no "session" in the document. Paths: `list_tabs`, `create_tab`, `get_tab`, `rename_tab`, `remove_tab`, `move_tab`, the pane, attach, input, view, stop and health operations. `MoveTab` is a `oneOf` of three single-key objects, `CreateTab.parent` is optional, and `ViewingRecord.selection` is optional.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and debug and release builds pass.

On Linux (aarch64, Debian trixie in the `ship-linux-probe` Docker image, from a copy of the working tree), fmt, Clippy, and debug and release builds passed. The same CLI tree, curl move, `server status` and pty client probes passed with identical results. The protocol-4 client and OpenAPI checks ran on macOS only; neither depends on the platform. Windows is unverified.

### Slice 1 (2026-10-08): `Arc` at every level

No deviation from the skeleton. Measured cost of decision 2, from `git diff --stat` over the slice's source changes:

```
 crates/ship-client/src/ui/draw.rs    |  4 ++--
 crates/ship-core/src/model.rs        |  5 +++--
 crates/ship-core/src/tree.rs         |  6 ++++--
 crates/ship-server/src/state.rs      |  6 +++---
 crates/ship-server/src/state/tree.rs | 17 ++++++++++++-----
 5 files changed, 24 insertions(+), 14 deletions(-)
```

Decision 2 stands. Top-level-only `Arc` would need the top-level half of these edits in slice 2 anyway (the first `make_mut` in `tab_mut`, `Arc::new` on insert, `Arc::as_ref` in `tab`). The extra for every level is roughly ten lines: `make_mut` on each ancestor and on the target in `tab_mut`, the `Arc<Tab>` signatures of `take_tab` and `place`, and `Tab::clone(&tab)` in the `Move` handler, which still copies the moved tab for the reply as `tab.clone()` did before.

Evidence, on macOS against a foreground `ship server --port <p>`:

- `ship tab get` for a session › tab › tab › tab › pane tree is byte-identical to the a403137 build once IDs are normalized.
- A nested tab with a running shell pane moved from `work` to a tab in a second session and back; the pane read `running` throughout, and the source tab was left with no children while moved.
- In the deep tab, a pane's OSC title (`deep-after-move`) showed in `pane get`, and `pane rename` on the moved shell pane took effect.
- `ship attach work`, driven under `expect` in a 30×100 pty, drew `work › nest › …`, echoed a shell command and detached on `C-b d`; the same probe passes on the pre-slice binary.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and debug and release builds pass.

Linux and Windows are unverified for this slice, which the build order only requires on macOS.
