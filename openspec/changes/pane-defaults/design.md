# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Cyan locked all three on 2026-10-21. The product paper was amended that day for FR-003. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P6, FR-001 to FR-011 with the amended FR-003, success criteria SC-001 to SC-005, no open unknowns.
- [`design/architecture.md`](design/architecture.md): shape A, the latest feed plus a 256-revision ring chosen per attachment, and key decisions 1 to 11.
- [`design/program.md`](design/program.md): the touched files and signatures, four choices made while drafting, the six slices and the deviation log.

## Context

Replicas reach clients through a `watch` channel that keeps only the newest value, and every reader relies on that: the saver, input routing, the full-screen client and waits. A pane that exits and then closes would leave a waiter seeing only "pane gone". The shell and history size are server-wide settings read when a pane spawns, and right-click passthrough has no default. See proposal.md for why the change is wanted.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (a shell's pane closes when it exits), P2 (an agent reads output after exit), P3 (failures stay), P4 (one defaults table), P5 (changing a live pane) and P6 (settings survive a restart).

**Non-goals:** see the product paper's Non-goals.

## Decisions

1. **Keep the latest feed and add a ring beside it** (architecture decisions 1, 2 and 6). `commit()` publishes the same `Arc<Replica>` to the existing `watch` and to a `broadcast` ring. Each attachment picks `feed: latest | every`. A wrapper type owns both channels and the rule that an empty ring isn't a closed sink. Rejected: server-side waits (the server would own every wait condition), recent exits in the replica (history in a state snapshot), broadcast only (rebuilds "current value" by hand), and Zellij-style `create --wait`.
2. **Overflow ends the stream with `lost`** (decisions 3 and 4). Ring of 256, fixed in code. Rejected: silently skipping, which could misreport an exit, and unbounded per-client queues.
3. **Exit, then close, as two revisions** (decision 5). The one ordering promise is written into the pane-automation spec. Other commits stay free to be batched later.
4. **Pane settings resolved once at creation** (decisions 7 and 9). The state actor reads `[server.defaults.pane]` once per create and fills absent fields. `pane::spawn` no longer reads the file. Saved panes from before this change get the built-in defaults.
5. **Live history size through one session command** (decision 8), using libghostty's `set_scrollback_max_bytes`, which prunes at once when lowered.
6. **Protocol 15** (decision 10).

## Risks / Trade-offs

- [A waiter more than 256 revisions behind misses the exit] → It fails loudly with "fell behind" instead. FR-003 was amended to allow this.
- [`every` streams serialize every revision] → Only waits use them, and only while attached. Not profiled. Measure replica serialization during an output burst if it ever feels slow.
- [An exited pane may show for one frame before it closes] → The UI's `latest` feed usually merges the two revisions.
- [Future commit batching could hide the exit revision] → The spec's "Exit before close" requirement guards it.
- [The `[server] shell` and `scrollback_bytes` settings break with no alias] → Accepted in the product paper. `ship config check` names the field and its line.

## Migration Plan

Move `shell` and `scrollback_bytes` from `[server]` to `[server.defaults.pane]`. Client and server come from the same build and the protocol goes to 15, so an old client is refused with a clear error. Saved tabs from before this change restore with `close` and 10,000,000 bytes.
