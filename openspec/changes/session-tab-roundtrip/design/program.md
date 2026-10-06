# Session/tab roundtrip program

Status: Locked again on 2026-10-06 after Cyan approved architecture amendment A5 and this paper's A5 additions in chat ("they're good to go"); the additions are marked inline and land in slice 7. Previously: Locked on 2026-10-05 after Cyan approved it in Plannotator with “LGTM”. Written fresh from the locked [product](product.md) and [architecture](architecture.md) papers and the current crates. No earlier program draft was consulted. Nothing here is created source, and no snippet has been compiled.

## Rationale

The plan adds as little machinery as the locked architecture allows. Each architectural part gets one obvious home, and the shared pieces are kept generic only where several call sites use them.

- **`ship-core`** holds the data both sides serialize: typed IDs, the session/tab/pane tree, request bodies, the replica envelope and `SseEvent`. It has no actors, channels or HTTP.
- **`ship-server`** holds one state actor, the ported `RelayBus`, a shared replica watch channel, and thin Axum handlers that forward to the state actor. There is no notifier actor (architecture amendment A3).
- **`ship-client`** gains one method per route plus an SSE stream for `attach`.
- **`ship`** gains `session`, `tab`, `pane` and `attach` subcommands. The observer's stdin controls live in their own module so they can be deleted when a real UI exists.

Three choices do most of the simplifying.

1. **Copy-on-write sessions.** The state actor stores `IndexMap<IdOf<Session>, Arc<Session>>`. Every mutation clones the map (cheap, it only clones `Arc`s), edits the clone with `Arc::make_mut`, and swaps it in only if the edit returned `Ok`. `make_mut` copies exactly the sessions an edit touches, which is the candidate copy architecture decision 5 asks for. The same `Arc`s are shared with published replicas, so publishing never deep-copies the tree. Validation happens naturally inside the edit: any `Err` throws the candidate away, so there is no separate validate-then-apply pass.
2. **One commit path.** Structural edits, attach/detach, selection and session switching all go through `ServerState::commit`. It edits the candidate, repairs every viewing record against the old and new trees, bumps the revision and publishes. Selection repair is one rule (keep the selection if it still exists in the attached session, otherwise walk its old ancestors), and it covers pane removal, recursive tab removal and cross-session moves at once.
3. **Full replica, one publication type.** Each replica carries every session and every viewing record. The relay has one publication type (`Arc<Replica>`) and one sink (the replica watch channel). The observer picks its own record by attachment ID and renders only its attached session. Metadata is small and the stream is compressed, so per-session filtering is not worth its code yet.

## Skeleton map

### Added

#### `crates/ship-core/src/id.rs` (slice 1; `UntaggedEither` in slice 2)

```rust
use std::{fmt, hash::Hash, marker::PhantomData, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use uuid::Uuid;

use crate::AppError;

/// Stable external kind prefix, e.g. `tab` in `tab:3f2a...`.
pub trait Prefixed {
    fn prefix() -> &'static str;
}

/// Associates an entity, or a choice of entities, with its typed identity.
pub trait Identified {
    type Id: Clone + Eq + Hash + fmt::Debug + Serialize + for<'de> Deserialize<'de>;
}

pub type IdOf<T> = <T as Identified>::Id;

/// UUID v4 tagged with its entity kind. Displayed, parsed and serialized as
/// `{prefix}:{uuid in simple form}`. Parsing rejects any other prefix.
#[derive(SerializeDisplay, DeserializeFromStr)]
pub struct Id<T> {
    uuid: Uuid,
    kind: PhantomData<fn() -> T>,
}

impl<T: Prefixed> Id<T> {
    pub fn new() -> Self;
}

impl<T: Prefixed> fmt::Display for Id<T>;
impl<T: Prefixed> FromStr for Id<T> { type Err = AppError; } // ErrorCode::Validation

/// Hand-written: a string matching `^{prefix}:[0-9a-f]{32}$`.
impl<T: Prefixed> utoipa::PartialSchema for Id<T>;
impl<T: Prefixed> utoipa::ToSchema for Id<T>;
// Clone, Copy, PartialEq, Eq, Hash and Debug are implemented by hand so they
// do not require bounds on the marker type T. Check at implementation that the
// serde_with derives place `Display`/`FromStr` bounds on `Id<T>`, not on `T`.

/// Untagged choice of two identified kinds. Strict prefixes make it unambiguous.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UntaggedEither<A, B> {
    Left(A),
    Right(B),
}

impl<A: Identified, B: Identified> Identified for UntaggedEither<A, B> {
    type Id = UntaggedEither<IdOf<A>, IdOf<B>>;
}

/// Hand-written `oneOf` of the two inner schemas.
impl<A: utoipa::ToSchema, B: utoipa::ToSchema> utoipa::PartialSchema for UntaggedEither<A, B>;
impl<A: utoipa::ToSchema, B: utoipa::ToSchema> utoipa::ToSchema for UntaggedEither<A, B>;

/// Tries `A`, then `B`. Lets Clap parse a tab parent argument directly.
impl<A: FromStr, B: FromStr> FromStr for UntaggedEither<A, B> { type Err = AppError; }

/// Parent of root sessions. Its identity is the unit value.
pub struct ServerRoot;
impl Identified for ServerRoot { type Id = (); }

/// Marker for stream attachments; never instantiated.
pub enum Attachment {}
impl Prefixed for Attachment { fn prefix() -> &'static str { "attachment" } }
impl Identified for Attachment { type Id = Id<Attachment>; }
```

#### `crates/ship-core/src/model.rs` (slice 1; tabs in slice 2; panes in slice 3; `NodeId` in slice 4)

```rust
use std::{fmt, str::FromStr};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::id::{UntaggedEither, Id, IdOf, Identified, Prefixed, ServerRoot};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: IdOf<Session>,
    pub name: SessionName,
    pub tabs: IndexMap<IdOf<Tab>, Tab>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: IdOf<Tab>,
    pub name: String,
    pub tabs: IndexMap<IdOf<Tab>, Tab>,
    pub panes: IndexMap<IdOf<Pane>, Pane>,
}

/// Metadata-only leaf. Owns no terminal, layout or children.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pane {
    pub id: IdOf<Pane>,
    pub name: String,
}

// prefix(): "session", "tab", "pane". Each Identified::Id is Id<Self>.
impl Prefixed for Session; impl Identified for Session;
impl Prefixed for Tab;     impl Identified for Tab;
impl Prefixed for Pane;    impl Identified for Pane;

pub type TabParent = UntaggedEither<Session, Tab>;

/// Associates an entity with its parent kind and creation input.
pub trait Creatable: Identified {
    type Parent: Identified;
    type Input;
}

impl Creatable for Session { type Parent = ServerRoot; type Input = Named<SessionName>; }
impl Creatable for Tab     { type Parent = TabParent;  type Input = Named; }
impl Creatable for Pane    { type Parent = Tab;        type Input = Named; }

/// Creation and rename input. Sessions use `SessionName`; tabs and panes any string.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Named<N = String> {
    pub name: N,
}

/// A session name: non-empty and without ':' (product P4). The only place the
/// rule lives. The server gets it by deserializing session create/rename bodies;
/// the CLI gets it by parsing `SessionRef`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub struct SessionName(String);

impl FromStr for SessionName { type Err = crate::AppError; } // ErrorCode::Validation
impl fmt::Display for SessionName;

/// Any selectable entity. Untagged; the prefix decides the variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodeId {
    Session(IdOf<Session>),
    Tab(IdOf<Tab>),
    Pane(IdOf<Pane>),
}

impl FromStr for NodeId { type Err = crate::AppError; }
```

Every serialized type in `model.rs` and `protocol.rs` also derives `utoipa::ToSchema`. `SessionName` uses `#[schema(value_type = String)]`. `Create<T>` and `Named<N>` rely on Utoipa's generic schema support; if the associated-type field `IdOf<T::Parent>` defeats the derive, write `ToSchema` by hand for `Create<Tab>` and `Create<Pane>`, the only two used in routes. The attach route documents its response as `text/event-stream` whose `data:` lines are `SseEvent`.

Ordered children serialize as JSON objects keyed by ID, in order. `jq '.tabs[]'` iterates them and `jq -r '.id'` reads a created entity's ID.

#### `crates/ship-server/src/state.rs` (slice 1; grows through slice 6)

```rust
use std::sync::Arc;

use indexmap::IndexMap;
use kameo::{Actor, actor::ActorRef, message::{Context, Message}};
use ship_core::{prelude::*, id::*, model::*, protocol::*};
use tokio::sync::watch;
use uuid::Uuid;

use crate::relay::RelayBus;

mod tree;

pub(crate) type Sessions = IndexMap<IdOf<Session>, Arc<Session>>;
pub(crate) type Viewers = IndexMap<IdOf<Attachment>, ViewingRecord>;

/// Sole owner of the tree and of every active attachment's viewing record.
#[derive(Actor)]
pub struct ServerState {
    incarnation: Uuid,
    revision: u64,
    sessions: Sessions,
    viewers: Viewers,
    bus: ActorRef<RelayBus>,                  // slice 4
}

impl ServerState {
    pub fn new(bus: ActorRef<RelayBus>) -> Self;

    /// Clone `sessions` and `viewers`, run `edit` on the clones, repair every
    /// viewing record (deleting those whose session is gone), swap the clones
    /// in, bump `revision` and publish.
    /// An `Err` from `edit` discards the clones, leaving state unchanged.
    /// A publish failure after the swap returns `Unavailable`; the edit stays committed.
    async fn commit<R>(
        &mut self,
        edit: impl FnOnce(&mut Sessions, &mut Viewers) -> Result<R>,
    ) -> Result<R>;

    /// Keep `record.selection` if it is still inside `record.session` in `new`.
    /// Otherwise walk its ancestors in `old`, nearest first, and pick the first
    /// one still inside that session in `new`. `None` when the session is gone,
    /// which deletes the record and ends its stream (amendment A1).
    fn repair(old: &Sessions, new: &Sessions, record: ViewingRecord) -> Option<ViewingRecord>;

    fn replica(&self) -> Arc<Replica>;
}

// Internal messages. HTTP bodies are the shared types in ship-core::protocol.
pub struct ListSessions;
pub struct Get<T: Identified>(pub IdOf<T>);
pub struct Rename<T: Identified> { pub id: IdOf<T>, pub name: String }
pub struct Remove<T: Identified>(pub IdOf<T>);
pub struct Move { pub id: IdOf<Tab>, pub to: MoveTab }
pub struct Attach { pub attachment: IdOf<Attachment>, pub request: AttachRequest }
pub struct Detach(pub IdOf<Attachment>);
pub struct Select { pub attachment: IdOf<Attachment>, pub selection: NodeId }
pub struct SwitchSession { pub attachment: IdOf<Attachment>, pub session: IdOf<Session> }

// Reply types:
impl Message<ListSessions> for ServerState { type Reply = Sessions; }
impl Message<Create<Session>> for ServerState { type Reply = Result<Session>; } // and Tab, Pane
impl Message<Get<Session>> for ServerState { type Reply = Result<Session>; }    // and Tab, Pane
impl Message<Rename<Session>> for ServerState { type Reply = Result<Session>; } // and Tab, Pane
impl Message<Remove<Session>> for ServerState { type Reply = Result<()>; }     // and Tab, Pane
impl Message<Move> for ServerState { type Reply = Result<Tab>; }
/// Commit the new record and return it with the committed replica as the seed.
impl Message<Attach> for ServerState { type Reply = Result<Attached>; }
impl Message<Detach> for ServerState { type Reply = (); }  // idempotent
impl Message<Select> for ServerState { type Reply = Result<ViewingRecord>; }
impl Message<SwitchSession> for ServerState { type Reply = Result<ViewingRecord>; }
```

Session create and rename reject a name containing `:` by deserializing `SessionName` (product P4); the JSON extractor's rejection status, 422, applies, as it does to a wrong-kind ID in a body (spec amended 2026-10-05). Error categories used by handlers: unknown target `NotFound`, duplicate session name `Conflict`, wrong parent kind/self or descendant move/missing sibling/selection outside the attached session `InvalidStructure`.

Attach rules (slice 4, retention in slice 6): a missing requested session is `NotFound` (amendment A1). A requested selection is kept only if it is inside the requested session now; otherwise the record starts at the session root.

#### `crates/ship-server/src/state/tree.rs` (slice 2; panes in slice 3)

```rust
use ship_core::{prelude::*, id::*, model::*, protocol::Placement};

use super::Sessions;

/// Ancestry of `node`, session first and `node` last. `None` if absent.
pub(crate) fn path(sessions: &Sessions, node: NodeId) -> Option<Vec<NodeId>>;

pub(crate) fn tab<'a>(sessions: &'a Sessions, id: IdOf<Tab>) -> Result<&'a Tab>;
pub(crate) fn pane<'a>(sessions: &'a Sessions, id: IdOf<Pane>) -> Result<&'a Pane>;

/// Mutable access through `Arc::make_mut`, copying only the owning session.
pub(crate) fn tab_mut<'a>(sessions: &'a mut Sessions, id: IdOf<Tab>) -> Result<&'a mut Tab>;
pub(crate) fn children_mut<'a>(
    sessions: &'a mut Sessions,
    parent: IdOf<TabParent>,
) -> Result<&'a mut IndexMap<IdOf<Tab>, Tab>>;

/// Detach a tab with its subtree and panes from its parent.
pub(crate) fn take_tab(sessions: &mut Sessions, id: IdOf<Tab>) -> Result<Tab>;

/// Insert at the placement sibling, or append when `None`.
pub(crate) fn place(
    children: &mut IndexMap<IdOf<Tab>, Tab>,
    tab: Tab,
    placement: Option<Placement>,
) -> Result<()>;
```

A move runs inside `commit`: reject a destination whose `path` contains the moving tab, then `take_tab`, `children_mut`, `place`. A placement sibling equal to the moving tab is already gone after `take_tab`, so it fails as a missing sibling. Any failure drops the candidate.

#### `crates/ship-server/src/relay.rs` (slice 4)

A close port of an earlier project's `RelayBus` and sinks. Kept as is: `Sink`, `SendResult`, the `mpsc::Sender`, `Recipient` and closure sinks, `SinkExt::{filter, filter_map}`, `FilterSink`, `FilterMapSink`, `Subscribe`, and the `TypeId`-keyed subscription table. Added: a `watch::Sender` sink. Dropped: the inner `kameo_actors::MessageBus` and its direct `Register` path, `BusMessage`, `register_actor!` and the `publish!` macros, because Ship has one sink registration and nothing that uses them.

```rust
use std::{any::{Any, TypeId}, collections::HashMap, ops::ControlFlow};

use kameo::prelude::*;
use tokio::sync::{mpsc, watch};

#[derive(Actor, Default)]
pub struct RelayBus {
    subscriptions: HashMap<TypeId, Vec<Box<dyn Any + Send>>>,
}

pub struct Subscribe<T> {
    pub sink: Box<dyn Sink<T> + 'static>,
}
pub struct Publish<T>(pub T);

impl<T: Clone + Send + 'static> Message<Subscribe<T>> for RelayBus { type Reply = (); }

/// Clones the publication into each sink for `T`. `Break` removes that sink.
impl<T: Clone + Send + 'static> Message<Publish<T>> for RelayBus { type Reply = (); }

pub trait Sink<T>: Send + 'static {
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendResult { Sent, Full, Closed }

impl<T: Send + 'static> Sink<T> for mpsc::Sender<T>;          // Full -> Continue, Closed -> Break
impl<T: Clone + Send + 'static> Sink<T> for Recipient<T>;     // MailboxFull -> Continue(Full)
impl<T, F> Sink<T> for F where F: FnMut(T) -> ControlFlow<SendResult, SendResult> + Send + 'static;

/// New. Replaces the latest value, so it is never `Full`; `Closed` once every
/// receiver is gone. This is the sink for authoritative replicas.
impl<T: Send + Sync + 'static> Sink<T> for watch::Sender<T>;

pub trait SinkExt<T>: Sink<T> + Sized {
    fn filter<F>(self, predicate: F) -> FilterSink<Self, F>
    where F: FnMut(&T) -> bool + Send + 'static;
    fn filter_map<U, F>(self, f: F) -> FilterMapSink<Self, U, F>
    where F: FnMut(U) -> Option<T> + Send + 'static;
}
impl<T, S: Sink<T> + Sized> SinkExt<T> for S {}

pub struct FilterSink<S, F> { inner: S, predicate: F }
pub struct FilterMapSink<S, U, F> { inner: S, f: F, _phantom: PhantomData<fn(U)> }
impl<T, S: Sink<T>, F: FnMut(&T) -> bool + Send + 'static> Sink<T> for FilterSink<S, F>;
impl<T, U, S: Sink<T>, F: FnMut(U) -> Option<T> + Send + 'static> Sink<U> for FilterMapSink<S, U, F>;
```

Startup subscribes one sink: `Subscribe::<Arc<Replica>> { sink: Box::new(replica_tx) }`, where `replica_tx` is the `watch::Sender` whose receiver `AppState` holds. The `Recipient` sink is ported but must not carry authoritative state, because it drops a publication when the mailbox is full. The state actor's `bus.tell(Publish(..)).await` waits for relay mailbox space, so that hop is awaited and lossless.

#### `crates/ship-server/src/app.rs` (slice 1; relay and replicas in slice 4)

```rust
use kameo::actor::ActorRef;

use std::sync::Arc;

use ship_core::protocol::Replica;

use tokio::sync::watch;

use crate::{relay::RelayBus, state::ServerState};

/// The one Axum state. Every handler extracts this; each actor ref is cheap to clone.
#[derive(Clone)]
pub struct AppState {
    pub state: ActorRef<ServerState>,
    pub bus: ActorRef<RelayBus>,
    /// Latest published replica. Each SSE stream clones this receiver.
    pub replicas: watch::Receiver<Arc<Replica>>,
}

impl AppState {
    /// Ask the state actor and flatten the actor send error into
    /// `ErrorCode::Unavailable`.
    pub(crate) async fn ask<M>(&self, message: M) -> ship_core::Result<Reply<M>>
    where ServerState: Message<M, Reply = ship_core::Result<Reply<M>>>;
}
```

Handlers stay thin and still go through the state actor for every read and write, so a `GET` right after a `POST` always sees the write. Reading without the actor is a later option, noted under Unresolved.

#### `crates/ship-server/src/routes.rs` (slice 1; tabs in 2; panes in 3)

One concrete function per route, each annotated with `#[utoipa::path]` and registered through the existing `OpenApiRouter`/`routes!` path, so `openapi()` describes every route. Reuse comes from the generic messages and `AppState::ask`, not from generic handlers. Each body is one line; see below.

Every new route path is written out in full with the `/api/v0` prefix in its annotation, e.g.:

```rust
#[utoipa::path(
    get,
    path = "/api/v0/tabs/{id}",
    operation_id = "get_tab",
    params(("id" = String, Path, description = "Tab ID, e.g. tab:3f2a...")),
    responses(
        (status = 200, body = Tab),
        (status = 404, body = AppError),
    ),
)]
pub(crate) async fn get_tab(State(app): App, Path(id): Path<IdOf<Tab>>) -> Response;
```

```rust
use axum::{Json, extract::{Path, State}, http::StatusCode, response::{IntoResponse, Response}};
use ship_core::{id::*, model::*, protocol::*};

use crate::{AppState, state::*};

type App = State<AppState>;

pub(crate) async fn list_sessions(State(app): App) -> Response;
pub(crate) async fn create_session(State(app): App, Json(body): Json<Named<SessionName>>) -> Response;
pub(crate) async fn get_session(State(app): App, Path(id): Path<IdOf<Session>>) -> Response;
pub(crate) async fn rename_session(State(app): App, Path(id): Path<IdOf<Session>>, Json(body): Json<Named<SessionName>>) -> Response;
pub(crate) async fn remove_session(State(app): App, Path(id): Path<IdOf<Session>>) -> Response;

pub(crate) async fn create_tab(State(app): App, Json(body): Json<Create<Tab>>) -> Response;
pub(crate) async fn get_tab(State(app): App, Path(id): Path<IdOf<Tab>>) -> Response;
pub(crate) async fn rename_tab(State(app): App, Path(id): Path<IdOf<Tab>>, Json(body): Json<Named>) -> Response;
pub(crate) async fn remove_tab(State(app): App, Path(id): Path<IdOf<Tab>>) -> Response;
pub(crate) async fn move_tab(State(app): App, Path(id): Path<IdOf<Tab>>, Json(body): Json<MoveTab>) -> Response;

pub(crate) async fn create_pane(State(app): App, Json(body): Json<Create<Pane>>) -> Response;
pub(crate) async fn get_pane(State(app): App, Path(id): Path<IdOf<Pane>>) -> Response;
pub(crate) async fn rename_pane(State(app): App, Path(id): Path<IdOf<Pane>>, Json(body): Json<Named>) -> Response;
pub(crate) async fn remove_pane(State(app): App, Path(id): Path<IdOf<Pane>>) -> Response;
```

Every handler returns `axum::response::Response` and ends with `.into_response()`. Bodies stay one line, because `Result<T, E>` is itself `IntoResponse` when both sides are:

```rust
app.ask(Get::<Tab>(id)).await.map(Json).into_response()
app.ask(create).await.map(|tab| (StatusCode::CREATED, Json(tab))).into_response()
app.ask(Remove::<Tab>(id)).await.map(|()| StatusCode::NO_CONTENT).into_response()
```

The `#[utoipa::path]` `responses(...)` list documents each status and body, since the `Response` return type no longer carries that information.


Routing follows the approved architecture table, with every path under `/api/v0` (for example `POST /api/v0/tabs/{id}/move`, `POST /api/v0/attach`). The existing `/health` route stays unprefixed.

#### `crates/ship-server/src/attach.rs` (slice 4; controls in slice 5)

```rust
use std::convert::Infallible;

use axum::{Json, extract::{FromRequestParts, State}, response::{IntoResponse, Response, sse::{Event, Sse}}};
use futures_util::Stream;
use kameo::actor::ActorRef;
use ship_core::{id::*, protocol::*};

use ship_core::AppError;

use crate::{AppState, state::ServerState};

/// Mints the attachment ID before asking the state actor, so cancellation at
/// any point still detaches the right ID. Drop spawns `state.tell(Detach(id))`
/// and ignores a send failure after the actor has stopped.
struct AttachmentGuard {
    attachment: IdOf<Attachment>,
    state: ActorRef<ServerState>,
}
impl Drop for AttachmentGuard;

/// Asks the state actor for the seed, clones `app.replicas`, emits `Attached`,
/// then `State` for each replica whose revision is newer than the seed. Ends
/// when a replica no longer lists this attachment (its session was removed) or
/// when the watch sender is dropped at shutdown. The guard lives inside the
/// stream. Keepalive comments every 15 seconds.
/// A5 (slice 7): each end first emits `Ended(SessionRemoved)` or
/// `Ended(ServerShutdown)` respectively.
pub(crate) async fn attach(State(app): State<AppState>, Json(body): Json<AttachRequest>)
    -> Response;

/// Requires and parses `X-Ship-Attachment-Id`; missing or malformed is 400.
/// Whether the ID is active is the state actor's 404, not the extractor's.
pub(crate) struct AttachmentHeader(pub IdOf<Attachment>);
impl<S: Send + Sync> FromRequestParts<S> for AttachmentHeader { type Rejection = AppError; }

pub(crate) async fn select(
    State(app): State<AppState>,
    attachment: AttachmentHeader,
    Json(body): Json<SelectRequest>,
) -> Response;

pub(crate) async fn switch_session(
    State(app): State<AppState>,
    attachment: AttachmentHeader,
    Json(body): Json<SwitchSessionRequest>,
) -> Response;
```

#### `crates/ship/src/commands.rs` (slice 1; tabs in 2; panes in 3)

```rust
use ship_client::Client;
use ship_core::prelude::*;

use crate::cli::{PaneCommand, SessionCommand, TabCommand};

/// Each command prints its JSON result on stdout, or nothing for removal.
pub async fn session(client: &Client, command: SessionCommand) -> Result<()>;
pub async fn tab(client: &Client, command: TabCommand) -> Result<()>;
pub async fn pane(client: &Client, command: PaneCommand) -> Result<()>;
```

#### `crates/ship/src/observe.rs` (slice 4; controls in 5; reconnect in 6)

```rust
use std::time::Duration;

use ship_client::Client;
use ship_core::{prelude::*, id::*, model::*, protocol::*};

const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// `ship attach <session>`. One loop selects over the SSE stream, stdin control
/// lines and SIGINT. On EOF or a body error it reports the disconnection on
/// stderr and reattaches with backoff (250 ms doubling, capped at 5 s), sending
/// the remembered session ID and selection. A `404` on reattach means the
/// session was removed: it prints `session removed` and exits 0 (product P1).
/// Otherwise exits only on SIGINT.
/// A5 (slice 7): `Ended(SessionRemoved)` prints `session removed` and exits 0
/// without reattaching; `Ended(ServerShutdown)` reports
/// `disconnected: server shutting down` and reattaches like any other end.
pub async fn run(client: &Client, session: IdOf<Session>) -> Result<()>;

struct Observer {
    attachment: Option<IdOf<Attachment>>,
    replica: Option<Replica>,
}

impl Observer {
    /// `Attached` replaces everything. `State` applies only for the same
    /// incarnation and a higher revision. Returns whether anything changed.
    fn apply(&mut self, event: SseEvent) -> bool;

    /// This observer's record, matched by attachment ID.
    fn record(&self) -> Option<&ViewingRecord>;

    /// What to send on reattach: the last record's session ID and selection.
    fn remembered(&self) -> AttachRequest;

    /// Indented tree of the attached session, selection marked with `*`.
    fn render(&self) -> String;
}
```

Output: the rendered tree goes to stdout after every applied event. Status lines go to stderr: `attached attachment:... to session:...`, `session removed`, `disconnected: <error>; retrying in 1s`. Example render:

```text
session:6a1f... "work" (revision 14)
  tab:02c9... "editor"
    pane:77b0... "left"
    pane:1d3e... "right" *
  tab:9f40... "logs"
```

#### `crates/ship/src/controls.rs` (slice 5)

```rust
//! Temporary same-process observer controls. Delete with the real client UI;
//! observe.rs then drops its stdin branch and nothing else changes.

use ship_core::{prelude::*, id::*, model::NodeId};

/// `select <id>` or `switch <session name or ID>`, one per stdin line.
/// `switch` resolves the name through `Client::resolve_session` first.
pub enum Control {
    Select(NodeId),
    Switch(SessionRef),
}

pub fn parse(line: &str) -> Result<Control>;
```

### Moved

None.

### Replaced

#### `crates/ship-core/src/error.rs` (slice 1)

Add two `ErrorCode` variants for the architecture's status table: `InvalidStructure` maps to 422 and `Unavailable` maps to 503. Add an optional `axum` feature so the server returns `AppError` directly, with no wrapper error type. The orphan rule requires this impl to live in `ship-core`; the feature keeps Axum out of every other crate that depends on core.

```rust
/// Status from `ErrorCode::http_status`; body is the serialized `AppError`
/// so the client can show the original message.
#[cfg(feature = "axum")]
impl axum::response::IntoResponse for AppError;
```

#### `crates/ship-core/src/protocol.rs` and `lib.rs` (slices 1, 2, 4, 5)

Keep the health items. Add `pub mod id; pub mod model;` to `lib.rs` and these shared bodies to `protocol.rs`:

```rust
use std::sync::Arc;

use indexmap::IndexMap;
use uuid::Uuid;

use crate::{id::*, model::*};

pub const ATTACHMENT_HEADER: &str = "x-ship-attachment-id"; // slice 5

/// POST /tabs and /panes body: `{"parent": "...", "name": "..."}`.
#[derive(Serialize, Deserialize)]
pub struct Create<T: Creatable> {
    pub parent: IdOf<T::Parent>,
    #[serde(flatten)]
    pub input: T::Input,
}

/// POST /tabs/{id}/move body. No placement appends.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveTab {
    pub parent: IdOf<TabParent>,
    pub placement: Option<Placement>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Placement {
    Before(IdOf<Tab>),
    After(IdOf<Tab>),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachRequest {
    pub session: IdOf<Session>,
    pub selection: Option<NodeId>,
}

#[derive(Serialize, Deserialize)]
pub struct SelectRequest { pub selection: NodeId }

#[derive(Serialize, Deserialize)]
pub struct SwitchSessionRequest { pub session: IdOf<Session> }

/// Server-owned view of one attachment. Always names a session; the record
/// is deleted when the attachment ends or its session is removed.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewingRecord {
    pub session: IdOf<Session>,
    pub selection: NodeId,
}

/// Complete replicated state. Session `Arc`s are shared with the state actor.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replica {
    /// UUID v7 chosen at server start; a later start sorts higher.
    pub incarnation: Uuid,
    pub revision: u64,
    pub sessions: IndexMap<IdOf<Session>, Arc<Session>>,
    pub viewers: IndexMap<IdOf<Attachment>, ViewingRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SseEvent {
    Attached(Attached),
    State(Arc<Replica>),
    Ended(EndReason),                          // A5, slice 7: last event before close
}

/// A5, slice 7. Derived per stream, never published through the relay.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EndReason {
    SessionRemoved,
    ServerShutdown,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attached {
    pub attachment: IdOf<Attachment>,
    pub replica: Arc<Replica>,
}
```

#### `crates/ship-server/src/lib.rs` (slices 1, 4, 7)

`router()` becomes `router(app: AppState)`. `serve` creates the replica watch channel, spawns the `RelayBus` with a bounded mailbox of 64, subscribes the watch sender, spawns the state actor, and puts the receiver in `AppState`.

```rust
pub fn router(app: AppState) -> Router;                   // slice 1
pub async fn serve(address: SocketAddr, shutdown: impl Future<Output = Result<()>> + Send + 'static)
    -> Result<()>;                                       // signature unchanged
```

Slice 7 adds two behaviors to `serve`. When any actor stops, shutdown begins with an `Internal` error. When shutdown begins for any reason, the actors stop first. Stopping the `RelayBus` drops the watch sender, which ends every SSE stream and lets Axum's existing five-second drain finish. The compression layer also lands in slice 7:

```rust
CompressionLayer::new()
    .zstd(true)
    .gzip(true)
    .compress_when(SizeAbove::new(32).and(NotForContentType::GRPC).and(NotForContentType::IMAGES))
// Same as the default predicate minus NotForContentType::SSE.
```

`openapi()` now describes every route. Nothing else about how it is built changes.

#### `crates/ship-client/src/lib.rs` and `api.rs` (slices 1 to 5)

The private `request` helper currently only sends bodiless requests and treats every unexpected status as a bare `UpstreamHttpStatus`. It is replaced by one helper that takes an optional JSON body and an optional attachment ID. On an unexpected status it decodes the body as `AppError` and returns that error, so server messages such as "session name already exists" reach the CLI. If decoding fails it falls back to the status and body text. The existing two-second timeout, URL scrubbing and `http_error` classification stay.

Session names are a client convenience (product P2 to P4, architecture A1). The server never sees a name except as a session's `name` field.

```rust
/// A session ID or name, as typed on the command line. Tries the ID first;
/// `SessionName` then rejects anything containing ':', so `tab:3f2a` is an error.
pub enum SessionRef {
    Id(IdOf<Session>),
    Name(SessionName),
}
impl FromStr for SessionRef { type Err = AppError; }

/// A tab parent as typed on the command line. The tab ID comes first, matching
/// `UntaggedEither`'s try-in-order parsing, so a tab ID is never read as a session.
pub type TabParentRef = UntaggedEither<IdOf<Tab>, SessionRef>;

/// Collection path for the generic entity methods.
pub trait Resource: Identified + DeserializeOwned {
    const COLLECTION: &'static str; // "/api/v0/sessions", "/api/v0/tabs", "/api/v0/panes"
}

impl Client {
    pub async fn health(&self) -> Result<HealthResponse>;                          // unchanged
    pub async fn sessions(&self) -> Result<IndexMap<IdOf<Session>, Session>>;
    pub async fn create_session(&self, name: &SessionName) -> Result<Session>;
    pub async fn create_tab(&self, parent: IdOf<TabParent>, name: &str) -> Result<Tab>;
    pub async fn create_pane(&self, parent: IdOf<Tab>, name: &str) -> Result<Pane>;
    pub async fn get<T: Resource>(&self, id: IdOf<T>) -> Result<T>;
    pub async fn rename<T: Resource>(&self, id: IdOf<T>, name: &str) -> Result<T>;
    pub async fn remove<T: Resource>(&self, id: IdOf<T>) -> Result<()>;
    pub async fn move_tab(&self, id: IdOf<Tab>, to: &MoveTab) -> Result<Tab>;

    /// IDs pass through untouched; a name is looked up with `GET /sessions`.
    /// The only place a session name becomes an ID.
    pub async fn resolve_session(&self, session: &SessionRef) -> Result<IdOf<Session>>;
    pub async fn resolve_parent(&self, parent: &TabParentRef) -> Result<IdOf<TabParent>>;

    /// Attach-or-create for `ship attach <name>`: resolve, create if absent,
    /// and resolve again after a `409` from a concurrent creator.
    pub async fn ensure_session(&self, name: &SessionName) -> Result<IdOf<Session>>;      // slice 4

    /// The two-second timeout covers only the response head. The returned
    /// stream yields decoded events and ends on EOF.
    pub async fn attach(&self, request: &AttachRequest)
        -> Result<impl Stream<Item = Result<SseEvent>>>;                           // slice 4
    pub async fn select(&self, attachment: IdOf<Attachment>, selection: NodeId)
        -> Result<ViewingRecord>;                                                  // slice 5
    pub async fn switch_session(&self, attachment: IdOf<Attachment>, session: IdOf<Session>)
        -> Result<ViewingRecord>;                                                  // slice 5
}
```

The attachment ID is a call argument. Each observer passes whichever ID its current stream received, so observers never share control context.

#### `crates/ship/src/cli.rs` (slices 1 to 4)

Every variant is a unit or a newtype around a named struct. The existing `Server` variant moves to `Server(ServerArgs)` with the same flags.

```rust
#[derive(Subcommand)]
pub enum Command {
    /// Run a foreground server on 127.0.0.1; stop with SIGINT or SIGTERM
    Server(ServerArgs),
    /// Create, inspect, rename or remove sessions
    #[command(subcommand)]
    Session(SessionCommand),
    /// Create, inspect, rename, remove or move tabs
    #[command(subcommand)]
    Tab(TabCommand),
    /// Create, inspect, rename or remove logical panes
    #[command(subcommand)]
    Pane(PaneCommand),
    /// Observe a session as text; stdin accepts `select <id>` and `switch <session-id>`
    Attach(AttachArgs),
}

#[derive(Subcommand)]
pub enum SessionCommand {
    List,
    Create(NameArgs),
    Get(SessionArgs),
    Rename(RenameSessionArgs),
    Rm(SessionArgs),
}

#[derive(Subcommand)]
pub enum TabCommand {
    Create(CreateTabArgs),
    Get(IdArgs<Tab>),
    Rename(RenameArgs<Tab>),
    Rm(IdArgs<Tab>),
    /// Move under PARENT; appends unless --before or --after names a sibling
    Move(MoveTabArgs),
}

#[derive(Subcommand)]
pub enum PaneCommand {
    Create(CreatePaneArgs),
    Get(IdArgs<Pane>),
    Rename(RenameArgs<Pane>),
    Rm(IdArgs<Pane>),
}

#[derive(Args)]
pub struct ServerArgs {
    #[arg(long, default_value_t = DEFAULT_PORT, value_parser = clap::value_parser!(u16).range(1..))]
    pub port: u16,
    #[arg(long, hide = true)]
    pub background_child: bool,
}

#[derive(Args)]
pub struct NameArgs { pub name: SessionName }

#[derive(Args)]
pub struct IdArgs<T: Identified> where IdOf<T>: FromStr<Err = AppError> {
    pub id: IdOf<T>,
}

#[derive(Args)]
pub struct RenameArgs<T: Identified> where IdOf<T>: FromStr<Err = AppError> {
    pub id: IdOf<T>,
    pub name: String,
}

/// Session name or ID
#[derive(Args)]
pub struct SessionArgs { pub session: SessionRef }

#[derive(Args)]
pub struct RenameSessionArgs { pub session: SessionRef, pub name: SessionName }

#[derive(Args)]
pub struct CreateTabArgs {
    /// Session name or ID, or tab ID
    pub parent: TabParentRef,
    pub name: String,
}

#[derive(Args)]
pub struct CreatePaneArgs {
    /// Tab ID
    pub tab: IdOf<Tab>,
    pub name: String,
}

#[derive(Args)]
pub struct MoveTabArgs {
    pub id: IdOf<Tab>,
    /// Session name or ID, or tab ID
    pub parent: TabParentRef,
    #[arg(long, conflicts_with = "after")]
    pub before: Option<IdOf<Tab>>,
    #[arg(long)]
    pub after: Option<IdOf<Tab>>,
}

#[derive(Args)]
/// A name attaches or creates (`Client::ensure_session`); an ID never creates.
pub struct AttachArgs { pub session: SessionRef }
```

`IdArgs<T>` and `RenameArgs<T>` now serve tabs and panes only. The `FromStr` bound sits on the generic structs, not on `Identified`, because `ServerRoot`'s identity `()` does not implement `FromStr`. A throwaway scratch crate (clap 4, outside the repo, now discarded evidence only) confirmed that Clap's derive accepts these generic structs with the bound in a `where` clause and reports the prefix error as `invalid value 'session:7' for '<ID>': expected tab ID, got session`.

Clap parses IDs through `FromStr`, so `ship pane create session:... x` fails before any request with the prefix error.

#### `crates/ship/src/main.rs` (slice 1)

The URL validation and client construction in `dispatch` move into `async fn connect(cli: &Cli, explicit_target: bool) -> Result<Client>`, which keeps the current rule: an explicit `--server-url` is used as is, otherwise the default local server is reused or started. Bare `ship` keeps printing health. Entity and attach commands call `connect` and then `commands::*` or `observe::run`.

#### Proposed dependency settings

Not real manifests. Versions are the current compatible releases at implementation time.

```toml
# workspace
indexmap = { version = "2", features = ["serde"] }
uuid = { version = "1", features = ["v4", "v7", "serde"] }   # v7 for incarnation
serde_with = "3"                                         # SerializeDisplay, DeserializeFromStr
serde = { version = "1", features = ["derive", "rc"] }   # add "rc" for Arc<Session>
kameo = "<current 0.x>"
futures-util = "0.3"
tokio-stream = { version = "0.1", features = ["sync"] }  # WatchStream
eventsource-stream = "0.2"
reqwest = { ..., features = ["json", "rustls", "stream", "zstd", "gzip"] }
tower-http = { ..., features = ["trace", "compression-zstd", "compression-gzip"] }

# ship-core: + indexmap, uuid, serde_with; optional axum behind feature "axum"
# ship-server: ship-core with features = ["axum"]
# ship-server: + kameo, indexmap, uuid, futures-util, tokio-stream
# ship-client: + indexmap, futures-util, eventsource-stream
# ship: + indexmap (if commands print maps through it)
```

### Untouched

- `crates/ship-server/src/health.rs`, the health route and its OpenAPI entry.
- `crates/ship/src/diagnostics.rs` and `crates/ship/src/local.rs`. Local startup is reused through `connect`.
- The `AppError` structure, `err!` macro and `ResultExt`.
- Loopback-only binding, the five-second drain and the unauthenticated loopback trust model.
- `experiments/`, `docs/research/`, `.scratch/`.

### Unresolved

1. **OpenAPI coverage, resolved.** Every new route is annotated and appears in `openapi()` (Cyan, 2026-10-05).
2. **Attachment ID minting, resolved.** The attach route creates the ID before asking the state actor (Cyan, 2026-10-05). This changes architecture decision 10's wording and is recorded as amendment A4 there.
3. **Full replica per event.** Decision 8 does not say whether a replica covers all sessions or only the observer's. This plan sends all sessions and all records. Filtering per session would need a `filter` adapter per observer and is deferred until payload size matters.
4. **Upstream amendments, resolved.** Product P1 to P4 and architecture A1 to A3 were approved on 2026-10-05; this paper follows them.
5. **Reads without the state actor.** `AppState.replicas` lags a just-acknowledged write by the relay hop, so `GET` handlers keep asking the state actor. If direct reads are wanted later, the state actor would update the watch channel itself inside `commit`, skipping the bus for replicas. Not planned for this change.
6. **`/health` and the `/api/v0` prefix, resolved.** `/health` stays unprefixed as the version-independent discovery endpoint; only new routes use `/api/v0` (Cyan, 2026-10-05).

## Build order

Each slice ends with the workspace building and `cargo clippy` clean, plus the listed real CLI workflow. Run workflows against `ship server --port <p>` in the foreground and `--server-url http://127.0.0.1:<p>` on every client. No permanent tests are added. Slices 1 to 5 also rerun the initial-scaffolding OpenAPI check: a disposable consumer outside the repo calls `ship_server::openapi()`, and the document lists that slice's `/api/v0` paths with their request and response schemas.

### 1. Session CRUD

Files: `ship-core` `id.rs` (without `UntaggedEither`), `model.rs` (Session only), `error.rs` variants, `protocol.rs` `Create` stub; `ship-core` `axum` feature; `ship-server` `state.rs` (no relay or viewers yet; `commit` only swaps), `app.rs`, `routes.rs` session handlers, `lib.rs` router state and actor spawn; `ship-client` request helper, session methods, `SessionRef` and `resolve_session`; `ship` `cli.rs` `Session`, `commands.rs`, `main.rs` `connect`.

Delivers: `ship session create work | jq -r .id` prints `session:...`. `list`, `get`, `rename` and `rm` work by ID or by name (`SessionRef`, `resolve_session`).

Checks: a duplicate create and a rename onto an existing name both fail with a readable `ship: ...` message and exit 1, and `session list` output is byte-identical before and after. `get` on a removed ID reports not found. A bad prefix (`ship session get tab:...`) fails in argument parsing. `ship session create a:b` fails with the `:` rule and changes nothing. `get work` and `get session:...` print the same entity.

### 2. Recursive tabs and moves

Files: `id.rs` `UntaggedEither`; `model.rs` `Tab`, `TabParent`; `protocol.rs` `MoveTab`, `Placement`; `state/tree.rs`; tab messages in `state.rs`; tab handlers in `routes.rs`; client tab methods and `resolve_parent`; `cli.rs` `Tab`; `commands.rs` `tab`.

Delivers: nested tabs under sessions and tabs, append by default, `move` with `--before`/`--after`, cross-session moves.

Checks: build two sessions with a three-level tab tree, creating root tabs with the session name as parent. Reorder siblings, move a subtree to the other session, and confirm with `session get | jq` that the moved tab IDs are unchanged. Moving a tab under itself or its descendant, `--before` a non-sibling, and a nonexistent parent each fail, and both sessions' `get` output is byte-identical before and after. Removing a middle tab removes its descendants. Removing a session removes all its tabs.

### 3. Logical panes

Files: `model.rs` `Pane`, `Tab::panes`; pane parts of `tree.rs` and `state.rs`; pane handlers; client pane methods; `cli.rs` `Pane`; `commands.rs` `pane`.

Delivers: pane create/get/rename/rm under a tab, visible in session and tab `get` output. No process is started.

Checks: SC-001 end to end. Pane IDs are unchanged after a cross-session move of their owning tab. Removing one pane leaves sibling panes and child tabs. Removing a tab removes its panes and its descendants' panes. `pane create session:...` and `pane create pane:...` fail, and a raw `curl` POST with a session parent returns 422 with state unchanged. `ps` shows no new child processes of the server.

### 4. Live observation

Files: `relay.rs`, `app.rs` replica receiver, `attach.rs` (stream and guard only), `state.rs` viewers/revision/incarnation/publish/`Attach`/`Detach`, `model.rs` `NodeId`; `protocol.rs` `AttachRequest`, `ViewingRecord`, `Replica`, `SseEvent`, `Attached`; `lib.rs` actor wiring and `/attach` route; client `attach` and `ensure_session`; `cli.rs` `Attach`; `observe.rs` without controls or reconnect (exits on disconnect).

Delivers: `ship attach <session>` prints its attachment ID on stderr and the tree on stdout, and reprints after every edit.

Checks: two observers on one session both show each CLI edit. An observer started while a shell loop is creating tabs shows the final tree without another edit. Slow client: `kill -STOP` one observer, make 50 edits, `kill -CONT`, and it prints the final tree without a further edit. Closing an observer removes its record; the other observer's next printed replica no longer lists it. Removing the observed session makes both observers print `session removed` and exit (SC-005). `ship attach scratch` creates `scratch` when absent; two concurrent `ship attach scratch` runs end with one `scratch` in `session list` (SC-007).

### 5. Selection and session switching

Files: `controls.rs`; `observe.rs` stdin branch; `attach.rs` `AttachmentHeader`, `select`, `switch_session`; `state.rs` `Select`, `SwitchSession`, `repair`; `protocol.rs` header constant, `SelectRequest`, `SwitchSessionRequest`; client `select`, `switch_session`.

Delivers: typing `select <id>` or `switch <session>` into an observer changes only that observer's marker.

Checks: SC-003. Two observers select different tabs, then an empty tab and a pane; each keeps its own marker. Removing a selected pane moves that marker to its tab. Removing a selected tab's ancestor moves it to the nearest surviving tab. A within-session move keeps the marker on the moved pane. A cross-session move falls back to the nearest surviving source parent and the observer stays on its session. `switch <name>` moves a live observer to another session, resolving the name first. `curl` `POST /attach` for a removed session ID returns 404. `curl` without the header returns 400; with an ended attachment ID, 404.

### 6. Reconnect

Files: `observe.rs` reconnect loop and `remembered`; `state.rs` attach retention rule.

Delivers: a running observer survives a dropped connection and resumes with current state.

Checks: SC-004. Put `socat TCP-LISTEN:<q>,fork,reuseaddr TCP:127.0.0.1:<p>` between one observer and the server. Kill `socat`: the observer reports disconnection and retries at growing intervals capped at 5 s. Make edits directly against the server, restart `socat`, and the observer prints current state without another edit. Repeat with its selected pane still in the session (kept), removed (session root), and moved to another session with its tab (session root). Repeat with its session removed during the outage: reattach gets 404, and it prints `session removed` and exits (SC-005). The old attachment's record disappears from the other observer's replica.

### 7. Compression, failure and shutdown

Files: `lib.rs` compression layer, actor-stop monitoring and shutdown ordering; dependency features for zstd and gzip. A5: `protocol.rs` `SseEvent::Ended` and `EndReason`, `attach.rs` final event, `observe.rs` handling.

Delivers: compressed SSE through the real client path, clean shutdown with observers attached, and streams that say why they end (A5).

Checks: `curl -N -D - -H 'Accept-Encoding: zstd' -X POST .../api/v0/attach -d '{"session":"session:..."}' | head -c 200` shows `content-encoding: zstd` and binary frames arriving before the stream ends. The real observer prints each edit as it happens through the same server, and `RUST_LOG=reqwest=trace` or a debug log of the response's `content-encoding` header confirms zstd was negotiated rather than identity. Repeat with gzip-only `Accept-Encoding` via `curl`. `kill -TERM` on the server with two observers attached exits within five seconds with status 0, and both observers report disconnection. Run slices 1 to 7 on Linux as well as macOS; if Linux is unavailable, record it as unverified for SC-006.

Actor failure is hard to trigger from outside. Check it once by temporarily making the `RelayBus` panic on a chosen publication, confirm the server exits with an error instead of serving stale observers, and revert. Record that it was a temporary probe.

A5 checks: removing an observed session makes `curl` receive an `ended` event with `sessionRemoved` before the stream closes, and the observer prints `session removed` and exits 0 with no `disconnected` line and no reattach request in the server log. `kill -TERM` on the server makes `curl` receive `ended` with `serverShutdown`, and the observer prints `disconnected: server shutting down` and keeps retrying. Cutting the relay still gives a plain disconnection, and removing the session during an outage still exits through the reattach `404`. The `openapi()` consumer shows `SseEvent` with the `Ended` variant.

## Deviation log

### Slice 1: session CRUD (2026-10-05)

Slice 1 works end to end and every slice 1 check passed. No locked paper was reopened, but four places differ from the snippets above, and two observed behaviors disagree with the spec delta and need Cyan's call (see "Open against the spec").

#### Differences from the snippets

- **`Id<T>` schema uses `ComposeSchema`.** Utoipa 6 implements `PartialSchema` for every `T: ComposeSchema` and treats a field typed `IdOf<Session>` as a generic, so a hand-written `PartialSchema` impl conflicts. `id.rs` implements `utoipa::__dev::ComposeSchema` (ignoring the entity's schema) plus an empty `ToSchema`. The component is named `Id_Session`. `__dev` is a hidden Utoipa module, so a Utoipa upgrade may move it.
- **`AppError` derives `ToSchema`.** The error types gain `ToSchema` derives so routes can document `body = AppError`. `InternalError::caused_by` needs `#[schema(no_recursion)]`; without it, router construction overflowed the stack at server start. The serialized structure is unchanged.
- **No `AppState::ask`.** Cyan chose (2026-10-05) to drop the wrapper: `ship-core` gains an optional `kameo` feature with `impl<M> From<kameo::error::SendError<M, AppError>> for AppError`, which passes a handler's own error through and maps every other send failure to `Unavailable` (503). Handlers call `app.state.ask(..).await.map_err(AppError::from)`. This puts an optional actor dependency in `ship-core`, which the Rationale above kept free of actors; `ship-server` is the only crate enabling it. `ListSessions` replies `Result<Sessions>`, not `Sessions`, so every message replies with a `Result`. `commit` is synchronous and edits only `Sessions` until slice 4 adds viewers and publishing.
- **CLI global option.** The existing `args_conflicts_with_subcommands = true` rejected `ship --server-url URL session ...`, which every workflow in this paper uses. It is removed. To keep the old rejection of `ship --server-url URL server`, `dispatch` now returns a `Configuration` error for that combination. `connect` takes the URL string rather than `&Cli`. `ATTACHMENT_HEADER` landed in slice 1 because task 1.3 puts the header in the request helper.

- **Code review changes (Cyan, 2026-10-05).** `Id<T>` parsing no longer requires the simple form: after the prefix check it accepts any spelling `Uuid::parse_str` does (uppercase, hyphenated, braced, `urn:uuid:`), and Display still writes the simple lowercase form. The schema pattern still describes the form the server emits. The attachment prefix is `attachment`, not `att`; the observer status line and task 4.2 are updated to match. Handlers return `ship_core::Result<Response>` and use `?` through the `SendError` `From` impl, rather than returning `Response`.

#### Spec amended for two observed behaviors

Cyan approved amending the `session-structure` spec on 2026-10-05 rather than changing the code:

- A JSON body that parses but does not fit the operation gets Axum's JSON rejection, HTTP 422 with a plain-text body. That covers a session name containing `:` and, from slice 2, a wrong-kind ID such as a session parent for a pane, which slice 3's check already expects as 422. HTTP 400 now means a malformed path ID or invalid JSON syntax. Architecture decision 3's "malformed input `400`" is read the same way; its text is unchanged.
- Clap rejects invalid arguments (`ship session get tab:...`, `ship session create a:b`) before any request with exit 2, the conventional usage-error code. Other failed operations, such as a duplicate name or a missing target, exit 1.

#### Evidence

macOS 26.6 arm64, Rust 1.98.0. Foreground `ship server --port 43901`; clients used `--server-url http://127.0.0.1:43901`.

| Check | Observation |
| --- | --- |
| 1.1 ID round trip | Disposable consumer outside the repo: `session:<32 hex>` survived Display, `FromStr` and JSON. A `tab:` ID failed with `expected session ID, got tab`; `a:b` failed with `session name 'a:b' must not contain ':'` from both `FromStr` and JSON. `cargo build --workspace` passed. |
| 1.2 curl | `POST /api/v0/sessions` returned 201 with the entity; a duplicate returned 409 with `{"source":"internal","message":"session name 'work' already exists","type":{"kind":"Conflict"}}`; an unknown ID returned 404 with an `AppError` body; a `tab:` path ID returned 400; `/health` unchanged. |
| 1.3 CLI | `ship session create work \| jq -r .id` printed `session:e2507b35...`; `ship --help` lists `session`, and `ship session --help` lists `list`, `create`, `get`, `rename`, `rm`. |
| 1.4 failures | Duplicate create and rename onto an existing name each printed `ship: request to .../api/v0/sessions... failed: session name 'work' already exists` and exited 1; `session list` stayed byte-identical (`cmp`). `get` of a removed ID printed `... session session:2f87... not found`, exit 1. `get tab:...` and `create a:b` failed in Clap parsing (exit 2) and the list was unchanged. `get work` and `get session:...` output matched byte for byte. An unknown name printed `ship: no session named 'nowhere'`, exit 1. |
| 1.4 OpenAPI | Disposable consumer of `ship_server::openapi()`: OpenAPI 3.1.0 lists `list_sessions`, `create_session`, `get_session`, `rename_session` and `remove_session` at `/api/v0/sessions` and `/api/v0/sessions/{id}` with their statuses, `Named_SessionName` request bodies and `Session` responses. The list response is an object of `Session`. Every `$ref` resolves to a component; `Id_Session` is a string with pattern `^session:[0-9a-f]{32}$`. |
| 1.4 build gates | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace` and `cargo build --workspace --release` passed. |

The disposable consumers and the test server were removed or stopped afterwards. Linux was not exercised.

### Slice 2: recursive tabs and moves (2026-10-05)

Slice 2 works end to end and every slice 2 check passed. No locked paper was reopened. The differences below are all at the snippet level.

#### Differences from the snippets

- **Five tab handlers, not six.** Task 2.2 says "six `/api/v0` tab handlers", but the architecture route table and the `routes.rs` snippet define five: create, get, rename, remove and move. Those five are implemented.
- **`tree::path` is keyed by `IdOf<TabParent>` until slice 4.** `NodeId` lands in slice 4, so `path` takes and returns `IdOf<TabParent>` for now. Slice 4 widens it to `NodeId` when panes and selection need it.
- **Recursive `tabs` schema is hand-written.** Utoipa's derive treats the `IdOf<Tab>` map key as a generic and inlines `Tab`'s schema to describe it, so `Tab`'s schema recursed until the server overflowed its stack while building the router. `#[schema(no_recursion)]` did not help. `Session::tabs` and `Tab::tabs` use `schema_with = tabs_schema`: an object whose `propertyNames` is the tab ID pattern and whose values `$ref` `Tab`.
- **`UntaggedEither`'s schema ignores the schemas the derive passes it,** as `Id<T>` already does. For a field typed `IdOf<TabParent>`, the derive passes the session-or-tab *entity* schema, not the ID schemas. `MoveTab::parent` is `#[schema(inline)]`, so it renders as a `oneOf` of the session and tab ID patterns, matching `Create_Tab.parent`. The derive still registers an unreferenced `UntaggedEither` component (the entity choice). It is accurate but unused.
- **`Create<T>` schema is hand-written,** the fallback this paper names: the derive cannot resolve `IdOf<T::Parent>` or `T::Input`. It is one generic impl for every `T: Creatable<Input = Named>` (tabs now, panes in slice 3), rendering `{parent, name}` as a flat object (Cyan, code review 2026-10-05). Its `ToSchema::name` is `Create`, and the route macro appends the entity, so the component is `Create_Tab`. Sessions are not covered; their route takes `Named<SessionName>` directly. Tab rename documents its body as `Named<String>`. A bare `Named` produced a dangling `$ref` to `String`.
- **`UntaggedEither` also implements `Display`.** If both `FromStr` attempts fail, the error message includes both errors.
- **Clap generic args need more bounds.** `IdArgs<T>` and `RenameArgs<T>` require `IdOf<T>: FromStr<Err = AppError> + Send + Sync + 'static`. Without `Send + Sync + 'static`, `ArgMatches::remove_one` rejects the field.
- `Tab` has no `panes` field yet; slice 3 adds it.

#### Evidence

macOS 26.6 arm64. Foreground `ship server --port 43902`; clients used `--server-url http://127.0.0.1:43902`.

| Check | Observation |
| --- | --- |
| 2.1 curl | Sessions `a` (`t1 > t2 > t3`) and `b`. `POST /tabs/{t1}/move` with parent `t3` and with parent `t1` each returned 422 `cannot move tab ... into itself or its own descendant`. `cmp` showed both sessions' `GET` output byte-identical before and after. |
| 2.2 CLI | `ship tab create work editor` printed `{"id":"tab:7bd6...","name":"editor","tabs":{}}`, and `session get work` listed it as a root tab. `ship tab --help` lists `create`, `get`, `rename`, `rm` and `move`. |
| 2.3 tree and moves | Built `work`: `A{A1{A1x},A2},B,C` and `api`: `D`, with session names as root parents. `move C work --before A` gave `C,A,B`. `--after B` gave `A,B,C`. `move A api` appended A last in `api` and removed it from `work`. The `jq` list of every ID in A's subtree was identical before and after. |
| 2.3 failures | Self move, descendant move, `--before` a non-sibling, `--before` the moving tab itself, an unknown tab parent and an unknown session name each printed a `ship: ...` message and exited 1. `cmp` showed both sessions byte-identical after each one. `tab get session:...` failed in Clap with exit 2 (`expected tab ID, got session`). Raw HTTP: an `attachment:` parent returned 422, an unknown session parent 404, and a `session:` path ID on `/tabs/{id}` 400. |
| 2.3 removal and names | After a rename and a duplicate `D` under `api`, both `D` tabs existed. Removing `A1` removed `A1x` too (`tab get A1x` → not found) and left `A2`. `session rm api` made `A` and `D` not found, and `session list` showed only `work`. |
| 2.3 OpenAPI | Disposable consumer of `ship_server::openapi()`, outside the repo: `create_tab`, `get_tab`, `rename_tab`, `remove_tab` and `move_tab` appear at `/api/v0/tabs`, `/api/v0/tabs/{id}` and `/api/v0/tabs/{id}/move` with their statuses. Request bodies are `Create_Tab`, `Named_String` and `MoveTab`, and responses are `Tab`. Every `$ref` resolves. |
| 2.3 build gates | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace` and `cargo build --workspace --release` passed. The workflow was rerun on the final build with identical results, and the release binary served `GET /api/v0/sessions`. |

The disposable consumer and test servers were removed or stopped afterwards. Linux was not exercised.

### Slice 3: logical panes (2026-10-05)

Slice 3 works end to end and every slice 3 check passed. No locked paper was reopened. The two differences below are at the snippet level.

#### Differences from the snippets

- **`tree::pane_owner` added.** Rename and remove need mutable access to the pane's owning tab, so `tree.rs` gains `pane_owner(sessions, id) -> Result<IdOf<Tab>>` next to `pane`. Those two handlers find the owner and edit `tab_mut(owner).panes`. That copies only the owning session through `Arc::make_mut`. Create uses `tab_mut(parent)` directly.
- **`Tab::panes` schema is hand-written,** like `tabs`. `panes_schema` is an object whose `propertyNames` is the pane ID pattern and whose values `$ref` `Pane`. That way the map key is described as an ID, not as an inlined `Pane`.

#### Evidence

macOS 26.6 arm64. Foreground `ship server --port 43903`; clients used `--server-url http://127.0.0.1:43903`.

| Check | Observation |
| --- | --- |
| 3.1 CLI | `ship pane create <tab> left` printed `{"id":"pane:c4ef...","name":"left"}`, and the pane appeared under the tab's `panes` in both `tab get` and `session get`. |
| 3.2 SC-001 build | Sessions `work` and `api`. `work` got `A{A1{A1x},A2},B,C`, using session names as root parents. Panes `left` and `right` went under `A1`, `deep` under `A1x` and `top` under `A`. IDs were extracted with `jq -r .id`. `A1` listed `left,right` in creation order. |
| 3.2 renames and reorder | Renaming tab `A2` and pane `left` printed the renamed entities with unchanged IDs. `move C work --before A` gave `C,A,B`. |
| 3.2 cross-session move | `move A api` appended A to `api` and removed it from `work`. The sorted `jq` list of every ID in A's subtree (8 IDs: 4 tabs, 4 panes) was identical before and after. |
| 3.2 removal | `pane rm right` left `A1` with `left-renamed` and child tab `A1x`. `get` on the removed pane reported not found, exit 1. `tab rm A1` made `A1`'s pane, `A1x` and `A1x`'s pane `deep` not found. `A` kept `A2-renamed` and pane `top`. `session rm api` made `top` not found. |
| 3.2 invalid parents | `pane create session:... x` and `pane create pane:... x` failed in Clap with exit 2 (`expected tab ID, got session` / `got pane`). A raw `curl` POST to `/api/v0/panes` with a session parent returned 422 (`parent: expected tab ID, got session`). `cmp` showed both sessions byte-identical before and after. |
| 3.2 no processes | `pgrep -P <server pid>` found no child processes after the workflow. |
| 3.2 OpenAPI | Disposable consumer of `ship_server::openapi()`, outside the repo: OpenAPI 3.1.0 lists `create_pane` (`POST /api/v0/panes`, `Create_Pane` to `Pane`, 201/404/422/503), `get_pane`, `rename_pane` (`Named_String`) and `remove_pane` at `/api/v0/panes/{id}`. `Create_Pane.parent` is the tab ID pattern only. `Tab.panes` is keyed by the pane ID pattern with `Pane` values. Every `$ref` resolves. |
| 3.2 build gates | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace` and `cargo build --workspace --release` passed. The release binary created and served a pane under a tab. |

The disposable consumer and test servers were removed or stopped afterwards. Linux was not exercised.

### Slice 4: live observation (2026-10-05)

Slice 4 works end to end and every slice 4 check passed. No locked paper was reopened. The differences below are at the snippet level, plus one note on what the stalled-observer check did and did not exercise.

#### Differences from the snippets

- **Reqwest `stream` lands in slice 4, not slice 7.** `Client::attach` reads the body through `bytes_stream()`, which needs the feature. The zstd and gzip features stay in slice 7.
- **Utoipa `rc_schema` feature.** `SseEvent::State(Arc<Replica>)` and `Attached::replica` need a schema for `Arc<Replica>`. Utoipa cannot put `value_type` on a tuple-variant field, so the workspace enables `rc_schema` instead.
- **Hand-written replica map schemas.** `Replica::sessions` and `Replica::viewers` use `schema_with`, like `Session::tabs`, so their keys are described by ID pattern. `Attached::attachment` also uses `schema_with`, because the `Attachment` marker has no schema for the derive to pass. `ViewingRecord` is reached only through the hand-written `viewers` `$ref`, so `ApiDoc` registers it with `components(schemas(...))`. Without that the document had a dangling `$ref`.
- **`Detach` replies `Result<()>`,** matching slice 1's every-message-replies-`Result` change. It is idempotent: an attachment already gone, for example with its removed session, commits nothing. `Attach` rejects a duplicate attachment ID with `Conflict`.
- **`tree::path` is not widened to `NodeId` yet.** The slice 2 entry said slice 4 would widen it, but nothing in slice 4 looks up a pane path: attach always starts at the session root, and selection repair and attach retention arrive in slices 5 and 6. It widens with the first caller.
- **Seeding the watch channel.** `ServerState::replica()` is `pub`; `serve` builds the state actor's value first and uses its empty revision 0 replica as the watch channel's initial value. `commit` logs `publishing replica` at debug level with the new revision.
- **`relay.rs` lives in `ship-core`, gated by its `kameo` feature (Cyan, 2026-10-05).** It moved there so a client can reuse it later. This goes further than slice 1's optional `kameo` dependency against the Rationale's "no actors, channels or HTTP" in core: core now holds an actor and channel sinks. The `kameo` feature also enables an optional `tokio` dependency with only `sync`. `ship-server` is still the only crate that enables the feature, and `cargo tree -p ship-client` shows no Kameo. As public library items, the unused `mpsc`, `Recipient`, closure, filter and filter-map adapters need no dead-code allowance. Kameo 0.22 added `SendError::ActorRestarting`; the `Recipient` sink treats it as `Full`.
- **Client request helper split.** `request` now composes `build` (URL scrubbing, body, attachment header), `open` (send, status check and `AppError` decoding) and response decoding under one two-second timeout. `stream` applies the timeout to `open` alone and returns the live response for `attach`.
- **SSE frames carry only `data:`.** No `event:` name is set; the JSON `type` tag (`attached`, `state`) distinguishes them.
- **Slice 4 observer exits.** A clean end of stream prints `session removed` and exits 0, a body error exits 1 with the error, and SIGINT exits 0. Until slice 6 adds reconnect, a server shutdown also ends the stream cleanly and would print `session removed`. The observer reprints whenever the replica changes, including when only viewing records change, such as another observer attaching.

#### Evidence

macOS 26.6 arm64. Foreground `ship server --port 43904` for tasks 4.1 to 4.3 and `--port 43905` for 4.4; clients used `--server-url` with the matching port.

| Check | Observation |
| --- | --- |
| 4.1 publish | With `RUST_LOG=ship_server=debug`, session create, tab create and two renames logged `publishing replica` with revisions 1, 2, 3 and 4, one per commit. A duplicate create (409) and the `GET` lookups logged nothing. |
| 4.2 curl | `curl -N -X POST /api/v0/attach` with `{"session": "session:..."}` first received `{"type":"attached","attachment":"attachment:d856...","replica":{...,"revision":5,...}}`, whose `viewers` held that attachment at the session root. A later `ship tab create work logs` produced `{"type":"state",...,"revision":6}` containing the new tab. An unknown session ID returned 404 and a `tab:` ID 422. |
| 4.3 CLI | With no `work` session, `ship attach work` created it and printed `attached attachment:8620... to session:e920...` on stderr. It then reprinted the tree after a tab create, a pane create and a child tab create, ending with `editor > left` and `editor > child`. |
| 4.4 two observers | Two observers of `work` each printed every later edit (tab create, pane create, tab rename). Their final renders matched byte for byte. |
| 4.4 attach during edits | An observer started 0.15 s into a loop creating 40 tabs showed 22 tabs in its first render. Its last render showed all 40 at revision 48 with no further edit. |
| 4.4 stalled observer | `kill -STOP` (state `T`), then 50 renames of one tab. Render count stayed 51 while stopped. After `kill -CONT` it printed through revision 149 with the tab named `r100`, with no further edit. All 50 intermediate replicas still fit in socket buffers, so watch-channel coalescing was not exercised; the final-state requirement was. |
| 4.4 closing an observer | A `curl` stream on `work` listed observer A's attachment in its seed. After SIGINT to A, A exited 0, and the curl stream's next event (revision 151, no other edit) no longer listed it. |
| 4.4 session removal | With observers B and E on `work`, `ship session rm work` made both print `session removed` and exit 0. The curl stream on `work` also ended. |
| 4.4 attach-or-create | `ship attach scratch` created `scratch` and attached. In five rounds of two concurrent `ship attach scratch` runs, each round left exactly one `scratch` in `session list`, and both observers named the same session ID. The server logged five 409 responses on `POST /api/v0/sessions`, so every round went through the re-resolve path. `ship attach session:000...` failed with `not found`, exit 1, and `session list` was unchanged. |
| 4.4 OpenAPI | Disposable consumer of `ship_server::openapi()`, outside the repo: OpenAPI 3.1.0 lists `attach` at `POST /api/v0/attach` with an `AttachRequest` body, a 200 `text/event-stream` response whose schema is `SseEvent`, and 404, 422 and 503. `SseEvent` is a `oneOf` of `Attached` and `Replica`, each with its `type` tag. `Replica.sessions` and `Replica.viewers` are keyed by the session and attachment ID patterns. Every `$ref` resolves. |
| 4.4 build gates | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace` and `cargo build --workspace --release` passed. The release binary attached by name, showed a tab create, and exited 0 with `session removed` when its session was removed. |

The disposable consumer and test servers were removed or stopped afterwards. Linux was not exercised.

### Slice 5: selection and session switching (2026-10-05)

Slice 5 works end to end and every slice 5 check passed. No locked paper was reopened. The differences below are at the snippet level.

#### Differences from the snippets

- **`tree::path` now takes and returns `NodeId`,** as the snippet always said. `repair` is its first pane caller. `tree::not_found` also takes a `NodeId` and covers panes. `model.rs` gains `From<IdOf<TabParent>> for NodeId`; `take_tab` maps the parent entry back to a `TabParent` inline.
- **Selecting an entity that does not exist is 404, not 422.** 422 is kept for an existing entity outside the attached session. The attachment is checked first, so an ended attachment is 404 whatever the body. A missing or malformed header is 400 with an `AppError` body.
- **`controls.rs` has two more functions.** `send` runs a parsed control against the observer's own attachment, resolving a `switch` name with `resolve_session`, which never creates a session. `lines` reads stdin on a plain `std::thread` and forwards lines over a Tokio channel. Tokio's own stdin reads on the blocking pool, and dropping the runtime waits for that pool, so a read blocked on an open terminal could delay exit after `session removed` or SIGINT. Deleting `controls.rs` still leaves only the stdin branch in `observe.rs` to remove. The `ship` crate enables Tokio's `sync` feature for the channel.
- **Observer control behavior.** A failed control prints `ship: <error>` on stderr and the observer keeps running. Blank lines are ignored. At end of stdin the observer stops reading controls and keeps observing. A switch prints no extra status line; the reprinted tree names the new session.

#### Evidence

macOS 26.6 arm64. Foreground `ship server --port 43906`; clients used `--server-url http://127.0.0.1:43906`. Observers read controls from FIFOs held open by the test shell.

| Check | Observation |
| --- | --- |
| 5.1 curl | With a `curl -N` attach stream on `work`: `PUT /api/v0/attach/selection` without the header returned 400 `missing x-ship-attachment-id header`, and with a `tab:` header 400 `expected attachment ID, got tab`. Selecting a `work` tab returned 200 with the record. Selecting an `api` tab returned 422 `... is not in attached session ...`, and an unknown tab returned 404. `PUT /api/v0/attach/session` to `api` returned 200 at `api`'s root. The stream carried revisions 6 and 7 with each record change. After the stream was killed, both routes returned 404 `attachment ... not found`. |
| 5.2 select | Typing `select <tab>` into observer 1 moved only its `*` to the tab; observer 2 stayed at the session root. `bogus line` and `select pane:000...` each printed a `ship: ...` line on stderr and the observer kept running. |
| 5.3 independent markers | Observer 1 selected `A` and observer 2 selected `B`, then observer 1 selected the empty tab `E` and observer 2 selected pane `p1`. Each render marked only its own selection. |
| 5.3 pane removal | `pane rm p1` moved observer 2's marker to `p1`'s tab `A1`. |
| 5.3 ancestor removal | Observer 1 selected `A1y` under `A1`; `tab rm A1` moved both observer 1 (from `A1y`) and observer 2 (from `A1`) to `A`. |
| 5.3 moves | Both observers selected pane `pc` in new tab `C`. `tab move C B` kept both markers on `pc` under `B`. `tab move C api` moved both markers to `B`, `C`'s former parent, and both observers stayed on `work`. |
| 5.3 switch | `switch api`, typed as a name into observer 1, reprinted `api`'s tree, including the moved `C` and `pc`, with `api` itself marked. Observer 2 stayed on `work` at `B`. Both exited 0 on SIGINT. |
| 5.3 removed session | `curl` `POST /api/v0/attach` with a removed session's ID returned 404 `session ... not found`. |
| 5.3 OpenAPI | Disposable consumer of `ship_server::openapi()`, outside the repo: OpenAPI 3.1.0 lists `select` at `PUT /api/v0/attach/selection` and `switch_session` at `PUT /api/v0/attach/session`. Each has a required `x-ship-attachment-id` header parameter, a `SelectRequest` or `SwitchSessionRequest` body, a 200 `ViewingRecord` response and `AppError` 400, 404, 422 (select) and 503 responses. `NodeId` is a `oneOf` of the session, tab and pane ID schemas. The document has 11 paths and every `$ref` resolves. |
| 5.3 build gates | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace` and `cargo build --workspace --release` passed. The release binary attached, moved its marker with `select`, and the observer and server each exited 0 on SIGINT and SIGTERM. |

An early verification run hung only because the script backgrounded a shell function, so SIGINT went to a subshell that ignores it, not to the observer. Running the binary directly showed SIGINT exits 0 with stdin held open. The disposable consumer and test servers were removed or stopped afterwards. Linux was not exercised.

### Slice 6: reconnect (2026-10-06)

Slice 6 works end to end and every slice 6 check passed. No locked paper was reopened. The relay in front of the observer was a Python stand-in for `socat`, which is not installed here. The other differences below are at the snippet level.

#### Differences from the snippets

- **Python relay instead of `socat`.** A disposable single-process TCP relay in the session scratchpad played `socat TCP-LISTEN:<q>,fork,reuseaddr TCP:127.0.0.1:<p>`. Killing it closes every relayed connection at once. With `socat`'s `fork`, killing only the parent can leave a forked child holding the observer's connection open.
- **`remembered` returns `Option<AttachRequest>`.** It is `None` before the first `Attached` event. `run` then keeps the request it last sent, so a reattach always targets a session ID and never a name.
- **Every stream end reports a disconnection, including session removal.** The observer cannot tell a server-side end of stream (its session was removed) from any other end, so, as the `run` snippet says, it reports and reattaches. Removing a connected observer's session now prints `disconnected: attach stream ended; retrying in 250ms`, then `session removed` from the reattach's 404, and exits 0. Slice 4's exit behavior still holds; only the extra line is new.
- **Backoff timing.** The first reattach waits 250 ms after the disconnection, and each failure doubles the wait up to 5 s. A successful reattach resets it. Durations print in Rust's `Debug` form (`250ms`, `1s`). An initial `ship attach` that fails still exits 1. Only a running observer reconnects.
- **Disconnect reason text.** When the connection is cut mid-stream, Reqwest reports the body error as `error decoding response body`, which `http_error` classifies as `Serialization`. A refused reattach reads `error sending request`. The wording is Reqwest's and is not rewritten.
- **Controls during an outage.** The observer forgets its attachment ID on disconnect, so `select` or `switch` typed during an outage prints `ship: not attached` and the observer keeps retrying. SIGINT during an outage exits 0.
- **Status line session.** `attached attachment:... to session:...` now names the session in the request just sent, which after a `switch` and a reattach is the switched-to session rather than the command-line one.

#### Evidence

macOS 26.6 arm64. Foreground `ship server --port 43907`; the relay listened on 43917. Observer O1 ran `--server-url http://127.0.0.1:43917` with stderr timestamped. A `curl -N` attach stream on `work` went direct to the server, and edits used `--server-url http://127.0.0.1:43907`.

| Check | Observation |
| --- | --- |
| 6.1 retention | `Attach` keeps a requested selection only when `inside` the requested session now, else starts at the session root. |
| 6.2 backoff | Killing the relay with O1 selecting pane `p1` printed `disconnected: error decoding response body; retrying in 250ms`. Reattach failures followed at 250 ms, 500 ms, 1 s, 2 s, 4 s and 5 s spacing by timestamp, each line naming the next wait (`500ms`, `1s`, `2s`, `4s`, `5s`, `5s`). |
| 6.2 current state | During that outage tab `C` was created directly. After the relay restarted, O1 printed `attached attachment:b787...` and a render at revision 12 that included `C`, with no further edit. |
| 6.2 selection kept | That render still marked `p1` with `*`. |
| 6.2 selection removed | Relay down, `pane rm p1`, relay up: O1 reattached and marked `work` itself, not `p1`'s tab `A`. |
| 6.2 selection moved away | O1 selected `p2`; relay down, `tab move A api`, relay up: O1 reattached to `work` with the session marked, and `A` gone from its tree. |
| 6.2 session removed | Relay down, `session rm work`, relay up: O1 printed `session removed` and exited 0. `session list` showed only `api`, so nothing recreated `work`. The curl stream also ended. |
| 6.2 old record | The curl stream listed O1's first attachment `e072bf44` through revision 9. Revision 10, published when the relay died and before any edit, no longer listed it. Each later outage removed that cycle's attachment the same way, and each reattach added a new one. |
| 6.2 release binary | The release observer printed `disconnected: attach stream ended; retrying in 250ms` and `session removed`, then exited 0, when its session was removed while connected. After the server was killed, it kept retrying and exited 0 on SIGINT. |
| 6.2 build gates | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace` and `cargo build --workspace --release` passed. |

The relay script and test servers were removed or stopped afterwards. Linux was not exercised.

### Slice 7, A5 part: stream end reasons (2026-10-06, in progress)

Architecture amendment A5 was approved on 2026-10-06, and its code is in place. Task 7.3 stays open: its shutdown check needs task 7.4's shutdown ordering. The rest of slice 7 (compression, actor-stop monitoring, shutdown ordering, Linux) has not started.

#### Differences from the snippets

- **`Ended { reason }`, not `Ended(EndReason)`.** Serde's internally tagged enums write a newtype variant around a unit enum as `{"type":"ended","sessionRemoved":null}`. A struct variant writes `{"type":"ended","reason":"sessionRemoved"}`, and the OpenAPI schema shows `reason` as a `$ref` to the `EndReason` string enum.
- **The stream is a `stream::unfold` over the watch receiver; `tokio-stream` is gone.** `WatchStream` plus `take_while` cannot end right after a final event. It would wait for one more replica before noticing. The unfold loop calls `changed()` and returns its own state as `None` after `Ended`, so the stream closes immediately. Nothing else used `tokio-stream`, so it is removed from `ship-server` and the workspace manifest, though the proposal lists it among the new dependencies. `changed()` still delivers an unseen value before reporting a closed channel, so a final replica is not skipped.
- **Observer handling.** `Ended(SessionRemoved)` prints `session removed` and exits 0. `Ended(ServerShutdown)` takes the disconnection path with the reason `server shutting down`. `Observer::apply` ignores `Ended`.

#### Evidence

macOS 26.6 arm64. Foreground `ship server --port 43909`; the Python relay from slice 6 listened on 43919.

| Check | Observation |
| --- | --- |
| Removal while connected | With a `curl` stream and an observer on `gone`, `session rm gone` made curl's last frame `{"type":"ended","reason":"sessionRemoved"}`, after which curl exited. The observer printed only `attached ...` and `session removed`, with no `disconnected` line, and exited 0. The server log had the same two `/api/v0/attach` requests before and after, so there was no reattach. |
| Relay cut, removal during outage | Killing the relay gave `disconnected: error decoding response body; retrying in 250ms`. After `session rm cut` and a relay restart, the reattach got 404 and the observer printed `session removed` and exited 0, as in slice 6. |
| SIGTERM, not yet passing | With a curl stream and an observer attached, `kill -TERM` gave no `ended` frame. Curl's last frame was `attached`, the observer reported a body error and retried, and the server exited 1 after 5.02 s with `server shutdown exceeded five seconds`. Shutdown does not stop the actors, so the replica watch sender is never dropped and streams never end. That is task 7.4's ordering change, and this check reruns with it. |
| OpenAPI | Disposable consumer of `ship_server::openapi()`, outside the repo: `SseEvent` is a `oneOf` of `attached`, `state` and an `ended` object requiring `type` and `reason`. `EndReason` is a string enum of `sessionRemoved` and `serverShutdown`. The attach 200 description names the `ended` event, and every `$ref` resolves. |
| Build gates | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace` and `cargo build --workspace --release` passed. |

The relay, disposable consumer and test servers were removed or stopped afterwards. Linux was not exercised.
