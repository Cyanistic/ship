# Proposal

This is the planning artifact for a locked plan. Cyan locked the product paper in `design/` on 2026-10-15 and the architecture and program papers on 2026-10-16. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

When the server stops, through `ship server stop`, a crash or a reboot, every tab, split and directory is lost and the user rebuilds the workspace by hand. The product brief's milestone 2 asks for it to come back.

## What Changes

- **Saved tabs.** The server saves its tab tree to `~/.local/state/ship/tabs/<port>.json`, or `$XDG_STATE_HOME/ship/tabs/<port>.json`.
  - Saved: tab order and nesting, tab and pane IDs and names, split proportions, zoom, `right_click_passthrough`, and each pane's current directory.
  - Not saved: a pane's program and arguments, title, exit status, screen and scrollback.
  - The file is pretty JSON in `ship tab list`'s shape, mode 0600 in a 0700 directory, one per port.
- **Saving.** The first unsaved change schedules a save 5 seconds later; writing takes additional time and later changes do not postpone the timer. Pending changes are also saved when the server stops through `ship server stop`, SIGTERM or SIGINT. A server that changes nothing never writes. Saves are atomic. A failed save is logged as an error and retried.
- **`.prev` copy.** A run's first save keeps the old file as `<port>.json.prev`. Recovery from it is by hand.
- **Restore.** On startup, before accepting connections, the server rebuilds the saved tabs with the same IDs, so open clients keep their selection. Every restored pane runs a fresh shell in its saved directory and never reruns its old program.
- **Restore problems** go to the server log only.
  - A missing directory starts the shell in home.
  - A shell that can't start leaves its pane out.
  - An unreadable or unparsable file starts empty and stays on disk. A file that parses is trusted without structural validation.
- **`ship server --no-restore`** starts without restoring. Bare `ship` and every launched background server always restore.
- **`--starter`** creates its tab only when nothing was restored. A client opened by the bare `ship` that launched the server opens on the first tab's first pane, restored or the starter.
- `right_click_passthrough` now survives a restart.

Not in this change (the product paper lists them all):
- Agent conversation resume.
- Rerunning a pane's program, or holding it behind a prompt.
- Restoring screen contents or scrollback.
- Detecting operating-system shutdown.
- Telling clients about restore problems.
- An explicit save or restore command, or saving to a named file.
- More than one kept copy, or restoring from `.prev` automatically.
- The 60-second hold for exits, which #16 needs once an exit changes the saved tabs.

## Capabilities

### New Capabilities

- `session-restore`:
  - What the saved tabs hold, and where and how the file is written.
  - When changes are saved, including on stop.
  - The `.prev` copy.
  - Restore on startup, `--no-restore`, and restore problems.

### Modified Capabilities

- `session-structure`: `right_click_passthrough` survives a server restart.
- `health-exchange`:
  - The launched server restores, and its client opens on the first tab's first pane.
  - `ship server` gains `--no-restore`.
  - `--starter` creates its tab only when nothing was restored.

## Impact

- `ship-server`:
  - A new `saver` module loads, projects and writes the saved tabs.
  - `ServerState::new` takes the saved tabs and starts restored panes, and `start_pane` takes the pane's ID.
  - `serve` takes the save path and a restore flag, subscribes pane events before building the state, skips the starter after a restore, and runs and awaits the saver.
  - `tempfile` becomes a dependency.
- `ship`:
  - `--no-restore` on `ship server`, and an updated `--starter` help.
  - `main.rs` creates the private `tabs/` directory.
  - `runlog` gains `state_dir` and `create_private_dir`.
- `ship-core`: `PartialEq` on `Tab`, `Pane`, `Layout`, `Split` and `TabGeometry`. Protocol version unchanged.
- `ship-client`: none.
- Docs: guide sections on saved tabs, `--no-restore` and `.prev` recovery; the README's "What works today"; "Saved tabs" in the glossary; the product brief's milestone 2.
- Estimated at about 205 lines of implementation Rust. Measured counts go in [tasks.md](tasks.md).
