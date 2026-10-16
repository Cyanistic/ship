# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` by 2026-10-16, and they are the contract. `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `ship-core` checks with and without `clap`;
- `openspec validate session-restore --strict`.

Use owned temporary servers on ports 43321 to 43327, with `XDG_STATE_HOME` pointed at a temporary directory so the real saved tabs and logs are untouched. Don't restart existing user work, including the server on 43179, and clean up only what the exercise created. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

## 1. Restore from a file

- [ ] 1.1 Add loading and restore:
  - `saver.rs` with `load`, `check`, `project`, `project_tab`, `project_pane` and `prev`.
  - `ServerState::new` takes the saved tabs and builds its tree through `restore` and `restore_tab`; `start_pane` takes the pane's ID, and `CreateTab` and `CreatePane` pass `Id::new()`.
  - `PartialEq` on `Tab`, `Pane`, `Layout`, `Split` and `TabGeometry`.
- [ ] 1.2 Wire startup:
  - `serve` takes `save` and `restore`, subscribes pane events before the constructor, loads unless `restore` is false, and creates the starter only when nothing was restored.
  - `--no-restore` and the new `--starter` help in `cli.rs`.
  - `runlog::state_dir` and `runlog::create_private_dir`, with `RunLog::start` using the latter; `main.rs` creates `tabs/` and passes `tabs/<port>.json`.
  - The `open_first` comment in `main.rs`.
- [ ] 1.3 Verify with a hand-written file, made by saving `ship tab list` output as `<state>/ship/tabs/<port>.json`:
  - Nested tabs, a three-way split with custom ratios, a zoom, names and passthrough on one pane come back after a restart; `ship tab list` matches except command, title, status and geometry, and each pane's `cwd` is its old directory (SC-001 by hand).
  - An open client reconnects to the same pane.
  - A deleted directory starts in home, and the log names the pane and directory.
  - A configured shell that doesn't exist leaves panes out with warnings, and a tab's zoom survives when its zoomed pane did.
  - Garbage, a duplicated pane ID and a zoom on a foreign pane each start empty with a warning naming the file and `.prev`, and leave the file untouched.
  - `--no-restore` starts empty. `--starter` adds a tab only when nothing was restored, including the empty-file and `--no-restore` cases.
  - Bare `ship` opens on the first restored tab's first pane. `ship server --help` shows both help texts.
  - Without a file, existing workflows behave as before.

## 2. Save as things change

- [ ] 2.1 Add `Saver` (`new`, `run`, `save`) and `replace` to `saver.rs`, and `tempfile` to `ship-server`'s manifest.
- [ ] 2.2 Wire the saver in `serve`: compute the baseline from the first replica, spawn `Saver::run` under a root `saver` span after the starter check, and await it after `stop_actors`.
- [ ] 2.3 Verify:
  - A split, a rename, a passthrough toggle and a `cd` each reach the file within 5 seconds, and a `cd` every second still saves within 5 seconds.
  - SIGKILL 5 seconds after a change restores it, 20 out of 20 (SC-002).
  - Killing the server during saves in a loop of at least 200 iterations never leaves a file the next start rejects (SC-003).
  - Start and stop with no change leaves the file's mtime alone (FR-007); a change followed by `ship server stop` within 5 seconds is saved.
  - The first save of a run writes `.prev` with the old bytes, including a garbage file's, and later saves leave `.prev` alone.
  - The file is `0600` and `tabs/` is `0700` (`stat`).
  - After running `sleep 1234` in a pane, `grep` finds no `1234` in the file (SC-006).
  - A read-only `tabs/` logs an error every 5 seconds and the server keeps working.
  - Ten restarts through bare `ship` keep the top-level tab count (SC-007).

## 3. Docs and size

- [ ] 3.1 Document the user-visible behavior:
  - `docs/guide.md`: saved tabs, `--no-restore`, `.prev` recovery, and where restore warnings go.
  - `README.md` "What works today".
  - `GLOSSARY.md` gains "Saved tabs" with Workspace, Session, Snapshot and Saved state to avoid (FR-018).
  - The product brief's milestone 2 (FR-017).
- [ ] 3.2 Verify:
  - `openspec validate session-restore --strict`, every link resolves, and the guide's commands run as written.
  - Cyan's feel check of bare `ship` with 10 tabs and 30 panes against a single starter tab (SC-005).
  - Implementation Rust and test lines reported separately against the count before this change.
- [ ] 3.3 When the PR opens, record the 60-second exit-window requirement on #16 (architecture decision 12), and file issues for anything left open.
