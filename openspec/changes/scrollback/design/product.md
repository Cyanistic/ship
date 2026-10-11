# Scrollback, mouse and edit-scrollback

Status: Locked again on 2026-10-11, approved by Cyan in Plannotator review ("LGTM") after an amendment deferring edit-scrollback, made at Cyan's request in chat; see [the amendment](#amendment-2026-10-11-edit-scrollback-deferred). Deferred text is marked in place, not removed. Previously locked, approved by Cyan on 2026-10-11 after Plannotator review. Written on 2026-10-10 from a design conversation with Cyan, with decisions recorded in [notes](../notes.md) and settled in the same conversation. Probe evidence is in `experiments/scrollback-probe/`.

## Summary

Panes get history you can scroll back through, with the wheel or keys, and every client viewing a pane sees the same scroll position. ~~`alt-e` opens a pane's history in your editor, in the pane's own spot, until the editor exits.~~ (Deferred.) Ship starts capturing the mouse: clicks select panes and sidebar rows, programs that ask for the mouse get it, and dragging over a shell selects text and copies it on release. Copies made by programs, such as Neovim's `"+y`, reach your clipboard too. Agents and scripts can read a pane's text with `ship pane read`.

Copy mode and search are deferred. Cyan rarely uses them~~, and edit-scrollback covers both until then: the editor already has search, selection and yank~~. Edit-scrollback is now deferred too. Until one of them lands, `ship pane read --lines N` piped into an editor or `grep`, mouse selection and plain scrolling stand in.

Needs attention in review:

- The 2026-10-11 amendment defers edit-scrollback (FR-014 to FR-022, P2, SC-002) and names `ship pane read` as the stand-in for search in milestone 1.
- ~~Edit-scrollback opens a separate pane that covers the original, as Zellij does, rather than reusing the original's ID (FR-014 to FR-020).~~ Deferred.
- Panes today keep about 10 KB of history, not the 1,000 lines the notes assumed. This change sets a 10 MB default (FR-005).
- ~~Closing a covered pane leaves its editor as an ordinary pane rather than closing it too (FR-019).~~ Deferred.

## Goal

Let Cyan look back through what a pane printed, copy from it, and drive Ship with the mouse, without giving up the outer terminal's selection for nothing. Let agents read a sibling pane's output. Together with layout, this finishes milestone 1's terminal workspace.

## User stories

### P1: Scroll back through a pane

Given a pane that has printed a long build log, when Cyan turns the wheel over it or presses `alt-pageup`, then the pane shows older output, its frame shows how far up it is, and new output doesn't pull it back down. When Cyan types into it, it returns to the bottom and the keys reach the program.

- Given two clients viewing the same pane, when one scrolls up, then both show the same history.
- Given a pane running `less`, when Cyan turns the wheel, then `less` scrolls by receiving arrow keys.

Tested independently: run `seq 5000` in a pane, scroll to the top with the wheel and keys, check a second client shows the same lines, then type and check it returns to the bottom.

### P2: Edit a pane's history (deferred)

Deferred on 2026-10-11 with edit-scrollback. Kept as written for its tracking issue.

Given a pane with a long test log, when Cyan presses `alt-e`, then the editor opens in that pane's spot with the log in it and the cursor at the end. Other panes stay where they are. When the editor exits, the original pane comes back in the same spot, still running.

- Given the editor is zoomed with `alt-f`, when it exits, then the original comes back zoomed.
- Given an agent targets the original pane with `ship pane read` while the editor covers it, then it reads the original's output, not the editor's.

Tested independently: run a counting loop, press `alt-e`, search the history in the editor, quit it, and check that the loop kept counting and its pane is back in place.

### P3: Click around

Given a split tab, when Cyan clicks a pane, then it becomes the selected pane. When Cyan clicks a sidebar row, then that tab is selected. When Neovim with `mouse=a` is the selected pane, then clicks, drags and the wheel reach Neovim.

Tested independently: click between three panes and two sidebar rows, then open Neovim with `mouse=a` and click to move its cursor.

### P4: Select and copy

Given a shell pane, when Cyan drags across text, then the text is highlighted and is on the clipboard when the button is released. A double-click selects a word and a triple-click a line. When Neovim yanks to the `+` register, the text reaches the same clipboard.

Tested independently: drag-select a line in a shell and paste it elsewhere, then yank with `"+y` in Neovim and paste it elsewhere, on both Linux and macOS.

### P5: Read a pane from a script

Given an agent that started `cargo test` in a sibling pane, when it runs `ship pane read --pane <id> --lines 50`, then it gets the last 50 lines as plain text, whatever the pane is scrolled to.

Tested independently: print 200 numbered lines in a pane, scroll it up, and check `ship pane read --lines 50` prints lines 151 to 200.

## Non-goals

- Copy mode and search. Both are deferred with tracking issues that carry the probe findings. ~~Edit-scrollback stands in for them, including for milestone 1's "search" evidence.~~ `ship pane read` stands in for them, including for milestone 1's "search" evidence.
- Edit-scrollback (FR-014 to FR-022), deferred on 2026-10-11 with a tracking issue carrying the design work so far.
- A bindable action that runs a command against a pane, and `ship clipboard copy`. Raised while designing edit-scrollback; deferred with a tracking issue.
- Dragging pane borders to resize, and dragging the sidebar's edge.
- The wheel over the sidebar. It does nothing in this change.
- Rectangular selection, a setting for copy-on-select, right-click menus and clickable links.
- Letting programs read the clipboard through OSC 52. Only writes are passed on.
- A fallback clipboard for outer terminals without OSC 52 (U-1).
- Keeping history across a server restart. That belongs to persistence.
- Closing a pane when its program exits (#16). It stays its own change.
- Floating panes~~, though covered panes (FR-015) are groundwork for them~~.
- Sending text to panes and waiting for output from the CLI. Those are milestone 4.
- Per-client scroll positions.

## Functional requirements

### Scroll and history

- FR-001: Each pane SHALL have one scroll position shared by every client viewing it. Mouse selection (FR-025) is likewise shared.
- FR-002: `server.pane.scroll` and `ship pane scroll` SHALL scroll the selected or targeted pane by lines, by pages, or to the top or bottom of its history. The defaults SHALL bind `alt-pageup` and `alt-pagedown` to one page up and down in normal mode.
- FR-003: A scrolled pane SHALL stay on the same content while new output arrives. When that content leaves the history, the pane SHALL show the oldest content that remains.
- FR-004: Input reaching a scrolled pane, from keys, a paste or `client.send` on any client, SHALL return the pane to the bottom before the program receives it.
- FR-005: `[server] scrollback_bytes` SHALL limit each pane's history, with a default of 10,000,000. There SHALL be no line limit. A change SHALL apply to panes created afterwards.
- FR-006: A scrolled-up pane SHALL show how far it is from the bottom: on its border, or at its top right when it has no border.
- FR-007: A pane on the alternate screen SHALL have no history to scroll. Scroll actions on it SHALL do nothing.
- FR-008: Exited panes SHALL scroll like running ones.

### Mouse

- FR-009: The client SHALL capture the mouse unless `[client] mouse = false`, and a saved change SHALL apply like other `[client]` settings. With capture off, Ship SHALL handle no mouse events.
- FR-010: A click in a pane that isn't selected SHALL select it and SHALL NOT reach its program.
- FR-011: In the selected pane, when its program has asked for mouse input, clicks, drags, releases, motion and the wheel SHALL reach the program in the form it asked for, with coordinates relative to the pane.
- FR-012: Otherwise the wheel SHALL scroll the pane's history on the normal screen. On the alternate screen it SHALL send arrow keys when the program has alternate scroll on, and do nothing when it has it off.
- FR-013: A click on a sidebar row SHALL select that tab as the keyboard does. A click on a row's fold marker SHALL fold or unfold it.

### Edit-scrollback (deferred)

Deferred on 2026-10-11. FR-014 to FR-022 are kept as written for the tracking issue and are not part of this change.

- FR-014: `server.pane.edit_scrollback` and `ship pane edit-scrollback` SHALL open the targeted pane's history and screen in an editor, as plain text with soft-wrapped lines joined. The defaults SHALL bind `alt-e` in normal mode. It SHALL work on exited panes.
- FR-015: The editor SHALL run in a new pane with its own ID, in the targeted pane's exact place in the layout. The targeted pane SHALL become covered: hidden, still running at its size, still recording output, and still a valid target for every pane command.
- FR-016: The editor SHALL be `$VISUAL`, else `$EDITOR`, else `vi`, from the server's environment. Editors in the vi family SHALL open at the end of the file.
- FR-017: Clients that had the covered pane selected SHALL have the editor pane selected. Every viewer of the tab SHALL see the editor.
- FR-018: When the editor pane closes, by its program exiting or by closing it, the temporary file SHALL be deleted. If it still covers a pane, the covered pane SHALL return to the editor pane's place, and clients that had the editor pane selected SHALL have it selected. A tab zoomed on the editor pane SHALL stay zoomed on the returned pane.
- FR-019: Closing a covered pane SHALL leave its editor pane in place as an ordinary pane that covers nothing. When that editor pane later closes, it SHALL close like any other pane.
- FR-020: Edit-scrollback on a covered pane or on an editor pane SHALL fail with "already editing" and change nothing.
- FR-021: `ship pane get` SHALL report which pane a covered pane is covered by, and which pane an editor pane covers.
- FR-022: The temporary file SHALL be readable only by the user, and SHALL be deleted when the server stops.

### Selection and clipboard

- FR-023: In a pane whose program hasn't asked for mouse input, dragging SHALL select text, a double-click SHALL select a word, and a triple-click SHALL select a line. Dragging past the pane's top or bottom edge SHALL scroll it.
- FR-024: Releasing the button after a selection SHALL copy its text to the clipboard of the client that made it, through its terminal's clipboard escape (OSC 52). A click without a drag SHALL copy nothing.
- FR-025: A selection SHALL be highlighted for every viewer. It SHALL follow its text as output arrives and the pane scrolls, and SHALL end at the next click in that pane, on input to it, or when its text leaves the history.
- FR-026: Clipboard writes from a pane's program SHALL reach the clipboard of every client viewing that pane.
- FR-027: Programs SHALL NOT be able to read any client's clipboard.

### Reading panes

- FR-028: `ship pane read` SHALL print a pane's text: the visible screen by default, or with `--lines N` the last N lines including history. `--unwrap` SHALL join soft-wrapped lines and `--ansi` SHALL keep colors and styles. The pane SHALL default like other pane commands.
- FR-029: It SHALL read from the bottom regardless of the pane's scroll position. On the alternate screen it SHALL read that screen only. It SHALL work on ~~covered and~~ exited panes.

## Success criteria

- SC-001: A 5,000-line build log can be scrolled from bottom to top and back with the wheel and with keys, and it feels at least as quick as the same log in Herdr.
- ~~SC-002: Edit-scrollback on a pane holding a full 10 MB history shows the editor in under a second.~~ Deferred with edit-scrollback.
- SC-003: Neovim with `mouse=a`, Pi, `less` and a plain shell each behave under the mouse inside Ship as they do in the outer terminal, apart from FR-010's selecting click.
- SC-004: Drag-select in a shell and `"+y` in Neovim both reach the system clipboard in Ghostty on macOS and in a Linux terminal emulator. Any platform not checked is reported unverified.
- SC-005: An agent starts a test run in a sibling pane and reads its result with `ship pane read`, with no user help.
- SC-006: Implementation Rust and tests are reported separately. The estimate is ~~500 to 630~~ 370 to 500 lines of implementation Rust, after deferring edit-scrollback.

## Decisions

Settled in conversation on 2026-10-10:

| Decision | Why |
| --- | --- |
| Scroll and mouse selection belong to the pane, not the client | Clients on one pane already see identical screens; tmux copy mode works the same way (notes) |
| Copy mode and search deferred, ~~edit-scrollback~~ `ship pane read` stands in | Cyan rarely uses them~~; the editor already searches and yanks~~ |
| Mouse selection with copy on release replaces copy mode as the stand-in for the outer terminal's selection | Ghostty's gesture API covers clicks, words, lines and autoscroll; Shift-drag still reaches the outer terminal |
| A 10 MB byte cap and no line cap | Matches Herdr and Ghostty; a byte cap bounds memory, which is the point of a cap |
| (Deferred) Edit-scrollback replaces the pane in place, with its own ID, like Zellij | Composes with zoom (`alt-f` gives Herdr's full-tab version); automation aimed at the original never types into the editor |
| (Deferred) The returned pane inherits the editor's zoom | Putting it back in the same place isn't a real layout change; you return to the view you left |
| A click on an unselected pane only selects it | A stray click can't land in an agent's TUI; tmux and Zellij pass it through |
| `ship pane read` takes `--lines`, `--unwrap` and `--ansi` | Herdr's useful subset without its `--source` naming |
| ~~`alt-e` for edit-scrollback,~~ `alt-pageup` and `alt-pagedown` for scrolling | `alt-e` is Cyan's Herdr binding; outer terminals often take `shift-pageup` |
| (Deferred) Closing a covered pane leaves its editor as an ordinary pane | Each pane's lifetime stays its own; a close never takes a second, unnamed pane with it. Rejecting the close was the alternative, but it makes scripts clear editors first |
| Exited panes scroll ~~and support edit-scrollback~~ | Crash output is what you most want to read back |
| The wheel over the sidebar does nothing in this change | Nothing in the sidebar scrolls yet |

### Changes to locked papers and other documents

- ~~Shared zoom (`session-observation`): a covered pane returning in place of its editor isn't a layout edit and doesn't unzoom (FR-018). Closing the editor pane by other means follows the same rule.~~ Deferred with edit-scrollback.
- `AGENTS.md`: "viewing position" means which tab or pane a client views, not how far a pane is scrolled.
- Product brief: milestone 1's search evidence is met by ~~edit-scrollback~~ `ship pane read` until search lands.
- New tracking issues: copy mode, search, and the 10 KB history default as it stands before this change. Added on 2026-10-11: edit-scrollback, and a bindable run action with `ship clipboard copy`.

### Amendment 2026-10-11: edit-scrollback deferred

Asked for by Cyan in chat after the program paper's review opened questions about how covered panes fit the layout ("this is breaking my brain and there's other stuff we can get out of the way now that's much more straightforward"). Slices for scrolling, reading, the mouse and selection never touch covering, so they ship without it.

- Deferred: P2, FR-014 to FR-022, SC-002, and the decisions marked (Deferred) above.
- Changed: `ship pane read` replaces edit-scrollback as the stand-in for search, and FR-029 no longer mentions covered panes.
- The edit-scrollback tracking issue carries the design work so far: a leaf holding the visible pane and the one it covers, layout commands on a covered pane acting on its slot, covered panes keeping their size until they return, a `NamedTempFile` for the history, and how tmux, Zellij, Herdr and kitty do it.
- A separate tracking issue carries a bindable run action with the target pane in its environment, plus `ship clipboard copy`.

### Probe evidence

From `experiments/scrollback-probe/` against the pinned libghostty-vt, on macOS only:

- A fresh terminal has a 10,000-byte history cap that overrides the line limit. A 200-column pane keeps about 250 to 290 rows today.
- Formatting 10,000 rows at 200 columns takes 2.4 ms and gives 1 MB of text. About 38,600 rows fit in 64 MB at that width.
- A scrolled viewport stays on its content while output arrives and moves to the oldest row once its content is pruned.
- Tracked positions follow their text and report when it has been pruned.
- The alternate screen has no history, and alternate scroll is on by default.

## Open unknowns

Settled by Cyan on 2026-10-10, on the recommendations below:

- U-1: Outer terminals without OSC 52, such as macOS Terminal.app, won't receive copies. Decision: no fallback in this change; it's a known limitation.
- U-2: Memory per pane at the 10 MB cap, and whether Ghostty's history compression counts against it. Decision: keep 10 MB and measure during architecture.
- U-4: The scroll indicator's text and the selection's colors. Decision: settle during rendering review, as with the layout change's deferred details.

Still open:

- U-3: Whether `alt-pageup` and `alt-pagedown` reach Ship in Ghostty, Kitty and the common Linux terminals. Decision: check on Linux and macOS before locking. Blocks FR-002's default keys.
  - 2026-10-10: foot on Linux (Hyprland), over SSH into a Ship client and server on macOS. `cat -v` inside a Ship pane printed `^[[5;3~` and `^[[6;3~`, so both keys survive foot, SSH and Ship's client. Ghostty, Kitty and macOS outer terminals: unverified. The keys are rebindable defaults, so a terminal that swallows them costs a config line.
