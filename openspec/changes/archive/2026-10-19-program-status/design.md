# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Cyan locked all three papers on 2026-10-18; the architecture paper's decision 3 was amended during program review the same day and relocked. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P5, FR-001 to FR-028, success criteria SC-001 to SC-008.
- [`design/architecture.md`](design/architecture.md): shape A, a temporary byte scanner ahead of libghostty, and key decisions 1 to 8.
- [`design/program.md`](design/program.md): the planned files, types and signatures, gates G1 to G6, the build order and the deviation log.

## Context

Someone running several coding agents in Ship has to visit every pane to learn which one needs an answer or has finished. Claude Code, Codex, Pi and crush 0.98.1 already report their state through OSC 7501, the Program Status Protocol (revision 0.4). This change keeps those reports on each pane, shows them in the sidebar and on tabs, and notifies the user with sounds and optional popups when an agent finishes or needs them. Ship owns no per-agent knowledge. It covers milestone 3 of the product brief.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (see which agent needs me), P2 (hear and see when an agent finishes or needs me), P3 (jump to what needs me), P4 (make it sound and look the way I want) and P5 (arrange the sidebar).

**Non-goals:** see the product paper's Non-goals. The main ones:

- Agent hooks, screen-reading detection and per-agent knowledge.
- Showing child records (`id`) or `progress` in the sidebar and tabs. The pane data still carries every record.
- Saving status across a server restart, and telling a client about agents that finished while it wasn't attached.
- OS notifications sent by Ship itself. Popups go through the outer terminal.
- Status from agents inside tmux inside Ship.

## Decisions

- **A temporary scanner ahead of `vt_write`** (architecture decisions 1 and 2). It finds OSC 7501 reports, OSC 133 prompt starts and RIS in each PTY batch, answers the support query before libghostty can answer a DA1 in the same batch, and emits reports shaped like upstream's `GhosttyTerminalProgramStatus`. The whole scanner goes once libghostty-rs exposes the program status and semantic prompt callbacks (decision 3, amended).
- **The server owns the record lifecycle; clients own attention** (decision 4). The pane task applies revision 0.4's rules and publishes the pane's full record set; `ServerState` stores it on `Pane` and clears it on exit; the saver blanks it.
- **Records are `Pane.program`, a map from id to record** (program G1). The root record's key is `ROOT`, the empty string, which the id grammar can't produce. At most 64 records per pane (G6).
- **Changes are found by comparing replicas on the client** (decision 5). No protocol additions; protocol version unchanged.
- **Reports are checked once, where they're parsed.** The scanner applies the specification's limits and control-character rules; the server only strips bidirectional formatting characters.
- **The sidebar becomes a frame with two sections**: tabs, keeping its tree state so it can scroll, and agents, with a draggable client-local split.
- **Sound files become clips when the config loads.** A bad file falls back to the bundled clip for its slot with a warning; the audio thread only plays clips (decision 6).
- **Popups are OSC 9 through the client's terminal** (decision 7).
- **Typed config** (decision 8, program G2 to G4): `[client.agents]` with `rows`, `delay_ms` and `popups`, `[client.agents.sounds.default]` and `[client.agents.sounds.apps.<app>]`, `client.agents.next` on `alt-a`, and `client.sidebar.resize` taking up and down for the split.

## Build order

Program paper, "Build order":

1. Records on panes.
2. Sidebar sections and display states.
3. Unseen, notifications and the jump key.
4. Sounds.
5. Documents.

## Risks / Trade-offs

Architecture paper, "Risks and unknowns". The ones to watch:

- The scanner and Ghostty could read a malformed report differently until the swap.
- Sequences nested in DCS or other strings, such as tmux passthrough, are read as top-level until the scanner is deleted.
- rodio links ALSA on Linux, a new build dependency, unverified until built there.
- Terminals inside nested multiplexers may report focus unreliably.

Estimated size: about 920 lines of implementation Rust.

## Migration Plan

None. Records aren't saved, and nothing on the wire changes beyond a new optional `program` field on panes.

## Open Questions

None.
