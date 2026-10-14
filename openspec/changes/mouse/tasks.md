# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` on 2026-10-13, and they are the contract. `design/program.md` holds the file map, signatures and full checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Cyan approved six consolidated checkpoints after slice 1: original section 1; sections 2–3 together; sections 4–5 together; section 6; sections 7–8 together; and section 9. The original task numbers below stay stable. Stop after verifying each consolidated checkpoint.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `ship-core` checks with and without `clap`.

Use owned temporary servers and config files, don't restart existing user work, and clean up only what the exercise created. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

## 1. `to` on pane resize

- [x] 1.1 Implement the resize changes and bump the protocol:
  - Add `Resize.to` with the clap conflict, and `ResizePane.to`.
  - Add `layout::Change`, and have the resize handler choose between `amount` and `to`.
  - Pass `to` through `execute`.
  - Set protocol version 11.

  Verify against a temporary server with a client attached to a two-pane tab:
  - `ship pane resize --direction right --to 40` on the left pane puts the frame's right column at 40 in `ship tab get`'s geometry. This settles the off-by-one.
  - Repeating it changes nothing and bumps no revision.
  - `--to 2` stops with the neighbor at 1 column.
  - `--direction left --to 40` on the right pane moves the same edge.
  - Plain `--amount` behaves as before.
- [x] 1.2 Verify the rejections and the version:
  - `--to 5 --amount 2` exits 2.
  - The same pair in an HTTP body is rejected and changes nothing.
  - `ship server status` reports protocol 11.

### Slice 1 evidence

Verified on macOS with an owned temporary server and the real full-screen client attached in a 120×32 PTY (94×32 tab area). A temporary Python probe exercised CLI commands and HTTP inspection:
- `--to 40` drew the border at column 40; repeating it preserved the whole tab and revision.
- `--to 2` left the left pane's content one column wide; `--to 0` could not move it further and preserved the revision.
- Leftward absolute resize from the right pane moved the same border. Relative `--amount 5`, the omitted one-cell amount, and absolute opposite-edge fallback passed.
- Mixed `to`/`amount` gave CLI exit 2 and HTTP 400, preserving the tab and revision. Health reported protocol 11.
- Additional geometry checks passed for vertical/up/fallback resize, a nested split with a nonzero origin, and `to = 65535` clamping.
- Formatting, workspace Clippy with warnings denied, debug/release workspace builds, and `ship-core` checks with and without `clap` passed. No permanent test code added.

The supplemental nested checks compare geometry rather than unrelated pane metadata: new panes can publish their discovered working directory during the exercise. The required two-pane no-op and rejection checks compare the whole tab and revision. Linux and manual terminal coverage remain unverified. `docs/guide.md` documents the shipped `--to` behavior; the remaining mouse docs stay in slice 9.

## 2. `set` replaces `rename`, with the passthrough attribute

- [x] 2.1 Make the core and server changes:
  - Add `Switch`, `Pane.right_click_passthrough`, `PaneInput.right_click_passthrough`, and the `SetTab` and `SetPane` bodies, and remove `Named`.
  - Replace `tab::Set` and `pane::Set` for the `Rename` commands, and add `--right-click-passthrough` on create.
  - Add the `Set` handlers, which resolve `toggle` and commit only on change, and the `set_tab` and `set_pane` routes. Copy the flag in `start_pane`.

  Verify against a temporary server:
  - Run P6's four scenarios.
  - `toggle` twice returns to the first value.
  - `ship pane create --right-click-passthrough -- sh` shows `rightClickPassthrough: true`, and a tab's starter pane shows `false`.
  - `--right-click-passthrough maybe` exits 2.
  - An empty `set` leaves the revision unchanged.
- [x] 2.2 Add `set` to the API client and `execute`. Verify:
  - A binding to `server.tab.rename` fails `ship config check` with its line and column.
  - `server.pane.set = { right_click_passthrough = "toggle" }` passes the check and toggles from a key in a running client.
  - A second client's replica shows the new value.

## 3. Mouse tracking on published screens

- [x] 3.1 Record `is_mouse_tracking()` in the vendored session's rendered snapshot, copy it in the pane actor's `capture`, and add `Screen.mouse`, which is skipped when false. Verify on a raw attach stream that the flag flips with each program's mode change, with no other screen change needed:
  - A pane's screen carries `mouse: true` while Neovim runs with `mouse=a`.
  - It has no `mouse` at the shell prompt, in `less`, or after Neovim quits.
  - `less --mouse` turns it on.

### Consolidated slice 2 evidence

Verified on macOS with an owned temporary server, two real full-screen clients in 120×32 PTYs, and a raw attach observer:
- P6 passed: name and passthrough changed together in one revision; a blank tab name restored the derived `~` label in both clients; empty and unchanged sets preserved the revision; both old rename commands exited 2.
- Toggling twice restored the value. Pane create with passthrough started on; the starter pane started off. Invalid switch values exited 2. Blank and whitespace-only pane names cleared; HTTP null fields left attributes alone. Unknown fields and invalid HTTP switch values were rejected without partial edits or revision changes.
- The old rename binding failed with line 2 and column 35. The new set/create bindings loaded, and real `alt-p` presses toggled the selected pane. A second observer's replica carried the new value. A real create key started a selected pane with passthrough on.
- Raw screen events omitted `mouse` at the shell prompt and in plain `less`; Neovim `mouse=a` and `less --mouse` published `mouse: true`. Neovim mode off/on and both programs' exits changed the flag as expected.
- A raw-mode program emitting only mouse-mode sequences published on/off screens with identical cells and cursor, without changing the shared revision.
- Formatting, workspace Clippy with warnings denied, debug/release workspace builds, and `ship-core` checks with and without `clap` passed. The slice-1 resize probe passed again, guide examples ran as documented, and strict OpenSpec validation passed.

The patch name serializer writes a present clearing `OptionalName` as `""`, not `null`, because null means leave unchanged. Tracking-query errors use the vendored input router's existing false fallback. No permanent test code added. Linux and manual terminal coverage remain unverified.

## 4. Gestures and border drags

- [x] 4.1 Implement the border drag:
  - In the lane, add `Edge`, `EdgeAt` and `Item::Resize`, with a waiting resize coalesced in place.
  - In `Mouse`, add `Gesture::Pressed` and `Gesture::Border`, plus `edges_at`, and make sidebar clicks act on release.
  - In `Local`, add `Scene` and `Local::scene`, and the `Resize` turn. `redraw` and `draw` take `Local`, and `panes::draw` takes `Scene` and the held edges.

  Verify P1's drag scenarios by hand, with `ship tab get` geometry matching what's drawn:
  - A 2x2 junction moves both edges.
  - A border click without motion does nothing.
  - The held border shows the selected-border color.
- [x] 4.2 Verify ordering and feel:
  - A press on an unselected pane's border leaves the selection alone, even with a `sleep`-delayed create waiting.
  - Closing the dragged pane from a script mid-drag ends the drag with no error.
  - A fast drag over Neovim and a shell feels like a tmux drag and leaves no backlog (SC-002).
  - Through a proxy adding 100 ms each way, the border trails and then settles at the pointer.
  - Sidebar rows select and fold on release, and a row press that moves does neither.
  - Every other click behavior is as before (SC-003).

## 5. Sidebar width

- [x] 5.1 Implement the width:
  - In `Sidebar`, add `configured`, `chosen`, `set_width`, `nudge` and the `Divider` hit.
  - In `Mouse`, add `Gesture::Divider`.
  - In the keymap, add `Sidebar::Resize`, `Side` and the label arm.
  - Bind `shift-h` and `shift-l` in `defaults.toml`.
  - In `Local`, add `now` and the `Width` handling, with `configure` called on reload.

  Verify P3's scenarios in two attached clients. On a 40-column terminal, the width stops where the pane area keeps 20.
- [x] 5.2 Verify the keys:
  - `client.sidebar.resize` with `direction = "up"` fails `ship config check` with its line and column.
  - The resize mode's hints end with `shift-h sidebar left  shift-l sidebar right  esc normal`.
  - With the sidebar hidden the keys do nothing, and showing it keeps the width.

### Consolidated slice 3 evidence

Tasks 4.1–5.2 are complete. Cyan accepted section 4 after a live test on 2026-10-14, reporting fully real-time, smooth dragging and successful two-axis corner grabs, and explicitly requested its tasks be marked complete. No later slice started.

Verified on macOS with owned temporary servers, real full-screen clients in PTYs, rendered ANSI cells inspected through a temporary pyte dependency, CLI/HTTP geometry, and an owned proxy adding 100 ms per forwarding direction:
- A vertical border moved five columns, retained the selected pane, clamped its neighbor to one content column, and accepted a changed final position on release. An unmoved press preserved the full tab and revision. The held cell used the cyan selected-border color, and rendered border positions matched published geometry.
- A 2x2 junction moved both axes. Closing its grabbed pane ended subsequent motion/release without an error.
- Sidebar rows and fold markers acted on release, not press; motion away and back suppressed both actions.
- Two clients kept independent widths. Divider motion added ten columns; three `shift-l` presses added three, `shift-h` narrowed, and the documented five-column binding worked. Hiding kept the chosen width and made resize keys no-ops. An unrelated reload preserved it; a changed configured width reset it. The pane area kept 20 columns on 40- and 25-column terminals, with a 10-column sidebar minimum when space permitted.
- Resize hints included `shift-h sidebar left  shift-l sidebar right  esc normal`. An `up` sidebar direction failed config checking with line and column.
- Existing input paths passed: an unselected content click selected without reaching the program; a selected program's drag/release followed beyond its pane; wheel input reached an unselected program; a shell text drag copied through OSC 52.
- Through the delay proxy, a selecting create ran before border resizing. The border press did not overwrite that selection. 81 motions plus final release settled with one resize request, even with a key queued between positions. Fast motion across Neovim and a shell settled at the final published geometry without a backlog. This demonstrates ordering and coalescing, not subjective feel.

Formatting, workspace Clippy with warnings denied, debug/release workspace builds, `ship-core` checks with and without `clap`, strict OpenSpec validation, and `git diff --check` passed on the final changes.

Temporary probe: `/tmp/ship-mouse-slice3-probe.py`; results: `/tmp/ship-mouse-slice3-results.log`. No permanent test code added. README and guide describe only the newly implemented behavior. Cyan's live feedback supplies the accepted responsiveness and junction checks; the comparison reported was with Herdr, not a formal tmux comparison. Two-terminal coverage, the complete hand-run SC-003 regression set, and Linux remain unverified; they are not claimed by this checkpoint acceptance.

## 6. Tab drops

- [x] 6.1 Implement the drop:
  - In `Sidebar`, add `drawn`, `gap` and `movable`, and draw the drop line.
  - In the lane, add `TabMove` and `Item::Move`.
  - In `Mouse`, add `Gesture::Row` and `gap`.
  - Add the `Move` turn, which re-checks, unfolds per G1 and then moves.
  - Pass the gap from `draw`.

  Verify P2's scenarios, checking `ship tab list` against the line shown before each release.
- [x] 6.2 Verify the edge cases:
  - The depth range holds at a gap between a deep row and a shallower one.
  - Dropping into the dragged tab's own subtree shows no line and does nothing.
  - A drop whose target a script closes before its turn does nothing and shows no error.
  - A folded parent unfolds after a child drop.
  - The selection doesn't change, and a moved selected tab stays selected.

### Consolidated slice 4 evidence

Tasks 6.1–6.2 are complete. No menus or passthrough routing started. All runtime checks used owned temporary servers on random loopback ports; Cyan's running server was not contacted or changed.

Verified on macOS with real full-screen clients in PTYs, rendered ANSI cells inspected through a temporary pyte dependency, and `ship tab list` inspected before and after each release:
- P2 passed: C moved before A with the line on the blank row under the heading; B became A's child with its subtree and live shell intact; a child returned to the top level below the last row; a drop under a folded parent appended after its hidden children and unfolded only the dragging client's parent. A shell command confirmed the moved program still ran.
- Every valid tested line underlined exactly the cells from the target indent to the divider. No tree edit occurred before release, and a second client showed no line. Top-level and nested gaps matched the resulting parent and sibling positions.
- A deep-to-shallow gap accepted depths 0, 1 and 2; the divider column clamped to depth 2. A gap above a depth-2 child clamped a left-edge pointer to depth 2 and inserted before that child.
- Own-subtree and no-op drops had no line and preserved the tree and revision. A one-cell row drag did not click or fold. Release outside the sidebar cancelled; other-button drags did nothing; an unmoved row click still selected only on release. Mouse-disabled config reload prevented drags.
- Both clients retained their selections, including a moved selected pane and a moved selected tab. An unselected client stayed unselected, with its line starting without the highlight prefix.
- Removing a source or target while held cleared invalid feedback without further pointer motion. An owned proxy held an earlier selecting create: a parent closed before the queued drop's turn produced zero move requests and no error. A target moved into the source's subtree before that turn was also rejected quietly. A target closed after the turn-time re-check produced one request and a quiet server not-found result, leaving the moving subtree intact.
- The prior border/sidebar/input probe passed again, including junction drags, coalescing through 100 ms each-way delay, config reloads, pane selection, program mouse forwarding, wheel input and OSC 52 text copying.
- Formatting, workspace Clippy with warnings denied, debug/release workspace builds, `ship-core` checks with and without `clap`, strict OpenSpec validation, and `git diff --check` passed.

The tree widget's `rendered_at` returns its final identifier even below the last row. The drawn-row snapshot therefore removes repeated identifiers, and click hit-testing requires an actual drawn row; this keeps the final drop gap anchored to the real last row. No permanent test code or new dependency added. Temporary probe: `/tmp/ship-mouse-slice4-probe.py`; results: `/tmp/ship-mouse-slice4-results.log`; regression results: `/tmp/ship-mouse-slice4-regression.log`. README and guide describe tab drops. Live hand checks in two terminals and Linux remain unverified.

## 7. Menus

- [x] 7.1 Implement the menus:
  - Add `ui/menu.rs`.
  - In the keymap, add `Menus`, `MenuItem` and `MenusFile`, and put `[client.menus]` in `defaults.toml`.
  - Add `keys::handled`.
  - In `Mouse`, add right-click routing, `owner` and the heading button.
  - In `Sidebar`, add the `Heading`, `Button` and `Below` hits, and draw the button.
  - In `Local`, add `menu`, menu-first routing, and select-then-open.
  - Draw the menu last.

  Verify P4's scenarios by hand, including a config that replaces a target and one that empties it:
  - A menu near the bottom-right corner fits on screen.
  - A right-click on a border opens the menu for the pane that context-menus names, at a junction too.
  - A terminal resize closes an open menu (G2).
- [x] 7.2 Verify the actions and the config:
  - An item that fails shows its error on the status line.
  - `hide sidebar` and `reload config` act at once.
  - A menu action after a pending create acts on the clicked target (SC-004).
  - `[client.menus] pain = []` and an item with `lable` fail `ship config check` with their line and column.
  - With `mouse = false`, no menu opens.
  - `menu.rs` is under about 250 lines.

### Slice 7 evidence

Tasks 7.1–7.2 are complete. Cyan requested original slice 7 alone, so slice 8's passthrough routing remains untouched. All runtime checks used owned temporary servers, config files and proxies on random loopback ports; Cyan's running server was not contacted or changed.

Verified on macOS with real full-screen clients in PTYs, ANSI cells inspected through temporary pyte, and CLI inspection of the resulting tabs, panes and attachment selections:
- P4's observable scenarios passed: the default pane menu selected its clicked target and highlighted the first item; mouse picking split right selected the new pane; `j` twice and Enter zoomed the target. Arrow keys and `k` also navigated. The passthrough item toggled the shared attribute, without implementing program forwarding.
- Tab menus created a selected sibling shell just after the clicked tab, or appended a selected child shell. Right-clicking a fold marker selected the tab without folding it. The heading button, heading right-click and below-last-row right-click opened the global menu without changing selection.
- The remaining default actions passed: split down selected the lower pane; pane close removed only that pane; tab close removed its subtree; global new tab created a selected sibling shell; detach exited cleanly.
- Label cells, shared vertical and horizontal borders, junctions and outer edges selected the lowest/rightmost containing frame across a five-pane layout. A bottom-right menu fit fully inside the terminal. Terminal resize closed it. Menus appeared only in the opening client, with its cursor hidden; another client continued sending keys to a raw-mode capture program.
- Typed keys, bound chords and bracketed paste were swallowed while open. Esc and outside left/middle presses closed without selecting another target or forwarding input. Valid config reload closed an overlay; an invalid live save kept it and the last good menus while showing the config error.
- Replacing only `pane` retained tab/global defaults. Label fallback, empty targets, and the previous selected-program right-click path passed. `mouse = false` prevented menus. Unknown targets, item fields and actions, missing/empty actions and multiple actions failed `ship config check` with line and column.
- A failed menu resize showed the same red status-line request error as a key action. An owned proxy held a selecting create before the server: the menu opened while it waited, and the picked zoom ran after target selection, zooming the clicked existing pane rather than the created pane. Hide sidebar acted while the create was held. Explicit reload disabled mouse before the file watcher's debounce and before the held create finished. A mode item entered resize mode immediately.
- The border/sidebar/input probe passed again. The tab-drop probe also passed with empty menus configured, preserving its pre-menu right-click assumptions; this included stale drops, depth clamping, local folding and delayed-lane re-checks.
- Formatting, workspace Clippy with warnings denied, debug/release workspace builds, `ship-core` checks with and without `clap`, strict OpenSpec validation and `git diff --check` passed. `ui/menu.rs` is 146 lines. No new dependency or permanent test code added.

README and guide describe menus, configuration and the still-pending program passthrough. Temporary probe: `/tmp/ship-mouse-slice7-probe.py`; results: `/tmp/ship-mouse-slice7-results.log`; regressions: `/tmp/ship-mouse-slice7-border-regression.log` and `/tmp/ship-mouse-slice7-tab-regression.log`. Live two-terminal hand checks, responsiveness judgment under SC-004, and Linux remain unverified.

Cyan's menu follow-up adds hover highlighting and terminal-palette colors. Mouse movement highlights the visible item without activating it; movement over the border or outside keeps the last highlight. Ratatui's persistent `ListState` supplies the rendered viewport offset, so hovering a clipped list does not shift the viewport or address the wrong item. The menu uses terminal-default foreground/background, ANSI cyan borders, and bold ANSI black text on an ANSI cyan highlight, rather than fixed RGB colors or reverse video.

The updated macOS PTY probe passed the full menu suite, including movement across multiple items without activation, border/outside movement, hover followed by Enter, and hovering the top visible row of a 45-item scrolled menu without shifting it. It also verified all-motion capture, indexed cyan/black cell styles, default normal-row colors, bold highlighting and the absence of reverse video. Existing border and tab-drop regression probes passed again. No visual contrast checks in foot or other terminal themes are claimed; Linux and live terminal/theme comparisons remain unverified.

Cyan approved the review's naming amendment: bindings and menu items now both accept `label`, and the action fallback is `Action::label()`. The composed `chord label` status text remains a hint. Default sidebar bindings, guide examples, published keymap/status-line specs and the active mouse paper/spec use the new field. Old binding `hint` fields are rejected as unknown fields, without a compatibility alias.

The temporary `/tmp/ship-binding-label-probe.py` passed local config checks for labels in bindings and menus and rejection of obsolete `hint` fields with line and column. In a real macOS PTY, resize mode rendered `f fill`, the unlabeled fallback `g zoom`, and both default sidebar labels; an unbound chord was omitted. A custom menu label rendered, and the labeled key still zoomed its pane. The full menu probe passed again. Results: `/tmp/ship-binding-label-results.log`. Checks used only owned temporary servers and config files; no running user server or config was changed.

## 8. Passthrough at the router

- [x] 8.1 Add `passes` and the passthrough branch to the mouse router. Verify P5's scenarios with Neovim `mouse=a` in two clients:
  - A right drag in Neovim's visual mode follows past the pane's edge.
  - A passthrough pane's label and border still open the menu.
  - The shell prompt in a passthrough pane opens the menu.
  - The right-clicked pane isn't selected when the click passes through.

### Slice 8 evidence

Verified on macOS with an owned temporary server, two real full-screen clients in 120×32 PTYs, and Neovim running with `mouse=a`. Temporary expression mappings logged right presses, drags and releases while preserving Neovim's built-in handling:
- Creating the editor pane with passthrough on exposed `rightClickPassthrough: true`. Both clients right-clicked its contents without opening a menu or changing their shell-pane selection.
- The default menu item toggled passthrough off and on from the editor's label. Off restored the contents menu; enabling it in one client changed routing in the other.
- Labels, outer borders and the shared border still opened the correct pane menu without sending input to Neovim.
- A visual-mode right drag crossed the sidebar and neighboring pane; Neovim received each drag and the outside release, its visual cursor moved, and both clients kept their shell selection.
- Disabling passthrough after a press did not redirect the held drag or release. Neovim with mouse tracking off, a passthrough shell prompt, and the exited editor all opened the pane menu.
- Formatting, workspace Clippy with warnings denied, debug/release workspace builds, and `ship-core` checks with and without `clap` passed. The full menu, border/sidebar and current tab-drop regression probes passed. No permanent test code added.

Probe: `/tmp/ship-mouse-slice8-probe.py`; results: `/tmp/ship-mouse-slice8-results.log`. Regression results: `/tmp/ship-mouse-slice8-menu-regression.log`, `/tmp/ship-mouse-slice8-border-regression.log`, and `/tmp/ship-mouse-slice8-tab-regression.log`. The obsolete slice-4 tab probe expected pre-menu right-click behavior and failed that assertion; the current slice-7 tab regression passed. Linux and physical-terminal hand checks remain unverified.

## 9. Docs, spec sync and size report

- [ ] 9.1 Update the docs:
  - `docs/guide.md`: drags, menus and `[client.menus]`, `set`, `--to`, passthrough, and the new resize-mode keys.
  - `README.md`'s "What works today" and keys, if they list mouse resizing as missing or show `rename`.
  - The product brief's milestone-1 item.

  Verify by running each documented command and config example as written against a temporary server, and check that every link resolves.
- [ ] 9.2 Bring the deltas under `specs/` in line with what was built, and run `openspec validate mouse --strict`. Verify that every deviation in the program paper's log appears in a delta or is noted as not user-visible.
- [ ] 9.3 Report implementation Rust, vendored Rust and test line counts separately, against `dac695b` and SC-006's estimate.
- [ ] 9.4 Report SC-001 terminal coverage on macOS, with Linux unverified unless Cyan runs it there.
