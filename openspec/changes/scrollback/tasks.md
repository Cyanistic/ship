# Tasks

Checkpoints for implementing the locked plan, one slice at a time. The papers in `design/` were locked on 2026-10-11 and are the contract; `design/program.md` holds the file map, signatures and full checks for each slice. Complete and verify one slice, then stop for Cyan's review before the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds, and ship-core checks with and without `clap`. Run the wrapper's own tests in a copy outside the repo, as before. Use owned temporary servers, don't restart existing user work, and clean up only what the exercise created. Checks run on macOS; Linux counts only when run there, and is otherwise reported unverified.

## 1. Scroll keys and `ship pane scroll` to a shared, stable scrolled view

- [x] 1.1 Wrapper changes 12 and 13, and the outlive-EOF part of 15, recorded in `PROVENANCE.md`: a byte history cap with no line cap, `Scroll` commands, snap to bottom on keys and pastes, `Rendered.scrolled`, and the session loop serving commands after PTY EOF. Verify with the wrapper's tests in an outside copy, and confirm `PROVENANCE.md` describes each change separately.
- [x] 1.2 Add `[server] scrollback_bytes` (default 10,000,000) read with the shell at each spawn. Verify that `seq 5000` in a 200-column pane on the default setting scrolls to line 1, that `seq 50000` prunes older rows within the 10 MB cap, and that `scrollback_bytes = 20000` limits only panes created after saving.
- [x] 1.3 Add `ScrollPane`, `Screen.scrolled` and protocol version 9 in `ship-core`, the `Scroll` command with `--direction` and `--amount`, `POST /api/v0/panes/{id}/scroll` through the live pane map, and the pane task's forwarding of scroll after exit. Verify that `ship pane scroll --direction top|bottom|up|down` with and without `--amount` moves the view as specified, that `ship server status` reports protocol 9, that a misspelled binding argument is rejected, and that an exited pane still scrolls.
- [x] 1.4 Bind `alt-pageup` and `alt-pagedown` and draw `Screen.scrolled` as the indicator on the border, or at the content's top right without one. Verify that `alt-pageup` scrolls a page and shows the indicator, a second client shows the same lines, `yes | head -1000` while scrolled doesn't move the view, typing returns to the bottom with the key reaching the shell, and scrolling `nvim` does nothing.
- [x] 1.5 Measure U-2: one pane's RSS before and after filling 10 MB of history. Verify by recording the numbers in the program paper's deviation log, and stop for Cyan if they're far from the 10 MB cap.

## 2. `ship pane read` to text an agent can use

- [x] 2.1 Wrapper change 14: `read` with a reply, formatting the active screen or the last N rows, plain or with styles, wrapped or unwrapped. Verify with the wrapper's tests in an outside copy.
- [x] 2.2 Add `ReadPane`, the CLI-only `Read` command, `GET /api/v0/panes/{id}/text`, the pane task's read forwarding after exit, and `Outcome::Text` printed raw. Verify P5: 200 numbered lines, scrolled up, `ship pane read --lines 50` prints lines 151 to 200. Also verify that `--ansi` keeps a red `ls --color` entry red through `cat`, that `--unwrap` joins a 300-character line in an 80-column pane, that `nvim` reads as its screen only, and that an exited pane still reads.
- [x] 2.3 Verify SC-005: an agent pane runs `cargo test` in a sibling pane and reads its result with `ship pane read`, unaided.

## 3. Clicks and the wheel to selected panes, sidebar rows and program input

- [ ] 3.1 Wrapper change 15's pointer part: `send_pointer` routing to the program when it tracks the mouse, else alternate-scroll arrows, else wheel scroll, else gestures. Verify with the wrapper's tests in an outside copy.
- [ ] 3.2 Add `InputFrame::Mouse` and route it from the input stream to the pane task, tagged with the sending attachment. Verify with a temporary NDJSON probe that a mouse frame reaches the right pane, and that an exited pane still accepts the wheel.
- [ ] 3.3 Add `[client] mouse`, capture through `TerminalGuard`, and release on every restore path. Verify that `mouse = false` takes effect on save and returns drags to the outer terminal, and that detaching or killing the client leaves no mouse escapes at the shell prompt.
- [ ] 3.4 Add `ui/mouse.rs` hit-testing, sidebar `row_at` and the drawn pane area. Verify that clicking among three panes selects each with no stray `^[[<0;…M` in the shells, sidebar clicks select tabs, a fold-marker click folds, and the wheel over the sidebar does nothing.
- [ ] 3.5 Verify mouse behavior in real programs. In `nvim` with `mouse=a`, a click moves the cursor and the wheel scrolls the buffer. In `less`, the wheel sends arrows. In a shell, it scrolls history. The wheel over an unselected pane scrolls it without selecting it (G2). Pi behaves as in the outer terminal (SC-003).

## 4. Drag selection and program copies to the system clipboard

- [ ] 4.1 Wrapper change 16 and the gesture part of 15: Ghostty selection from gestures, autoscroll ticks, `Reversed` drawing of selected ranges (G5), `Copied` and `ClipboardWrite` events. Verify with the wrapper's tests in an outside copy and confirm `PROVENANCE.md` records 12 to 16.
- [ ] 4.2 Publish `Clipboard` on the pane bus, deliver `SseEvent::Clipboard` from the attach stream to the copying attachment or every viewer of the tab, and write it with crossterm's `CopyToClipboard` (`osc52` feature). Verify that a drag highlights for both clients and its release reaches only the copying client's clipboard (`pbpaste`), that double- and triple-clicks take a word and a line, and that a plain click copies nothing.
- [ ] 4.3 Verify the selection's lifetime and the clipboard's trust boundary. Dragging past the top edge scrolls and keeps selecting. Typing clears the highlight. Output arriving keeps it on its text. `"+y` in `nvim` reaches both clients on the tab but not a client on another tab. `printf '\e]52;c;?\a'` gets no answer.
- [ ] 4.4 Verify SC-004 on Linux if available: in foot over SSH, as in the U-3 check, a drag reaches `wl-paste`. Otherwise report Linux unverified.

## 5. Docs, tracking issues and size report

- [ ] 5.1 Update `README.md` with the scroll and read commands, `alt-pageup`/`alt-pagedown`, mouse behavior and both settings. Verify by running each documented command as written against a temporary server.
- [ ] 5.2 Update `AGENTS.md` ("viewing position" means which tab or pane a client views, not scroll) and `docs/product-brief.md` (`ship pane read` stands in for milestone 1's search evidence). Verify against the product paper's "changes to locked papers and other documents".
- [ ] 5.3 Draft five tracking issues from `.github/ISSUE_TEMPLATE/issue.md`: copy mode and search, each with the probe's findings; the 10 KB history default; edit-scrollback, with the deferred requirements and design notes from the product paper's amendment; and a bindable run action with `ship clipboard copy`. Verify by showing Cyan the drafts. File them only after Cyan approves the text.
- [ ] 5.4 Report implementation Rust and test line counts separately against the 320 to 420 line estimate, with the wrapper's uncounted lines listed on their own.
