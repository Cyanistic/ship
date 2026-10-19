# Proposal

This is the planning artifact for a locked plan. Cyan locked the product, architecture and program papers in `design/` on 2026-10-18. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

Someone running several coding agents in Ship has to visit every pane to learn which one needs an answer or has finished. Claude Code, Codex, Pi and crush 0.98.1 already report their own state through OSC 7501, the Program Status Protocol (revision 0.4). The product brief's milestone 3, "Know which agent needs attention", asks Ship to use it.

## What Changes

- **Status records on panes.** Every pane answers the protocol's support query ahead of any other reply. It keeps what its program reports as `program`, a map from record id to record with the root record under `""`, following revision 0.4's rules for ids, nesting and `clear`.
  - At most 64 records per pane, evicting the one updated longest ago.
  - Reports that break the specification's limits or carry control characters are dropped. Bidirectional formatting characters are stripped from `title` and `msg`.
  - Records go when the program exits or the terminal is reset. Working and blocked records also go when the shell starts a new prompt through OSC 133.
  - `ship pane get`, `ship tab list` and the attach replica carry them. The saved tabs don't.
- **Display states.** Each client derives `blocked`, `error`, `working`, `unseen` or `idle` for every pane with a root record. Each state has its own icon and color. A tab shows the highest-ranked state among its panes and descendant tabs.
- **The sidebar gets two sections.** Tabs on top, agents below, split 50/50, with a divider the user can drag or move with `shift-j` and `shift-k` in resize mode. Each section scrolls with the wheel on its own. Agent rows are laid out from configurable lines of tokens. Clicking a row selects its pane.
- **Notifications.** A pane entering `blocked` or `error` notifies for attention. A pane going from `working` to `idle` or `done` notifies as done and marks the pane unseen at once. A notification fires only after its state holds for the delay (1 second by default), and never while the pane is in view in a focused window. Attaching starts with nothing unseen.
- **Sounds and popups.** Bundled done and attention sounds play on the client's machine. Each can be turned off or replaced by a file, by default or per reported `app`. Optional popups go to the outer terminal as OSC 9.
- **Jump key.** `alt-a` (`client.agents.next`) selects the next blocked or errored pane, then the next unseen one, and cycles every agent when none of those exist.
- **Config.** `[client.agents]` with `rows`, `delay_ms` and `popups`, and `[client.agents.sounds.default]` and `[client.agents.sounds.apps.<app>]`. `client.sidebar.resize` accepts `up` and `down`.

Ship owns no per-agent knowledge: no detection rules, no hooks and no table of agent names.

Not in this change (the product paper lists them all):
- Agent hooks, screen-reading detection and a command for reporting status on a program's behalf.
- Child records and `progress` in the sidebar and on tabs. The pane data still carries them.
- Saving status across a server restart, or telling a client what finished while it wasn't attached.
- OS notifications sent by Ship itself.
- Status from agents inside tmux inside Ship.
- Remembering a dragged split or sidebar width across client launches.
- The upstream libghostty-rs binding, which is on hold.

## Capabilities

### New Capabilities

- `program-status`:
  - The support query and the record rules.
  - The record limit, clean text and record lifetimes.
  - Records in the pane data.
- `agent-attention`:
  - Display states, icons and the tab rollup.
  - The agents section's rows, the jump action and row clicks.
  - Notifications, unseen marks, focus, sounds and popups.

### Modified Capabilities

- `pane-programs`: inspection reports a pane's status records.
- `session-restore`: status records aren't saved.
- `terminal-client`: the sidebar splits into a tabs section and an agents section, and tab rows show rollup icons.
- `mouse-input`: the wheel scrolls the sidebar section under it, clicking an agent row selects its pane, and the split between the sections can be dragged.
- `keymap`: `alt-a` and `client.agents.next`; `client.sidebar.resize` takes `up` and `down`, bound to `shift-k` and `shift-j` in resize mode.
- `config-file`: the `[client.agents]` settings.

## Impact

- `vendor/ratatui-ghostty`: a temporary `program_status` scanner ahead of `vt_write`, and two session events. `base64` becomes a dependency. The scanner goes once libghostty-rs exposes the program status and semantic prompt callbacks.
- `ship-core`: `StatusRecord`, `ReportedState`, `BlockedKind`, `ROOT` and `Pane.program`. The new field is optional in the JSON, so the protocol version is unchanged.
- `ship-server`: a new `records` module, `PaneChange::Program`, clearing on exit in `state.rs` and in the saver's `blank`.
- `ship-client`:
  - New `ui/agents.rs`, `ui/attention.rs` and `sound.rs`.
  - `ui/sidebar.rs` becomes `ui/sidebar/` with `mod.rs`, `tabs.rs` and `agents.rs`.
  - Focus reporting and OSC 9 in `terminal.rs`, and `Observer::in_view`.
  - New settings and actions in `keymap.rs`, and bundled sounds with attribution.
  - `rodio` becomes a dependency, which needs ALSA headers to build on Linux.
- Docs: the guide, the README, the glossary (**Status record**, **Display state**), the product brief's milestone 3 and `docs/agents/codebase.md`.
- Estimated at about 920 lines of implementation Rust. Measured counts go in [tasks.md](tasks.md).
