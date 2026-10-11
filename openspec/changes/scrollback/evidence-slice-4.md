# Slice 4: selection and clipboard implementation and verification

Drag, word and line selection, the shared highlight, copy on release and program clipboard writes are implemented across the wrapper, protocol, server and client. Tasks 4.1, 4.2 and 4.4 are verified. Task 4.3 is verified at the protocol level and waits on one open question, listed under "Open".

## Implementation

- `vendor/ratatui-ghostty/src/session.rs`: without mouse tracking, left-button pointer events from `send_pointer` feed Ghostty's selection `Gesture`. Presses are stamped on arrival (500 ms repeat interval, one cell of repeat distance). Each cell maps to a 4-pixel square, so autoscroll starts only past the grid's top or bottom row. While a held drag autoscrolls, the session loop wakes every 50 ms for a tick. Selections are set live with `set_selection`, so every published screen carries them. Release pushes `SessionEvent::Copied` with plain, unwrapped, trimmed text. `on_clipboard_write` pushes `SessionEvent::ClipboardWrite` for `text/plain` writes to the standard clipboard. `widget.rs` toggles `Reversed` on each row's selected range (G5). Recorded as change 16 and the gesture part of 15 in `PROVENANCE.md`, which now records 12 to 16.
- `crates/ship-core/src/protocol.rs`: `SseEvent::Clipboard(ClipboardText { text })`. The protocol stays at version 9.
- `crates/ship-server/src/pane.rs`: the pane task remembers the attachment that last sent pointer input. `publish` turns `Copied` into `Clipboard { to: Some(that attachment) }` and `ClipboardWrite` into `Clipboard { to: None }`, published on the bus as a separate value from `PaneEvent`.
- `crates/ship-server/src/attach.rs`: each attach stream subscribes to `Clipboard` when it starts. It sends a copy addressed to its own attachment, or an unaddressed one for a pane it watches, meaning a pane in its viewed tab.
- `crates/ship-client`: the `osc52` crossterm feature; `TerminalGuard::copy` writes `CopyToClipboard`; the UI loop writes each `SseEvent::Clipboard` without redrawing.

Counted Rust in this slice: about 66 added lines across `ship-core`, `ship-server` and `ship-client` (git numstat, before cloc). Wrapper: about 230 lines, uncounted.

## Evidence

On macOS:

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release workspace builds, `cargo check -p ship-core --no-default-features`, `cargo check -p ship-core --features clap` and `git diff --check` passed. The wrapper's changed files pass `rustfmt --edition 2024 --check`.
- The wrapper's tests ran in an outside copy (`/tmp/ship-wrapper-slice4`): 80 passed, the one pre-existing crash-reproduction test ignored. Log: `/tmp/ship-wrapper-slice4-tests.log`. No permanent tests were authored.

Temporary probe `/tmp/ship-slice4-probe/probe.py` used an owned server, independent attach streams sending NDJSON mouse and key frames, real clients under PTYs and a real Neovim 0.12.5. Its final run passed every check:

- A drag across `HELLO` highlights cells 0 to 4 on both viewers' screens. The release copies `HELLO` to the dragging attachment's stream only. A leftward drag from `D` to `W` copies `WORLD`, so both end cells are included in either direction.
- A plain click clears the highlight on both viewers and copies nothing. A double-click copies and highlights `WORLD`, and a third click copies the line `HELLO WORLD foo`.
- With `MARK` selected, ten lines of output arriving through a FIFO move it from row 25 to row 18, and the highlight moves with it on both viewers. A key typed into the pane clears the highlight on both.
- In an exited pane holding `seq 1 300`, pressing on `300` and dragging to the row above the content autoscrolls to offset 10. The release copies 37 lines from `264` down to the anchor cell (`3`), and the offset stops changing after release. So exited panes select too.
- In Neovim with `g:clipboard = 'osc52'`, `"+yy` delivers `LINE001\n` to both attachments viewing its tab and nothing to an attachment viewing another tab.
- A program printing `\e]52;c;?\a`, then reading its terminal for one second, reads nothing, and no attachment gets a clipboard event.
- Two real clients on the same pane: a drag in one writes `ESC ] 52 ; c ; TUFSSw==` (`MARK`) to its own terminal, and the other writes no OSC 52.

Artifacts: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-select-slice4-crmox66t/`, including `server.log` and both clients' output. The probe's owned server, clients and programs were stopped. Cyan's running server was not touched.

## Open

- **Lone panes can't be dragged past their edges.** A tab's only pane is drawn unbordered across the full terminal height, and terminals clamp mouse reports to the window. So the pointer can't get above the top row or below the bottom row, and autoscroll never starts. Bordered panes have a border row to drag onto. The probe's autoscroll check sent `y = -1` over NDJSON, which a real terminal can't produce for such a pane. This needs Cyan's decision; see the report.
- **Physical clipboard.** Cyan confirmed that a drag's copy reaches the system clipboard on macOS and from Linux over SSH (tasks 4.2 and 4.4). Neovim's `"+y` and Pi's own copy reaching a physical clipboard weren't checked separately; the OSC 52 they produce is verified above.
- **Linux.** Building and running Ship on Linux remains unverified; the client ran on the Mac.
