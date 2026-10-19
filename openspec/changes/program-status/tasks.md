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

Implementation Rust under `crates/*/src/`, counted by cloc as code lines and leaving out `vendor/`: 9,614 before this change, 10,730 now (+1,116, after slice 5 replaced `Display::label` with strum's `IntoStaticStr`). The vendored widget gained another 259. The physical-line count used by earlier slices, crates and vendor together, is 13,651 before and 15,295 now. Test lines: none under `crates/`, 988 code lines under `vendor/`, unchanged; this change added none.

## 2. Sidebar sections and display states

- [x] 2.1 Split the sidebar:
  - Move `ui/sidebar.rs` to `ui/sidebar/mod.rs` (the frame) and `ui/sidebar/tabs.rs` (the tree, keeping its `TreeState`).
  - Add the split, the divider row, `Scroll`, nested `Hit`s and section scrolling.
  - Route `Hit::Tabs`, `Hit::Agent` and `Hit::Split`, `Clicked::Split`, `Clicked::SidebarScroll` and `Gesture::Split` in `mouse.rs`.
- [x] 2.2 Add the agents section:
  - `ui/agents.rs` with `Display`, `Agent`, `display`, `rollup` and `list`, without unseen.
  - `ui/sidebar/agents.rs` with `Agents`, `icon` and `row`, and rollup icons on tab rows.
  - `rows` and the `Direction` resize in `keymap.rs`; `shift-j` and `shift-k` in `defaults.toml`; `Local.agents` replaced on reload; the new `draw` call in `draw.rs`.
- [x] 2.3 Verify:
  - The four agents' rows and their tabs' icons match their screens within 2 seconds (SC-001's display part).
  - Thirty tabs and twenty reporting panes in a short terminal scroll independently and feel snappy (SC-008).
  - Dragging and `shift-j`/`shift-k` clamp at 10% and 90% with at least 3 rows per section.
  - Clicking an agent entry selects its pane.
  - Folding, tab dragging with its gap underline, the heading menu and right-click menus behave as before the move.
  - `rows = [["icon", "app"], ["title"]]` reloads live, and a misspelled token fails with line and column.

### Slice 2 verification checkpoint

Tasks 2.1 to 2.3 are complete. Cyan accepted task 2.3 on 2026-10-18 after daily use through slice 3: Claude Code and Pi showed the right states live, and `rows = [["icon", "app"], ["title"]]` saved to Cyan's config redrew the entries without a restart. Codex ran live only in the scripted runs below. crush was never run. Snappiness is judged by Cyan's use.

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

- [x] 3.1 Add the notifier:
  - `ui/attention.rs` with `Attention`, `Watched`, `Pending` and `Fired`.
  - `Observer::in_view`, used by `apply`'s `Screen` arm.
  - Focus reporting and `TerminalGuard::notify` in `terminal.rs`.
- [x] 3.2 Wire it:
  - `attached` on attach, `observe` on each replica, `set_focused` on focus events, `viewed` after each redraw, and a `sleep_until` arm that calls `fire` and writes popups.
  - Unseen in `agents::display`, and `agents::next` for `ClientAgents::Next`.
  - `ClientAgents`, `delay_ms` and `popups` in `keymap.rs`; `alt-a` in `defaults.toml`.
- [x] 3.3 Verify:
  - Attaching to five finished agents marks nothing unseen (SC-002).
  - A working-to-idle flip shorter than the delay marks nothing in 10 of 10 tries (SC-003's marking part).
  - With the pane in view, focused marks nothing and unfocused marks unseen until focus returns.
  - `alt-a` selects the blocked pane, then the unseen one, and cycles every agent when nothing needs the user.
  - With `popups = true`, Ghostty shows `<app> needs you · <path>: <msg>`, and escapes in a label or `msg` don't reach it (SC-005).

### Slice 3 verification checkpoint

Tasks 3.1 to 3.3 are complete. Cyan accepted task 3.3 on 2026-10-18 after seeing a popup arrive as a desktop notification in foot on Linux, from a release client running on the Mac over SSH with `popups = true`. Ghostty itself was not tried, and which notice it was isn't recorded; the popup text and escape stripping were checked from the client's output.

On macOS, formatting, workspace Clippy with warnings denied, debug and release workspace builds, `ship-core` checks without default features and with `clap`, strict OpenSpec validation and `git diff --check` pass.

A temporary script ran 12 checks through an owned server with isolated config and state, a real client in a pty, and scripted reporters. It read popups as OSC 9 sequences in the client's output and icons from a pyte replay, then stopped everything it started. All 16 passed. The run was repeated after the unseen mark moved to the moment of the change the selected agent's entry gained its highlight, idle turned green, and tab icons moved to a left gutter:
- Five agents that went working and then idle or done before the client attached showed `○ idle`, with no unseen mark and no popup (SC-002).
- Ten working, idle, working flips inside the 1 s delay left no popup, and no `◆` once the pane was working again (SC-003).
- A pane in an unviewed tab that went working to idle showed a cyan `◆` on its row and tab within 0.4 s, before any popup, and wrote `q finished · quick` about 1.1 s after the report. Selecting the pane cleared both.
- With the pane in view and focused, a blocked report and a finish fired nothing and marked nothing. After a focus-out sequence, a finish wrote its popup and marked the pane unseen, and a focus-in sequence cleared the mark.
- From another tab, `alt-a` selected the blocked pane and then the unseen one. With nothing blocked or unseen, it went through all seven agents once, starting with the working one, and wrapped back to it. Cyan confirmed the fallback in a release client.
- Claude's blocked report in `ship › api` wrote exactly `claude needs you · ship › api: Apply these changes?`.
- A tab name and a pane name containing ESC and BEL reached the popup without them. A report with ESC in `msg` was dropped by the scanner, as in slice 1.
- Saving `popups = false` and `delay_ms = 300` reloaded live: the next finish marked unseen in about 0.2 s and wrote no popup.
- Selecting a pane put its agent entry, and only that entry, on a dark gray band with default-colored text. Selecting its tab or another tab's shell left every entry unbanded. On the band, the idle icon and `state` text drew green on dark gray.
- Every tab icon and agent icon sat in column 1, whatever the tab's depth or label length. A separate script (`first.py`) showed the icons on a fresh attach with no selection, a click on a row's gutter selecting that tab's pane with the band across the gutter, the fold marker folding and unfolding, and two drags landing where their drop lines pointed.
- A fresh attach started with no unseen panes.
- `ship config check` rejected `delay_ms = "soon"` at line 2, column 12, and accepted a custom `alt-y` binding to `client.agents.next`.

Evidence: `/tmp/ship-slice3/` (rig, checks, reporter) and the run directory `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-slice3-z1z9g8f1/`. Not checked: a zoomed tab's hidden panes (in view comes from the server's narrowed geometry, which slice 3 didn't change), and live agents, since Codex's and Pi's transitions were already recorded in slices 1 and 2. No permanent tests were added. Linux is unverified.

Implementation Rust physical lines under crate/vendor `src/`: 14,653 before, 15,055 after (+402). Added permanent test lines: 0.

## 4. Sounds

- [x] 4.1 Add the speaker and clips:
  - `sound.rs` with `Clip`, `Cue` and `Speaker`, and `pub mod sound` in the client's `lib.rs`.
  - The bundled `done.wav` and `attention.wav` with `ATTRIBUTION.md`, and `rodio` in the manifests.
  - `Notice`, `Sound`, `Sounds`, `NoticeSounds`, `Sounds::load` and `Sounds::resolve` in `keymap.rs`.
- [x] 4.2 Wire `Local.speaker` and play each fired notification's resolved clip.
- [x] 4.3 Verify:
  - Done and attention play their bundled sounds.
  - A `{ path = ... }` sound plays, including one relative to the config file and one under `~`.
  - A missing or non-audio file logs a warning naming it when the config is saved, and the bundled sound plays in its place.
  - `apps.droid.done = "off"` silences only Droid's finishes.
  - SC-003 and SC-004 with sound.
  - With no output device, or a forced open failure, the client attaches, works and logs once (SC-007).
  - The Linux build is reported unverified unless built there.

### Slice 4 verification checkpoint

Tasks 4.1 to 4.3 are complete. Cyan accepted task 4.3 on 2026-10-18 after hearing the sounds play on the Mac while the scripted runs below were going; which clip played at each moment was not confirmed by ear.

On macOS, formatting, workspace Clippy with warnings denied, debug and release workspace builds and strict OpenSpec validation pass. The Linux build, which needs ALSA headers for cpal, is unverified, as is playback on Linux. On Linux, libasound can also print to stderr on its own, outside rodio's `tracing` feature; whether that reaches the client's screen is unchecked.

A temporary script ran 13 checks through the slice 3 rig: an owned server, isolated config and state, a real client in a pty and scripted reporters. It took a sound as played when macOS listed an `audio-out` power assertion created for the client's PID. All 13 passed.
- An idle client held no audio output, at 0.0% CPU per second over five seconds. A first version that kept the output open held the assertion the whole time, which blocks idle sleep, at 0.6–0.8% CPU; see the deviation log.
- A finish and a blocked report in an unviewed tab each played, and the output was released afterwards.
- `done = { path = "sounds/ding.wav" }` beside the config and `{ path = "~/.cache/…" }` each played without a warning.
- Saving a config with a missing `done` file and a text file as `attention` logged one warning naming each, and the next finish still played.
- With `apps.droid.done = "off"`, Droid finishing played nothing but still wrote its popup, Droid's blocked report played, and Claude finishing played.
- Five working, idle, working flips inside the delay played nothing (SC-003), and a blocked report in view and focused played nothing (SC-004).
- `ship config check` accepted a config whose sound files were missing or not audio and printed nothing, and rejected `{ builtin = "ding" }` at line 2, column 20, naming `done` and `attention`.
- The slice 3 script's 16 checks still passed.

A second script, with the device open temporarily forced to fail and reverted afterwards, had three finishes write their popups, typing reach the shell, and the client log one `cannot open audio output for sounds` warning in all (SC-007).

Evidence: `/tmp/ship-slice4/` (`s4.py`, `nodev.py`, test sounds) with the run directory `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-slice3-649ksyw7/`. Not checked: which clip played, since the assertion shows only that the client played something; Codex's `idle` turn ends sounding through a live agent; and a device unplugged mid-session.

Implementation Rust physical lines under crate/vendor `src/`: 15,055 before, 15,299 after (+244). Added permanent test lines: 0.

## 5. Documents and size

- [x] 5.1 Document the user-visible behavior:
  - `docs/guide.md`: the agents section, display states, the jump key, notification settings, sounds, popups, focus, the OSC 133 requirement for exit recovery and the tmux limitation.
  - `README.md`: agent status in "What works today", `alt-a` in the default keys, and ALSA headers in the Linux install notes.
  - `GLOSSARY.md`: **Status record** and **Display state** (FR-027).
  - The product brief's milestone 3 (FR-026), and ALSA headers in `docs/agents/codebase.md`.
- [x] 5.2 Verify:
  - `openspec validate program-status --type change --strict`, every link resolves, and the guide's config examples pass `ship config check`.
  - Implementation Rust and test lines reported separately against the count before this change.
- [ ] 5.3 When the PR opens, file the tracking issue for adopting the libghostty-rs program status and semantic prompt bindings and deleting the scanner, and issues for anything else left open.

### Slice 5 checkpoint

Tasks 5.1 and 5.2 are complete and wait on Cyan's review. Task 5.3 waits on the PR.

- `docs/guide.md` gained an Agents section (entries, icons, rollup, unseen marks and focus, `alt-a`, the split), Notifications and sounds, When status is wrong or missing, and Agent settings under Configuration.
- `client.sidebar.resize` in the guide no longer says only `left` and `right` are accepted.
- The OSC 133 paragraph gives a two-line zsh hook. A temporary probe ran it in an owned server: an agent that reported `working` and died by `kill -9` before the prompt was cleared with the hook and stayed `working` without it.
- `README.md` changed beyond the task's three items: the warning banner and the overview said agent status wasn't built, and both now say agents can't drive the workspace yet. The mouse sentence names clicking agents and the second divider, and the resize-mode row gained `shift-j` and `shift-k`.
- The product brief's milestone 3 names OSC 7501 as the reporting interface and moves hooks and screen detection to later (FR-026). Its "Pi lifecycle" completion line, the deferred "Every agent integration" row and the open "Agent reporting protocol" decision changed to match.
- `GLOSSARY.md` gained **Status record** and **Display state** (FR-027), and `docs/agents/codebase.md` the Linux ALSA headers.

Checks on macOS: strict OpenSpec validation passes; every relative link and in-page anchor in the five changed documents resolves, and the two new external links return 200; all 8 TOML blocks in the guide and README pass `ship config check`, which rejects a misspelled token at its line and column. This slice changed no Rust, so formatting, Clippy and builds were not rerun after the last commit, where they passed. The ALSA package names and the Linux build are unverified.

Evidence: `/tmp/ship-slice5/` (`osc133.sh`, the extracted TOML blocks).

Implementation Rust physical lines under crate/vendor `src/`: 13,651 before this change, 15,304 now (+1,653, including the 5 lines of the Ghostty log bridge). Test lines: 1,147 before and after; this change added none.
