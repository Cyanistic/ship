# Ship typed relay contract

This is a self-contained design reference for Ship's proposed typed actor relay. It records the relevant behavior of the implementation inspected during planning without requiring another repository. It is not implemented Ship code, a dependency selection or runtime verification.

## Responsibilities

The relay accepts typed internal publications and subscriptions. Producers do not construct SSE events, know connected sockets or name the notifier. Initialization wires typed mappings onto recipients; the notifier receives the mapped output.

```text
subscribe Input to a recipient of Output
  -> wrap recipient with optional filtering/mapping
  -> store subscription under Input's type identity

publish Input
  -> find subscriptions for Input's type identity
  -> give each subscription the publication
  -> mapping returns no output: deliberately skip delivery
  -> mapping returns Output: deliver to its recipient
```

The subscription is typed at registration. Internally, heterogeneous subscriptions can be stored by Rust type identity with type-erased storage; delivery recovers the typed sink for that publication. No string-based message naming is required for dispatch.

## Composable delivery adapters

A sink accepts an input and reports delivery disposition. Adapters compose around an underlying sink:

- **Filter:** forwards inputs satisfying a predicate; rejected inputs are deliberate no-ops.
- **Map:** converts input into the recipient's output type.
- **Filter-map:** returns an optional output, combining filtering and conversion.
- **Actor recipient:** forwards the resulting message to a typed Kameo recipient.
- **Channel/latest-value recipient:** can support other delivery mechanisms when required; it is not a reason to build unused adapters upfront.

Registration should support direct typed recipients and mapped recipients of explicitly chosen output types. A registration helper must not hardcode one delivery message such as `Push`. Ship only needs the adapters exercised by its actual publications.

## Client delivery vocabulary

The SSE format is an explicit shared `SseEvent` enum with named payload types. Serde supplies its tagged wire representation. Subscription mappings select variants and delivery targets; neither a per-message wire-name trait nor a schema-discovery macro is required.

An attachment publication can carry its ID and complete current view. Mapping that domain publication to SSE does not give the producer knowledge of the notifier's socket bookkeeping. Shared session snapshots may be shared immutably across attachment publications internally without changing their serialized representation.

## Delivery semantics must be explicit

The inspected best-effort sink distinguished successful delivery, a full destination and a closed destination. A full destination retained the subscription but dropped that publication; a closed destination removed the subscription. This distinction is useful bookkeeping, but it does not establish latest-state delivery.

Ship's locked product requires the final state to reach a recovered slow observer even if no later mutation occurs. The selected architecture therefore uses:

```text
state owner
  -> bounded awaited enqueue into relay
  -> typed mapping
  -> bounded awaited enqueue into notifier
  -> replace latest attachment view
  -> wake stream reader
```

Actor mailboxes are not socket buffers. The notifier replaces latest-value slots without awaiting client writes. A stalled socket may lag, but superseded not-yet-consumed views do not require an unbounded FIFO. Bytes already consumed by HTTP/compression/TCP cannot be retracted by application coalescing.

Do not silently reuse best-effort `Full` handling on authoritative publication hops. Closure/failure must be surfaced; mapping filters are intentional omissions, not overload loss. The notifier must never call back into a state owner that is awaiting it, or publication/registration can form an actor wait cycle.

## Attachment registration

The state owner coordinates registration with a current-view seed. Registration completes before it allows another structural command to execute. Older in-flight publications cannot overwrite a newer seed because slots compare incarnation/revision. Stream drop unregisters the attachment through a drop guard; asynchronous cleanup is initiated rather than awaited in `Drop`.

Reconnect opens a new attachment. Local selection retention is client behavior, not a reason to persist disconnected server-side client records.

## Evidence and limits

The typed dispatch, composable sinks and registration-time mapping behavior were inspected in source during planning. That source inspection is evidence for the reusable pattern, not for a copied Ship implementation or compatibility with a selected Kameo version. The reliable publication policy and streaming attachment lifecycle above are Ship design proposals evaluated through the current [architecture paper](../../openspec/changes/archive/2026-10-06-session-tab-roundtrip/design/architecture.md).

Implementation must verify direct publication, mapped publication, deliberate filter omission, downstream failure, attachment seeding and final-state delivery through real command/client workflows. No permanent tests or new actor framework are authorized by this reference.
