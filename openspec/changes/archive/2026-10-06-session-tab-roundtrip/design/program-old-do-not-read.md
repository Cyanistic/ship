# Session/tab/pane roundtrip program

Status: Draft for program review. [Product](product.md) and [architecture](architecture.md) are locked. This paper proposes implementation interfaces and checkpoints; approval does not start implementation.

## Rationale

Keep the existing four crates. `ship-core` describes identities, entities and transport data. `ship-server` owns all metadata and selections in one actor, with a typed relay and notifier delivering complete replicated state. `ship-client` handles HTTP, SSE and reconnect. `ship` provides entity commands and text observation.

This is a **document-only virtual skeleton**. Every path and Rust declaration below is proposed, not a created source file. Snippets omit bodies, routine derives and some imports; they are not independently compilable and have not been compilation-verified. No branches, manifests, builds or permanent tests are created during this stage.

Cyan resolved two program choices in chat:

- An observer receives its attached session structure and viewing records for attachments in that same session. Its own record is always present; an unattached observer receives its own unattached record without another session's structure.
- The running observer accepts temporary stdin line commands, `select <id>` and `session <id>`. This checkpoint adapter must be deleted when the real client UI replaces it. It is not a permanent command language, TUI, keybinding system or configuration interface.

The state actor owns validation, atomic candidate commits and selection repair. Routes, CLI and observer do not duplicate those rules. Entity creation returns the entity itself. The observer matches its attachment ID to the server-owned viewing record and derives its view locally.

## Skeleton map

### Planned added files

Each file names its first implementation slice. Later slices extend the same interfaces rather than introducing parallel ownership.

#### `crates/ship-core/src/identity.rs` — S1

One typed UUID wrapper, one shared identity alias and a reusable either. External IDs use strict `kind:uuid` strings. Entity prefixes are `session`, `tab`, `pane`; active attachment addressing uses `attachment`. Prefix spellings are proposed external contracts for this review, not implementation details to change casually.

```rust
use std::{fmt, marker::PhantomData, str::FromStr};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::Result;

pub trait Kind { fn prefix() -> &'static str; }
pub trait Identified {
    type Id: Clone + Eq + std::hash::Hash;
    fn id(&self) -> Self::Id;
}
pub type IdOf<T> = <T as Identified>::Id;
pub struct Id<T> { value: Uuid, kind: PhantomData<fn() -> T> }
impl<T: Kind> Id<T> { pub fn new() -> Self; }
impl<T: Kind> fmt::Display for Id<T> { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result; }
impl<T: Kind> FromStr for Id<T> { type Err = crate::AppError; fn from_str(s: &str) -> Result<Self>; }
impl<T: Kind> Serialize for Id<T>;
impl<'de, T: Kind> Deserialize<'de> for Id<T>;

#[serde(untagged)]
pub enum UntaggedEither<L, R> { Left(L), Right(R) }
impl<L: Identified, R: Identified> Identified for UntaggedEither<L, R> {
    type Id = UntaggedEither<IdOf<L>, IdOf<R>>;
    fn id(&self) -> Self::Id;
}
```

Implement `Id<T>` copy/equality/hash without requiring the entity itself to implement those traits. The either's wire representation is the active typed ID string, not an external `Left`/`Right` tag. No separate per-entity ID aliases or stringly typed destinations.

#### `crates/ship-core/src/entity.rs` — S1, extended S2/S3

Serializable metadata only. Child tabs and panes are separate collections. Pane storage order does not introduce layout or pane-reordering behavior.

```rust
use indexmap::IndexMap;
use crate::identity::{Id, IdOf, Identified, Kind, UntaggedEither};

pub struct ServerRoot;
pub struct Session { pub id: Id<Self>, pub name: String, pub tabs: IndexMap<IdOf<Tab>, Tab> }
pub struct Tab {
    pub id: Id<Self>, pub name: String,
    pub tabs: IndexMap<IdOf<Tab>, Tab>,
    pub panes: IndexMap<IdOf<Pane>, Pane>,
}
pub struct Pane { pub id: Id<Self>, pub name: String }

impl Identified for ServerRoot { type Id = (); fn id(&self); }
impl Identified for Session { type Id = Id<Self>; fn id(&self) -> IdOf<Self>; }
impl Identified for Tab { type Id = Id<Self>; fn id(&self) -> IdOf<Self>; }
impl Identified for Pane { type Id = Id<Self>; fn id(&self) -> IdOf<Self>; }
impl Kind for Session { fn prefix() -> &'static str; }
impl Kind for Tab { fn prefix() -> &'static str; }
impl Kind for Pane { fn prefix() -> &'static str; }

pub trait Creatable: Identified { type Parent: Identified; type Input; }
pub trait Renameable: Identified {}
pub trait Removable: Identified {}
pub trait Movable: Creatable {}
pub struct Name { pub name: String }
impl Creatable for Session { type Parent = ServerRoot; type Input = Name; }
impl Creatable for Tab { type Parent = UntaggedEither<Session, Tab>; type Input = Name; }
impl Creatable for Pane { type Parent = Tab; type Input = Name; }
impl Renameable for Session;
impl Renameable for Tab;
impl Renameable for Pane;
impl Removable for Session;
impl Removable for Tab;
impl Removable for Pane;
impl Movable for Tab;
```

All three entities serialize and deserialize the same representation used by operation results and snapshots. No `Movable` implementation for panes or sessions. `ServerRoot` is a creation destination concept, not a serialized server entity or new UUID.

#### `crates/ship-core/src/action.rs` — S1, extended S2/S3

Reusable payloads and supported action categories. HTTP remains operation-specific. Named variants carry named payloads; there is no universal external action or action-result endpoint.

```rust
use crate::entity::*;
use crate::identity::IdOf;

pub struct Create<E: Creatable> { pub parent: IdOf<E::Parent>, pub value: E::Input }
pub struct Inspect<E: Identified> { pub id: IdOf<E> }
pub struct Rename<E: Renameable> { pub id: IdOf<E>, pub value: Name }
pub struct Remove<E: Removable> { pub id: IdOf<E> }
pub struct Move<E: Movable> { pub id: IdOf<E>, pub parent: IdOf<E::Parent>, pub placement: Placement }
pub struct TabMoveBody { pub parent: IdOf<<Tab as Creatable>::Parent>, pub placement: Placement }
pub struct Sibling { pub id: IdOf<Tab> }
pub enum Placement { Append, Before(Sibling), After(Sibling) }
pub struct ListSessions;

pub enum SessionAction {
    Create(Create<Session>), Inspect(Inspect<Session>), Rename(Rename<Session>),
    Remove(Remove<Session>), List(ListSessions),
}
pub enum TabAction {
    Create(Create<Tab>), Inspect(Inspect<Tab>), Rename(Rename<Tab>),
    Remove(Remove<Tab>), Move(Move<Tab>),
}
pub enum PaneAction {
    Create(Create<Pane>), Inspect(Inspect<Pane>), Rename(Rename<Pane>), Remove(Remove<Pane>),
}
```

Create bodies use `parent` and `value`; root session creation uses `parent: null`. Rename bodies carry `Name`; the path supplies the target ID. Move bodies carry parent and placement; the path supplies the moving tab ID. Absent placement means append. Program review approves these concrete request shapes; typed actor payloads include the path target, while HTTP bodies do not redundantly supply it. Parent and target kinds are checked before mutation.

#### `crates/ship-core/src/replica.rs` — S4

Wire data has no actor refs, channels, futures or future PTY handles. Named SSE variants use a tagged Serde envelope with `type` and `data`, lowercase variant names; each SSE frame's JSON data is one complete `SseEvent`.

```rust
use indexmap::IndexMap;
use uuid::Uuid;
use crate::{entity::*, identity::*};

pub enum Selection { Session(IdOf<Session>), Tab(IdOf<Tab>), Pane(IdOf<Pane>) }
pub struct SelectedInSession { pub session: IdOf<Session>, pub selection: Selection }
pub enum Viewing { Attached(SelectedInSession), Unattached }
pub struct Attachment { pub id: Id<Self>, pub viewing: Viewing }
impl Kind for Attachment { fn prefix() -> &'static str; }
impl Identified for Attachment { type Id = Id<Self>; fn id(&self) -> IdOf<Self>; }

pub struct Revision { pub incarnation: Uuid, pub commit: u64 }
pub struct Replica {
    pub revision: Revision,
    pub session: Option<Session>,
    pub attachments: IndexMap<IdOf<Attachment>, Attachment>,
}
pub struct Attach { pub session: Option<IdOf<Session>>, pub selection: Option<Selection> }
pub struct Select { pub selection: Selection }
pub struct SwitchSession { pub session: IdOf<Session> }
pub struct Attached { pub id: IdOf<Attachment>, pub state: Replica }
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum SseEvent { Attached(Attached), State(Replica) }
```

`Selection` serializes as an untagged typed ID string, using the same strict prefix parsing as destinations. A missing or null session is an unattached opening used after a running observer has become unattached; a new `ship observe` invocation requires a session argument. A nonexistent requested session also produces unattached state, as approved in architecture. Each attached replica includes exactly its session and that session's attachment records. An unattached replica includes no session and only its own record. Explicit viewing-record changes and attachment entry/exit update the same revision ordering domain as structural commits.

#### `crates/ship-server/src/state.rs` — S1, extended S2/S3/S4

One Kameo actor owns the authoritative collections. Individual actor messages have operation-specific replies, even if action enums help internal dispatch. The signatures below show the handlers' domain interface, not an invented second state service. Kameo `Message` implementations wrap these operations inside the actor.

```rust
use indexmap::IndexMap;
use kameo::actor::ActorRef;
use ship_core::{action::*, entity::*, identity::*, replica::*, Result};
use crate::{relay::RelayBus, notifier::Notifier, attachment::RegisteredStream};

pub(crate) struct ServerState {
    sessions: IndexMap<IdOf<Session>, Session>,
    attachments: IndexMap<IdOf<Attachment>, Attachment>,
    revision: Revision,
    relay: ActorRef<RelayBus>,
    notifier: ActorRef<Notifier>,
}
pub(crate) struct Detach { pub id: IdOf<Attachment> }
pub(crate) struct PublishedReplica { pub recipient: IdOf<Attachment>, pub state: Replica }

impl ServerState {
    async fn create_session(&mut self, input: Create<Session>) -> Result<Session>;
    fn list_sessions(&self) -> Vec<Session>;
    fn inspect_session(&self, input: Inspect<Session>) -> Result<Session>;
    async fn rename_session(&mut self, input: Rename<Session>) -> Result<Session>;
    async fn remove_session(&mut self, input: Remove<Session>) -> Result<()>;
    async fn create_tab(&mut self, input: Create<Tab>) -> Result<Tab>;
    fn inspect_tab(&self, input: Inspect<Tab>) -> Result<Tab>;
    async fn rename_tab(&mut self, input: Rename<Tab>) -> Result<Tab>;
    async fn remove_tab(&mut self, input: Remove<Tab>) -> Result<()>;
    async fn move_tab(&mut self, input: Move<Tab>) -> Result<Tab>;
    async fn create_pane(&mut self, input: Create<Pane>) -> Result<Pane>;
    fn inspect_pane(&self, input: Inspect<Pane>) -> Result<Pane>;
    async fn rename_pane(&mut self, input: Rename<Pane>) -> Result<Pane>;
    async fn remove_pane(&mut self, input: Remove<Pane>) -> Result<()>;
    async fn attach(&mut self, input: Attach) -> Result<RegisteredStream>;
    async fn select(&mut self, id: IdOf<Attachment>, input: Select) -> Result<Attachment>;
    async fn switch_session(&mut self, id: IdOf<Attachment>, input: SwitchSession) -> Result<Attachment>;
    async fn detach(&mut self, input: Detach) -> Result<()>;
}
```

Candidate edits clone only affected metadata. Validation and selection repairs precede authoritative replacement. Traversal discovers ancestry and owners; no second authoritative global index. Rejected operations leave structure, attachment records and revision unchanged. Same-session moves retain selected pane/tab IDs. Cross-session moves repair source selections using the old source ancestry, not the moved destination ancestry. Every publication contains a complete coherent structure/record pair.

The lifecycle composition is added in S4. Earlier CRUD slices instantiate the same state actor with publication disabled because no stream route or attachment can yet exist; they do not create a second ownership implementation. Once observation is exposed, every authoritative publication uses the reliable relay path and failures trigger shutdown.

#### `crates/ship-server/src/relay.rs` — S4

Implement only the local [typed relay contract](../../../../../docs/research/typed-relay.md) exercised by this slice. Typed publication is type-erased internally for heterogeneous subscriptions. Mapping is a subscription adapter, not another actor.

```rust
use std::{any::TypeId, future::Future, pin::Pin, sync::Arc};
use kameo::actor::Recipient;
use ship_core::Result;

pub(crate) type Delivery<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
pub(crate) trait Sink<M>: Send + Sync { fn deliver(&self, message: Arc<M>) -> Delivery<'_>; }
pub(crate) struct ActorSink<M> { recipient: Recipient<M> }
pub(crate) struct MapSink<I, O, F, S> { inner: S, map: F, types: std::marker::PhantomData<fn(I) -> O> }
pub(crate) struct FilterMapSink<I, O, F, S> { inner: S, map: F, types: std::marker::PhantomData<fn(I) -> O> }
pub(crate) struct Subscribe<M> { pub sink: Box<dyn Sink<M>> }
pub(crate) struct Publish<M> { pub value: Arc<M> }
pub(crate) struct RelayBus { subscriptions: std::collections::HashMap<TypeId, Box<dyn std::any::Any + Send>> }
```

Await bounded actor mailbox capacity; do not use best-effort `try_send` or silently drop `Full`. A mapping returning `None` is deliberate filtering, not overload handling. Register the `PublishedReplica` to notifier-message mapping before the server accepts attachments. Awaited tell establishes enqueue, not completed downstream processing; actor termination and undelivered publications are therefore covered by coordinated failure shutdown, not claimed as durable delivery.

#### `crates/ship-server/src/notifier.rs` — S4

Notifier registrations own latest-value senders, never sockets or authoritative selections.

```rust
use std::{collections::HashMap, sync::Arc};
use tokio::sync::watch;
use ship_core::{identity::IdOf, replica::*, Result};

pub(crate) struct Register { pub id: IdOf<Attachment>, pub seed: Arc<Replica> }
pub(crate) struct Unregister { pub id: IdOf<Attachment> }
pub(crate) struct ReplaceReplica { pub id: IdOf<Attachment>, pub state: Arc<Replica> }
pub(crate) struct Slot { revision: Revision, latest: watch::Sender<Arc<Replica>> }
pub(crate) struct Notifier { slots: HashMap<IdOf<Attachment>, Slot> }
impl Notifier {
    fn register(&mut self, input: Register) -> Result<watch::Receiver<Arc<Replica>>>;
    fn replace(&mut self, input: ReplaceReplica) -> Result<()>;
    fn unregister(&mut self, input: Unregister);
}
```

A register ask acknowledges that the slot is seeded before the state actor accepts another command. Incarnation/revision checks reject older queued updates. Duplicate or invalid registration is a machinery error, not a silent overwrite. Replacement of an already removed slot is harmless lifecycle cleanup, not a reason to resurrect it. The notifier never calls back into state or relay. Watch replacement retains the final state and wakes the reader even without another mutation.

#### `crates/ship-server/src/attachment.rs` — S4, extended S6

Stream framing, header extraction and lifetime cleanup sit outside metadata ownership.

```rust
use std::sync::Arc;
use axum::{extract::FromRequestParts, http::request::Parts, response::sse::{Event, Sse}};
use tokio::sync::watch;
use ship_core::{identity::IdOf, replica::*, Result};
use crate::{runtime::RuntimeHandle, http::HttpError};

pub(crate) struct AttachmentContext(pub IdOf<Attachment>);
impl<S: Send + Sync> FromRequestParts<S> for AttachmentContext {
    type Rejection = HttpError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> std::result::Result<Self, HttpError>;
}
pub(crate) struct AttachmentLease { id: IdOf<Attachment>, runtime: RuntimeHandle }
pub(crate) struct RegisteredStream {
    pub initial: Attached,
    pub latest: watch::Receiver<Arc<Replica>>,
    pub lease: AttachmentLease,
}
impl Drop for AttachmentLease { fn drop(&mut self); }
pub(crate) fn stream(registration: RegisteredStream) -> Sse<impl futures_core::Stream<Item = std::result::Result<Event, HttpError>>>;
```

The extractor only requires/parses `X-Ship-Attachment-Id`; active identity checks belong to state. Missing/malformed header returns 400, unknown/ended attachment returns 404. It does not consume a control request's JSON body.

The state actor creates the lease as soon as an attachment ID is allocated and retains ownership through registration. Failed registration drops it. On success, the lease travels inside the actor reply and then the HTTP stream, so cancellation before the handler receives or returns the stream still drops the lease. Verify that the selected Kameo reply path drops an unreceived reply; if it does not, return to this lifecycle design before exposing attachment. `Drop` initiates idempotent cleanup of the exact ID using the runtime's tracked asynchronous cleanup path; it does not await, use an unbounded publication queue or lose cleanup on a full mailbox.

Keep the `Attached` handshake separate from the coalesced slot. Emit it first, then newer complete `State` frames. EOF, shutdown or body cancellation releases the lease. A removed session does not end the stream; its own record becomes unattached and a later control can switch it.

#### `crates/ship-server/src/http.rs` — S1, extended S2/S3/S4/S6

Routes extract input, ask the actor and serialize its result. Preserve route-derived OpenAPI documentation; schema annotations belong with these handlers and shared wire types, not a separate hand-maintained registry.

```rust
use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json, Router};
use ship_core::{action::*, entity::*, identity::*, replica::*, AppError};
use crate::{runtime::RuntimeHandle, attachment::AttachmentContext};

pub(crate) struct HttpError { status: StatusCode, error: AppError }
impl IntoResponse for HttpError { fn into_response(self) -> axum::response::Response; }
pub(crate) fn routes() -> utoipa_axum::router::OpenApiRouter<RuntimeHandle>;
pub(crate) async fn list_sessions(State(runtime): State<RuntimeHandle>) -> std::result::Result<Json<Vec<Session>>, HttpError>;
pub(crate) async fn create_session(State(runtime): State<RuntimeHandle>, Json(input): Json<Create<Session>>) -> std::result::Result<(StatusCode, Json<Session>), HttpError>;
pub(crate) async fn inspect_session(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Session>>) -> std::result::Result<Json<Session>, HttpError>;
pub(crate) async fn rename_session(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Session>>, Json(input): Json<Name>) -> std::result::Result<Json<Session>, HttpError>;
pub(crate) async fn remove_session(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Session>>) -> std::result::Result<StatusCode, HttpError>;
pub(crate) async fn create_tab(State(runtime): State<RuntimeHandle>, Json(input): Json<Create<Tab>>) -> std::result::Result<(StatusCode, Json<Tab>), HttpError>;
pub(crate) async fn inspect_tab(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Tab>>) -> std::result::Result<Json<Tab>, HttpError>;
pub(crate) async fn rename_tab(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Tab>>, Json(input): Json<Name>) -> std::result::Result<Json<Tab>, HttpError>;
pub(crate) async fn remove_tab(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Tab>>) -> std::result::Result<StatusCode, HttpError>;
pub(crate) async fn move_tab(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Tab>>, Json(input): Json<TabMoveBody>) -> std::result::Result<Json<Tab>, HttpError>;
pub(crate) async fn create_pane(State(runtime): State<RuntimeHandle>, Json(input): Json<Create<Pane>>) -> std::result::Result<(StatusCode, Json<Pane>), HttpError>;
pub(crate) async fn inspect_pane(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Pane>>) -> std::result::Result<Json<Pane>, HttpError>;
pub(crate) async fn rename_pane(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Pane>>, Json(input): Json<Name>) -> std::result::Result<Json<Pane>, HttpError>;
pub(crate) async fn remove_pane(State(runtime): State<RuntimeHandle>, Path(id): Path<IdOf<Pane>>) -> std::result::Result<StatusCode, HttpError>;
pub(crate) async fn attach(State(runtime): State<RuntimeHandle>, Json(input): Json<Attach>) -> std::result::Result<impl IntoResponse, HttpError>;
pub(crate) async fn select(State(runtime): State<RuntimeHandle>, context: AttachmentContext, Json(input): Json<Select>) -> std::result::Result<Json<Attachment>, HttpError>;
pub(crate) async fn switch_session(State(runtime): State<RuntimeHandle>, context: AttachmentContext, Json(input): Json<SwitchSession>) -> std::result::Result<Json<Attachment>, HttpError>;
```

`TabMoveBody` is defined once in the shared action module and used by both client and server; there are no duplicated payload definitions. All deserialization rejections use the approved 400 malformed-input policy rather than Axum's unmodified JSON defaults. Structural relation failures map to 422, machinery failure to 503; existing error categories need not gain new wire variants merely to carry server-owned HTTP status. Successful creates use 201, removals 204, other JSON operations 200. No view-fetch route or generic action endpoint.

#### `crates/ship-server/src/runtime.rs` — S1, extended S4/S6

Server composition owns actors, their failure observation and shutdown. Routes receive a cloneable runtime handle, not mutable structure.

```rust
use kameo::actor::ActorRef;
use ship_core::Result;
use crate::{state::ServerState, relay::RelayBus, notifier::Notifier};

#[derive(Clone)]
pub(crate) struct RuntimeHandle { state: ActorRef<ServerState>, cleanup: CleanupHandle }
pub(crate) struct CleanupHandle;
pub(crate) struct Runtime {
    handle: RuntimeHandle,
    relay: ActorRef<RelayBus>,
    notifier: ActorRef<Notifier>,
}
impl Runtime {
    pub(crate) async fn start() -> Result<Self>;
    pub(crate) fn handle(&self) -> RuntimeHandle;
    pub(crate) async fn failed(&self) -> ship_core::AppError;
    pub(crate) async fn shutdown(self) -> Result<()>;
}
impl RuntimeHandle {
    pub(crate) fn state(&self) -> &ActorRef<ServerState>;
    pub(crate) fn schedule_detach(&self, id: ship_core::identity::IdOf<ship_core::replica::Attachment>);
}
```

S1 starts only state; S4 adds relay/notifier and wires subscriptions before binding the stream route. The declarations show the final composition, not mandatory early allocation of unused machinery. A cleanup handle tracks spawned cleanup work and watches shutdown; it is not a new persistent client resource or expiry manager. Before actors stop, stop accepting requests, end streams, drain initiated cleanup and then stop state, relay and notifier in producer-to-consumer order. Abnormal actor death or undelivered authoritative publication causes coordinated server shutdown. The existing five-second server drain limit bounds the total shutdown, not five seconds per actor. Shutdown never waits for cleanup delivery to actors already stopped.

#### `crates/ship-client/src/stream.rs` — S4, extended S5/S6

One stream decoder, distinct from the existing bounded-duration JSON request helper. Decode arbitrary byte boundaries and SSE framing through a standard SSE parser, not newline splitting of HTTP chunks.

```rust
use ship_core::{replica::*, Result};
use crate::Client;

pub struct AttachmentStream { response: reqwest::Response }
impl Client { pub async fn attach(&self, input: &Attach) -> Result<AttachmentStream>; }
impl AttachmentStream { pub async fn next(&mut self) -> Result<Option<SseEvent>>; }
```

Bound request establishment, not the lifetime of the SSE body. Check HTTP status/content type before decoding; consume the POST response incrementally using Reqwest's automatic negotiated decoding. EOF and body/decoder failure are reported to the observer's single reconnect loop. Heartbeats are transport-only and never advance state revision. `AttachmentStream`'s declaration abbreviates parser storage, not a requirement to repeatedly take the response body for every event.

#### `crates/ship-client/src/observer.rs` — S4, extended S5

Reusable observation/control logic knows nothing about stdin, terminal widgets or eventual pane runtimes.

```rust
use tokio::sync::mpsc;
use ship_core::{identity::IdOf, entity::Session, replica::*, Result};
use crate::Client;

pub enum ObserverControl { Select(Select), Session(SwitchSession) }
pub struct ObserverEvent { pub attachment: IdOf<Attachment>, pub state: Replica }
pub enum ConnectionStatus { Connecting, Connected, Disconnected }
pub struct Observer {
    client: Client,
    session: Option<IdOf<Session>>,
    selection: Option<Selection>,
    active: Option<IdOf<Attachment>>,
    state: Option<Replica>,
}
impl Observer {
    pub fn new(client: Client, session: IdOf<Session>) -> Self;
    pub async fn run(
        self,
        controls: mpsc::Receiver<ObserverControl>,
        events: mpsc::Sender<ObserverEvent>,
        status: mpsc::Sender<ConnectionStatus>,
    ) -> Result<()>;
}
```

On handshake, replace active attachment context and establish the stream's incarnation baseline. On `State`, accept only newer revisions in that incarnation. Derive session/selection from the matching own record; remembered values are recovery hints, not another authority. After a switch, remember the newly accepted session, including when confirmation arrives through the stream rather than the HTTP response. A late control response must not overwrite a newer stream state; use the stream as replication authority and treat successful control responses as acknowledgments.

Disconnect clears active control context immediately. Controls while disconnected fail visibly and are not queued for replay; failed or ambiguous mutations are never retried. Reconnect uses remembered session/selection with bounded exponential backoff capped at five seconds. Old stream events/control completions cannot update a replacement stream's baseline. This is local task/stream ownership, not a server-side persistent generation protocol. Selection whose pane/tab no longer belongs to the requested session falls back according to the locked reconnect rule. Unattached clients can keep observing and explicitly switch sessions.

Observer output channels are bounded. A temporarily blocked output consumer may slow this observer but not the state actor/notifier; resumed consumption still reaches the final replica without another mutation. The observer must not mistake intentional output backpressure for an HTTP idle timeout.

#### `crates/ship/src/commands.rs` — S1, extended S2/S3/S4

CLI dispatch translates parsed arguments to shared requests and prints returned entities as JSON. It does not create IDs, repair selection or implement tree mutations.

```rust
use ship_client::Client;
use ship_core::Result;
use crate::cli::{SessionCommand, TabCommand, PaneCommand};

pub(crate) async fn session(client: &Client, command: SessionCommand) -> Result<()>;
pub(crate) async fn tab(client: &Client, command: TabCommand) -> Result<()>;
pub(crate) async fn pane(client: &Client, command: PaneCommand) -> Result<()>;
pub(crate) async fn observe(client: Client, session: ship_core::identity::IdOf<ship_core::entity::Session>) -> Result<()>;
```

#### `crates/ship/src/temporary_observer.rs` — S4, extended S5; delete when real UI replaces it

**Temporary checkpoint adapter. Delete this entire file and its CLI wiring when the real client UI takes over.** Keep the transport-independent `Observer` and server control routes. Do not extend this adapter into a command language, persistent console, layout manager or configurable UI.

```rust
use ship_client::{Client, observer::ObserverControl};
use ship_core::{entity::Session, identity::IdOf, Result};

pub(crate) async fn run(client: Client, session: IdOf<Session>) -> Result<()>;
fn parse_line(line: &str) -> Result<ObserverControl>;
```

Only `select <session|tab|pane id>` and `session <session id>` are accepted. Stdin EOF disables controls but does not terminate an output-only observer; process signal/shutdown terminates it. Print complete observation envelopes as one JSON record per line to stdout, including attachment ID so `jq` can match its viewing record. Connection transitions, rejected controls and each new attachment ID are diagnostics on stderr. No prompts or decorative text contaminate JSON stdout. Use readiness-based, nonblocking stdin input on the supported Unix platforms rather than Tokio's blocking-backed stdin reader; cancellation must not leave a read thread preventing process exit. Handle terminal, pipe/FIFO and regular-file input explicitly, and restore any terminal/input flags changed by this temporary adapter. Scripted stdin and a FIFO are sufficient for real workflow checks; no permanent test harness is introduced.

### Planned moved files

None. Existing source remains in its current crate. No skeleton branch or source reorganization is needed.

### Planned replaced or extended files

These are future edits, not changes performed by program design.

| Existing path | Proposed change | Slices |
| --- | --- | --- |
| `crates/ship-core/src/lib.rs` | Declare/export identity, entity, action and replica modules; keep health/error exports | S1–S4 |
| `crates/ship-server/src/lib.rs` | Declare server modules; compose stateful operation routes and actors in `serve`; retain health, tracing, loopback validation and OpenAPI generation | S1–S4, S6 |
| `crates/ship-client/src/lib.rs` | Export stream/observer; extend JSON helpers with bodies, error-body decoding and 204 handling; keep bounded JSON timeout separate from SSE | S1–S5 |
| `crates/ship-client/src/api.rs` | Add operation-specific entity methods and header-scoped control methods; keep `Client { http, url }` and `health` | S1–S4 |
| `crates/ship/src/cli.rs` | Add session/tab/pane/observe commands while preserving existing bare-health/server behavior | S1–S4 |
| `crates/ship/src/main.rs` | Wire entity dispatch and temporary observer; enable supported response decoding, retain retry-never for mutations and attach | S1–S4, S6 |
| `Cargo.toml` | Proposed workspace dependencies/features below | S1, S4, S6 |
| `crates/ship-core/Cargo.toml` | Inherit UUID/IndexMap and required serialization/schema features | S1 |
| `crates/ship-server/Cargo.toml` | Inherit Kameo and streaming utilities; enable compressed SSE features | S1, S4, S6 |
| `crates/ship-client/Cargo.toml` | Inherit SSE parser/stream utilities and streaming decoding features | S4, S6 |
| `crates/ship/Cargo.toml` | Inherit any directly used shared dependency/features for CLI typed arguments and async stdin | S1, S4 |

Proposed replacement interfaces, without bodies:

```rust
// crates/ship-core/src/lib.rs
pub mod identity;
pub mod entity;
pub mod action;
pub mod replica;

// crates/ship-server/src/lib.rs
mod state;
mod runtime;
mod http;
mod relay;
mod notifier;
mod attachment;
pub fn openapi() -> utoipa::openapi::OpenApi;
pub(crate) fn router(runtime: runtime::RuntimeHandle) -> axum::Router;
pub async fn serve(address: std::net::SocketAddr, shutdown: impl std::future::Future<Output = ship_core::Result<()>> + Send + 'static) -> ship_core::Result<()>;

// crates/ship-client/src/api.rs
impl Client {
    pub async fn sessions(&self) -> Result<Vec<Session>>;
    pub async fn create_session(&self, input: &Create<Session>) -> Result<Session>;
    pub async fn session(&self, id: IdOf<Session>) -> Result<Session>;
    pub async fn rename_session(&self, input: &Rename<Session>) -> Result<Session>;
    pub async fn remove_session(&self, id: IdOf<Session>) -> Result<()>;
    pub async fn create_tab(&self, input: &Create<Tab>) -> Result<Tab>;
    pub async fn tab(&self, id: IdOf<Tab>) -> Result<Tab>;
    pub async fn rename_tab(&self, input: &Rename<Tab>) -> Result<Tab>;
    pub async fn remove_tab(&self, id: IdOf<Tab>) -> Result<()>;
    pub async fn move_tab(&self, input: &Move<Tab>) -> Result<Tab>;
    pub async fn create_pane(&self, input: &Create<Pane>) -> Result<Pane>;
    pub async fn pane(&self, id: IdOf<Pane>) -> Result<Pane>;
    pub async fn rename_pane(&self, input: &Rename<Pane>) -> Result<Pane>;
    pub async fn remove_pane(&self, id: IdOf<Pane>) -> Result<()>;
    pub async fn select(&self, attachment: IdOf<Attachment>, input: &Select) -> Result<Attachment>;
    pub async fn switch_session(&self, attachment: IdOf<Attachment>, input: &SwitchSession) -> Result<Attachment>;
}

// crates/ship-client/src/lib.rs
mod stream;
pub mod observer;
pub use stream::AttachmentStream;
impl Client {
    async fn json<T: serde::de::DeserializeOwned>(&self, request: reqwest::RequestBuilder, expected: reqwest::StatusCode) -> Result<T>;
    async fn empty(&self, request: reqwest::RequestBuilder, expected: reqwest::StatusCode) -> Result<()>;
}

// crates/ship/src/cli.rs
pub enum Command { Server(ServerArgs), Session(SessionCommand), Tab(TabCommand), Pane(PaneCommand), Observe(ObserveArgs) }
pub struct ObserveArgs { pub session: IdOf<Session> }
pub enum SessionCommand { Create(CreateSessionArgs), List, Inspect(SessionTarget), Rename(RenameSessionArgs), Remove(SessionTarget) }
pub enum TabCommand { Create(CreateTabArgs), Inspect(TabTarget), Rename(RenameTabArgs), Remove(TabTarget), Move(MoveTabArgs) }
pub enum PaneCommand { Create(CreatePaneArgs), Inspect(PaneTarget), Rename(RenamePaneArgs), Remove(PaneTarget) }

// crates/ship/src/main.rs
mod commands;
mod temporary_observer;
async fn dispatch(cli: cli::Cli, explicitly_selected_server: bool) -> ship_core::Result<()>;
```

The server's `router` helper currently has no argument. This plan changes it to accept an initialized runtime rather than hide actor startup inside synchronous route construction. Make it crate-private with the runtime handle, since `serve` and `openapi` are the intended public entry points. OpenAPI construction remains possible without spawning actors. This proposed helper visibility/signature change is part of this review; do not silently preserve a public no-argument helper that cannot supply authoritative state.

Concrete CLI argument proposal: `ship session create <name>`, `ship session list`, `ship session inspect|remove <id>`, `ship session rename <id> <name>`; equivalent tab/pane CRUD with creation `--parent <typed id>`. `ship tab move <id> --parent <typed id> [--before <tab id> | --after <tab id>]`; placement flags are mutually exclusive. `ship observe <session id>` launches the temporary adapter. Tabs/panes are listed through their owning session/tab inspection; no new global listing route. Declare Clap argument structs beside these enums; their fields are the corresponding typed targets, name, parent and optional placement. Bare `ship` still uses existing local health/startup behavior; explicit entity/observe commands use the selected server URL and do not gain implicit startup/fallback policy.

Proposed manifest settings, **inside this paper only**:

- Retain the current Axum 0.8, Reqwest 0.13.4, Tower HTTP 0.7 and Utoipa/utoipa-axum lines unless dependency resolution demonstrates incompatibility. No speculative upgrades.
- Add UUID 1 with `v4` and `serde`; IndexMap 2 with `serde`; Kameo 0.22.2 with bounded-mailbox support. Add only features actually used by the chosen Kameo API.
- Add `futures-core`/`futures-util` 0.3 and `eventsource-stream` 0.2 for incremental SSE parsing. Use Serde's `rc` feature only if shared `Arc` values cross serialization; owned wire data does not require it.
- Extend Tokio with `sync` and `io-util` for latest-value channels and stream utilities. The temporary Unix stdin adapter uses readiness/nonblocking I/O rather than `io-std`; extend the app's existing nix dependency with the descriptor/flag operations actually needed, without changing server/client input policy.
- Extend Reqwest with `stream`, `zstd` and `gzip`; keep JSON/rustls and redirects disabled. Preserve retry-never for mutation and attachment requests; stream reconnect remains explicit.
- Extend Tower HTTP with `compression-zstd` and `compression-gzip`. Explicitly permit SSE in its predicate while retaining the ordinary exclusions for unrelated response types. Keep compression and framing outside the state actor.
- Continue route-derived OpenAPI. Typed IDs appear as strings, either parents as alternatives of strict typed strings, and nested maps as their actual serialized map shape. Do not use schema generation to invent different wire representations.
- Bound actor mailboxes using Kameo's explicit bounded configuration. Exact capacities are tuning values, not user-visible contracts; choose conservative small capacities and record them during implementation. Do not turn them into configuration surface in this slice.
- Cargo regenerates `Cargo.lock` during separately authorized dependency work. It is generated output and is never manually edited. Lockfile changes belong to the slice adding each dependency.

**Review choices and blocking evidence:** This draft exposes concrete prefix/request/CLI spellings and the router helper change for approval. Dependency resolution, Kameo unreceived-reply destruction, incremental zstd decoding and platform runtime behavior are future implementation checks, not proof supplied by this paper. If those checks invalidate the approved lifecycle or compression design, stop and reopen the affected paper rather than invent a fallback. No unresolved product choice is hidden in the skeleton.

### Planned untouched files and areas

- `crates/ship-core/src/protocol.rs`: health constants and response remain intact; replica data lives in its new module.
- `crates/ship-core/src/error.rs`: existing error representation/macros remain intact; server HTTP mapping supplies operation-specific statuses.
- `crates/ship-server/src/health.rs`: existing health response behavior.
- `crates/ship/src/local.rs` and `crates/ship/src/diagnostics.rs`: local process startup/readiness, tracing and signal behavior.
- Experiments, reference/vendor source, archived OpenSpec records, generated/schema files and broader planning documents. Their historical evidence is not rewritten to claim implementation.
- PTY/runtime owners, layout geometry, terminal rendering, persistence, authentication and non-loopback exposure.
- No permanent test, fixture, probe, script or benchmark files. Use real independent workflows and disposable external tooling when implementation is separately requested.

## Build order

These are future implementation checkpoints, not actions performed during program design. Each slice crosses the necessary layers to a real observable result. No initial types-only or actors-only slice is called complete. Later slices extend working earlier behavior.

### S1. Session CRUD through the real CLI

**Files:** add core `identity.rs`, `entity.rs`, `action.rs`; server `state.rs`, `runtime.rs`, `http.rs`; app `commands.rs`. Extend core/server/client exports, client `api.rs` and JSON helpers, app `cli.rs`/`main.rs`, workspace/core/server/app manifests and generated lockfile.

**Input → output:** CLI creates an empty session, prints its entity JSON with stable typed ID, lists/inspects it, renames it and removes it. State exists behind one actor, not a route-local collection. Preserve bare health, loopback restriction and startup/shutdown behavior.

**Completion evidence:** run a server and use real CLI plus `jq -r '.id'` to inspect/rename/remove. Duplicate session name and duplicate rename return clear conflicts without changes. Wrong/malformed/missing targets produce approved HTTP errors. Removal returns 204 and client does not try decoding JSON from it. Compare route-derived OpenAPI against actual operation wire shapes. Existing health workflow remains successful. No observation claims yet.

### S2. Recursive tab CRUD and indivisible movement

**Files:** extend core entity/action/identity exports, server state/http, client api, app commands/cli. No separate topology owner or global index is added.

**Input → output:** create nested tabs using session-or-tab parent IDs; inspect/rename/reorder; move a full subtree within/between sessions; remove recursively.

**Completion evidence:** create two sessions and nested tabs, record IDs, run append/before/after moves and compare inspected sibling order. Verify IDs survive cross-session movement. Reject self/descendant moves, missing destination, wrong sibling and wrong parent kind; compare full before/after structure after each rejection. Recursive removal deletes exactly its requested subtree. Creation remains primitive and returns entity JSON directly.

### S3. Logical pane CRUD inside tabs

**Files:** extend core entity/action, server state/http, client api, app commands/cli. No terminal or layout dependency.

**Input → output:** create/list-through-inspection/inspect/rename/remove leaf panes under tabs; tab moves carry panes; tab/session removal deletes them recursively.

**Completion evidence:** duplicate pane names work. Reject session/pane parents without changing state. Removing one pane preserves its siblings and owning tab. Move a nested tab subtree across sessions and compare recorded pane IDs; recursively remove it and prove every owned pane disappears. Creating panes starts no child processes and leaves child tabs intact.

### S4. Two live observers with independent server-owned selection

**Files:** add core `replica.rs`; server `relay.rs`, `notifier.rs`, `attachment.rs`; client `stream.rs`, `observer.rs`; app `temporary_observer.rs`. Extend core exports, server state/runtime/http/lib, client exports/api, app commands/cli/main and streaming/actor/parser manifest settings plus generated lockfile.

**Input → output:** `POST /attach` establishes a stream and first handshake. Two temporary text observers see shared structure and same-session viewing records, match their own IDs, and accept same-process select/session controls through `X-Ship-Attachment-Id`. Ordinary CRUD still needs no header. Session removal leaves streams unattached and switchable.

**Completion evidence:** select different panes/tabs in two processes, inspect their JSON records and mutate from a third CLI process. Verify independent selections, selected-pane deletion fallback, ancestor deletion fallback, within-session move retention and cross-session source fallback. Attach while making edits and verify seeded state plus subsequent final state without requiring an extra mutation. Switch one observer into another session and verify the structure and its own record agree; both old/new session peers see viewing-record membership update. Remove a session, verify unattached output, then type a new session command without restarting. Missing/malformed/stale header returns 400/400/404. Cancel a stream before first-frame consumption and after handshake; inspect same-session records to prove attachments are removed. Closing an old stream cannot remove a replacement attachment. Demonstrate direct/mapped relay delivery and deliberate filtering through disposable execution if the real observer path does not exercise a filter; do not commit a harness.

### S5. Recovery, stale-state rejection and slow-output convergence

**Files:** extend client stream/observer and app temporary adapter; extend server attachment/state/notifier only where integration evidence exposes a violation of the existing contract.

**Input → output:** a running disconnected observer reports loss, rejects controls until attached, reconnects with a fresh ID and replaces its state from the current snapshot. Slow consumption eventually receives the final state without another edit.

**Completion evidence:** interrupt the observer connection without stopping the server, edit while disconnected, restore connectivity and verify automatic recovery, unchanged surviving pane selection and a fresh attachment ID/header. Repeat after selected-pane deletion and cross-session movement; reconnect falls back to the surviving session root, not the former parent. Repeat after session deletion; remain unattached and explicitly switch from the same process. Disconnect while a control response is in flight and verify no replay and no late old-stream update. Pause output consumption, commit a final edit, resume and observe that final revision without further mutation. Feed an older captured frame through disposable tooling to demonstrate rejection without permanent test files. Report whether output was delayed by application backpressure or already-buffered transport; do not claim bytes were retracted.

### S6. Negotiated compression, machinery failure and bounded shutdown

**Files:** extend server attachment/runtime/lib/http, client stream/lib, app main as needed for supported decoding. Extend workspace/server/client manifests with approved codec features and generated lockfile; no custom compression implementation.

**Input → output:** real compressed POST SSE works through the actual client, with negotiated zstd and gzip available. A relay/notifier failure cannot leave the server accepting edits while observers silently stagnate. Process shutdown ends streams, initiates attachment cleanup and terminates within the existing total bound.

**Completion evidence:** inspect actual response headers and incremental observations through Reqwest, not an explicitly decoded synthetic probe. Force each supported negotiation path using disposable tooling; demonstrate a final frame arriving before response EOF. If automatic incremental zstd decoding fails, reopen architecture instead of substituting a custom compressor. Exercise actor termination/closed delivery and show coordinated server failure and client disconnection, while distinguishing ambiguous committed commands from rejected ones. Terminate the server with open, stalled and cancelled streams and verify bounded shutdown without waiting on stdin or cleanup to stopped actors. Repeat accepted CRUD/observer/recovery workflows on macOS and Linux; unavailable checks remain explicitly unverified.

Deletion of `temporary_observer.rs` belongs to the later real-client UI change, not this slice's acceptance work. That change removes its stdin parser and CLI wiring, and drives the reusable observer/control interface from the real UI instead. Do not preserve the adapter as a hidden legacy mode by default.

## Deviation log

Empty. No implementation has started. During separately requested implementation, record each surprise, the question asked or assumption approved, and which locked papers it affects. Reopen affected papers under the product → architecture → program ripple rule before continuing.
