# Scrollback and mouse program

Status: Locked on 2026-10-11, approved by Cyan in Plannotator review of revision 3 ("LGTM"), which accepts G1, G2 and G5 as recommended. Revision 3: edit-scrollback removed, following the 2026-10-11 amendment in the [product](product.md) and [architecture](architecture.md) papers, which are reopened for it. Revision 2 followed Cyan's first Plannotator review. Written on 2026-10-11 from those papers, against the crates at `e5b2eb8`. Nothing here authorizes implementation.

Settled gates, each accepted as recommended:

- **G1, the scroll command's shape.** Revised after review to match `ship pane resize`: `ship pane scroll --direction up|down|top|bottom [--amount N]`, where the amount is lines and omitting it scrolls one page. In the config: `server.pane.scroll = { direction = "up" }`, or `{ direction = "up", amount = 5 }` for a fixed number of lines. Top and bottom ignore the amount.
- **G2, the wheel over a pane that isn't selected.** The product paper only covers clicks there (FR-010) and the selected pane (FR-011). Decision: the wheel acts on the pane under the pointer without selecting it, scrolling its history or reaching its program by FR-011 and FR-012's rules, as tmux and Zellij do. The alternative is that the wheel does nothing outside the selected pane. The wheel's step stays the wrapper's fixed 3 lines; making it a setting is possible but not in the product paper.
- **G5, the selection's look until rendering review (U-4).** Decision: selected cells swap foreground and background (`Reversed`, toggled), as tmux and Herdr do by default. Rendering review can still change it.

G3, G4 and G6 (covered-pane storage, splitting `$EDITOR`, the temporary file) went with edit-scrollback and move to its tracking issue.

## Rationale

This is a document-only plan. Nothing below exists as source yet, and none of it has been compiled.

The skeleton follows the architecture's structure, with one refinement. A copy isn't sent as a `PaneChange`: every `PaneEvent` makes the state actor commit, and a copy changes no shared state. So the pane task publishes a separate `Clipboard` value on the same bus, and each attach stream subscribes to it (architecture decision 7, same routing, its own type).

Applying `docs/agents/code.md` throughout:

- **Name your shapes:** the client's hit-tests return `Clicked` and `MouseInput` rather than tuples.
- **Duplicates that stay, and why:** the new commands restate their request bodies' fields, as `Resize` does, to keep rejecting misspelled keys in key bindings (see `command.rs` below). The wrapper's `Scroll`, `Read`, `Scrolled` and `Pointer` mirror wire types in `ship-core`. Sharing them would make `ship-core`, and so the client, depend on the wrapper and build Ghostty.

- **Wrapper (`vendor/ratatui-ghostty`):** recorded changes 12 to 16. They cover the history cap, outliving the PTY, scroll commands and the scroll offset, reading text, pointer input with alternate-scroll arrows, gestures and autoscroll, drawing the selection, and clipboard events (architecture decisions 1 to 3, 5, 8, 9, 12 and 13).
- **`ship-core`:** wire types (`InputFrame::Mouse`, `SseEvent::Clipboard`, `Screen.scrolled`), and two commands (`scroll` and `read`).
- **`ship-server`:** the pane task's new commands and its forwarding after exit, the history setting, two routes, the input route's mouse frames, and the attach stream's clipboard events.
- **`ship-client`:** mouse capture, hit-testing (new `ui/mouse.rs`), sidebar clicks, the scroll indicator, OSC 52 output, and the new commands.

Estimated change: 320 to 420 lines of implementation Rust in Ship, up from today's 6,712, plus about 300 uncounted wrapper lines. By area:

| Area | Lines |
| --- | --- |
| Wire types and commands | about 90 |
| Pane task, settings, input and attach | about 110 |
| Routes and API wrappers | about 80 |
| Client mouse, sidebar clicks, indicator and clipboard | about 120 |

## Skeleton map

### Proposed settings

The client manifest turns on crossterm's `osc52` feature:

```toml
# crates/ship-client/Cargo.toml
crossterm = { workspace = true, features = ["bracketed-paste", "event-stream", "osc52"] }
```

The config file gains two settings:

```toml
[server]
scrollback_bytes = 10000000      # FR-005; read when a pane starts

[client]
mouse = true                     # FR-009; reloads like other [client] settings
```

The default keys gain two bindings in normal mode:

```toml
"alt-pageup"   = [{ server.pane.scroll = { direction = "up" } }]
"alt-pagedown" = [{ server.pane.scroll = { direction = "down" } }]
```

Omitting `amount` scrolls one page. A user binding sets it in lines, for example `"alt-k" = [{ server.pane.scroll = { direction = "up", amount = 5 } }]`.

`PROTOCOL_VERSION` goes from 8 to 9 in slice 1, and later slices extend version 9 on the same branch.

### Added

#### `vendor/ratatui-ghostty/src/session.rs`, wrapper changes 12 to 16 (slices 1 to 4)

```rust
pub struct SessionConfig {
    /// History cap in bytes; `None` is Ghostty's own default. Replaces `scrollback`.
    pub scrollback_bytes: Option<usize>,        // change 12
    pub scroll_delta: isize,
    // …as today
}

/// Where to move the viewport.                 // change 13
pub enum Scroll { Lines(isize), Pages(isize), Top, Bottom }

/// What `read` returns.                       // change 14
pub struct Read {
    /// `None`: the active screen. `Some(n)`: its last `n` rows, history included.
    pub lines: Option<u32>,
    pub unwrap: bool,
    pub ansi: bool,
}

/// How far the viewport is above the bottom; absent at the bottom.  // change 13
pub struct Scrolled { pub offset: u32, pub history: u32 }

/// Session-relative position that may lie outside the grid while a drag runs past an edge.  // change 15
pub struct Pointer {
    pub kind: crossterm::event::MouseEventKind,
    pub modifiers: crossterm::event::KeyModifiers,
    pub x: i32,
    pub y: i32,
}

pub enum SessionEvent {
    // …as today, plus:
    /// A mouse selection ended with text in it.                        // change 16
    Copied(String),
    /// The program wrote the clipboard, OSC 52 already decoded.        // change 16
    ClipboardWrite(String),
}

impl SessionHandle {
    pub fn send_scroll_to(&self, scroll: Scroll);                             // change 13
    /// Runs on the session thread; `reply` gets the text.
    pub fn read(&self, read: Read, reply: Box<dyn FnOnce(String) + Send>);  // change 14
    /// Program first, then alternate-scroll arrows, then wheel scroll, then gestures.
    pub fn send_pointer(&self, pointer: Pointer);                             // change 15
    /// `take_if_dirty` also hands back the scroll offset rendered with the cells.
    pub fn take_if_dirty(&self, dest: &mut Buffer, area: Rect) -> Option<Rendered>;  // change 13
}

pub struct Rendered { pub cursor: CursorState, pub scrolled: Option<Scrolled> }
```

Behavior inside the session thread, one recorded change each:

- **12:** `scrollback_bytes` goes to `set_scrollback_max_bytes`, with no line cap.
- **13:** `Scroll` maps to `ScrollViewport`, a page being the grid's rows. Keys and pastes scroll to the bottom and clear the selection before writing. `scrollbar()` fills `Rendered.scrolled`. A pane on the alternate screen ignores `Scroll`.
- **14:** `Read` formats with `Formatter`, or with a selection from row `total - n` for `lines`. Long reads run on this thread.
- **15:**
  - The loop keeps serving commands after PTY EOF, until shutdown. Before, the loop broke on `pty_rx` disconnect.
  - `Pointer` goes to the program when it tracks the mouse, clamped to the grid.
  - Otherwise a wheel event on the alternate screen with `ALT_SCROLL` set sends `scroll_delta` arrow keys, a wheel event on the normal screen scrolls, and left-button events feed `Gesture`, which stamps each press with the time it arrives.
  - While `Gesture::autoscroll` is active, the loop wakes every 50 ms and applies an autoscroll tick.
- **16:**
  - Releasing after a drag, a double-click or a triple-click sets Ghostty's selection and pushes `Copied` with its plain, unwrapped text. A click without a drag clears the selection and copies nothing.
  - `on_clipboard_write` pushes `ClipboardWrite`.
  - `widget.rs` toggles `Reversed` on each row's `RowSelection` range (G5).
  - `PROVENANCE.md` records 12 to 16.

#### `crates/ship-client/src/ui/mouse.rs` (slice 3)

```rust
//! Terminal mouse events to selections, folds and pane input.

use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ship_core::{geometry::TabGeometry, id::IdOf, model::{NodeId, Pane, Tab}, protocol::{InputFrame, MouseInput}};

/// What a mouse event does.
pub(super) enum Clicked {
    /// Select this pane, or land on this tab as the keyboard does.
    Select(NodeId),
    /// Fold or unfold a sidebar row.
    Fold(IdOf<Tab>),
    Frame(InputFrame),
}

/// Which pane a press started in, so drags and the release go to it even
/// past its edges. Cleared on release.
#[derive(Default)]
pub(super) struct Mouse { held: Option<IdOf<Pane>> }

impl Mouse {
    /// `panes` is the pane area as drawn, `sidebar` the sidebar's rendered rows.
    pub fn route(
        &mut self,
        event: MouseEvent,
        panes: Rect,
        geometry: Option<&TabGeometry>,
        selected: Option<IdOf<Pane>>,
        sidebar: &super::sidebar::Sidebar,
    ) -> Option<Clicked>;
}

/// `event` for `pane`, relative to its `content`, negative above or left.
fn input(event: MouseEvent, pane: IdOf<Pane>, content: Rect) -> MouseInput;
```

### Moved

Nothing moves.

### Replaced

#### `crates/ship-core/src/protocol.rs` (slices 1, 3 and 4)

```rust
pub const PROTOCOL_VERSION: u32 = 9;

pub enum InputFrame {
    Key(KeyInput),
    Paste(PasteInput),
    Mouse(MouseInput),                     // slice 3
}

/// A mouse event inside or dragged from `pane`, relative to its content's
/// top-left cell; negative above or left of it.
pub struct MouseInput {
    pub pane: IdOf<Pane>,
    #[schema(value_type = Object)]
    pub kind: MouseEventKind,
    #[schema(value_type = String)]
    pub modifiers: KeyModifiers,
    pub x: i32,
    pub y: i32,
}

pub enum SseEvent {
    // …as today, plus:
    /// Text for this client's clipboard.  // slice 4
    Clipboard(ClipboardText),
}

pub struct ClipboardText { pub text: String }

/// POST /panes/{id}/scroll body.          // slice 1
pub struct ScrollPane { pub direction: ScrollDirection, #[serde(default)] pub amount: Option<u32> }
pub enum ScrollDirection { Up, Down, Top, Bottom }

/// GET /panes/{id}/text query.            // slice 2
#[serde(default)]
pub struct ReadPane { pub lines: Option<u32>, pub unwrap: bool, pub ansi: bool }

```

#### `crates/ship-core/src/screen.rs` (slice 1)

```rust
pub struct Screen {
    // …as today, plus:
    /// Absent at the bottom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrolled: Option<Scrolled>,
}

/// Rows above the bottom, out of the history's rows.
pub struct Scrolled { pub offset: u32, pub history: u32 }
```

#### `crates/ship-core/src/command.rs` (slices 1 and 2)

```rust
pub enum PaneCommand {
    // …as today, plus:
    /// Scroll a pane's history
    Scroll(Scroll),                        // slice 1
    /// Print a pane's text: the visible screen, or its last lines
    #[serde(skip)]
    Read(Read),                            // slice 2, CLI-only like `get`
}

/// Like `Resize`: the target plus the body's fields, with `deny_unknown_fields`.
pub struct Scroll {
    #[cfg_attr(feature = "clap", command(flatten))]
    #[serde(default)]
    pub pane: PaneTarget,
    /// up, down, top or bottom
    #[cfg_attr(feature = "clap", arg(long))]
    pub direction: ScrollDirection,
    /// Lines for up or down; omitted means one page
    #[cfg_attr(feature = "clap", arg(long))]
    #[serde(default)]
    pub amount: Option<u32>,
}

pub struct Read {
    #[cfg_attr(feature = "clap", command(flatten))]
    pub pane: PaneTarget,
    /// The last N lines, history included; the visible screen when omitted
    #[cfg_attr(feature = "clap", arg(long))]
    pub lines: Option<u32>,
    /// Join soft-wrapped lines
    #[cfg_attr(feature = "clap", arg(long))]
    pub unwrap: bool,
    /// Keep colors and styles as escape sequences
    #[cfg_attr(feature = "clap", arg(long))]
    pub ansi: bool,
}
```

The commands restate the bodies' fields, as `Resize` and `ResizePane` do today. Flattening the body into the command would state them once, but serde can't combine `flatten` with `deny_unknown_fields`, so a misspelled key in a key binding would be silently ignored instead of rejected.

#### `crates/ship-server/src/pane.rs` (slices 1 to 4)

```rust
pub(crate) struct Launch {
    // …as today, plus:
    pub scrollback_bytes: usize,                              // slice 1
}

pub(crate) enum PaneCommand {
    Key(KeyEvent),
    Paste(String),
    Resize(Size),
    Scroll(ScrollPane),                                       // slice 1
    Read(ReadPane, oneshot::Sender<String>),                  // slice 2
    Mouse { input: MouseInput, from: IdOf<Attachment> },      // slice 3
}

/// A copy for clients, on the bus. `to: None` is every viewer of `pane`.  // slice 4
#[derive(Clone)]
pub(crate) struct Clipboard { pub pane: IdOf<Pane>, pub to: Option<IdOf<Attachment>>, pub text: String }
```

`PaneTask::command` still drops keys, pastes and resizes after the program exits, but forwards scroll, read and mouse commands (architecture decision 2). `capture` copies `Rendered.scrolled` into `Screen.scrolled`. `publish` turns `Copied` into `Clipboard { to: Some(last mouse sender) }` and `ClipboardWrite` into `Clipboard { to: None }`. `PaneEnv::shell` becomes `PaneEnv::settings`, which reads `[server]` once per spawn for both the shell and `scrollback_bytes`.

#### `crates/ship-server/src/settings.rs` (slice 1)

```rust
pub struct ServerSettings {
    pub shell: Option<PathBuf>,
    /// FR-005. Default 10,000,000.
    #[serde(default = "default_scrollback_bytes")]
    pub scrollback_bytes: usize,
}
```

#### `crates/ship-server/src/routes.rs` and `lib.rs` (slices 1 and 2)

```rust
/// POST /api/v0/panes/{id}/scroll, 204. Through `live`, not the state actor.
pub(crate) async fn scroll_pane(State(app): App, Path(id): Path<IdOf<Pane>>, Json(body): Json<ScrollPane>) -> Result<StatusCode>;

/// GET /api/v0/panes/{id}/text, 200 text/plain. Exited panes answer too.
pub(crate) async fn read_pane(State(app): App, Path(id): Path<IdOf<Pane>>, Query(read): Query<ReadPane>) -> Result<String>;
```

Both are registered in `api_router`.

#### `crates/ship-server/src/input.rs` and `attach.rs` (slices 3 and 4)

`route` takes the attachment and turns `InputFrame::Mouse` into `PaneCommand::Mouse { input, from }`.

`Progress` subscribes to `Clipboard` on the bus when the stream starts. It sends `SseEvent::Clipboard` when the copy is addressed to its own attachment, or when `to` is `None` and the pane is one it is watching.

#### `crates/ship-client` (slices 1 to 4)

- **`api.rs`:** `scroll` and `read` (text).
- **`execute.rs`:**
  - `Outcome::Text(String)`, which `ship/src/commands.rs` prints raw.
  - Arms for `Scroll` and `Read`.
- **`keymap.rs`:** `ClientSettings.mouse: bool`, default `true`.
- **`keymap/defaults.toml`:** the two bindings above.
- **`ui/terminal.rs`:** `TerminalGuard::set_mouse(bool)` sends `EnableMouseCapture` or `DisableMouseCapture` and records the state `mouse()` reports, and `restore` disables it. `TerminalGuard::copy(&str)` writes crossterm's `CopyToClipboard`.
- **`ui/mod.rs`:**
  - `Event::Mouse` goes through `Mouse::route` when `TerminalGuard` reports capture on. `Select` sets `chosen`, a tab landing through `Memory`, and `Fold` calls `Sidebar::toggle`.
  - `SseEvent::Clipboard` calls `guard.copy`.
  - A reload applies `set_mouse` and drops a held press.
- **`ui/draw.rs`:** `split(area, sidebar_width)` gives the sidebar and pane areas, shared by drawing, hit-testing and `tab_area`.
- **`ui/sidebar.rs`:** `draw` takes `&mut self` and keeps the last `TreeState`. `row_at(Position) -> Option<Row>` reads `rendered_at` and answers `Row { tab, marker }`, where `marker` says the fold marker was hit (the marker symbol's columns after the highlight symbol and `2 × depth` of indent), or `None` below the rows. The tree's symbols are constants, so the widths come from what is drawn. `Mouse::route` turns a row into `Fold` or `Select`.
- **`ui/panes.rs`:** draws `Screen.scrolled` as `[offset/history]` at the frame's top right, or at the content's top-right corner without a border (FR-006, wording open under U-4).

### Untouched

- **Ship's crate layout and the transport:** unchanged. SSE out, NDJSON in, zstd.
- **Copy mode, search and edit-scrollback:** not here, and tracked by issues filed in slice 5. `Pane`, `Layout` and the state actor are unchanged.
- **Layout edits:** split, close, swap and resize logic unchanged.
- **`ship-macros`, `health.rs` and `app.rs`:** unchanged.

## Build order

Each slice runs on Cyan's request and stops for review before the next. Checks run on macOS, and Linux is reported unverified unless run there.

1. **History and scrolling by keys.**
   - Files:
     - Wrapper `session.rs`, `PROVENANCE.md` (changes 12, 13 and the outlive-EOF part of 15).
     - `ship-core` `protocol.rs`, `screen.rs` and `command.rs`.
     - `ship-server` `settings.rs`, `pane.rs`, `routes.rs` and `lib.rs`.
     - `ship-client` `api.rs`, `execute.rs`, `keymap/defaults.toml` and `ui/panes.rs`.
   - Delivers: FR-001 to FR-008 by keys and CLI, and P1 apart from the wheel.
   - Checks:
     - Build, fmt and Clippy. The wrapper's own tests pass, run in a copy outside the repo as before.
     - `seq 5000` in a 200-column pane, then `ship pane scroll --direction top`, shows line 1. `seq 50000` prunes older rows within the 10 MB cap (approved retention-check adjustment below).
     - `alt-pageup` scrolls one page and the indicator appears. A second client shows the same lines. While scrolled, `yes | head -1000` doesn't move the view. Typing returns to the bottom, and the key reaches the shell.
     - `scrollback_bytes = 20000` limits a new pane's history; existing panes keep theirs.
     - `ship pane scroll --direction up` on `nvim` does nothing.
     - After `exit` in a shell, `ship pane scroll --direction top` on the exited pane still scrolls.
     - Measure U-2: RSS of one pane before and after filling 10 MB of history, recorded in the deviation log.

2. **`ship pane read`.**
   - Files: wrapper `session.rs` (change 14); `ship-core` `protocol.rs` and `command.rs`; `ship-server` `pane.rs`, `routes.rs` and `lib.rs`; `ship-client` `api.rs` and `execute.rs`; `ship/src/commands.rs`.
   - Delivers: FR-028 and FR-029, P5 and SC-005.
   - Checks:
     - P5's test: print 200 numbered lines, scroll up, and `ship pane read --lines 50` prints lines 151 to 200.
     - `--ansi` keeps a red `ls --color` entry red when piped to `cat`. `--unwrap` joins a 300-character line in an 80-column pane.
     - With `nvim` open, `read` gives nvim's screen only. An exited pane still reads.
     - An agent pane runs `cargo test` in a sibling pane and reads its result unaided (SC-005).

3. **The mouse.**
   - Files: wrapper `session.rs` and `input.rs` (change 15's pointer part); `ship-core` `protocol.rs`; `ship-server` `pane.rs` and `input.rs`; `ship-client` `keymap.rs`, `ui/mouse.rs`, `ui/terminal.rs`, `ui/sidebar.rs`, `ui/draw.rs` and `ui/mod.rs`.
   - Delivers: FR-009 to FR-013, P3, and SC-003 apart from selection.
   - Checks:
     - Clicking between three panes selects each one, and the click doesn't reach the shell (no stray `^[[<0;…M`).
     - Clicking sidebar rows selects tabs, and clicking `▾` folds a row.
     - In `nvim` with `mouse=a`, a click moves its cursor and the wheel scrolls its buffer.
     - In `less`, the wheel scrolls by arrows. In a shell, it scrolls history.
     - The wheel over an unselected pane scrolls it without selecting it (G2).
     - `mouse = false` takes effect on save, and Shift-free drags select in the outer terminal again.
     - Pi under the mouse behaves as in the outer terminal (SC-003).

4. **Selection and the clipboard.**
   - Files:
     - Wrapper `session.rs`, `widget.rs` and `PROVENANCE.md` (change 16 and the gesture part of 15).
     - `ship-core` `protocol.rs`.
     - `ship-server` `pane.rs` and `attach.rs`.
     - `ship-client` `ui/terminal.rs`, `ui/mod.rs` and the manifest (`osc52`).
   - Delivers: FR-023 to FR-027, P4 and SC-004.
   - Checks:
     - Dragging across a shell line highlights it for both clients, and releasing puts it on the macOS clipboard (`pbpaste`). A double-click takes a word and a triple-click a line.
     - Dragging past the top edge scrolls and keeps selecting. A click without a drag clears the highlight and leaves the clipboard alone.
     - Typing clears the highlight, and output arriving keeps it on its text.
     - `"+y` in `nvim` reaches the clipboard of both clients viewing the tab, but not a client on another tab.
     - `printf '\e]52;c;?\a'` gets no answer.
     - Linux: in foot over SSH, as in the U-3 check, a drag reaches the Linux clipboard (`wl-paste`). Otherwise reported unverified.

5. **Docs, issues and size.**
   - Files: `README.md` (keys, commands, settings), `AGENTS.md` ("viewing position"), `docs/product-brief.md` (milestone 1's search note), `openspec/changes/scrollback/design.md`.
   - Delivers: the "changes to other documents" in the product paper.
   - Checks:
     - Five tracking issues drafted from the template:
       - Copy mode, with the probe's findings.
       - Search, with the probe's findings.
       - The 10 KB history default.
       - Edit-scrollback, carrying the product requirements deferred on 2026-10-11, the leaf-with-cover model, layout commands on a covered pane acting on its slot, sizes left alone while covered, `NamedTempFile` and `shell-words`, and what tmux, Zellij, Herdr and kitty do.
       - A bindable run action with the target pane in its environment, plus `ship clipboard copy` reusing slice 4's clipboard route, with tmux, Zellij and Herdr as precedent for running it on the server.
     - Issues are filed only after Cyan approves their text.
     - Implementation Rust and test line counts reported separately against the estimate.

## Deviation log

### Slice 1: retention check adjusted with Cyan's approval

A macOS live probe used an owned debug server, a 200-column by 24-row attachment, the default `scrollback_bytes = 10000000`, and `seq 50000`. Scrolling to the top showed line `44507`; `Screen.scrolled` reported `offset = 5472`, `history = 5472`. A second run reproduced this. Both temporary servers and their panes were stopped.

The original requirement that this cap retain line 1 of 50,000 rows was incorrect. The product probe's about 38,600 rows at 200 columns fitting in 64 MB also does not support it. Cyan approved keeping 10 MB and revising the retention check through the slice-1 questionnaire: "Keep 10 MB (Recommended)". The check now uses `seq 5000` for line-1 retention and checks pruning separately with `seq 50000`. No increase to the default cap is authorized. The byte cap bounds storage, not a fixed row count. The pinned libghostty-vt's `Terminal::set_scrollback_max_bytes` documentation calls it an estimate, with pruning at page granularity of about 400 KB. Thus a 20,000-byte setting can retain more than 20 KB, but still reduces new panes' history substantially; the live check retained 235 history rows instead of about 5,500. Ship uses this native cap rather than adding its own pruning mechanism.

The spawn implementation reads settings once inside `pane::spawn`, rather than adding a redundant cap field to `Launch`: the configured shell and cap come from that same read, including for explicit programs.

### Slice 1: U-2 RSS evidence

On macOS, an owned debug server held one 200-column by 24-row pane, with one attach stream and no TUI. Its shell waited on a FIFO before each fill. Server RSS from `ps -o rss=`:

| State | RSS, KiB | Increase from baseline, bytes |
| --- | ---: | ---: |
| Pane waiting before output | 31,648 | 0 |
| After `seq 50000`, history full | 41,552 | 10,141,696 |
| After another `seq 100000` | 41,648 | 10,240,000 |

At the first fill, the oldest retained line was `44506`, with 5,473 history rows. At the second fill it was `94465`, with 5,514 history rows. RSS is close to the 10,000,000-byte cap and levels off with further output. This is process RSS, including incidental runtime/render/transport allocations, not an isolated Ghostty allocator measurement. Probe: `/tmp/ship-scrollback-rss.py`; server log: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-scroll-slice1-rv_xvhe9/server.log`. The owned server and pane were stopped. Linux is unverified.

### Slice 1: verification

Debug and release workspace builds, workspace Clippy with warnings denied, formatting, and ship-core checks with and without `clap` passed on macOS. The wrapper's existing tests passed in an outside copy: 80 passed, one pre-existing crash test ignored.

Temporary probe `/tmp/ship-scrollback-slice1.py` exercised real HTTP/CLI requests, two independent attach streams, and a real client under a PTY. It verified retained line 1 for 5,000 rows at 200 columns; all four scroll directions, explicit amounts and page defaults; shared screens; a stable scrolled viewport during `yes | head -1000`; key input returning to the bottom and reaching the program; scrolling after exit; the saved 20,000-byte setting affecting only newly spawned panes; Neovim ignoring history scroll; real Alt-PageUp/PageDown input and the borderless indicator; the bordered indicator on two real clients; typing and bracketed paste snapping to the bottom and reaching the shell; malformed config falling back to defaults with a logged error; detaching; and rejection of a binding's misspelled `direciton`. The probe's PTY output is drained continuously so a full pipe cannot stall the client. Servers, clients and panes created by the checks were cleaned up. Final successful live-run log: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-scroll-slice1-xf6e7mdm/server.log`. The indicator uses the program paper's provisional `[offset/history]` form; U-4 rendering review remains open, and actual Ghostty/Kitty/macOS outer-terminal delivery of the Alt chords remains unverified under U-3. The PTY check verifies Ship receiving and handling their escape sequences, not those terminals' own key handling.

### Slice 3: implementation and verification

Mouse routing, input frames, capture/reload/restoration, hit-testing and sidebar clicks are implemented. [Slice-3 evidence](../evidence-slice-3.md) records the macOS builds, 80 passing wrapper tests, live client/NDJSON checks, Neovim, less, Pi in regular mode, and Cyan's confirmation of real mouse-wheel scrolling. It also records the temporary probe's cleanup correction and the sidebar fix needed to keep explicit ancestor folds from immediately reopening. Gesture/clipboard work stays in slice 4.

After review, Cyan approved reshaping the client's routing: `row_at` returns a sidebar-local `Row` instead of `Clicked`, `route` returns `Option<Clicked>` without a `None` variant, and the sketches above now say so.

Tasks 3.1 to 3.5 are checked. Cyan confirmed task 3.3's physical drag check from foot on Linux, over SSH to the Mac running the client: `mouse = false` returns Shift-free drag selection to the outer terminal, `true` takes it back, and detaching leaves no mouse escapes. Building and running Ship on Linux remains unverified.

### Slice 4: implementation and verification

Selection, highlight, copy on release and program clipboard writes are implemented as sketched. [Slice-4 evidence](../evidence-slice-4.md) records the macOS checks. Departures from the sketches:

- The wrapper sets Ghostty's selection on each press, drag and autoscroll tick rather than only on release, so every viewer sees the highlight while a drag runs (FR-025).
- A cell drag spans from the pressed cell to the pointed cell inclusively in either direction. Ghostty's own cell drag includes those cells depending on where in them the pointer sits, which pointers in whole cells can't express. Word and line drags keep Ghostty's result.
- Only program writes to the standard clipboard are forwarded. Writes to the primary or selection clipboards, such as Neovim's `"*y`, are dropped, since the client writes only the standard clipboard.
- The pane task's trace log of each pointer's attachment is replaced by remembering that attachment for the next copy.

Tasks 4.1 to 4.4 are checked. Cyan confirmed that a drag's copy reaches the system clipboard on macOS and from Linux over SSH, and that dragging to a lone pane's edge autoscrolls, so no edge-row rule was added.

