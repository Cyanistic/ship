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

- [ ] 1.1 Add the scanner and session events:
  - `vendor/ratatui-ghostty/src/program_status.rs` with `ProgramStatus`, `Found`, `Scanner` and `report`.
  - `SessionEvent::ProgramStatus` and `SessionEvent::PromptStart`, and `scan` ahead of `vt_write` in `session.rs`, writing the support reply through the session's writer.
  - `base64` in the workspace and vendored manifests.
- [ ] 1.2 Add the records:
  - `StatusRecord`, `ReportedState`, `BlockedKind`, `ROOT`, `Pane.program` and `root_status` in `ship-core`.
  - `records.rs` with `Records` (`apply`, `prompt`, `snapshot`), `within` and `disarm`.
  - `PaneChange::Program` and the `records` field in `pane.rs`; replacing on `Program` and clearing on `Exited` in `state.rs`; clearing in the saver's `blank`.
- [ ] 1.3 Verify:
  - The support query gets its reply, before the DA1 reply when both are written together.
  - Claude Code, Codex, Pi and crush 0.98.1 each show their root record in `ship pane get` through a turn and a permission dialog.
  - `id=build` and `id=build/test` sit beside the root; `id=` and `id=a//b` are dropped; a 65th id evicts the oldest.
  - `clear`, a subtree clear, `ESC c`, an exit, and `kill -9` followed by an OSC 133 prompt each remove what program-status's Record lifetimes says.
  - Escapes in `app` or controls in a base64 `msg` don't land; U+202E is stripped from a kept `msg`.
  - A report split across two writes still parses.
  - `ship tab list` shows `program`; the saved-tabs file doesn't.

## 2. Sidebar sections and display states

- [ ] 2.1 Split the sidebar:
  - Move `ui/sidebar.rs` to `ui/sidebar/mod.rs` (the frame) and `ui/sidebar/tabs.rs` (the tree, keeping its `TreeState`).
  - Add the split, the divider row, `Scroll`, nested `Hit`s and section scrolling.
  - Route `Hit::Tabs`, `Hit::Agent` and `Hit::Split`, `Clicked::Split`, `Clicked::SidebarScroll` and `Gesture::Split` in `mouse.rs`.
- [ ] 2.2 Add the agents section:
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
