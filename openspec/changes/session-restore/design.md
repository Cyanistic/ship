# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Cyan locked the product paper on 2026-10-15, and the architecture and program papers on 2026-10-16; the architecture paper's decision 2 was amended during program review the same day. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P5, FR-001 to FR-020, success criteria SC-001 to SC-007.
- [`design/architecture.md`](design/architecture.md): shape A, a saver task watching the replica, and key decisions 1 to 13.
- [`design/program.md`](design/program.md): the planned files, types and signatures, gates G1 to G3, the build order and the deviation log.

## Context

When Ship's server stops, through `ship server stop`, a crash or a reboot, every tab, split and directory is lost. This change saves the tab tree to `~/.local/state/ship/tabs/<port>.json` as it changes and rebuilds it when the server starts, with a fresh shell in each pane's last directory. It covers the save-and-restore half of the product brief's milestone 2. It depends on diagnostics, now merged, for the state directory and the server log, where restore problems are reported.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (get the workspace back after a restart), P2 (changes are saved without asking), P3 (shutdown doesn't erase the workspace), P4 (a directory that no longer exists) and P5 (starting fresh, and getting the old workspace back).

**Non-goals:** see the product paper's Non-goals. The main ones:

- Agent conversation resume, and rerunning or holding a pane's saved program.
- Restoring screen contents or scrollback.
- Detecting operating-system shutdown.
- Telling clients about restore problems.
- An explicit save or restore command, more than one kept copy, or restoring from `.prev` automatically.

## Decisions

- **A saver task watches the replica** (architecture decision 1). It projects each replica, compares it with the last projection written, and writes at most 5 seconds after the first unsaved change, never postponed (decision 5). It flushes once more when the replica stream closes at shutdown (decision 6).
- **Restore is constructor input** (decision 2, amended). `ServerState::new` takes the saved tabs and starts each pane with its saved ID before the actor runs. Pane events are subscribed before the constructor (decision 3).
- **The file is the existing `Tabs`, projected** (decision 4): pretty JSON in `ship tab list`'s shape, with the pane's directory in `cwd` and live fields blanked.
- **Atomic writes and one `.prev` copy** per run (decision 7).
- **Restore problems are warnings** (decisions 8 and 9). A bad directory falls back to home, a failed shell leaves its pane out, and an unreadable or inconsistent file starts empty and stays on disk.
- **`--starter` only when nothing was restored** (decision 10).
- **The 60-second exit window waits for #16** (decision 12). No exit changes the saved tabs today.
- **Settled program gates:** G1, build on merged diagnostics. G2, the starter tab is a saved change, so `--no-restore --starter` replaces the saved tabs 5 seconds after start, with `.prev` keeping the old file. G3, sibling tabs repeated in one JSON object keep the last.
- **Protocol version unchanged.** Nothing on the wire changes.

## Build order

Program paper, "Build order":

1. Restore from a file.
2. Save as things change.
3. Docs, specs and size.

## Risks / Trade-offs

Architecture paper, "Risks and unknowns". The ones to watch:

- Every commit costs a projection and a comparison, including title and status changes.
- The directory isn't synced after the rename, so a power cut right after a save can bring back the previous file.
- 30 shells start together in the constructor, which could slow bare `ship` (SC-005).

Estimated size: about 205 lines of implementation Rust.

## Migration Plan

None. The saved file is new; a server without one starts as it does today. Protocol 13 is unchanged.

## Open Questions

None.
