# Session/tab roundtrip

Status: Locked again on 2026-10-05 after Cyan approved the [amendments](#amendments-2026-10-05) in Plannotator with “lgtm”. Text they supersede is kept and marked. Previously: Locked after revised product review. Cyan approved the logical-pane scope in Plannotator with “looks good to me!” The previously approved session/tab-only scope is superseded by this paper; architecture remains open and must be reconciled with the added pane requirements. This paper defines observable behavior, not architecture or implementation. The broader [product brief](../../../../../docs/product-brief.md) remains the roadmap. Session text in this paper is superseded by the drop-sessions change (2026-10-08), which replaces sessions with top-level tabs; it is kept as written and marked where a requirement defines sessions.

## Goal

Give Cyan a runnable way to organize server-owned sessions, recursive tabs and logical panes through the CLI, and observe that structure from independently attached clients. Prove entity CRUD, tab movement, recursive removal, pane selection and connection recovery before adding real terminals or pane layouts.

## User stories

### P1: Organize work through explicit targets

**Given** a running server, **when** I create a session, tabs and logical panes, **then** creation returns each created entity, including its stable ID, as machine-readable data that I can use in subsequent commands.

**Given** a session with tabs, **when** I create a child tab, rename a tab, change its parent or reposition it before/after a sibling, **then** inspection shows the requested hierarchy and sibling order. Creation appends by default.

**Given** a tab, **when** I create, inspect, rename or remove a logical pane, **then** the operation targets that pane by its stable ID without requiring a terminal process or layout. A pane must have a tab parent and cannot contain tabs or panes. Duplicate pane names are allowed, as with tabs.

**Given** a tab with descendants and panes, **when** I remove it, **then** the entire subtree, including panes owned by every removed tab, disappears. Removing a session removes all its tabs and panes. Removing one pane leaves its tab, sibling panes and child tabs intact.

**Given** a tab subtree containing panes, **when** I move that subtree, **then** its panes move with their owning tabs and retain their IDs.

**Given** an invalid target, duplicate session name or attempted move into the moving tab's own subtree, **when** I submit the operation, **then** it fails clearly without partially changing the structure.

Independent verification: use CLI commands against a running server; extract creation IDs with `jq`, inspect resulting structure/order, and compare state before and after rejected operations. No attached observer is required.

### P1: Observe shared structure without sharing selection

**Given** two clients attached to the same session, **when** I edit its structure through the CLI, **then** both clients show the current structure through text-based observation.

**Given** empty sessions or tabs, **when** a client selects one, **then** selection is valid without any pane existing. Each client can report/change its own selection without changing another client's selection.

**Given** a logical pane without a terminal, **when** a client selects it, **then** selection is valid and remains independent of another client's selection.

**Given** a selected tab, pane or ancestor, **when** it is removed, **then** that client selects the nearest surviving parent, ultimately the session. *(Superseded by P1.)* If the session is removed, the client reports removal and becomes unattached rather than choosing another session or exiting.

**Given** a selected tab or pane inside a moved subtree, **when** that subtree moves within the same session, **then** selection continues to identify that entity. When the subtree moves to another session, the source client remains attached to its source session and falls back to the nearest surviving source parent.

Independent verification: run two text observers, select different tabs or panes, remove a selected pane to verify fallback to its tab, edit from a separate CLI process, and verify shared structure plus independent selections, removal fallback and cross-session behavior.

### P1: Recover an ongoing client connection

**Given** existing session state, **when** a client attaches, **then** it receives current state, including changes made during attachment, without waiting for another edit.

**Given** a running attached client, **when** its connection is temporarily lost, **then** it clearly reports disconnection and automatically attempts to reconnect. Once connectivity returns, it replaces its view with current server state rather than requiring manual replay.

**Given** that same client reconnecting to its surviving session, **when** its selected tab or pane still belongs to that session, **then** selection is retained; otherwise selection falls back to the session. *(Superseded by P1.)* If the session no longer exists, the client becomes unattached. A newly started client begins at the selected session root; restoring selection across client process restarts is not required.

Independent verification: interrupt an observer's connection without terminating the server, make CLI edits while it is disconnected, restore connectivity, and verify current state and selection without triggering an extra mutation.

## Amendments (2026-10-05)

Agreed with Cyan during program design, after comparing tmux, Zellij and Herdr session behavior. Each replaces only the marked text.

- **P1. Removing a session kicks its clients out.** *(Session text superseded by drop-sessions.)* When a session is removed, every client attached to it reports that the session was removed and exits. This applies whether the client sees the removal live or discovers it when reconnecting. Clients do not stay running in an unattached state. Replaces the session-removal sentences in the observation and recovery stories, the last sentence of FR-014, and SC-005.
- **P2. Attaching by name creates a missing session.** *(Session text superseded by drop-sessions.)* `ship attach <name>` attaches to the session with that name, creating it first if none exists. Attaching by session ID never creates anything; an unknown ID fails clearly. Reconnection always targets the session's ID, so a removed session is never recreated by a reconnecting client.
- **P3. The CLI accepts a session name wherever it accepts a session ID.** *(Session text superseded by drop-sessions.)* This covers session inspect/rename/remove, tab creation and movement destinations, and attach. Tabs and panes remain ID-only because their names may repeat. Name lookup is a client convenience; creation and inspection results still return the entity with its ID.
- **P4. Session names must not contain `:`.** *(Session text superseded by drop-sessions.)* IDs always contain `:`, so any argument containing `:` is an ID and any other argument is a name. Creating or renaming a session with `:` in its name fails clearly and changes nothing.

Replacement success criteria:

- **SC-005 (P1):** *(Session text superseded by drop-sessions.)* Removing a session while two text clients are attached makes both report the removal and exit. A client reconnecting after its session was removed during the outage does the same.
- **SC-007 (P2, P3, P4):** *(Session text superseded by drop-sessions.)* `ship attach work` creates `work` when absent and attaches to the existing `work` otherwise; two concurrent `ship attach work` commands end with one `work` session. Session inspect, rename, remove and tab creation work by name and by ID. A session name containing `:` is rejected without changing state.

## Non-goals

- Pane layouts, splits, geometry, sizing or spatial arrangement. Panes are logical leaf entities only.
- PTYs, shells, terminal emulation, rendering, terminal input or process lifecycle. Pane CRUD MUST NOT start a terminal process.
- Independent pane movement between tabs or pane reordering. Moving an owning tab carries its panes.
- Workspace entities. Sessions, recursive tabs and tab-owned leaf panes are the organizational model.
- A graphical/tree-navigation UI, mouse support or throwaway Ratatui controls. Observation is text-based and edits use the CLI.
- Disk persistence, server-restart restoration or agent conversation resume.
- Agent detection, agent status, notifications or an agent automation catalog.
- Configuration, configurable keybindings, plugins, remote exposure or authentication changes.
- Atomic multi-command batches or compound creation of a pre-populated hierarchy.
- Remembered selection history, client-process restart recovery or shared expansion/collapse state.
- A complete daily-driver terminal workspace.

## Functional requirements

- **FR-001:** *(Session text superseded by drop-sessions.)* The system MUST support server-owned sessions containing ordered root tabs, and tabs containing ordered child tabs recursively. There MUST be no separate workspace entity. A tab's parenthood MUST NOT turn it into a different kind of object; its own future pane layout and its children are independent concepts.
- **FR-002:** *(Session text superseded by drop-sessions.)* Empty sessions and tabs MUST be valid. Primitive creation MUST NOT implicitly create child tabs, panes or terminals.
- **FR-003:** *(Session text superseded by drop-sessions.)* Sessions, tabs and panes MUST have stable, externally kind-identifiable IDs. IDs MUST remain unchanged across renames and supported reordering/movement, including when a pane's owning tab moves between sessions.
- **FR-004:** *(Session text superseded by drop-sessions.)* Commands MUST target objects by explicit identity without requiring their full ancestry. Tab creation/movement MUST distinguish a session parent from a tab parent through the supplied typed identifier. Pane creation MUST identify a tab parent by typed ID and reject session or pane parents.
- **FR-005:** Creation MUST return the created entity as machine-readable data, including its ID, using the same serializable entity representation exposed to clients. Callers MUST NOT need to correlate a later update merely to obtain that ID.
- **FR-006:** *(Session text superseded by drop-sessions.)* The CLI MUST support creating, listing/inspecting, renaming and removing sessions, tabs and panes. Session/tab removal MUST be recursive; pane removal MUST remove only that pane. The CLI MUST additionally support tab movement and sibling reordering.
- **FR-007:** *(Session text superseded by drop-sessions.)* *(Extended by P4.)* Session names MUST be unique on the server, including after rename. Duplicate tab and pane names MUST be allowed.
- **FR-008:** Creation MUST append a tab to its parent's children. Movement MUST support changing parent and placement before/after a sibling; movement without explicit placement MUST append to the destination.
- **FR-009:** *(Session text superseded by drop-sessions.)* Tab movement MUST support destinations in the same session or another session, preserving the complete moved subtree, its tab-owned panes and all their IDs.
- **FR-010:** Invalid operations MUST fail with understandable errors and leave structure unchanged, including nonexistent targets, invalid destination kinds and self/descendant moves.
- **FR-011:** *(Session text superseded by drop-sessions.)* Tab removal MUST recursively remove its entire subtree and all panes owned by removed tabs by default. Session removal MUST recursively remove its contents. No configurable removal policy is required.
- **FR-012:** *(Session text superseded by drop-sessions.)* An attached client MUST receive current session structure and subsequent authoritative updates through text-based observation. Observing state MUST NOT require a graphical UI.
- **FR-013:** *(Session text superseded by drop-sessions.)* Two clients MUST be able to attach to the same session, report/change distinct selections, and observe shared changes without changing each other's selection. Selection MUST represent the most specific selected session, tab or pane, not require a full ancestry path. Selecting a logical pane MUST NOT require a layout or running terminal.
- **FR-014:** *(Session text superseded by drop-sessions.)* Selection MUST follow the surviving-parent and cross-session rules in the observation story. Removing a selected pane MUST select its owning tab if that tab survives; removing ancestors MUST continue fallback to the nearest surviving parent. Selecting a pane in a moved tab subtree MUST follow the same within-session retention and cross-session fallback rules as selecting a tab. *(Superseded by P1.)* Session removal MUST leave affected clients clearly unattached and available for an explicit new attachment.
- **FR-015:** Attachment and recovery MUST deliver current state without a missed-update gap or requiring another mutation. Older updates MUST NOT roll the client back to superseded state. A slow client MUST eventually receive the latest state after delivery capacity returns, even if no further mutation occurs.
- **FR-016:** A running disconnected client MUST report the loss, automatically attempt reconnection, and restore current state and selection according to the recovery story. Selection retention MUST NOT depend on disk persistence.
- **FR-017:** A logical pane MUST be a named leaf entity owned by exactly one tab, with no child tabs or panes. A tab's panes and ordered child tabs MUST coexist without requiring a pane layout. This slice MUST support pane CRUD as metadata only, without terminal processes, layouts or independent pane moves/reordering.

## Success criteria

- **SC-001:** *(Session text superseded by drop-sessions.)* A real CLI workflow creates two sessions, creates nested tabs and logical panes, extracts returned IDs with `jq`, renames tabs and panes, reorders tab siblings and moves a subtree across sessions while preserving tab and pane IDs.
- **SC-002:** Recursive removal deletes exactly the requested subtree and its panes; individual pane removal leaves the rest intact. Rejected operations, including pane creation under a session or pane, leave the inspected structure unchanged.
- **SC-003:** Two real attached text clients converge on shared structural changes while maintaining independent selections, including selection of empty tabs and logical panes, pane-to-tab removal fallback, and the specified ancestor-removal/move fallbacks.
- **SC-004:** Attachment during edits and reconnection after missed edits show current state without requiring an additional edit. Reconnection retains a surviving selected pane in the attached session and falls back to the session when that pane is gone or has moved to another session. A temporarily slow observer receives the final state after it resumes.
- **SC-005:** *(Session text superseded by drop-sessions.)* *(Superseded by P1.)* Removing an attached session leaves its clients unattached, and they can explicitly attach to another session without restarting their processes.
- **SC-006:** The accepted workflows are exercised on Linux and macOS. Unavailable platform checks are reported as unverified, not treated as proof of parity.

Verification is through independent real command/client workflows; this paper does not authorize permanent test scaffolding.

## Open unknowns

No blocking product decisions remain from this interview. Exact command spelling and output framing, endpoint organization, attachment addressing and retry mechanics belong to the subsequent architecture/interface review, not this product paper. They MUST preserve the observable requirements above; this product approval is not approval of an implementation.
