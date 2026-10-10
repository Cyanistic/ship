# Layout and sidebar

Status: Locked on 2026-10-09 with amendment A-1, re-approved by Cyan in chat ("yes it's approved"): a pane lives in its tab's layout rather than beside it, so the panes and their order are stored once (U-1 row). During the architecture draft, keeping a pane map beside the split tree meant two structures that had to agree, and Cyan asked for the panes to go into the tree instead ("now that we do, why not just stuff them in there?", then "i think that's much better"). Before that, locked on 2026-10-09, approved by Cyan in plannotator review ("LGTM"). Written on 2026-10-08 from a design conversation with Cyan, and reconciled on 2026-10-09 against the repository after the drop-sessions and keys-config changes landed. Revised the same day in conversation: zoom and swap stay in this change, mouse support moves to the scrollback change, and scrollback comes before floating panes ("Perfect! go for it!"). The open unknowns were then settled in a grilling session the same day, and their decisions are written in below ("lgpm! go for it!"). The locked [pane terminals](../../archive/2026-10-07-pane-terminals/design/product.md), [drop sessions](../../archive/2026-10-08-drop-sessions/design/product.md) and [keys and config](../../archive/2026-10-09-keys-config/design/product.md) papers stay in force except where a locked version of this paper amends them.

Amendment A-2: Locked on 2026-10-10, approved by Cyan in chat ("yep! that's approved to me!"). Cyan confirmed client-resolved directional swaps in chat ("let's do it this way!"). On 2026-10-10 Cyan also settled, in chat, the zoomed swap sequence and dropped the CLI's directional swap ("i agree withll of those! go for it!"); A-2 is revised for both. The original locked swap wording below is superseded by A-2 at the end of this paper. The approval doesn't authorize implementation; Cyan requests that separately.

## Summary

The client stops showing one pane at a time. A tab's panes appear together as splits, and a sidebar on the left shows the server's tabs as a tree you can fold, navigate and open. There is no tab bar on top. The look follows Herdr, with one structural difference: Herdr's flat workspace list and per-workspace tab row become a single tree of Ship's recursive tabs.

The keys and config change has landed. It replaced the `C-b` placeholder keys with Alt chords and already names the actions this change builds. Those actions are accepted in the config today and report "not available yet" when used. This change makes them work:

| Action | Today | After this change |
| --- | --- | --- |
| `server.pane.create` `direction` | adds a pane to the selected tab, ignoring `direction` | splits the selected pane right or down (FR-003) |
| `server.pane.resize` `direction`, `amount` | not available yet (also `ship pane resize`) | moves the selected pane's edge (FR-008) |
| `client.tab.next` / `client.tab.prev` | walk tabs in tree order | walk visible sidebar rows (keys and config FR-023) |
| `client.tab.select` `row` | not available yet, bound to `alt-1..9` | still not available, and no longer bound by default (FR-016) |
| `client.tab.expand` / `client.tab.collapse` | not available yet | unfold and fold in the sidebar |
| `client.pane.focus` `direction` | not available yet | selects the adjacent pane (FR-004) |
| `client.sidebar.toggle` | not available yet | hides and shows the sidebar (FR-013) |

The keys and config paper left zoom and swap to "their own changes". This paper is that change for both, and adds two actions and one default binding for an existing action:

| Action | Command | Default key |
| --- | --- | --- |
| `server.pane.zoom` | `ship pane zoom` | `alt-f` |
| `server.pane.swap` `direction` | `ship pane swap --direction` | `alt-shift-h/j/k/l` |
| `server.tab.create` under the selected tab | `ship tab create --parent ID` | `alt-shift-n` (FR-022) |

Decided, in conversation or by the locked papers:

- Layout and sidebar are one change. Keys and config was another, and has landed.
- The sidebar tree replaces any tab bar. The top tab row is removed entirely.
- Sidebar expansion stays client-local presentation state, as AGENTS.md and the glossary say.
- Zoom and swap are part of this change. They are layout operations Cyan uses daily in Herdr, and their default keys are Cyan's Herdr bindings.
- No mouse support in this change. Mouse capture costs the outer terminal's drag-to-select and the wheel needs history to scroll, so mouse moves to the scrollback change ([notes](../../scrollback/notes.md)).
- The server is one tree of top-level tabs with no sessions (drop sessions). The sidebar shows that whole tree.
- The selected pane is the focused pane. A client's selection is a tab, a pane or nothing, as the glossary defines it, and this paper does not add a separate focus.
- The default keys are fixed by the keys and config paper, except where this paper changes them. In particular, `tabs` mode (`alt-g`) moves the selection itself with `j`/`k`, folds with `h`/`l` and leaves with `esc`. It has no separate cursor and no Enter.
- New tabs from keys go after the selected tab under the same parent (keymap spec, "New tabs from keys"), and do not ask for a name.
- The split tree is the pane order. Read left to right and top to bottom, it replaces today's pane order wherever a rule uses it: derived tab labels, `client.pane.next` and selection repair (FR-018).
- Zoom is shared by every client viewing the tab, like the pane sizes it changes. Ending it follows tmux: moving focus out of the zoomed pane, or changing the tab's layout, unzooms (FR-006).
- New panes from keys start where the selected pane is: in its current directory when the shell reports one, else in its starting directory (FR-003).
- Jumping to a tab by row number (`alt-1..9`) is deferred. With recursive tabs, row numbers shift whenever something is folded, and with several machines it is unclear whose rows count. The default bindings go away; the action stays (FR-016).
- The sidebar has a plain "tabs" title, like Herdr's "Spaces", and a fixed width set in the config. It does not size itself to its labels, because derived labels change often and every change would resize the tab.
- Renaming from the UI is deferred. `ship tab rename` and `ship pane rename` cover it until then. How the UI should ask for input without diverging from the CLI is recorded, undecided, in [prompts and plugins](../../../../docs/research/prompts-and-plugins.md).

## Goal

Let Cyan work in Ship all day: see a shell, an editor and an agent side by side, split and close panes without leaving the client, and move around the server's tabs from a sidebar that shows the whole tree at once.

## Interface

The sketch is behavior, not exact rendering. Glyphs and colors are open.

```text
 tabs                    ┌ zsh ────────────────────────┐┌ pi ─────────────────────────────┐
 ▾ ship                  │ ~/projects/ship $ cargo b   ││ > what's next for ship?         │
   ▾ layout              │    Compiling ship-core      ││                                 │
       review            │                             ││ Working...                      │
     transport           ├ nvim ───────────────────────┤│                                 │
 ▸ herdr-notes           │ src/ui/draw.rs              ││                                 │
   scratch               │                             ││                                 │
                         │                             ││                                 │
                         └─────────────────────────────┘└─────────────────────────────────┘
 tabs  j/k move  h/l fold  esc back                                       layout › zsh
```

- **Sidebar title:** "tabs", on the first row.
- **Tree rows:** one row per tab, indented by depth, using the existing derived labels. `▾` and `▸` mark a tab with child tabs, expanded or folded. The row of the selected tab, or of the selected pane's tab, is highlighted. With nothing selected, no row is highlighted.
- **Pane area:** the selected tab's panes as splits. With two or more panes, each has a border carrying its label, and the selected pane's border stands out. A single pane fills the area with no border. A zoomed tab shows only its zoomed pane, with a zoom marker.
- **Bottom bar:** today's status line, which already shows tab and pane labels, exit status, the disconnected indicator, the active mode's name and the latest error. While a mode other than normal is active, it also shows that mode's key hints.

A tab can have both child tabs and its own panes. Opening `ship` shows `ship`'s own panes; `layout` is a separate place under it.

Reserved for milestone 3 and not part of this change: a state marker at the end of each tree row, rolled up from the agents beneath it, and an Agents section in the lower part of the sidebar.

## User stories

### P1: See several panes at once

**Given** a tab with one pane, **when** I split it right or down, **then** a new pane running my shell appears beside or below it in the same directory, both are visible, and the new pane is selected.

**Given** a tab with several panes, **when** I move focus left, down, up or right, **then** the pane in that direction is selected, and my keys go there.

**Given** a split pane, **when** I close it, **then** its program ends, the neighboring pane takes its space, and that pane is selected.

Independent verification: in one tab, run Neovim, a shell running tests, and Pi in three splits. Type into each, close one, and check that the others keep their content and redraw at the new size.

### P1: Move around the tab tree

**Given** a server with nested tabs, **when** I enter `tabs` mode, **then** `j`/`k` move the selection between visible rows and the pane area follows, `h`/`l` fold and unfold, and `esc` returns to normal mode on the tab I moved to.

**Given** a tab I worked in earlier, **when** I move back to it, **then** the pane I last used there is selected again.

**Given** a folded tab, **when** its descendant is selected by any means, **then** the sidebar shows the selected row.

**Given** a selected tab, **when** I press `alt-shift-n`, **then** a new tab with a shell appears as its child and is selected.

Independent verification: build a three-level tree with the CLI and `alt-shift-n`, fold and unfold it, move to tabs at each depth, and check that each tab shows its own panes.

### P2: Zoom, swap and resize

**Given** several panes, **when** I zoom the selected pane, **then** it fills the pane area for every client viewing the tab, until I zoom again or move focus out of it.

**Given** several panes, **when** I swap the selected pane with a neighbor, **then** the two exchange places and keep their programs and content.

**Given** resize mode, **when** I press a direction key, **then** the selected pane grows or shrinks in that direction by a step, and `esc` leaves the mode.

### P2: Hide the sidebar

**Given** an open client, **when** I press `alt-b`, **then** the sidebar hides and the pane area takes the width, and pressing it again brings it back with the same folds.

## Non-goals

- A tab bar, on top or bottom.
- The Agents section, per-row agent state markers and notifications. Those belong to milestone 3.
- Scrollback, copy mode, search, edit-scrollback, `ship pane read` and all mouse support, including clicking panes and sidebar rows and dragging the sidebar's edge. They form the next change, whose decisions so far are in [scrollback notes](../../scrollback/notes.md).
- Renaming tabs or panes from the UI, and any prompt for typed input. See [prompts and plugins](../../../../docs/research/prompts-and-plugins.md).
- Jumping to a tab by row number. `client.tab.select` stays accepted and not available.
- Dragging borders to resize.
- A sidebar that sizes itself to its labels, or a narrow collapsed strip like Herdr's. The strip waits for agent markers to have something to show.
- Floating panes. They are planned after the scrollback change: panes that keep running while hidden and toggle over the tiled layout, as in Zellij. The use case is keeping a dev server or build one key away from the editor.
- Throwaway popups and swap layouts.
- Changing the config file's location or how it loads. Keys and config owns them; this change only adds and removes the default bindings named in this paper.
- Saving layouts to disk or restoring them after a server restart.
- Themes or a configurable sidebar row layout.
- Full Windows support. Windows is best effort and unverified.

## Functional requirements

- **FR-001:** Each tab MUST hold its panes in a layout of nested horizontal and vertical splits. The layout and its split sizes MUST be server-owned shared state, because they decide each program's terminal size.
- **FR-002:** The client MUST show the selected tab's panes together, at the sizes the layout gives them. When a tab has two or more panes, each MUST have a border carrying its label, and the selected pane's border MUST be visually distinct. A tab with one pane MUST show it with no border.
- **FR-003:** `server.pane.create` with a `direction` MUST create a new pane next to the selected pane, right or down, and MUST select it. Without a command, the pane MUST run the shell `[server] shell` names, else the login shell, as today. From a key, the new pane MUST start in the selected pane's current directory when its shell has reported one (OSC 7), else in the selected pane's starting directory. From the CLI, it MUST keep starting in the caller's current directory.
- **FR-004:** `client.pane.focus` MUST select the pane adjacent in its direction. When several panes are adjacent, it MUST pick the one this client selected most recently, else the one whose center is closest to the selected pane's center.
- **FR-005:** Closing a pane MUST end its program as pane removal does today, and MUST give its space to the panes that shared its split. A client whose selected pane is closed MUST have its selection move to the pane that takes over the closed pane's space. This replaces the "next pane, else previous pane" rule of session-observation's "Selection repair"; the rest of that requirement stands.
- **FR-006:** `server.pane.zoom` MUST toggle zoom on the selected pane, making it fill its tab's pane area while the tab's other panes keep running. Zoom MUST be shared state on the tab, so every client viewing the tab sees the same zoomed pane. When a tab is zoomed:
  - A client whose selected pane in that tab is hidden MUST have its selection move to the zoomed pane.
  - `client.pane.focus` and `client.pane.next` from the zoomed pane MUST unzoom the tab for everyone, then move.
  - Creating, closing, swapping or resizing a pane in the tab MUST unzoom it first.
  - Selecting a different tab MUST leave the zoom alone.
- **FR-007:** `server.pane.swap` MUST exchange the selected pane with its neighbor in a direction, keeping both programs running and both screens intact. Ties between several neighbors MUST follow FR-004.
- **FR-008:** `server.pane.resize` MUST grow or shrink the selected pane's split in its direction by `amount` cells, or one step when `amount` is omitted. `ship pane resize` MUST do the same from the CLI, as every `server.` action must. The default `resize` mode MUST keep its current keys.
- **FR-009:** The sidebar MUST show a "tabs" title and, below it, the server's tabs as a tree in tree order, indented by depth, with fold markers on tabs that have children.
- **FR-010:** Fold state MUST be client-local. Two clients on the same server MUST be able to fold differently. The ancestors of the selected tab MUST always be expanded.
- **FR-011:** In the default `tabs` mode, `client.tab.next` and `client.tab.prev` (`j`/`k`) MUST move the selection between visible rows, `client.tab.collapse` and `client.tab.expand` (`h`/`l`) MUST fold and unfold the selected row, and `esc` MUST return to normal mode. The same actions MUST work from any mode they are bound in, including `alt-left`/`alt-right` in normal mode. A row is visible when every ancestor is expanded, whether or not the sidebar is hidden.
- **FR-012:** Selecting a tab MUST show its own panes, not its children's. When the tab has panes, the client MUST select the pane it last selected in that tab, else the first pane in reading order; in a zoomed tab, it MUST select the zoomed pane. That memory MUST be client-local. A tab with no panes MUST show the existing no-pane hint, and nothing selected MUST show the existing nothing-selected hint beside the sidebar.
- **FR-013:** `client.sidebar.toggle` MUST hide and show the sidebar per client, and the pane area MUST use the freed width.
- **FR-014:** The bottom bar MUST keep everything the status line shows today, and while a mode other than normal is active MUST also show that mode's key hints.
- **FR-015:** Tab geometry MUST keep following the smallest client viewing the tab, as terminal-client's pane size requirement says. Since keys and config amendment A-5, each client reports the area it draws a tab in; with this change, that area is the pane area, excluding the sidebar and bottom bar. Hiding the sidebar on the smallest client MUST therefore let the tab grow.
- **FR-016:** The default keys MUST no longer bind `alt-1` to `alt-9`, which amends the keymap spec's default bindings. `client.tab.select` MUST stay accepted in the config and keep reporting "not available yet".
- **FR-017:** `ship pane zoom` and `ship pane swap` MUST do what their `server.` actions do, as every `server.` action must.
- **FR-018:** A tab's pane order MUST be its split tree read left to right and top to bottom. Derived tab labels, `client.pane.next` and anything else that uses pane order MUST follow it.
- **FR-019:** `ship pane create --pane ID` MUST split that pane, in the given direction or right by default. Run inside a pane, `SHIP_PANE_ID` supplies `--pane` as it does today. With only `--tab`, it MUST split that tab's largest pane, breaking ties by pane order, and a tab with no panes MUST get its first pane.
- **FR-020:** The sidebar's width MUST be fixed, set by `sidebar_width` in `[client]`, defaulting to 26 columns.
- **FR-021:** The default keys MUST bind `alt-f` to `server.pane.zoom` and `alt-shift-h/j/k/l` to `server.pane.swap` left, down, up and right.
- **FR-022:** The default keys MUST bind `alt-shift-n` to create a tab with the starter shell as the last child of the selected tab, or of the selected pane's tab, and select its pane. With nothing selected, it MUST create a top-level tab, as `alt-n` does.

## Success criteria

- **SC-001:** On Linux and macOS, Neovim, Pi and a shell running tests work side by side in splits with no visible corruption, through repeated splitting, closing, swapping, zooming and resizing.
- **SC-002:** An alternate-screen program in one split enters and exits cleanly without disturbing its neighbors.
- **SC-003:** A three-level tab tree can be folded, navigated and opened from `tabs` mode, and each tab shows only its own panes.
- **SC-004:** Two clients on one server can fold the sidebar differently, and select different panes, without affecting each other. Both see the same zoomed pane.
- **SC-005:** Typing into one split feels as responsive as a single pane does today. Sustained output in one split does not make another feel sluggish.
- **SC-006:** Checks not run on a platform are reported as unverified there; Windows is reported as unverified throughout.

Verification is through real workflows and temporary probes; this paper does not authorize permanent test code.

## Open unknowns

None remain. The unknowns from the earlier draft were settled in conversation on 2026-10-09:

| Unknown | Decision |
| --- | --- |
| U-1 Layout model | The split tree is the pane order (FR-018); `ship pane create` placement (FR-019); selection repair on close (FR-005). The data model belongs to the architecture paper. A pane lives in its tab's layout, so the tab's panes and their order are stored once; floating panes, when they come, will be a separate list in the tab (A-1). |
| U-2 New child tabs | `alt-shift-n` (FR-022). |
| U-3 Visible rows | Rows under expanded ancestors, hidden sidebar or not (FR-011); `alt-1..9` removed (FR-016). |
| U-4 Sidebar header | A "tabs" title (FR-009). |
| U-5 New pane directory | The selected pane's directory (FR-003). Cyan's shell setup should report it, since Ship sets `TERM=xterm-256color`; not yet checked in a live pane. |
| U-6 Directional focus ties | Most recently selected, else nearest center (FR-004). |
| U-7 Zoom with several clients | Shared, tmux-style unzoom (FR-006). |
| U-8 Selecting a tab | Last pane this client used there (FR-012). |
| U-9 Sidebar width | Fixed and configurable (FR-020). |
| U-10 Pane borders | Only with two or more panes (FR-002). |
| U-11 Renaming from the UI | Deferred; see Non-goals. |
| U-12 Sidebar width in tab geometry | Hiding the sidebar lets the tab grow (FR-015). |
| U-13 Zoom and swap keys | Cyan's Herdr bindings (FR-021). |

## Amendment A-2: client-resolved swaps (locked)

Swap exchanges two explicitly named panes in the same tab in one server operation, preserving programs, screens and the split structure. The panes need not be adjacent. The server validates membership; it does not accept a direction or pane-selection history.

This supersedes the swap action table and FR-007, FR-017 and FR-021 above for swap only:

- **FR-007:** `server.pane.swap` takes the source pane and the other pane's ID. `client.pane.swap` takes a direction, resolves the neighbor from published geometry using FR-004's client-local recency rule, and sends the explicit pair. With no neighbor, it is a no-op.
- **FR-017:** `ship pane swap` names the other pane explicitly with `--other`, which is required. The CLI has no directional form: directional swap is `client.pane.swap`, and `client.` actions are reached from keys, not the CLI. This also spares the CLI a geometry read, which would need a new endpoint or a short-lived attach, and which tabs nobody views don't have.
- **FR-021:** `alt-shift-h/j/k/l` bind `client.pane.swap` left, down, up and right. Zoom's binding remains unchanged.

The request means "exchange these two IDs", not "re-evaluate which pane is in that direction when the request arrives". If geometry changes after resolution, the server still exchanges the named pair if both remain in the same tab. Missing or cross-tab targets are rejected. Cross-tab swapping is out of scope.

Pane recency stays client-local and is never sent to the server. In a zoomed tab, `client.pane.swap` first unzooms the tab for everyone, waits for the unzoomed geometry, and then resolves the neighbor. With no neighbor, the tab stays unzoomed and no swap is sent. This is the same sequence FR-006 gives pane focus out of a zoomed pane.
