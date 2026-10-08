# Pane terminals design

## Context

This is an assembled index, not a fourth design authority or a copy of the papers. All three papers are locked as of 2026-10-06. No implementation is authorized until Cyan requests it.

The papers are attached here:

- [Product](design/product.md): the observable behavior of Ship's first real terminal slice, where every pane runs a program, clients show its screen and drive it, and work survives detach.
- [Architecture](design/architecture.md): shape A, where the state actor owns pane runtimes and pane tasks publish screens, titles and exits on the relay bus at frame pace. It includes amendments A1 to A6 from program review.
- [Program](design/program.md): the virtual skeleton, seven independently verifiable implementation slices and an empty deviation log.

Evidence behind the architecture is in the [transport probe note](../../../docs/research/pane-terminals-transport-probe.md).

## Goals / Non-Goals

Use the [product paper](design/product.md) as scope authority. Splits and layouts, real keybindings, the sidebar and scrollback are later slices, and Windows stays best effort (see the product non-goals). Program snippets are declarations inside a document, not source files or compilation evidence.

## Decisions

Use the [architecture paper](design/architecture.md) as the approved shape. The [program paper](design/program.md#build-order) turns it into seven slices: optional names, every pane running a program, screens on the attach stream, input, view and size, the full-screen client, navigation and labels, and two clients with feel and platform checks.

Program review amended the architecture:

- Teardown sends SIGHUP explicitly, because the wrapper keeps its own copy of the master fd. Shutdown waits for every pane's teardown in the state actor's `on_stop` (A1).
- The server picks the first pane only on attach (A2).
- Only viewer sizes are stored. Tab and pane sizes are derived after each commit (A3).
- One `PUT /api/v0/attach/view` with `{selection, size}` replaces the selection, switch and resize paths. The input stream carries only keys and pastes (A4).
- Keys serialize through crossterm's own `serde` feature (A5).
- Programs start just before the commit, and `commit` drops runtimes whose pane didn't make it into the tree (A6).

## Risks / Trade-offs

See the architecture risks and the program's completion checks. Notable unknowns: screen size on the wire under sustained output, the wrapper rendering on every output batch, the narrow shutdown gap for panes removed just before stopping, and Linux, which is unverified for every transport and bus measurement. A mismatch found during implementation goes in the program deviation log and reopens the affected paper.

The [proposal](proposal.md), the delta specs under [specs/](specs/) and [tasks.md](tasks.md) are assembled from these papers and don't replace them. Tasks follow the program's seven slices.
