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

- [ ] 4.1 Implement the border drag:
  - In the lane, add `Edge`, `EdgeAt` and `Item::Resize`, with a waiting resize coalesced in place.
  - In `Mouse`, add `Gesture::Pressed` and `Gesture::Border`, plus `edges_at`, and make sidebar clicks act on release.
  - In `Local`, add `Scene` and `Local::scene`, and the `Resize` turn. `redraw` and `draw` take `Local`, and `panes::draw` takes `Scene` and the held edges.

  Verify P1's drag scenarios by hand, with `ship tab get` geometry matching what's drawn:
  - A 2x2 junction moves both edges.
  - A border click without motion does nothing.
  - The held border shows the selected-border color.
- [ ] 4.2 Verify ordering and feel:
  - A press on an unselected pane's border leaves the selection alone, even with a `sleep`-delayed create waiting.
  - Closing the dragged pane from a script mid-drag ends the drag with no error.
  - A fast drag over Neovim and a shell feels like a tmux drag and leaves no backlog (SC-002).
  - Through a proxy adding 100 ms each way, the border trails and then settles at the pointer.
  - Sidebar rows select and fold on release, and a row press that moves does neither.
  - Every other click behavior is as before (SC-003).

## 5. Sidebar width

- [ ] 5.1 Implement the width:
  - In `Sidebar`, add `configured`, `chosen`, `set_width`, `nudge` and the `Divider` hit.
  - In `Mouse`, add `Gesture::Divider`.
  - In the keymap, add `Sidebar::Resize`, `Side` and the hint arm.
  - Bind `shift-h` and `shift-l` in `defaults.toml`.
  - In `Local`, add `now` and the `Width` handling, with `configure` called on reload.

  Verify P3's scenarios in two attached clients. On a 40-column terminal, the width stops where the pane area keeps 20.
- [ ] 5.2 Verify the keys:
  - `client.sidebar.resize` with `direction = "up"` fails `ship config check` with its line and column.
  - The resize mode's hints end with `shift-h sidebar left  shift-l sidebar right  esc normal`.
  - With the sidebar hidden the keys do nothing, and showing it keeps the width.

## 6. Tab drops

- [ ] 6.1 Implement the drop:
  - In `Sidebar`, add `drawn`, `gap` and `movable`, and draw the drop line.
  - In the lane, add `TabMove` and `Item::Move`.
  - In `Mouse`, add `Gesture::Row` and `gap`.
  - Add the `Move` turn, which re-checks, unfolds per G1 and then moves.
  - Pass the gap from `draw`.

  Verify P2's scenarios, checking `ship tab list` against the line shown before each release.
- [ ] 6.2 Verify the edge cases:
  - The depth range holds at a gap between a deep row and a shallower one.
  - Dropping into the dragged tab's own subtree shows no line and does nothing.
  - A drop whose target a script closes before its turn does nothing and shows no error.
  - A folded parent unfolds after a child drop.
  - The selection doesn't change, and a moved selected tab stays selected.

## 7. Menus

- [ ] 7.1 Implement the menus:
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
- [ ] 7.2 Verify the actions and the config:
  - An item that fails shows its error on the status line.
  - `hide sidebar` and `reload config` act at once.
  - A menu action after a pending create acts on the clicked target (SC-004).
  - `[client.menus] pain = []` and an item with `lable` fail `ship config check` with their line and column.
  - With `mouse = false`, no menu opens.
  - `menu.rs` is under about 250 lines.

## 8. Passthrough at the router

- [ ] 8.1 Add `passes` and the passthrough branch to the mouse router. Verify P5's scenarios with Neovim `mouse=a` in two clients:
  - A right drag in Neovim's visual mode follows past the pane's edge.
  - A passthrough pane's label and border still open the menu.
  - The shell prompt in a passthrough pane opens the menu.
  - The right-clicked pane isn't selected when the click passes through.

## 9. Docs, spec sync and size report

- [ ] 9.1 Update the docs:
  - `docs/guide.md`: drags, menus and `[client.menus]`, `set`, `--to`, passthrough, and the new resize-mode keys.
  - `README.md`'s "What works today" and keys, if they list mouse resizing as missing or show `rename`.
  - The product brief's milestone-1 item.

  Verify by running each documented command and config example as written against a temporary server, and check that every link resolves.
- [ ] 9.2 Bring the deltas under `specs/` in line with what was built, and run `openspec validate mouse --strict`. Verify that every deviation in the program paper's log appears in a delta or is noted as not user-visible.
- [ ] 9.3 Report implementation Rust, vendored Rust and test line counts separately, against `dac695b` and SC-006's estimate.
- [ ] 9.4 Report SC-001 terminal coverage on macOS, with Linux unverified unless Cyan runs it there.
