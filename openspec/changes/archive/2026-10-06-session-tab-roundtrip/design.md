# Session/tab/pane roundtrip design

## Context

This is an assembled index, not a fourth design authority or a copy of the papers. All three papers are locked as of 2026-10-05. No implementation is authorized until Cyan requests it. The change retains its `session-tab-roundtrip` directory name after logical panes were added to scope.

The papers are attached here:

- [Product](design/product.md): locked observable scope, requirements and acceptance workflows, including amendments P1 to P4.
- [Architecture](design/architecture.md): locked state ownership, entity relationships, operation routes, attachment lifecycle and replication, including amendments A1 to A4.
- [Program](design/program.md): locked virtual files and interfaces, seven independently verifiable implementation slices and an empty deviation log.

`design/program-old-do-not-read.md` is a superseded earlier program draft kept by Cyan for comparison. It is not a design authority.

## Goals / Non-Goals

Use the [product paper](design/product.md) as scope authority. Logical panes are metadata-only leaves under tabs. Layouts, PTYs and terminal UI remain later work. Program snippets are declarations inside a document, not source files or compilation evidence.

## Decisions

Use the [architecture paper](design/architecture.md) as the approved shape. The [program paper](design/program.md#build-order) turns it into seven vertical slices: session CRUD, recursive tabs and moves, logical panes, live observation, selection and session switching, reconnect, and compression with failure and shutdown handling. Each slice names its files and observable completion evidence.

Program design reopened both upper papers once, and they were re-approved:

- Removing a session kicks its clients out, and they exit (P1, A1).
- `ship attach <name>` creates a missing session. Session names are a `ship-client` convenience; the server API is ID-only, and names may not contain `:` (P2 to P4, A1).
- Replicas reach SSE streams through a `watch` channel subscribed to a close port of an earlier project's `RelayBus`, so the latest state is never dropped and no notifier actor is needed (A2, A3).
- The attach route creates the attachment ID before asking the state actor, so cleanup always knows which ID to detach (A4).

Temporary same-process stdin controls are isolated in `controls.rs` for deletion when a real client UI replaces them.

## Risks / Trade-offs

Implementation compatibility and runtime evidence remain unverified. In particular, cancellation-safe attachment cleanup, Utoipa schema derivation for the generic payloads, and actual incremental Reqwest zstd decoding must be demonstrated in their real execution paths. A mismatch is recorded in the program deviation log and reopens the affected paper rather than silently changing its design. See the architecture risks and program completion checks for the full contract.

This index does not complete the default OpenSpec CLI workflow: proposal, delta specs and tasks have not been authored. Their schema dependencies remain unchanged. Future artifacts must reference these papers instead of rewriting them or modifying generated schema files to register the design stages.
