# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Cyan locked all three on 2026-10-19. The product paper was amended during architecture (FR-018, SC-003) and the architecture paper during program design (decision 4, the optional input header). Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P4, FR-001 to FR-023, success criteria SC-001 to SC-005, open unknowns U-1 and U-2.
- [`design/architecture.md`](design/architecture.md): shape A, an observer attachment with the waiting done in the CLI, and key decisions 1 to 7.
- [`design/program.md`](design/program.md): the planned files, types and signatures, gates G1 to G3, the build order and the deviation log.

## Context

Milestone 4 reduces to two commands because the rest of it already exists. `ship pane send` writes input to a pane, and `ship pane wait` waits on the program status a pane's program reports through OSC 7501. Ship doesn't detect agents; whatever a program reports is what the commands act on.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (give a sibling work and wait for it to finish), P2 (send input without waiting), P3 (wait for a particular status or for the program to end) and P4 (clear failures).

**Non-goals:** see the product paper's Non-goals. The main ones:

- Agent detection, names, kinds or an `agent` command group.
- Waiting for output text, an event stream for CLI callers, or a `--timeout` flag.
- Guarding plain `send`.
- Waiting on non-root records or several panes.

## Decisions

- **The published revision is the source of truth** (architecture decision 1). A wait sees what every client sees; a status that never reaches a revision didn't happen.
- **The waiting happens in the CLI** (decision 2). `wait` and `send --wait` attach as an observer, with no area and no selection (decision 3), and read `state` replicas. The server keeps no per-wait state, and a killed CLI detaches through the existing attachment guard.
- **Input uses the existing input stream without an attachment** (decision 4, amended). The header becomes optional; only mouse frames need it. Plain `send` checks the pane with `GET /panes/{id}` and writes, with no attach.
- **The input response's revision marks the write** (decision 5). Only replicas newer than it count after the write.
- **Errors before writing come from the `Attached` snapshot** (decision 6).
- **`send` and `wait` are CLI-only** (decision 7), `#[serde(skip)]` in `pane::PaneCommand`.
- **Program choices:**
  - One `automation.rs` module with one `judge` and one wait loop.
  - FR-018's gate and FR-019's stall are one `Option<Instant>`.
  - `--key` values are crokey `KeyCombination`s parsed by clap, so a bad key is exit 2.
  - A headerless mouse frame is a 422.
  - Errors after the write say the input was sent.
- **Protocol 13 to 14** (gate G3).

## Build order

Program paper, "Build order":

1. Observer attachments.
2. Headerless input and `ship pane send`, then gate G1 (U-1).
3. `ship pane wait` and `send --wait`, after gate G2 (U-2).
4. Docs, specs and size.

## Risks / Trade-offs

Architecture paper, "Risks and unknowns". The ones to watch:

- Once close-on-exit exists, exit and removal commits can arrive back to back and a waiter can miss the exit status. That change has to handle it; a tracking issue is filed when this one merges.
- Each waiting command costs two commits (attach and detach), each publishing a replica to every client. Not profiled.
- Observer records appear in `viewers`; anything that later counts viewers has to skip records without an area.
- `send --wait` on a pane already `working` returns when that turn ends.

Estimated size: about 220 lines of implementation Rust.

## Migration Plan

Protocol 13 to 14. A protocol-13 client against a protocol-14 server fails its health check with a clear incompatibility error, and the guide's Upgrading section says to update client and server together. Nothing is stored on disk.

## Open Questions

- **U-1, Enter right after a paste.** Probed in gate G1 at the end of slice 2, with Cyan's OK to launch real agents. Blocks FR-004.
- **U-2, what agents report around a prompt.** Probed in gate G2 before slice 3, with Cyan's OK. Blocks FR-018 and FR-019.
