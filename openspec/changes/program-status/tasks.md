# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` on 2026-10-18, and they are the contract. `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `ship-core` checks with and without `clap`;
- `openspec validate program-status --type change --strict`.

Use an owned temporary server on a free port, with `XDG_STATE_HOME` and the config pointed at temporary directories so the real saved tabs, config and logs are untouched. Run agents in scratch directories with throwaway configs, never their global ones. Don't restart existing user work, and clean up only what the exercise created. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

## 1. Records on panes

- [x] 1.1 Add the scanner and session events:
  - `vendor/ratatui-ghostty/src/program_status.rs` with `ProgramStatus`, `Found`, `Scanner` and `report`.
  - `SessionEvent::ProgramStatus` and `SessionEvent::PromptStart`, and `scan` ahead of `vt_write` in `session.rs`, writing the support reply through the session's writer.
  - `base64` in the workspace and vendored manifests.
- [x] 1.2 Add the records:
  - `StatusRecord`, `ReportedState`, `BlockedKind`, `ROOT`, `Pane.program` and `root_status` in `ship-core`.
  - `records.rs` with `Records` (`apply`, `prompt`, `snapshot`), `within` and `disarm`.
  - `PaneChange::Program` and the `records` field in `pane.rs`; replacing on `Program` and clearing on `Exited` in `state.rs`; clearing in the saver's `blank`.
- [x] 1.3 Verify:
  - The support query gets its reply, before the DA1 reply when both are written together.
  - Claude Code, Codex, Pi and crush 0.98.1 each show their root record in `ship pane get` through a turn and a permission dialog.
  - `id=build` and `id=build/test` sit beside the root; `id=` and `id=a//b` are dropped; a 65th id evicts the oldest.
  - `clear`, a subtree clear, `ESC c`, an exit, and `kill -9` followed by an OSC 133 prompt each remove what program-status's Record lifetimes says.
  - Escapes in `app` or controls in a base64 `msg` don't land; U+202E is stripped from a kept `msg`.
  - A report split across two writes still parses.
  - `ship tab list` shows `program`; the saved-tabs file doesn't.

### Slice 1 verification checkpoint

Tasks 1.1 to 1.3 are complete. Cyan accepted task 1.3 as verified on 2026-10-18 with Codex and Pi run live; Claude Code and crush were not run. Installed versions are Claude Code 2.1.300, Codex 0.163.0, Pi 1.3.0 and crush 0.76.0; the crush gate requires 0.98.1 or later.

On macOS, formatting, workspace Clippy with warnings denied, debug and release workspace builds, `ship-core` checks without default features and with `clap`, strict OpenSpec validation and `git diff --check` pass.

A temporary probe ran 16 checks through an owned server with isolated config and state, then stopped it and its pane processes. It verified BEL/ST query replies before DA1, pane inspection and tab listing, the actual attach replica, child ids and invalid ids, subtree/root clears, 64-record eviction and update order, prompt/reset lifetimes and ordering, unsafe text and limits, bidi stripping, split writes, oversized OSC recovery, optional base64 padding, complete replacement, process exit, real SIGKILL followed by a shell prompt, saved-tab exclusion and fresh records after restart. These scripted reports do not substitute for the pending live-agent checks.

At Cyan's request, tried a scratch Claude configuration with a temporary HOME/config directory, `--bare` (no keychain reads), no user/project/local settings and no tools. It stopped at first-run theme onboarding, before a model turn or status report. Following Cyan's instruction to stop early on setup issues, no onboarding/authentication workarounds or further agents were attempted. The owned server and Claude process were stopped. Evidence: `/tmp/ship-status-live-smoke.py` and `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-status-live-v1kaw2dg/result.json`.

Live agents ran through `omlx launch` on the local model `Qwen3.6-35B-A3B-PARO`, inside an owned server with isolated config and state. A real attach client in a pty typed the input, and `ship pane get` was polled every 250 ms. Codex used a scratch `CODEX_HOME`, with the oMLX settings passed only on its command line; Pi used a scratch `PI_CODING_AGENT_DIR`. Each record change matched the screen:
- Codex 0.163.0, run with `-a on-request -s read-only` and asked to write a file: idle, then working, then blocked on the approval dialog, then working after approval, then idle once the file was written. On exit the record cleared and the pane exited 0. The blocked record carries no `kind` or `msg`, and a finished turn reports `idle` rather than `done`.
- Pi 1.3.0 asked the support query, got its reply and reported idle, then working through a shell-command turn, then done. Its record cleared on exit. Pi has no permission dialog by default.

Live evidence: `/tmp/ship-live-scripts/` and the `codex-watch.log` and `pi-watch.log` files in `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-live-8ibwt_sr/`.

Temporary protocol evidence: `/tmp/ship-status-probe.py` and `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-status-slice1-q2155f0f/results.json`. No permanent tests were added. Linux is unverified.

Implementation Rust physical lines under crate/vendor `src/`: 13,651 before, 14,081 after (+430). Added permanent test lines: 0. The scanner is 248 lines and the record module 75; the scanner estimate was low once the specified decoding, validation and bounded split-sequence handling were implemented.

## 2. Sidebar sections and display states

- [x] 2.1 Split the sidebar:
  - Move `ui/sidebar.rs` to `ui/sidebar/mod.rs` (the frame) and `ui/sidebar/tabs.rs` (the tree, keeping its `TreeState`).
  - Add the split, the divider row, `Scroll`, nested `Hit`s and section scrolling.
  - Route `Hit::Tabs`, `Hit::Agent` and `Hit::Split`, `Clicked::Split`, `Clicked::SidebarScroll` and `Gesture::Split` in `mouse.rs`.
- [x] 2.2 Add the agents section:
  - `ui/agents.rs` with `Display`, `Agent`, `display`, `rollup` and `list`, without unseen.
  - `ui/sidebar/agents.rs` with `Agents`, `icon` and `row`, and rollup icons on tab rows.
  - `rows` and the `Direction` resize in `keymap.rs`; `shift-j` and `shift-k` in `defaults.toml`; `Local.agents` replaced on reload; the new `draw` call in `draw.rs`.
- [ ] 2.3 Verify:
  - The four agents' rows and their tabs' icons match their screens within 2 seconds (SC-001's display part).
  - Thirty tabs and twenty reporting panes in a short terminal scroll independently and feel snappy (SC-008).
  - Dragging and `shift-j`/`shift-k` clamp at 10% and 90% with at least 3 rows per section.
  - Clicking an agent entry selects its pane.
  - Folding, tab dragging with its gap underline, the heading menu and right-click menus behave as before the move.
  - `rows = [["icon", "app"], ["title"]]` reloads live, and a misspelled token fails with line and column.

### Slice 2 verification checkpoint

Tasks 2.1 and 2.2 are complete. Task 2.3 stays open for Cyan's review: Codex and Pi ran live, and Claude Code and crush did not, as in slice 1.

On macOS, formatting, workspace Clippy with warnings denied, debug and release workspace builds, `ship-core` checks without default features and with `clap`, and `git diff --check` pass.

Every exercise used an owned server with isolated config and state and a real client in a pty. Its output was replayed through pyte to read the drawn screen, and SGR mouse sequences and keys were written to the client's pty.
- Scripted reporters in nested tabs showed `▲ claude blocked` above `ship › api`, ordered blocked and error, then working, then idle, in tree order. A reported `done` showed as idle, and a record without `app` showed the pane's name. Icons and colors matched G5 on rows and on tabs, and a folded tab kept its blocked rollup. A report reached the sidebar in about 0.1 s, including the reporter's own 100 ms poll.
- Clicking an entry's second line selected its pane; pressing an entry and moving a cell before release didn't. Right-clicking an entry opened no menu.
- At 30 rows, dragging the divider row to the top stopped at row 5 (3 tab rows) and to the bottom at row 26 (3 agent rows). `shift-k` twice moved it 2 rows up, and repeated `shift-j` and `shift-k` stopped at the same limits.
- With 30 tabs and 20 reporting panes at 20 rows, wheel scrolling over either section moved only that section and stopped once its last row reached the bottom. Each wheel step redrew in about 3 ms. `alt-left` afterwards brought the newly selected row back into view.
- Fold markers, dragging a tab with its gap underline (including the gap above the first row on the blank row), the row, heading-area and `≡` menus behaved as before the move.
- Saving `rows = [["icon", "app"], ["title"]]` reflowed the entries live. Saving `rows = [["icon", "apps"]]` kept the running rows and showed the error, and `ship config check` reported line 2, column 18 with the accepted tokens and exited 1.
- Codex 0.163.0 and Pi 1.3.0 ran live through `omlx launch` on `Qwen3.6-35B-A3B-PARO`, with scratch homes copied from slice 1, in tabs `work › codex` and `pi`. A watcher compared `ship pane get` with the drawn sidebar every 250 ms. Codex's row and both tabs' icons went idle, working, blocked with the approval dialog on screen, working after approval, and idle, each within one poll of the record. Pi's went idle, working with its spinner on screen, and idle on `done`. Both rows left on exit, and the placeholder returned.

Evidence: `/tmp/ship-slice2/` (rig, renderer, reporters, watcher) and `codex-watch.log` and `pi-watch.log` in `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-slice2-70fuquya/`. No permanent tests were added. Linux is unverified.

Implementation Rust physical lines under crate/vendor `src/`: 14,081 before, 14,642 after (+561). Added permanent test lines: 0.

## 3. Unseen, notifications and the jump key

- [ ] 3.1 Add the notifier:
  - `ui/attention.rs` with `Attention`, `Watched`, `Pending` and `Fired`.
  - `Observer::in_view`, used by `apply`'s `Screen` arm.
  - Focus reporting and `TerminalGuard::notify` in `terminal.rs`.
- [ ] 3.2 Wire it:
  - `attached` on attach, `observe` on each replica, `set_focused` on focus events, `viewed` after each redraw, and a `sleep_until` arm that calls `fire` and writes popups.
  - Unseen in `agents::display`, and `agents::next` for `ClientAgents::Next`.
  - `ClientAgents`, `delay_ms` and `popups` in `keymap.rs`; `alt-a` in `defaults.toml`.
- [ ] 3.3 Verify:
  - Attaching to five finished agents marks nothing unseen (SC-002).
  - A working-to-idle flip shorter than the delay marks nothing in 10 of 10 tries (SC-003's marking part).
  - With the pane in view, focused marks nothing and unfocused marks unseen until focus returns.
  - `alt-a` selects the blocked pane, then the unseen one, and does nothing when nothing needs the user.
  - With `popups = true`, Ghostty shows `<app> needs you · <path>: <msg>`, and escapes in a label or `msg` don't reach it (SC-005).

## 4. Sounds

- [ ] 4.1 Add the speaker and clips:
  - `sound.rs` with `Clip`, `Cue` and `Speaker`, and `pub mod sound` in the client's `lib.rs`.
  - The bundled `done.mp3` and `attention.mp3` with `ATTRIBUTION.md`, and `rodio` in the manifests.
  - `Notice`, `Sound`, `Sounds`, `NoticeSounds`, `Sounds::load` and `Sounds::resolve` in `keymap.rs`.
- [ ] 4.2 Wire `Local.speaker` and play each fired notification's resolved clip.
- [ ] 4.3 Verify:
  - Done and attention play their bundled sounds.
  - A `{ path = ... }` sound plays, including one relative to the config file and one under `~`.
  - A missing or non-audio file logs a warning naming it when the config is saved, and the bundled sound plays in its place.
  - `apps.droid.done = "off"` silences only Droid's finishes.
  - SC-003 and SC-004 with sound.
  - With no output device, or a forced open failure, the client attaches, works and logs once (SC-007).
  - The Linux build is reported unverified unless built there.

## 5. Documents and size

- [ ] 5.1 Document the user-visible behavior:
  - `docs/guide.md`: the agents section, display states, the jump key, notification settings, sounds, popups, focus, the OSC 133 requirement for exit recovery and the tmux limitation.
  - `README.md`: agent status in "What works today", `alt-a` in the default keys, and ALSA headers in the Linux install notes.
  - `GLOSSARY.md`: **Status record** and **Display state** (FR-027).
  - The product brief's milestone 3 (FR-026), and ALSA headers in `docs/agents/codebase.md`.
- [ ] 5.2 Verify:
  - `openspec validate program-status --type change --strict`, every link resolves, and the guide's config examples pass `ship config check`.
  - Implementation Rust and test lines reported separately against the count before this change.
- [ ] 5.3 When the PR opens, file the tracking issue for adopting the libghostty-rs program status and semantic prompt bindings and deleting the scanner, and issues for anything else left open.
