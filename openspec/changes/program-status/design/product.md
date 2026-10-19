# Program status

Status: Locked on 2026-10-18 with Cyan's approval (Plannotator review: "LGTM"). Corrected the same day at Cyan's request to name `ship tab list`, which prints every pane, in place of `ship pane list`, which doesn't exist. Locking doesn't authorize implementation; Cyan requests that separately.

## Goal

Someone running several coding agents in Ship has to visit every pane to learn which one needs an answer or has finished. This change makes Ship show each agent's state in the sidebar and on its tab, and notify the user when an agent finishes or needs them. It covers milestone 3 of the product brief, "Know which agent needs attention".

Agents report their own state through OSC 7501, the Program Status Protocol (revision 0.4, https://www.superlogical.com/rex/docs/build/program-status). In live tests on 2026-10-17, Claude Code, Codex, Pi and crush 0.98.1 all emitted it. They reported working, blocked (including permission dialogs), idle and done accurately, and none of it depended on screen rules. Ship owns no per-agent knowledge: no detection rules, no hooks and no table of agent names. Once another agent adopts the protocol, it appears in Ship without a Ship update.

## User stories

### P1: See which agent needs me

A user runs Claude in one tab and Codex in a child tab of another, and works in a third.

- **Given** Claude shows a permission dialog, **when** the user looks at the sidebar, **then** Claude's row in the agents section shows the blocked icon and state, together with its tab path. Claude's tab, and every collapsed tab above it, shows the same blocked icon.
- **Given** Codex is mid-turn, **then** its row and its tab show working.
- **Given** a pane running a plain shell, or an agent that hasn't reported anything yet, **then** that pane has no row and its tab has no icon from it.
- **Given** the agent in a pane exits or crashes and the shell prints its next prompt, **then** the pane stops showing working or blocked.

Tested independently by running each of the four agents in Ship through a turn, a permission dialog and an exit, and comparing the sidebar with the agent's screen.

### P2: Hear and see when an agent finishes or needs me

- **Given** Claude is working in a tab the user isn't viewing, **when** it finishes, **then** the done sound plays and Claude's row and tab show unseen. The marker clears once the user views the pane.
- **Given** Codex asks for approval while the user is in another tab, **then** the attention sound plays.
- **Given** the agent's pane is in view and the client's window has focus, **when** it finishes or asks for approval, **then** nothing plays and no unseen marker appears.
- **Given** the agent's pane is in view but the window is unfocused (the user is on another monitor), **then** the sound plays and the pane is marked unseen until the window regains focus with the pane in view.
- **Given** an agent flips from working to idle and back within the delay, **then** nothing plays.
- **Given** five agents finished while no client was attached, **when** a client attaches, **then** nothing plays and none is marked unseen.
- **Given** popups are turned on, **then** each notification also appears as an outer-terminal notification such as `claude needs you · ship › api: Apply these changes?`.

Tested independently with two tabs, two terminal windows on separate monitors, and scripted state changes.

### P3: Jump to what needs me

- **Given** one agent is blocked and another finished unseen, **when** the user presses the jump key, **then** the client selects the blocked agent's pane. Pressing it again selects the unseen one.
- **Given** an agent row, **when** the user clicks it, **then** the client selects that pane.

Tested independently by pressing the key and clicking rows across nested tabs.

### P4: Make it sound and look the way I want

- **Given** `done = { path = "~/sounds/ding.mp3" }`, **then** that file plays when an agent finishes. **Given** `done = { builtin = "attention" }`, **then** the bundled attention sound plays instead.
- **Given** an override for the reported app `droid` setting `done = "off"`, **then** Droid finishing plays nothing while other agents still do.
- **Given** a custom file that is missing or can't be decoded, **then** the bundled default plays and the client log names the file.
- **Given** a machine with no audio device, **then** terminal work is unaffected.
- **Given** agent rows configured as `[["icon", "app"], ["title"]]`, **then** rows show those values, and a token with no value disappears.
- **Given** a misspelled token or builtin sound name, **then** loading the config fails with the line, column and the accepted names.

Tested independently by editing the config while a client is attached and triggering state changes.

### P5: Arrange the sidebar

- **Given** the sidebar is shown, **then** tabs fill the top section and agents the bottom, split 50/50 by default, and each scrolls on its own when it overflows.
- **Given** the user drags the divider between them, **then** the split follows, staying within 10–90% with at least 3 rows for each section, until the client exits.

Tested independently with 30 tabs and 20 reporting panes in a short terminal.

## Non-goals

- Agent hooks, a command for adapters to report status on a program's behalf, screen-reading detection and Herdr's detection manifests. The record and the command can be added later, unchanged, if an agent Cyan uses never adopts the protocol.
- Per-agent knowledge in Ship: no icons, names or defaults keyed to particular agents.
- Child records beyond the root record (`id`), in the sidebar and tabs. The pane data still carries them.
- Showing `progress` percentages.
- Herdr's layout language beyond lines of tokens: per-token styles, conditional colors, per-agent layouts and custom `$name` tokens.
- Remembering a dragged sidebar split, or the sidebar width, across client launches.
- Indexed jump keys (Herdr's `focus_agent` 1–9).
- The agent name on pane borders. A small follow-up.
- In-app toasts and OS notifications sent by Ship itself. Popups go through the outer terminal.
- Telling a client about agents that finished while it wasn't attached.
- Coordinating notifications across several clients: each client notifies on its own.
- Saving status across a server restart.
- Status from agents running inside tmux inside Ship. tmux doesn't pass the protocol on (its maintainer declined on 2026-10-09), so these agents report nothing. The guide documents this.
- Waiting on a state change from the CLI. That belongs to milestone 4.
- Contributing the libghostty-rs binding upstream. It's on hold. A tracking issue for adopting the binding is opened once this change merges.

## Functional requirements

### Reports

A **status record** is what a pane's program last reported about itself through OSC 7501: a state (`idle`, `working`, `done`, `blocked` or `error`), with an optional `kind`, `app`, `title`, `msg`, `progress` and `id`.

- **FR-001** Every pane MUST answer the protocol's support query so that programs which ask first, such as Claude Code and Pi, start reporting. The answer MUST reach the program before any other reply the same output provokes.
- **FR-002** The server MUST keep each pane's status records as reported, following revision 0.4's rules for `id`, nesting and `clear`. It MUST ignore keys and states it doesn't know rather than rejecting the report.
- **FR-003** Before a record is kept, its text fields MUST have control characters removed and be cut to a fixed maximum length.
- **FR-004** A pane's records MUST be removed when its program exits and when the terminal is fully reset. Working and blocked records MUST be removed when the shell reports a new prompt through OSC 133.
- **FR-005** Records MUST be part of the pane data that clients and `ship pane get` and `ship tab list` receive. They MUST NOT be saved with the saved tabs.
- **FR-006** This change MUST work with the terminal engine Ship uses today, without waiting on an upstream release.

### What a client shows

Each client derives a **display state** for each pane with a root record: `blocked`, `error`, `working`, `unseen` or `idle`.

- **FR-007** A reported `done` MUST display as `idle`, unless the client itself saw that pane change from `working` to `idle` or `done`. That change MUST make the pane `unseen` at once, without waiting for the notification delay, if the pane wasn't in view in a focused window.
- **FR-008** A pane is **in view** when the client draws it in the current tab, taking zoom into account. An `unseen` pane MUST become `idle` once it is in view while the client's window has focus. A terminal that never reports focus MUST be treated as focused.
- **FR-009** A client attaching MUST start with no `unseen` panes.
- **FR-010** Each display state MUST have its own icon, distinguishable by both shape and color. The same icons MUST be used in agent rows and on tabs.
- **FR-011** A tab MUST show the icon of the highest-priority display state among its own panes and all its descendant tabs, in the order `blocked` and `error`, `unseen`, `working`, `idle`. A tab with none MUST show no icon. The icon MUST sit in the same column on every tab row, whatever the row's depth or label length.

### The sidebar

- **FR-012** The sidebar MUST show a tabs section on top and an agents section below, with a divider between them, whenever the sidebar is shown. With no pane reporting, the agents section MUST show a "no agents" placeholder.
- **FR-013** The split MUST default to half the sidebar's height. Dragging the divider MUST change it, keeping 10–90% and at least 3 rows per section. A dragged split MUST last until the client exits.
- **FR-014** Each section MUST scroll independently when its content overflows.
- **FR-015** The agents section MUST list one row per pane with a root record, ordered by display state priority (FR-011), and by tree order (depth-first through the tabs) within each state. The row of the selected pane MUST be highlighted, and no other.
- **FR-016** An agent row MUST be laid out from configurable lines of tokens, drawn from `icon`, `app`, `state`, `tab`, `pane`, `title` and `msg`. `tab` MUST show the path of tab labels from the top level, shortened from the left when it doesn't fit. A token with no value MUST disappear, and so MUST a line left empty. The default MUST be `[["icon", "app", "state"], ["tab"]]`.
- **FR-017** When a record has no `app`, the `app` token MUST show the pane's label as the sidebar and pane borders show it today.
- **FR-018** Clicking an agent row MUST select its pane.
- **FR-019** A client action MUST select the next pane that needs the user: `blocked` and `error` panes first, then `unseen`, in agent list order, continuing past the currently selected pane. With nothing that needs the user, it MUST go through every listed agent the same way; with no agents, it MUST do nothing. It MUST have a default key.

### Notifications

- **FR-020** A pane entering `blocked` or `error` MUST trigger an **attention** notification. A pane changing from `working` to `idle` or `done` MUST trigger a **done** notification.
- **FR-021** A notification MUST fire only after the new state has held for the configured delay, 1 second by default. It MUST NOT fire if the pane is in view in a focused window.
- **FR-022** Notification sounds MUST play on the client's machine. Each of `done` and `attention` MUST be configurable as a bundled sound by name, a sound file by path, or off, and MUST be overridable for a reported `app` value. Paths MUST resolve relative to the config file, with `~` expanded. Sound MUST be on by default, with distinct bundled sounds for done and attention.
- **FR-023** A sound file that is missing or can't be decoded MUST fall back to the bundled default for that notification, and the client MUST log a warning naming the file. A missing or failing audio device MUST NOT affect terminal work.
- **FR-024** When popups are turned on, each notification MUST also be sent to the outer terminal as a one-line notification: `<app> needs you · <tab path>` for attention or `<app> finished · <tab path>` for done, followed by `: <msg>` when the record has one. Popups MUST be off by default.
- **FR-025** An unknown token, builtin sound name or other invalid status setting MUST fail config loading with the line, column and accepted values, like every other config error. Status settings MUST reload with the rest of the client config.

### Documents

- **FR-026** The product brief's milestone 3 MUST be amended. The reporting interface becomes OSC 7501. Hooks and screen-based detection move to later, needed only for an agent Cyan uses that never adopts the protocol.
- **FR-027** The glossary MUST gain **Status record** and **Display state**, defined as above.
- **FR-028** The bundled sounds are `confirmation_002` for done and `error_007` for attention, from Kenney's Interface Sounds pack, which is dedicated to the public domain under CC0 1.0. An attribution file credits the pack anyway.

## Success criteria

- **SC-001** Claude Code, Codex, Pi and crush 0.98.1 or later, each run inside Ship through a turn, a permission dialog (where the agent has one), an interrupt and an exit, show the state matching the agent's screen within 2 seconds at every step, on macOS. Linux is unverified until run there.
- **SC-002** Attaching to a server with 5 idle or done agents plays no sound and marks nothing unseen.
- **SC-003** A working-to-idle flip shorter than the delay plays nothing in 10 out of 10 tries.
- **SC-004** With the agent's pane in view, finishing plays nothing while the window is focused, and plays the done sound while it isn't.
- **SC-005** A report whose `msg`, `app` or `title` contains escape sequences changes neither the sidebar's layout nor the outer terminal's state.
- **SC-006** After `kill -9` of a working agent inside a shell with OSC 133 prompt marks, the pane stops showing working by the next prompt.
- **SC-007** With no audio device, or with a sound path that doesn't exist, a client attaches and works normally, and the log names the problem.
- **SC-008** With 20 reporting panes, the sidebar scrolls and updates without a noticeable delay, judged by feel.

## Settled in review

Cyan accepted these proposals by approving the paper without changes.

- **Default jump key (FR-019):** `alt-a`, which the default keymap leaves free.
- **Resizing the split from the keyboard (FR-013):** `shift-j` and `shift-k` in resize mode move the divider down and up, next to the `shift-h` and `shift-l` width keys.
- **`app` fallback (FR-017):** the pane's existing label.
