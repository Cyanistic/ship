# Sessions, recursive tabs and server state

Ship's agreed design direction is a server that owns sessions, with recursive tabs beneath each session. A tab owns both a pane layout and ordered child tabs. There is no workspace entity. This is a design record, not an implemented API or authorization to implement the full operation catalog.

The next checkpoint's product scope is locked in [the session/tab roundtrip product paper](../../openspec/changes/session-tab-roundtrip/design/product.md). Its [architecture paper](../../openspec/changes/session-tab-roundtrip/design/architecture.md) is under review; implementation is not authorized by this design record. [The terminal plan](first-terminal-slice.md) retains terminal feasibility and later workflow checks; [the transport handoff](../research/terminal-transport-handoff.md) records snapshot evidence.

## Ownership and selection: agreed direction

- A server owns sessions. A session contains root tabs; each tab owns its pane layout and child tabs recursively.
- Empty containers are valid. Primitive creation can create an empty tab; a TUI convenience can later compose tab creation with shell creation. Selecting a tab does not require a pane.
- Removing a tab removes its own panes and all descendant tabs by default. No configurable removal policy is part of this direction.
- Moving an owned subtree preserves its IDs. Cross-session moves are desired in the conceptual model. Later terminal runtimes should remain alive during such moves.
- Self/descendant moves must fail without losing metadata. Ownership prevents a representable cycle, but does not by itself make a detach-then-failed-insert operation safe.
- Client selection stores the most specific target as a `Session`/`Tab`/`Pane` enum, not an entire path. The client derives ancestry from shared state. When the selected target is removed, fallback goes to the nearest surviving parent, eventually the session.
- Sidebar expansion is client-local. Remembered focus and selection history are separate optional future behavior. For the locked roundtrip, a cross-session move leaves the source client in its source session and falls back to the nearest surviving source parent.

Nested `IndexMap` collections are the chosen representation direction for sibling order and average O(1) lookup within a local collection. Stable ID-only actions traverse the structure to locate the owner; a global index is not required initially. Local lookup cost is not a claim of O(1) lookup across the tree.

## Typed IDs and action shapes: agreed direction, illustrative schema

Use private UUID-backed `Id<T>` values with `PhantomData<fn() -> T>`. `Identified` supplies an associated `Id`, and the shared `IdOf<T>` alias names it; do not introduce a separate alias for every entity. The entity kind supplies a `prefix()` function, not a constant.

External display, serialization, parsing and deserialization use `kind:uuid` with strict kind-prefix checks. IDs identify targets, not proof that a target exists or that the caller is authorized. Shared state still validates existence and ownership.

Action categories are `Session`, `Tab` and `Pane` enums exposing directly supported actions. Reuse generic `Create`, `Move`, `Rename` and `Remove` payloads where appropriate. Enum variants use named payload types/newtypes, not inline variant struct fields. Prefer small composable traits over a large `Entity` trait. Exact trait names and wire schema remain illustrative, not implemented public interfaces.

### Creation destinations and moves

Keep `Creatable: Identified` with an associated `Parent: Identified` and creation `Input`. A reusable `UntaggedEither<L, R>` represents a choice of parent entities. When both inner types implement `Identified`, its associated ID is `UntaggedEither<IdOf<L>, IdOf<R>>`; its `id()` method maps the active variant to that entity's ID.

For a tab, `Parent` is `UntaggedEither<SessionState, TabState>`. A generic `Create<E>` carries `parent: IdOf<E::Parent>` and `value: E::Input`, so the request contains a session ID or tab ID, not an entity value. This preserves the parent/entity relationship without a separate `Destination` trait mechanism or a domain-specific parent enum.

The either enum has `Left` and `Right` variants and uses untagged Serde serialization. Those names are internal only: the wire parent is a plain `session:<uuid>` or `tab:<uuid>` string. Strict typed ID parsing distinguishes the variants without a second external tag. Prefixes must be stable and unique; overlapping untagged alternatives would make parsing depend on variant order. Exact declarations remain illustrative.

Root session creation needs no invented server ID. A named server-root destination concept could use unit as its destination; exact naming remains open. A single CLI parent argument, illustrated as `--parent`, can identify destination context through a typed prefixed ID. Exact spelling is not finalized. Commands need not supply a whole ownership path.

Creation returns the created entity as machine-readable data, including its ID, using the same serializable entity representation clients receive. Callers can extract `.id` with `jq`. Exact routes and status codes belong to architecture review; an ID-only result or correlated update is not required.

## Actors and typed subscriptions

Kameo is the chosen actor direction. Cyan selected one server-state owner for the roundtrip shape comparison, keeping cross-session edits local. The complete architecture and its delivery details remain under review.

The local [typed relay contract](../research/typed-relay.md) records the required typed publication, subscription and sink-adapter behavior. It is self-contained and does not depend on access to another repository. Concrete dependency versions and Rust integration belong to implementation planning, not historical manifests.

The intended event path is:

```text
internal typed publish
  -> RelayBus TypeId dispatch
  -> registration-time map/filter_map recipient adapter
  -> SseEvent wire enum
  -> notifier
  -> textual SSE
```

Subscription transforms are wired at initialization. Mapping belongs in the relay subscription adapter; a separate mapping actor is not inherently needed. A per-message type-name trait or schema proc-macro is not required to produce the wire enum. Registration must support the selected recipient output type rather than hardcode a `Push` message.

The historical `Full` result preserves a subscription but drops that message. This cannot guarantee final-state delivery. The future snapshot path must retain/coalesce the latest pending state and eventually flush it when capacity returns, even if no later mutation arrives. Neither the historical bus nor the loopback transport probe demonstrates that production guarantee.

## Shared state, transport and resource ownership

Server-authoritative shared structs are separate from the client's internal view and the server's live runtime. Terminal screen contents are outside structural `SessionState`. Ordinary struct mutation remains the editing model; no tracked setters or proxy state are required.

Compressed textual SSE full snapshots are the direction for both structural state and terminal screens. There is no adopted patch pipeline, patch-base revision contract or replay requirement. Publication should coalesce latest state instead of queuing every intermediate frame. Zstd is the leading codec candidate; gzip fallback is a recommendation, not a finalized production policy.

Session revisions reject stale or out-of-order snapshots. They do not close the missed-update gap between initial snapshot capture and subscription. Attachment must coordinate a current snapshot with subscription. Restart baseline identity/reset and exact revision scope remain open.

Cyan's architecture-review feedback favors one streaming `/attach` operation: its first event returns a server-assigned attachment ID and current view, then subsequent events replace the view. Attachment lifetime follows stream lifetime; reconnect creates a new attachment while the running client retains its selected ID locally. The architecture draft describes registration/seed coordination and drop-initiated cleanup. Exact controls and wire framing remain subject to architecture approval; the ID is not authentication.

For later PTYs, separate serializable metadata from server owners holding live resources. RAII ownership is the direction: dropping a runtime owner initiates cleanup. Async child reaping, task joins and shutdown guarantees remain to design. Dropping serialized metadata does not automatically clean up a process.

## Locked product checkpoint, not implementation approval

The [product paper](../../openspec/changes/session-tab-roundtrip/design/product.md) locks session/recursive-tab creation, inspection, rename, move/reorder, recursive removal, text-only observation, independent selections for two clients and automatic reconnect. Session names are unique; tab names may duplicate. The product scope validates structure before panes, PTYs, Ghostty and terminal lifecycle. Actor/HTTP/SSE details and program structure require their own reviewed papers.

Do not substitute fake processes for metadata or silently add configuration, plugins, persistence, batches or a full nested-creation catalog. The terminal checkpoint remains later work, with real shells and explicit lifecycle checks.

## Decisions requiring approval or further design

- Exact action/trait/HTTP/CLI schema and program structure.
- Complete architecture approval, including safe cross-session commits, reliable relay delivery and streaming attachment registration/cleanup.
- Reconnect coordination, revision scope and restart baseline identity/reset.
- Production compression/fallback policy, cadence and eventual-flush implementation.
- Terminal runtime move and asynchronous shutdown/reaping guarantees.
