# Slice 3: mouse implementation and verification

Mouse input is implemented across the wrapper, protocol, server and client. Tasks 3.1 to 3.5 are verified; task 3.3's physical outer-terminal drag check was confirmed by Cyan after review. No slice-4 selection or clipboard work was started.

## Implementation

- `vendor/ratatui-ghostty/src/session.rs`: `Pointer` preserves signed coordinates until the session thread processes it. Program input is clamped to the live grid. Tracking takes priority over fallback scrolling; without tracking, the vertical wheel scrolls primary-screen history or sends three mode-aware arrows on the alternate screen when `ALT_SCROLL` is enabled. Existing `send_mouse` shares that behavior. Gesture handling stays in slice 4 as the program paper's build order specifies. Change 15 is recorded in `PROVENANCE.md`.
- `crates/ship-core/src/protocol.rs`: `InputFrame::Mouse` and `MouseInput`, with pane-relative coordinates and crossterm event kinds/modifiers. Protocol remains version 9.
- `crates/ship-server/src/input.rs`, `pane.rs`: input frames become pane commands tagged with the sending attachment. Mouse input remains available after program exit. The attachment is carried through the pane task and logged; selection-copy routing belongs to slice 4.
- `crates/ship-client/src/keymap.rs`, `ui/terminal.rs`, `ui/mod.rs`: `[client] mouse` defaults to true and reloads on save or explicit reload. Capture is disabled by the shared restore function used by drop and the panic hook. Disabled capture ignores already decoded mouse events. Disconnect and reload clear held-pointer state.
- `crates/ship-client/src/ui/mouse.rs`, `ui/draw.rs`, `ui/sidebar.rs`: hit-test the same pane area drawing uses and the tree widget's last rendered rows. A selecting press and its release do not reach the program. Wheel input targets the pane under it without selecting it; sidebar wheel input does nothing. Forwarded drags retain their original pane outside its edges, and cannot fall through to another pane if the original disappears. Forwarding and no-op motion do not force a client redraw.

A live fold check found that the old sidebar reconciliation immediately reopened a folded ancestor of the selected tab. Cyan approved separating cleanup from reveal rather than caching a selection in `Sidebar`. `Sidebar::reconcile` now only removes obsolete folds. At the state-application boundary, `ui/mod.rs` compares the authoritative selection before and after `Observer::apply` and calls `Sidebar::reveal` only when it changes. Fold clicks therefore persist through ordinary redraws and state updates. There is no `revealed` field or pending-selection comparison.

## Evidence

On macOS:

After the approved sidebar refactor, formatting, Clippy, debug/release builds, ship-core checks with and without clap, and the full `/tmp/ship-mouse-slice3.py` probe passed again. The sidebar check selects a child, folds its parent, navigates to another visible row, then unfolds and selects the child again. Refactor-run artifacts: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-mouse-slice3-0os2gx57/`.

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Debug and release workspace builds passed.
- `cargo check -p ship-core --no-default-features` and `cargo check -p ship-core --features clap` passed.
- `git diff --check` passed.
- The wrapper's existing tests ran in an outside copy: 80 passed, one pre-existing crash-reproduction test ignored. Log: `/tmp/ship-mouse-wrapper-tests.log`. No permanent tests were authored.

Temporary probe `/tmp/ship-mouse-slice3.py` used an owned server, independent attach streams, real clients under PTYs, and real Neovim and less processes. Its successful final run verified:

- Clicking each of three panes selects it; keyboard input reaches the selected program; no mouse escapes appear in any `cat -v` pane.
- Sidebar clicks land on remembered panes. Fold-marker clicks hide and restore child rows, including when a child was selected.
- Mouse NDJSON scrolls the intended pane after exit, while another pane remains unchanged.
- The wheel over an unselected pane scrolls it without changing selection (G2), and the sidebar wheel does nothing.
- Saving `mouse = false` emits capture-disable sequences and makes decoded mouse input inert; saving true restores capture and click handling.
- Detach and SIGTERM emit mouse-disable sequences and restore the PTY's exact original termios attributes.
- Neovim with `mouse=a` moves its cursor on a click and scrolls its buffer on the wheel. A drag crossing into the sidebar and its release stay routed to Neovim.
- less scrolls by alternate-screen arrow input rather than scrolling Ship history. The final probe gates less startup until its attached size is established.

Successful run artifacts: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-mouse-slice3-7adffvs7/`, including `server.log` and captured TUI output.

Temporary probe `/tmp/ship-mouse-programs.py` verified alternate-scroll off is inert; alternate-scroll on sends three application-cursor arrows; tracking takes priority; signed out-of-bounds positions clamp correctly; and tracked wheel, motion and release produce the expected SGR bytes. It also ran a UI-only, offline Pi instance in regular mode with no tools, MCP, extensions, context files or session persistence. Wheel input scrolls its terminal history and returns to the unchanged Pi screen, without injecting mouse escapes. A direct-PTY Pi baseline confirms it uses the primary screen without requesting mouse tracking. This verifies regular mode, not every Pi mode or extension configuration.

Program probe artifacts: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-mouse-slice3-2d3y33hg/`, including `pi-direct.bin`.

Cyan also confirmed in chat: "scroll does indeed work in real ship pane using the mouse".

Owned servers, clients, programs and the stalled temporary probe were cleaned up. Cyan's running server was not restarted or stopped.

## Probe corrections

The first probe failure was hidden by a macOS PTY cleanup deadlock: one thread closed the master while the drain thread blocked reading it with the slave still open. Sampling showed `close` on the main thread and `read` on the drain thread. Closing the slave first and joining the drain thread fixes cleanup. The revealed assertion was a probe error: compact screen cells can omit `symbol`, so extraction must use its default rather than indexing it.

A repeat less check timed out when less started before the attachment established its size. The final probe starts less through a FIFO after the correct size is published and passes. The exact cause of the ungated run's timeout was not isolated; the gated check verifies the intended running-program behavior without conflating it with startup resizing.

## After review

Cyan asked for a cleanup of the client's routing after review, and approved reshaping it:

- `Sidebar::row_at` returns a sidebar-local `Row { tab, marker }` instead of `mouse::Clicked`, so the sidebar no longer imports from `mouse.rs`. The tree's highlight, open and closed symbols are constants shared by drawing and hit-testing, and the sidebar keeps the tree's own drawn rect.
- `Mouse::route` returns `Option<Clicked>`; `Clicked::None` is gone and the paper's `input` helper is restored.
- `draw::split` is the one sidebar/pane split, used by drawing, hit-testing and `tab_area`.
- `TerminalGuard` records capture state (`mouse()`), replacing a client-local copy. `reload` drops a held press itself.
- The wrapper's mouse routing moved into `SessionState::mouse`, with unchanged behavior and the file's own idioms.

The program paper's slice-3 sketches were updated to match.

Rerun on macOS after the refactor: formatting, Clippy with warnings denied, debug and release builds, `ship-core` without default features, and `git diff --check` passed. The wrapper's tests passed in an outside copy (80 passed, one pre-existing crash test ignored). `/tmp/ship-mouse-slice3.py` passed all of its checks. `/tmp/ship-mouse-programs.py` passed its wrapper checks on every run, but its Pi section passed on 2 of 4 runs, failing once on the final screen comparison and once on `pi bottom`. The probe snapshots Pi's screen at its first ready marker, so it likely races Pi's startup drawing; the refactor leaves the primary-screen wheel path unchanged and the probe's viewer bypasses the client. Whether the probe flaked before the refactor was not checked.

### Physical drag check (task 3.3)

Cyan ran the branch's client against a throwaway server, with the client on this Mac over SSH from foot on Linux. With `mouse = true`, a plain drag over a shell selected nothing in foot. Saving `mouse = false` while attached made a Shift-free drag select natively, including sidebar and border text, as expected for a terminal that sees one screen. Saving `true` again stopped native selection and clicks selected panes. After detaching, dragging at the shell prompt selected normally with no mouse escapes.

With capture on, a Pi instance that tracks the mouse drew its own selection, but its copy did not reach the clipboard. Program clipboard writes reach the client in slice 4 (`ClipboardWrite` to `SseEvent::Clipboard`), so this is expected for slice 3 and is the first thing to check there.

## Remaining verification

- The client and server have not been built or run on Linux; the drag check used foot on Linux only as the outer terminal. No containers, VMs or remote platform checks were used.
- Actual outer-terminal restoration after a panic was not exercised in a temporary panic build; the panic hook and normal exit paths share the inspected `restore` function, which now includes `DisableMouseCapture`.

## Reuse and additions

reused: crossterm capture and input types; Ghostty mouse/key encoders, modes and viewport; existing NDJSON transport and pane tasks; server-published geometry; tui-tree-widget's rendered-row lookup; the client's existing config watcher, navigation memory and terminal restore paths.

new code: typed pointer transport, live-mode wrapper routing, client capture setting, hit-testing and mouse-driven selections/folds. Ship owns routing and pane behavior, not a new terminal encoder or gesture mechanism.

proof: the build checks, wrapper tests, real PTY/HTTP workflows and program checks above, plus Cyan's real wheel confirmation. Physical outer-terminal dragging remains unverified.
