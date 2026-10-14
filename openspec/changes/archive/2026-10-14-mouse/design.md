# Design

This file summarizes the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Cyan locked product, architecture and program on 2026-10-13. Locking doesn't start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P6, FR-001 to FR-043, SC-001 to SC-007, and where each requirement of the superseded mouse-drag paper went.
- [`design/architecture.md`](design/architecture.md): shape A, where the client decides and the lane carries every effect, and key decisions 1 to 15.
- [`design/program.md`](design/program.md): the planned files, types and signatures, the build order and the deviation log.

At lock, program gate G1 amended architecture decision 5: a dropped tab's new parent unfolds at the drop's turn, before the move is sent, not after it succeeds. The program deviation log records this. The architecture paper's text is unchanged.

This change supersedes `openspec/changes/mouse-drag/`, whose papers predate server-owned selection and the ordered input lane. They are marked superseded, not rewritten.

## Context

The mouse can select panes and tabs but can't rearrange anything or reach an action without a key. This change adds three drags for things the keyboard and CLI can already do:

- dragging a pane border resizes panes;
- dragging the sidebar's edge resizes the sidebar;
- dragging a sidebar row moves a tab.

It also adds right-click menus for panes, tabs and the workspace, built from config the same way bindings are. A per-pane setting gives right-clicks on a pane's contents to its program instead. That setting arrives through a new `set` command for tab and pane attributes, which replaces `rename`. The change closes the "mouse pane resizing" item in milestone 1 of the product brief.

## Goals / Non-Goals

**Goals:** the product paper's user stories:

- P1: border drag.
- P2: tab drop.
- P3: sidebar width by drag or keys.
- P4: menus.
- P5: right-click passthrough.
- P6: `set`.

**Non-goals:** see the product paper's Non-goals. The main ones:

- Dragging panes to swap or move them.
- Scrolling the sidebar.
- Cancelling a drag with a key.
- Persisting a dragged sidebar width.
- Rename or any other item that needs a text prompt.
- Submenus or state-dependent labels.
- Keeping `rename` as an alias.
- Any new server operation beyond these:
  - `to` on pane resize;
  - `right_click_passthrough` on pane create;
  - `server.tab.set` and `server.pane.set`.

## Decisions

- **The client decides and the lane carries every effect** (architecture shape A). Menus open at once, client-local, and anything that reaches the server keeps its place in the one ordered lane.
- **Gestures in the router** (decision 1). A left press on a border, the divider or a sidebar row becomes a drag once it moves a cell. If it's released before that, it's a click, so sidebar clicks act on release.
- **Border drags queue absolute positions** (decisions 2 to 4, 14). Edges are named by a pane and `right` or `down`, taken from published frames. A waiting resize for the same edge is overwritten in place. `pane resize --to` is exclusive with `--amount`.
- **Drops resolve twice** (decisions 5 to 7). The drop line follows the outliner depth rule. At its turn the drop is re-checked against the replica, and a stale drop does nothing, with no error.
- **Sidebar width is a configured value plus a client-local override** (decision 8). The override is set by the divider drag or `client.sidebar.resize` (`shift-h`/`shift-l` in resize mode).
- **Menus are overlays owned by `Local`** (decisions 9, 10, 15). `Local` gets every event first while a menu is open. Picking an item queues its action exactly as a key would. A right-click selects its target and then opens the menu. Defaults are under `[client.menus]` in `defaults.toml`.
- **Passthrough is decided at press** (decisions 11, 12). It needs the pane's `right_click_passthrough` and that pane's `Screen.mouse` tracking flag, both published.
- **`set` reuses the rename routes** (decision 13). `PATCH /api/v0/tabs/{id}` and `/panes/{id}` take patch bodies, and the actor resolves `toggle` and commits only on change.
- **Settled program gates:**
  - G1: a drop unfolds its new parent at its turn, before the move.
  - G2: a terminal resize closes an open menu.
- **Protocol version 11.**

## Build order

The program paper's "Build order" has eight feature slices, then a slice for docs, specs and size:

1. `to` on pane resize.
2. `set` replaces `rename`; the passthrough attribute.
3. Mouse tracking on `Screen`.
4. Gestures and border drags.
5. Sidebar width.
6. Tab drops.
7. Menus.
8. Passthrough at the router.
9. Docs, specs and size.

## Risks / Trade-offs

The architecture paper's "Risks and unknowns" lists them all. These are the ones to watch:

- Border resizes trail the pointer by about two round trips on slow links, then settle.
- The `to` off-by-one, settled against `ship tab get` in slice 1.
- The tracking flag could lag. The program paper finds this mostly resolved by reading the vendored session, and slice 3 confirms it with Neovim and `less`.
- Menu size: revisit if `menu.rs` passes about 250 lines.

Measured with `cloc` against `dac695b`: +820 client code lines, +113 server/core and +5 vendored implementation Rust code lines, within SC-006's estimates. No authored tests added. Full counts, including physical lines, and coverage limitations are in [tasks.md](tasks.md).

## Migration Plan

- `rename` is removed: the `ship tab rename` and `ship pane rename` CLI commands and the `server.*.rename` bindings. Scripts move to `ship tab set --name` and `ship pane set --name`.
- A config binding to `server.tab.rename` or `server.pane.rename` fails `ship config check` with its line and column.
- Protocol 10 goes to 11, so server and client are rebuilt as a pair. Mismatched versions fail the health check as before.

## Open Questions

None. The product paper's drafting choices were approved with it, and both program gates are settled.
