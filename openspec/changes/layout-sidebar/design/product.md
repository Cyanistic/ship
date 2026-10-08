# Layout and sidebar

Status: Draft, not approved. Written on 2026-10-08 from a design conversation with Cyan. Nothing in this paper authorizes implementation until Cyan locks it. The locked [pane terminals](../../archive/2026-10-07-pane-terminals/design/product.md) paper stays in force except where a locked version of this paper amends it.

## Summary

The client stops showing one pane at a time. A tab's panes appear together as splits, and a sidebar on the left shows the session's tabs as a tree you can fold, navigate and open. There is no tab bar on top. The look follows Herdr, with one structural difference: Herdr's flat workspace list and per-workspace tab row become a single tree of Ship's recursive tabs.

This change depends on a separate keys and config change, planned to land first. That change replaces the `C-b` placeholder keys with configurable Alt chords and already names the actions this paper defines. Until this change lands, those actions report that they are not available yet.

Decided so far, in conversation:

- Layout and sidebar are one change. Keys and config are another, and come first.
- The sidebar tree replaces any tab bar. The top tab row is removed entirely.
- Sidebar expansion stays client-local presentation state, as AGENTS.md already says.
- Mouse support in this change is limited (see Non-goals). Text selection waits for scrollback.

Everything listed under Open unknowns has a recommendation but no decision.

## Goal

Let Cyan work in Ship all day: see a shell, an editor and an agent side by side, split and close panes without leaving the client, and move around a session's tabs from a sidebar that shows the whole tree at once.

## Interface

The sketch is behavior, not exact rendering. Widths, glyphs and colors are open.

```text
 work ▾                  ┌ zsh ────────────────────────┐┌ pi ─────────────────────────────┐
 ▾ ship                  │ ~/projects/ship $ cargo b   ││ > what's next for ship?         │
   ▾ layout              │    Compiling ship-core      ││                                 │
       review            │                             ││ Working...                      │
     transport           ├ nvim ───────────────────────┤│                                 │
 ▸ herdr-notes           │ src/ui/draw.rs              ││                                 │
   scratch               │                             ││                                 │
                         │                             ││                                 │
                         │                             ││                                 │
                         └─────────────────────────────┘└─────────────────────────────────┘
 NAV  j/k move  h/l fold  enter open  esc back                              work · 3 panes
```

- **Sidebar header:** the attached session's name.
- **Tree rows:** one row per tab, indented by depth, using the existing derived labels. `▾` and `▸` mark a tab with child tabs, expanded or folded. The selected tab's row is highlighted.
- **Pane area:** the selected tab's panes as splits, each with a border and its label. The focused pane's border stands out.
- **Bottom bar:** the active mode's key hints while a mode is active. Otherwise the status the current status line shows.

A tab can have both child tabs and its own panes. Opening `ship` shows `ship`'s own panes; `layout` is a separate place under it.

Reserved for milestone 3 and not part of this change: a state marker at the end of each tree row, rolled up from the agents beneath it, and an Agents section in the lower part of the sidebar.

## User stories

### P1: See several panes at once

**Given** a tab with one pane, **when** I split it right or down, **then** a new pane running my shell appears beside or below it, both are visible, and the new pane has focus.

**Given** a tab with several panes, **when** I move focus left, down, up or right, **then** focus moves to the pane in that direction, and my keys go there.

**Given** a split pane, **when** I close it, **then** its program ends and the neighboring pane takes its space.

Independent verification: in one tab, run Neovim, a shell running tests, and Pi in three splits. Type into each, close one, and check that the others keep their content and redraw at the new size.

### P1: Move around the tab tree

**Given** a session with nested tabs, **when** I enter nav mode, **then** a cursor appears in the sidebar, `j`/`k` move it between visible rows, `h`/`l` fold and unfold, Enter opens the row's tab and Esc leaves nav mode.

**Given** a folded tab, **when** its descendant is selected by any means, **then** the sidebar shows the selected row.

Independent verification: build a three-level tree with the CLI, fold and unfold it, open tabs at each depth, and check that each tab shows its own panes.

### P2: Zoom, swap and resize

**Given** several panes, **when** I zoom the focused pane, **then** it fills the pane area until I zoom again.

**Given** several panes, **when** I swap the focused pane with a neighbor, **then** the two exchange places and keep their programs and content.

**Given** resize mode, **when** I press a direction key, **then** the focused pane grows or shrinks in that direction by a step, and Esc leaves the mode.

### P2: Focus and pick with the mouse

**Given** several panes, **when** I click a pane, **then** it gets focus. **When** I click a sidebar row, **then** that tab opens.

**Given** a program that asks for mouse input, such as Neovim with `mouse=a`, **when** I click or scroll inside its pane, **then** the program receives the event.

### P2: Hide the sidebar

**Given** an attached client, **when** I toggle the sidebar, **then** it hides and the pane area takes the width, and toggling again brings it back with the same folds.

## Non-goals

- A tab bar, on top or bottom.
- The Agents section, per-row agent state markers and notifications. Those belong to milestone 3.
- Scrollback, copy mode, search, and text selection with the mouse. Once the mouse is captured, the outer terminal's own drag-to-select needs Shift in most terminals; a capture switch (FR-016) covers users who want their terminal's selection back.
- Dragging borders to resize.
- Floating panes, popups and swap layouts.
- Configurable keys, the config file and its location. The keys and config change owns them.
- Saving layouts to disk or restoring them after a server restart.
- Themes or a configurable sidebar row layout.
- Full Windows support. Windows is best effort and unverified.

## Functional requirements

- **FR-001:** Each tab MUST hold its panes in a layout of nested horizontal and vertical splits. The layout and its split sizes MUST be server-owned shared state, because they decide each program's terminal size.
- **FR-002:** The client MUST show the selected tab's panes together, at the sizes the layout gives them, each with a border and its label. The focused pane MUST be visually distinct.
- **FR-003:** Splitting MUST create a new pane next to the focused pane, right or down, running the user's `$SHELL`, and MUST give the new pane focus. The new pane's starting directory is open (U-5).
- **FR-004:** Moving focus by direction MUST pick the pane adjacent in that direction. When several panes are adjacent, the choice is open (U-6).
- **FR-005:** Closing a pane MUST end its program as pane removal does today, and MUST give its space to the panes that shared its split.
- **FR-006:** Zoom MUST make the focused pane fill the pane area until zoom is toggled off or another pane is focused.
- **FR-007:** Swapping MUST exchange the focused pane with its neighbor in a direction, keeping both programs running and both screens intact.
- **FR-008:** Resize mode MUST grow or shrink the focused pane's split in the pressed direction by a fixed step, and MUST leave the mode on Esc.
- **FR-009:** The sidebar MUST show the attached session's name and its tabs as a tree in tree order, indented by depth, with fold markers on tabs that have children.
- **FR-010:** Fold state MUST be client-local. Two clients on the same session MUST be able to fold differently. The ancestors of the selected tab MUST always be expanded.
- **FR-011:** Nav mode MUST put a cursor on the sidebar. `j`/`k` MUST move it between visible rows, `h`/`l` MUST fold and unfold, Enter MUST open the row's tab and leave the mode, and Esc MUST leave the mode without changing the selection.
- **FR-012:** Opening a tab MUST select it and show its own panes, not its children's. A tab with no panes MUST show the existing empty-state hint.
- **FR-013:** The sidebar MUST be hideable per client, and the pane area MUST use the freed width.
- **FR-014:** The bottom bar MUST show key hints while a mode is active, and otherwise MUST show what the status line shows today, including exit status and the disconnected indicator.
- **FR-015:** Clicking a pane MUST focus it, and clicking a sidebar row MUST open that tab. Mouse events inside a pane whose program asked for mouse input MUST reach that program.
- **FR-016:** Mouse capture MUST be possible to turn off, so the outer terminal's own selection works. The setting lives in the keys and config change's config file.
- **FR-017:** Tab geometry MUST keep following the smallest client viewing the tab, as FR-010 of pane terminals requires, with the pane area as the measured size.

## Success criteria

- **SC-001:** On Linux and macOS, Neovim, Pi and a shell running tests work side by side in splits with no visible corruption, through repeated splitting, closing, swapping and resizing.
- **SC-002:** An alternate-screen program in one split enters and exits cleanly without disturbing its neighbors.
- **SC-003:** A three-level tab tree can be folded, navigated and opened from nav mode and by mouse, and each tab shows only its own panes.
- **SC-004:** Two clients on one session can fold the sidebar differently, and focus different panes, without affecting each other.
- **SC-005:** Typing into one split feels as responsive as a single pane does today. Sustained output in one split does not make another feel sluggish.
- **SC-006:** With mouse capture off, the outer terminal's selection works across the client.
- **SC-007:** Checks not run on a platform are reported as unverified there; Windows is reported as unverified throughout.

Verification is through real workflows and temporary probes; this paper does not authorize permanent test code.

## Open unknowns

Each has a recommendation, not a decision.

1. **U-1 Layout model.** How the split tree relates to the tab's current pane order, and what `ship pane create` does to the layout. Recommendation: the split tree replaces pane order as the source of truth, and `ship pane create` splits the tab's last pane right unless told otherwise. The data model belongs to the architecture paper.
2. **U-2 New tab placement.** In a tree, a new tab can be a sibling after the selected tab or a child of it. Recommendation: the main "new tab" action adds a sibling; "new child tab" is its own action.
3. **U-3 Tab keys outside nav mode.** Recommendation: next and previous tab walk visible rows in order, and `alt+1..9` select visible rows 1 to 9.
4. **U-4 Session switching.** Recommendation: a session picker (`alt+o` in the proposed defaults) replaces cycling. The sidebar header could open it on click.
5. **U-5 New pane directory.** Herdr's `new_cwd = "follow"` starts new panes in the focused pane's live directory. Ship does not track live directories yet. Recommendation: follow when known, else use the focused pane's starting directory.
6. **U-6 Directional focus ties.** Recommendation: when several panes are adjacent, pick the one this client focused most recently, else the one closest to the focused pane's center.
7. **U-7 Zoom with several clients.** Zoom changes pane sizes, and sizes are shared. Recommendation: zoom is shared state on the tab, shown with a marker, so every client sees the same zoomed pane.
8. **U-8 Selecting a tab.** Which pane gets focus when a tab opens. Recommendation: the pane this client last focused in that tab, else the first pane.
9. **U-9 Sidebar width and collapsed form.** Recommendation: a fixed width near 26 columns, hidden entirely when toggled off. A narrow collapsed strip like Herdr's waits for agent markers to show.
10. **U-10 Pane borders.** Recommendation: borders only when a tab has more than one pane, like Herdr's `"auto"`.
11. **U-11 Renaming from the sidebar.** It needs an editable text field. Recommendation: defer, and keep `ship tab rename`.
12. **U-12 Sidebar width in tab geometry.** Clients can hide the sidebar independently, so the pane area differs per client. Recommendation: each client's pane area is what FR-017 measures, so hiding the sidebar on the smallest client lets the tab grow.
