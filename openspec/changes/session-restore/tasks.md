# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` by 2026-10-16, and they are the contract. `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `ship-core` checks with and without `clap`;
- `openspec validate session-restore --type change --strict`.

Use owned temporary servers on ports 43321 to 43327, with `XDG_STATE_HOME` pointed at a temporary directory so the real saved tabs and logs are untouched. Don't restart existing user work, including the server on 43179, and clean up only what the exercise created. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

## 1. Restore from a file

- [x] 1.1 Add loading and restore:
  - `saver.rs` with `Saved`, `load`, `project`, `blank` and `prev`.
  - `ServerState::new` takes the saved tabs and builds its tree through `restore` and `restore_tab`; `start_pane` takes the pane's ID, and `CreateTab` and `CreatePane` pass `Id::new()`.
  - `PartialEq` on `Tab`, `Pane`, `Layout`, `Split` and `TabGeometry`, and `panes_mut` on `Tab` and `Layout`.
- [x] 1.2 Wire startup:
  - `serve` takes `save` and `restore`, subscribes pane events before the constructor, loads unless `restore` is false, and creates the starter only when nothing was restored.
  - `--no-restore` and the new `--starter` help in `cli.rs`.
  - `runlog::state_dir` and `runlog::create_private_dir`, with `RunLog::start` using the latter; `main.rs` creates `tabs/` and passes `tabs/<port>.json`.
  - The `open_first` comment in `main.rs`.
- [ ] 1.3 Verify with a hand-written file, made by saving `ship tab list` output as `<state>/ship/tabs/<port>.json`:
  - Nested tabs, a three-way split with custom ratios, a zoom, names and passthrough on one pane come back after a restart; `ship tab list` matches except command, title, status and geometry, and each pane's `cwd` is its old directory (SC-001 by hand).
  - An open client reconnects to the same pane.
  - A deleted directory starts in home, and the log names the pane and directory.
  - A configured shell that doesn't exist leaves panes out with warnings, and a tab's zoom survives when its zoomed pane did.
  - Garbage starts empty with a warning naming the file and `.prev`, and leaves the file untouched. A duplicated pane ID or a zoom on a foreign pane is trusted as written, without structural validation (decision 9, amended).
  - `--no-restore` starts empty. `--starter` adds a tab only when nothing was restored, including the empty-file and `--no-restore` cases.
  - Bare `ship` opens on the first restored tab's first pane. `ship server --help` shows both help texts.
  - Without a file, existing workflows behave as before.

### Slice 1 verification checkpoint

Historical checkpoint: implementation initially paused on 2026-10-16 pending approval to extend the planned file list to `crates/ship-server/src/pane.rs`; see the program paper's deviation log. That extension subsequently shipped in `74b7a86`. The automated verification follow-up below rechecked the source and nonexistent-shell behavior, so task 1.1 is now complete. Task 1.3 remains unchecked for bare-launch verification.

Passed on macOS: formatting, workspace/all-target Clippy with warnings denied, debug and release builds, `ship-core` checks without default features and with `clap`, and strict OpenSpec validation. A temporary real-CLI/PTY probe verified nested tabs, three-way splits with custom ratios, names, zoom, passthrough, saved cwd rather than live `dir`, fresh shells rather than old commands, starter suppression, a full-screen client reconnecting to its selected pane after SIGKILL, restored screen access, missing-directory fallback with warnings, and failed-pane removal retaining a surviving zoom. Graceful shutdown closes the client as before; reconnect was exercised after SIGKILL. The configured nonexistent-shell check failed because the dependency falls back to the login shell. Remaining malformed-file/startup checks and bare-launch verification have not completed. Linux is unverified. No existing server, including port 43179, was changed; probe servers used port 43321 with temporary state.

Probe: `/tmp/ship-restore-slice1.py`; latest output: `/tmp/ship-restore-slice1.out`. No permanent tests were added.

## 2. Save as things change

- [x] 2.1 Add `Saver` (`new`, `run`, `save`) and `replace` to `saver.rs`, and `tempfile` to `ship-server`'s manifest.
- [x] 2.2 Wire the saver in `serve`: compute the baseline from the first replica, spawn `Saver::run` under a root `saver` span after the starter check, and await it after `stop_actors`.
- [ ] 2.3 Verify:
  - A split, a rename, a passthrough toggle and a `cd` each trigger a save 5 seconds after the first unsaved change; a `cd` every second does not postpone it. Writing takes additional time.
  - SIGKILL after a completed scheduled save restores the change, 20 out of 20 (SC-002, timing clarified after slice 2).
  - Killing the server during saves in a loop of at least 200 iterations never leaves a file the next start rejects (SC-003).
  - Start and stop with no change leaves the file's mtime alone (FR-007); a change followed by `ship server stop` within 5 seconds is saved.
  - The first save of a run writes `.prev` with the old bytes, including a garbage file's, and later saves leave `.prev` alone.
  - The file is `0600` and `tabs/` is `0700` (`stat`).
  - After running `sleep 1234` in a pane, `grep` finds no `1234` in the file (SC-006).
  - A read-only `tabs/` logs an error every 5 seconds and the server keeps working.
  - Ten restarts through bare `ship` keep the top-level tab count (SC-007).

## 3. Docs and size

- [x] 3.1 Document the user-visible behavior:
  - `docs/guide.md`: saved tabs, `--no-restore`, `.prev` recovery, and where restore warnings go.
  - `README.md` "What works today".
  - `GLOSSARY.md` gains "Saved tabs" with Workspace, Session, Snapshot and Saved state to avoid (FR-018).
  - The product brief's milestone 2 (FR-017).
- [ ] 3.2 Verify:
  - `openspec validate session-restore --type change --strict`, every link resolves, and the guide's commands run as written.
  - Cyan's feel check of bare `ship` with 10 tabs and 30 panes against a single starter tab (SC-005).
  - Implementation Rust and test lines reported separately against the count before this change.
- [ ] 3.3 When the PR opens, record the 60-second exit-window requirement on #16 (architecture decision 12), and file issues for anything left open.

### Slice 3 checkpoint

Task 3.1 is complete. Task 3.2 remains unchecked for Cyan's feel check (SC-005); task 3.3 waits for a PR, and none exists for `session-restore` at this checkpoint. No PR or tracking issues were created. This slice did not claim to finish earlier verification; the later automated follow-up below records the additional checks.

The guide now covers saved tabs, fresh shells, the save timer, `--no-restore`, private per-port files, manual `.prev` recovery and server-log-only warnings. README lists saved tabs as working and no longer lists restore as unbuilt. The glossary defines Saved tabs without introducing a workspace entity, and milestone 2 defers agent conversation resume. The proposal, design summary and delta specs now reflect the already-approved decision 9 amendment: parsed files are trusted without structural validation.

Timing clarification accepted by Cyan after slice 2: keep the five-second trigger, not an arbitrary four-second margin. Serialization and disk I/O take additional time. FR-005, SC-002, the delta scenarios and these checks now say that explicitly. The original exact-five-second SIGKILL trial lost its rename (0/1); it was not rerun or relabeled as a pass. The 200-interruption loop, read-only retry and remaining earlier verification had not run at that checkpoint; see the automated follow-up below.

Passed on macOS: formatting, workspace/all-target Clippy with warnings denied, debug/release builds, `ship-core` checks with no default features and with `clap`, strict OpenSpec validation, and local Markdown link/fragment validation for the touched docs and change artifacts. All four new saved-tabs guide command blocks ran as written with the debug binary on `PATH`, isolated `XDG_STATE_HOME`, and an owned server on 43326. The probe verified start/list/stop, an empty `--no-restore` start, and manual `.prev` recovery of the original tab/pane IDs. Existing user servers were untouched; the owned probe server stopped cleanly. Linux and the subjective 10-tab/30-pane feel comparison are unverified. No Rust or permanent tests changed in this slice.

Probe: `/tmp/ship-restore-slice3-guide.py`. State and logs: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-restore-guide-6ur81g4d`.

### Manual restore confirmation and spec sync, 2026-10-16

Cyan reported, "i tested it and it works! the session restore actually works!" This confirms a manual restore workflow. The report did not specify ten default-server restarts, the 10-tab/30-pane feel comparison or its platform, so it does not complete those checks. Tasks 1.3, 2.3 and 3.2 remain unchecked for that narrower outstanding evidence.

Synced the deltas into the main `session-restore`, `health-exchange` and `session-structure` specs before opening the PR. The structure delta also removes the obsolete "held in memory only" sentence, without changing the other tree rules. The change remains active; syncing is not archiving. Main-spec validation passes with long-requirement warnings on existing capabilities.

### Automated verification follow-up, 2026-10-16

Task 1.1 is complete: source inspection confirmed `Saved`, loading/projection, constructor restore, caller-supplied pane IDs, the five `PartialEq` derives and mutable pane traversal. The real nonexistent-shell check now passes with the configured-shell fix in `74b7a86`. Progress is 5/9 complete. Tasks 1.3 and 2.3 remain unchecked only for their exact bare-`ship` checks on the default port; existing user work on 43179 was not touched.

Passed on macOS with the debug binary, isolated state and owned servers:

- **Restore structure and reconnect, 43321:** nested tabs, three-way splits, custom ratios, names, zoom and passthrough round-trip from a hand-written `ship tab list` file; fresh shells use saved cwd rather than live fields or old commands. A real PTY client reconnects after SIGKILL to its selected pane, and restored screens are readable. Missing-directory fallback logs the pane/directory; failed-pane removal retains a surviving zoom.
- **Startup, 43321:** missing file without starter stays empty and unwritten; invalid JSON and a genuinely unreadable mode-000 file start empty, warn with the file/`.prev` paths, and stay untouched. Empty/garbage/missing-file starters work; restored tabs suppress the starter. `--no-restore` stays unwritten when idle, while its starter saves and backs up the original bytes. A nonexistent configured shell omits panes with warnings. Parsed duplicate IDs, a foreign zoom and mismatched keys are accepted under amended decision 9. Repeated JSON tab keys keep the last entry; help describes the flags and `.prev`.
- **SC-002, 43322: 20/20.** Rename, wait for its scheduled save to reach disk, SIGKILL, restart and compare the exact name. These checks pass the clarified criterion; the earlier exact-five-second trial is not relabeled.
- **SC-003, 43322: 200/200.** Observe a real temporary file during the write phase before SIGKILL: 100 trials during backup replacement and 100 during primary-file replacement, with large JSON fixtures to expose the windows. Every file parsed and every next server restored its exact old/new name. All 200 retained the old valid file; none hit a post-rename outcome. All 200 attempts observed their intended phase, so no missed windows were dropped or counted. This exercises interruption around temporary writes, not power-loss durability or every possible instruction boundary.
- **Failed-save retry, 43322:** read-only `tabs/` produced three error events, with retry gaps of 5.003792 and 5.004671 seconds. The server kept serving commands, preserved the old file, and saved the pending rename after permissions were restored.
- **Shutdown signals, 43322:** SIGTERM and SIGINT each flushed a pending rename before the timer; a restart restored each name.
- **Supplemental startup timing, 43327:** five alternating runs per fixture, `/bin/sh`, debug binary. Median compatible-health readiness was 13.5 ms for one restored pane versus 66.5 ms for 10 tabs/30 panes; startup through CLI retrieval of the first nonempty pane screen was 25.4 versus 78.9 ms. This measures foreground restart plus CLI polling, not bare launch or full-screen rendering, and does not complete Cyan's subjective SC-005 check.

Formatting, Clippy with warnings denied, debug/release builds, both `ship-core` feature checks and strict OpenSpec validation pass. No Rust or permanent tests changed. The probes stopped their owned servers; Linux remains unverified. Task 3.2 still needs Cyan's feel check and task 3.3 still waits for a PR.

Evidence:
- `/tmp/ship-restore-verification.py` with `startup`, `save` and `timing` modes; output in `/tmp/ship-restore-verify-{startup,save,timing}.out`.
- `/tmp/ship-restore-verify-structure.py` and `/tmp/ship-restore-verify-structure.out`.
- Startup state/logs: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-verify-startup-ofuj9ftf`.
- Save state/logs: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-verify-save-77egfnic`.
- Structure state/logs: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-restore-slice1-kqst92t9`.
- Timing samples: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-verify-timing-apxa0zhh/timing.json`.

### Size

Measured with `cloc 2.10` over tracked Rust under `crates/`, excluding blank/comment lines, `vendor/`, dependencies and experiments. Baseline is `4c3d2c8`, before this change; current implementation is `8aac488` (slice 3 changes only docs).

| Category | Before | After | Delta |
| --- | ---: | ---: | ---: |
| Implementation Rust | 9,335 | 9,614 | +279 |
| Authored test Rust | 0 | 0 | 0 |
| Full authored Rust code, including tests | 9,335 | 9,614 | +279 |

No generated Rust or test files/modules/attributes were found under `crates/`; vendored and dependency Rust are excluded rather than counted as authored implementation. The increase is 74 code lines (36%) above the approximately 205-line estimate. `saver.rs` is 167 code lines against about 125 estimated; restore wiring, mutable pane traversal and the configured-shell fix account for the rest. Implementation remains below the roughly 20,000-line ceiling.

Raw counts: `/tmp/ship-restore-size-before.json` and `/tmp/ship-restore-size-after.json`; baseline source export: `/tmp/ship-restore-size.ETeyIT`.
