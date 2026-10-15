//! A more opiniated wrapper around ghostty types.
//!
//! For full control use the conversion and render helpers with raw ghostty
//! types.
//!
//! This implements a more convenient API with the following tradeoffs
//! - Spawns a background thread to handle all terminal related processing.
//! - Notifies the a rerender is possible with the wake callback.
//! - As ghostty types dont implement Sync we keep a ratatui Buffer behind a mutex, and thus storing the current viewport twice.

use std::cell::RefCell;
use std::error::Error;
use std::io::{self, Read as IoRead, Write};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use std::{fmt, mem, panic};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::colors::{HostColors, Rgb};
use crate::input::{self, IntoKeyInput, IntoMouseInput, KeyInput, MouseInput};
use crate::widget::{CursorState, TerminalWidget};
use crossbeam_channel::{Receiver, Sender};
use libghostty_vt::fmt::{Format, Formatter, FormatterOptions};
use libghostty_vt::render::RenderState;
use libghostty_vt::screen::{GridRef, Screen};
use libghostty_vt::selection::gesture::{
    Autoscroll, AutoscrollTickEvent, Behavior, DragEvent, Geometry, Gesture, PressEvent,
    ReleaseEvent,
};
use libghostty_vt::selection::{FormatOptions, Selection};
use libghostty_vt::terminal::{
    ClipboardLocation, ColorScheme, CursorStyle, Mode, ScrollViewport, Terminal,
};
use libghostty_vt::terminal::{Point, PointCoordinate};
use libghostty_vt::{key, mouse};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

/// Most output the reader holds before waiting for the session thread to take it.
const PENDING_CAP: usize = 1 << 20;

/// How often a selection dragged past the top or bottom scrolls one row.
const AUTOSCROLL: Duration = Duration::from_millis(50);

/// Longest gap between presses that still counts as a double or triple click.
const REPEAT_CLICK: Duration = Duration::from_millis(500);

/// Ghostty's gestures work in surface pixels; pointers arrive in cells, so
/// each cell stands for a square this many pixels wide, and a pointer for
/// its center.
const CELL: u32 = 4;

/// Callback used to resize the PTY when the terminal dimensions change.
pub type Resizer = Box<dyn FnMut(u16, u16) -> Result<(), Box<dyn Error + Send + Sync>> + Send>;

/// I/O handles for the child process.
///
/// The session thread reads PTY output from `reader`, writes encoded input to
/// `writer`, and calls `resizer` when the terminal dimensions change.
pub struct SessionIo {
    pub reader: Box<dyn IoRead + Send>,
    pub writer: Box<dyn Write + Send>,
    pub resizer: Resizer,
}

/// Configuration for spawning a [`SessionHandle`].
pub struct SessionConfig {
    /// Host terminal colors forwarded into the child process. `None` skips color initialization.
    pub colors: Option<HostColors>,
    /// History cap in bytes; `None` keeps Ghostty's default. No line cap.
    pub scrollback_bytes: Option<usize>,
    /// Number of lines scrolled per mouse wheel tick.
    pub scroll_delta: isize,
    /// Whether the session starts with keyboard focus. Defaults to `true`.
    pub focused: bool,
    /// Size of the buffer used to read PTY output, in bytes.
    pub read_buffer_size: usize,
    /// Render only for [`SessionHandle::render`], so `wake` means the terminal
    /// changed rather than that a render landed. Defaults to `false`.
    pub render_on_request: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            colors: None,
            scrollback_bytes: None,
            scroll_delta: 3,
            focused: true,
            read_buffer_size: 4096,
            render_on_request: false,
        }
    }
}

/// Error returned by session operations.
#[derive(Debug)]
pub enum SessionError {
    /// Failed to initialize or operate the terminal emulator.
    Terminal(libghostty_vt::Error),
    /// An I/O error occurred (e.g. broken PTY pipe).
    Io(io::Error),
    /// The PTY resize callback failed.
    Resize(Box<dyn Error + Send + Sync>),
    /// The session thread panicked.
    Panicked,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Terminal(e) => write!(f, "terminal error: {e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Resize(e) => write!(f, "resize error: {e}"),
            Self::Panicked => write!(f, "session thread panicked"),
        }
    }
}

impl Error for SessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Terminal(e) => Some(e),
            Self::Io(e) => Some(e),
            Self::Resize(e) => Some(e.as_ref()),
            Self::Panicked => None,
        }
    }
}

impl From<libghostty_vt::Error> for SessionError {
    fn from(e: libghostty_vt::Error) -> Self {
        Self::Terminal(e)
    }
}

impl From<io::Error> for SessionError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Events emitted by a running session.
#[derive(Debug)]
pub enum SessionEvent {
    /// Terminal bell triggered.
    Bell,
    /// Window title changed.
    TitleChanged(String),
    /// Working directory changed.
    CwdChanged(String),
    /// Child process exited or session thread terminated normally.
    Exited,
    /// Session thread encountered an unrecoverable error.
    Error(SessionError),
    /// A mouse selection ended with text in it: plain, soft wraps joined.
    Copied(String),
    /// The program wrote the standard clipboard, OSC 52 already decoded.
    ClipboardWrite(String),
}

/// Builder for constructing and spawning a [`SessionHandle`].
///
/// Created via [`SessionHandle::builder`].
pub struct SessionBuilder<W> {
    config: SessionConfig,
    cols: u16,
    rows: u16,
    wake: W,
}

impl<W: Fn() + Send + 'static> SessionBuilder<W> {
    /// Set the full session configuration, replacing all defaults.
    pub fn config(mut self, config: SessionConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the host terminal colors.
    pub fn colors(mut self, colors: HostColors) -> Self {
        self.config.colors = Some(colors);
        self
    }

    /// Set the history cap in bytes, with no line cap.
    pub fn scrollback_bytes(mut self, bytes: usize) -> Self {
        self.config.scrollback_bytes = Some(bytes);
        self
    }

    /// Set the number of lines scrolled per mouse wheel tick.
    pub fn scroll_delta(mut self, delta: isize) -> Self {
        self.config.scroll_delta = delta;
        self
    }

    /// Set whether the session starts with keyboard focus.
    pub fn focused(mut self, focused: bool) -> Self {
        self.config.focused = focused;
        self
    }

    /// Set the PTY read buffer size in bytes.
    pub fn read_buffer_size(mut self, size: usize) -> Self {
        self.config.read_buffer_size = size;
        self
    }

    /// Render only when asked through [`SessionHandle::render`].
    pub fn render_on_request(mut self, on_request: bool) -> Self {
        self.config.render_on_request = on_request;
        self
    }

    /// Spawn the session with the given I/O handles.
    ///
    /// Consumes the builder and returns a running [`SessionHandle`].
    #[must_use = "dropping the handle immediately shuts down the session"]
    pub fn spawn(self, io: SessionIo) -> Result<SessionHandle, SessionError> {
        SessionHandle::spawn(self.config, io, self.cols, self.rows, self.wake)
    }
}

/// Handle to a running terminal session.
///
/// The session runs in a background thread that reads from the PTY reader and
/// processes commands sent via the `send_*` methods. The provided `wake`
/// callback is invoked from the thread whenever new output or events are
/// available, allowing the caller to unblock and redraw.
///
/// Dropping the handle sends a shutdown signal to the thread. The reader
/// thread exits when its channel is dropped (after the main session thread
/// finishes), so no explicit join is needed.
pub struct SessionHandle {
    cmd_tx: Sender<SessionCommand>,
    event_rx: mpsc::Receiver<SessionEvent>,
    shared: Arc<SharedBuffer>,
    alive: Arc<AtomicBool>,
}

impl SessionHandle {
    /// Create a builder for configuring and spawning a session.
    ///
    /// `cols` and `rows` set the initial terminal dimensions.
    /// `wake` is called from the session thread whenever there is new output or
    /// events to consume.
    pub fn builder<W: Fn() + Send + 'static>(cols: u16, rows: u16, wake: W) -> SessionBuilder<W> {
        SessionBuilder {
            config: SessionConfig::default(),
            cols,
            rows,
            wake,
        }
    }

    /// Spawn a new terminal session with the given dimensions.
    ///
    /// `wake` is called from the session thread whenever there is new output or
    /// events to consume, allowing the caller to unblock and redraw.
    #[must_use = "dropping the handle immediately shuts down the session"]
    pub fn spawn<W>(
        config: SessionConfig,
        io: SessionIo,
        cols: u16,
        rows: u16,
        wake: W,
    ) -> Result<Self, SessionError>
    where
        W: Fn() + Send + 'static,
    {
        let shared = Arc::new(SharedBuffer {
            render: Mutex::new(RenderSnapshot {
                buffer: Buffer::empty(Rect::new(0, 0, cols, rows)),
                cursor: CursorState::default(),
                scrolled: None,
                mouse: false,
            }),
            dirty: AtomicBool::new(true),
            cols: AtomicU16::new(cols),
            rows: AtomicU16::new(rows),
        });
        let alive = Arc::new(AtomicBool::new(true));

        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<SessionCommand>();
        let (event_tx, event_rx) = mpsc::channel::<SessionEvent>();

        let SessionConfig {
            colors,
            scrollback_bytes,
            scroll_delta,
            focused,
            read_buffer_size,
            render_on_request,
        } = config;
        let SessionIo {
            reader,
            writer,
            mut resizer,
        } = io;

        let shared_clone = Arc::clone(&shared);
        let alive_clone = Arc::clone(&alive);

        thread::Builder::new()
            .name("session".into())
            .spawn(move || {
                let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    session_thread(
                        ThreadConfig {
                            cols,
                            rows,
                            scrollback_bytes,
                            colors,
                            scroll_delta,
                            focused,
                            read_buffer_size,
                            render_on_request,
                        },
                        writer,
                        &mut resizer,
                        reader,
                        cmd_rx,
                        SessionContext {
                            shared: &shared_clone,
                            event_tx: &event_tx,
                            wake: &wake,
                        },
                    )
                }));

                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        let _ = event_tx.send(SessionEvent::Error(e));
                        wake();
                    }
                    Err(_) => {
                        let _ = event_tx.send(SessionEvent::Error(SessionError::Panicked));
                        wake();
                    }
                }

                alive_clone.store(false, Ordering::Relaxed);
                let _ = event_tx.send(SessionEvent::Exited);
                wake();
            })
            .expect("failed to spawn session thread");

        Ok(Self {
            cmd_tx,
            event_rx,
            shared,
            alive,
        })
    }

    /// Send a keyboard event to the terminal.
    pub fn send_key(&self, event: impl IntoKeyInput) {
        let _ = self
            .cmd_tx
            .send(SessionCommand::Key(event.into_key_input()));
    }

    /// Route a pane-relative pointer against the terminal's live modes.
    pub fn send_pointer(&self, pointer: Pointer) {
        let _ = self.cmd_tx.send(SessionCommand::Pointer(pointer));
    }

    /// Send a mouse event with session-relative coordinates.
    ///
    /// Returns `false` if the event could not be converted (e.g. unsupported mouse action).
    pub fn send_mouse(&self, event: impl IntoMouseInput) -> bool {
        if let Some(input) = event.into_mouse_input() {
            let _ = self.cmd_tx.send(SessionCommand::Mouse(input));
            true
        } else {
            false
        }
    }

    /// Notify the terminal of focus gain or loss.
    pub fn send_focus(&self, gained: bool) {
        let _ = self.cmd_tx.send(SessionCommand::Focus(gained));
    }

    /// Resize the terminal to the given dimensions.
    pub fn send_resize(&self, cols: u16, rows: u16) {
        let _ = self.cmd_tx.send(SessionCommand::Resize { cols, rows });
    }

    /// Scroll the viewport by `delta` lines (negative scrolls up).
    pub fn send_scroll(&self, delta: isize) {
        self.send_scroll_to(Scroll::Lines(delta));
    }

    /// Move the viewport. Alternate-screen sessions ignore this command.
    pub fn send_scroll_to(&self, scroll: Scroll) {
        self.cmd_tx.send(SessionCommand::Scroll(scroll)).ok();
    }

    /// Read from the active screen's bottom without moving the viewport.
    /// A failed read or stopped session drops `reply` without calling it.
    pub fn read(&self, read: Read, reply: Box<dyn FnOnce(String) + Send>) {
        self.cmd_tx.send(SessionCommand::Read(read, reply)).ok();
    }

    /// Render what changed since the last render, then call `reply`; take the
    /// result with [`Self::take_if_dirty`]. A stopped session drops `reply`
    /// without calling it.
    pub fn render(&self, reply: Box<dyn FnOnce() + Send>) {
        self.cmd_tx.send(SessionCommand::Render(reply)).ok();
    }

    /// Paste data into the terminal, wrapping in bracketed paste sequences when enabled.
    pub fn send_paste(&self, data: Vec<u8>) {
        let _ = self.cmd_tx.send(SessionCommand::Paste(data));
    }

    /// Write raw bytes directly to the PTY without any encoding or wrapping.
    pub fn send_raw(&self, data: Vec<u8>) {
        let _ = self.cmd_tx.send(SessionCommand::Raw(data));
    }

    /// Send a shutdown signal to the session thread.
    pub fn send_shutdown(&self) {
        let _ = self.cmd_tx.send(SessionCommand::Shutdown);
    }

    /// Returns `true` if the session thread is still running.
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Returns `true` if the session has new rendered output since the last [`Self::mark_clean`] call.
    pub fn is_dirty(&self) -> bool {
        self.shared.dirty.load(Ordering::Relaxed)
    }

    /// Clears the dirty flag after consuming the rendered buffer.
    pub fn mark_clean(&self) {
        self.shared.dirty.store(false, Ordering::Relaxed);
    }

    /// Polls for the next pending session event without blocking.
    pub fn poll_event(&self) -> Option<SessionEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Returns the current terminal dimensions as `(cols, rows)`.
    pub fn size(&self) -> (u16, u16) {
        (
            self.shared.cols.load(Ordering::Relaxed),
            self.shared.rows.load(Ordering::Relaxed),
        )
    }

    /// Copies the session's rendered cells into `dest` at `dest_area`.
    ///
    /// Only cells within both the session buffer and `dest_area` bounds are written.
    /// Call [`Self::mark_clean`] after blitting to acknowledge the update.
    pub fn blit_to(&self, dest: &mut Buffer, dest_area: Rect) {
        let snap = self.shared.render.lock().unwrap_or_else(|e| e.into_inner());
        snap.blit_to(dest, dest_area);
    }

    /// Copies the rendered cells into `dest` like [`Self::blit_to`] and returns
    /// the cursor, clearing the dirty flag, all under one lock. `None`, with
    /// `dest` untouched, when nothing was rendered since the last clear.
    ///
    /// Renders set the flag under the same lock, so a render the copy missed
    /// leaves it set, and a render it saw can't leave it set.
    pub fn take_if_dirty(&self, dest: &mut Buffer, dest_area: Rect) -> Option<Rendered> {
        let snap = self.shared.render.lock().unwrap_or_else(|e| e.into_inner());
        if !self.shared.dirty.swap(false, Ordering::Relaxed) {
            return None;
        }
        snap.blit_to(dest, dest_area);
        Some(Rendered {
            cursor: snap.cursor.clone(),
            scrolled: snap.scrolled.clone(),
            mouse: snap.mouse,
        })
    }

    /// Returns the current cursor state (position relative to the session, style, blinking).
    pub fn cursor_state(&self) -> CursorState {
        self.shared
            .render
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .cursor
            .clone()
    }
}

/// Widget that renders a [`SessionHandle`]'s current buffer into a ratatui frame.
///
/// This is a non-owning view; the session continues running independently.
pub struct SessionWidget<'a> {
    session: &'a SessionHandle,
}

impl<'a> SessionWidget<'a> {
    pub fn new(session: &'a SessionHandle) -> Self {
        Self { session }
    }
}

impl Widget for SessionWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.session.blit_to(buf, area);
    }
}

impl Drop for SessionHandle {
    fn drop(&mut self) {
        self.send_shutdown();
    }
}

/// Where to move the viewport; negative deltas scroll up.
pub enum Scroll {
    Lines(isize),
    Pages(isize),
    Top,
    Bottom,
}

/// What to read from the active screen, independently of the viewport.
pub struct Read {
    /// Last N rows including history; `None` reads the active grid.
    pub lines: Option<u32>,
    pub unwrap: bool,
    pub ansi: bool,
}

/// Distance above the bottom, absent when viewing the active screen.
#[derive(Clone, Debug)]
pub struct Scrolled {
    pub offset: u32,
    pub history: u32,
}

/// Metadata captured with the rendered cells under the same lock.
pub struct Rendered {
    pub cursor: CursorState,
    pub scrolled: Option<Scrolled>,
    /// Mouse tracking when these cells were rendered.
    pub mouse: bool,
}

/// Pane-relative coordinates, including positions outside a dragged pane.
#[derive(Debug, Clone)]
pub struct Pointer {
    pub kind: MouseEventKind,
    pub modifiers: KeyModifiers,
    pub x: i32,
    pub y: i32,
}

enum SessionCommand {
    Key(KeyInput),
    Mouse(MouseInput),
    Pointer(Pointer),
    Focus(bool),
    Resize { cols: u16, rows: u16 },
    Scroll(Scroll),
    Read(Read, Box<dyn FnOnce(String) + Send>),
    Render(Box<dyn FnOnce() + Send>),
    Paste(Vec<u8>),
    Raw(Vec<u8>),
    Shutdown,
}

struct RenderSnapshot {
    buffer: Buffer,
    cursor: CursorState,
    scrolled: Option<Scrolled>,
    mouse: bool,
}

impl RenderSnapshot {
    fn blit_to(&self, dest: &mut Buffer, dest_area: Rect) {
        let w = self.buffer.area().width.min(dest_area.width);
        let h = self.buffer.area().height.min(dest_area.height);
        for y in 0..h {
            for x in 0..w {
                let dx = dest_area.x + x;
                let dy = dest_area.y + y;
                if dx < dest.area().right() && dy < dest.area().bottom() {
                    dest[(dx, dy)] = self.buffer[(x, y)].clone();
                }
            }
        }
    }
}

/// Output the reader thread has read and the session thread hasn't written to
/// the terminal yet. Reads append to one buffer, so a flood of small PTY reads
/// wakes the session thread once per batch rather than once per read.
#[derive(Default)]
struct Pending {
    queue: Mutex<Queue>,
    /// Signaled when the session thread takes the bytes or stops.
    taken: Condvar,
}

#[derive(Default)]
struct Queue {
    bytes: Vec<u8>,
    /// The session thread stopped; the reader exits.
    closed: bool,
}

impl Pending {
    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn len(&self) -> usize {
        self.lock().bytes.len()
    }

    /// Swaps the waiting bytes into `batch`, which is empty, so the two
    /// buffers trade places and neither reallocates.
    fn take(&self, batch: &mut Vec<u8>) {
        mem::swap(&mut self.lock().bytes, batch);
        self.taken.notify_one();
    }
}

/// Closes the reader's queue when the session thread ends, however it ends,
/// so a reader waiting at the cap exits.
struct ClosePending(Arc<Pending>);

impl Drop for ClosePending {
    fn drop(&mut self) {
        self.0.lock().closed = true;
        self.0.taken.notify_one();
    }
}

/// What ended the session thread's wait.
enum Woke {
    Output,
    Command(SessionCommand),
    Autoscroll,
    /// The reader stopped: the PTY reached EOF or failed.
    Eof,
}

struct SharedBuffer {
    render: Mutex<RenderSnapshot>,
    dirty: AtomicBool,
    cols: AtomicU16,
    rows: AtomicU16,
}

struct SessionState<'a> {
    terminal: Box<Terminal<'static, 'static>>,
    key_encoder: key::Encoder<'static>,
    mouse_encoder: mouse::Encoder<'static>,
    writer: Rc<RefCell<Box<dyn Write + Send>>>,
    resizer: &'a mut Resizer,
    shared: &'a SharedBuffer,
    focused: bool,
    cols: u16,
    rows: u16,
    scroll_delta: isize,
    events: Rc<RefCell<Vec<SessionEvent>>>,
    selecting: Selecting,
    /// Callers waiting on [`SessionHandle::render`], answered after the next render.
    renders: Vec<Box<dyn FnOnce() + Send>>,
}

/// Left-button text selection through Ghostty's gesture state.
struct Selecting {
    gesture: Gesture<'static>,
    press: PressEvent<'static>,
    drag: DragEvent<'static>,
    release: ReleaseEvent<'static>,
    tick: AutoscrollTickEvent<'static>,
    /// Press times count from here; crossterm's events carry none.
    epoch: Instant,
    /// Where the held button last was, possibly past the grid; `None` when
    /// no press is held.
    held: Option<(i32, i32)>,
}

impl Selecting {
    fn new() -> Result<Self, libghostty_vt::Error> {
        Ok(Self {
            gesture: Gesture::new()?,
            press: PressEvent::new()?,
            drag: DragEvent::new()?,
            release: ReleaseEvent::new()?,
            tick: AutoscrollTickEvent::new()?,
            epoch: Instant::now(),
            held: None,
        })
    }
}

/// Ghostty's cell drag takes the pressed and pointed cells by where in
/// them the pointer is, which a cell-sized pointer can't say, so a cell drag
/// spans both cells whichever way it goes. Word and line drags keep
/// Ghostty's selection.
fn cell_span<'t>(
    gesture: &Gesture<'_>,
    terminal: &'t Terminal<'_, '_>,
    cell: GridRef<'t>,
    selection: Option<Selection<'t>>,
) -> Option<Selection<'t>> {
    if gesture.behavior(terminal).ok()? != Behavior::Cell {
        return selection;
    }
    if !gesture.dragged(terminal).ok()? {
        return None;
    }
    Some(Selection::new(gesture.anchor(terminal).ok()??, cell, false))
}

/// A pointer's position in the gestures' surface pixels.
fn surface(x: i32, y: i32) -> (f64, f64) {
    let center = |cell: i32| f64::from(cell) * f64::from(CELL) + f64::from(CELL / 2);
    (center(x), center(y))
}

impl<'a> SessionState<'a> {
    fn geometry(&self) -> Geometry {
        Geometry {
            columns: u32::from(self.cols),
            cell_width: CELL,
            padding_left: 0,
            screen_height: u32::from(self.rows) * CELL,
        }
    }

    /// The viewport cell nearest a pointer.
    fn viewport(&self, x: i32, y: i32) -> PointCoordinate {
        PointCoordinate {
            x: x.clamp(0, i32::from(self.cols.saturating_sub(1))) as u16,
            y: y.clamp(0, i32::from(self.rows.saturating_sub(1))) as u32,
        }
    }

    /// Press, drag and release of the left button: a press clears the
    /// selection or, repeated, selects a word or line; dragging extends it;
    /// releasing copies it. Errors leave the selection as it was.
    fn select(&mut self, kind: MouseEventKind, x: i32, y: i32, needs_render: &mut bool) {
        let at = self.viewport(x, y);
        let geometry = self.geometry();
        let (px, py) = surface(x, y);
        let terminal = &*self.terminal;
        let s = &mut self.selecting;
        let Ok(cell) = terminal.grid_ref(Point::Viewport(at)) else {
            return;
        };
        match kind {
            MouseEventKind::Down(_) => {
                s.held = Some((x, y));
                let time = s.epoch.elapsed();
                let selection = s
                    .press
                    .set_position(px, py)
                    .and_then(|press| press.set_time(time))
                    .and_then(|press| press.set_repeat_interval(REPEAT_CLICK))
                    .and_then(|press| press.set_repeat_distance(f64::from(CELL)))
                    .and_then(|press| press.apply(&mut s.gesture, terminal, cell));
                if let Ok(selection) = selection {
                    terminal.set_selection(selection.as_ref()).ok();
                    *needs_render = true;
                }
            }
            MouseEventKind::Drag(_) if s.held.is_some() => {
                s.held = Some((x, y));
                let selection = s
                    .drag
                    .set_position(px, py)
                    .and_then(|drag| drag.apply(&mut s.gesture, terminal, cell.clone(), geometry));
                if let Ok(selection) = selection
                    && let Some(selection) = cell_span(&s.gesture, terminal, cell, selection)
                {
                    terminal.set_selection(Some(&selection)).ok();
                    *needs_render = true;
                }
            }
            MouseEventKind::Up(_) if s.held.take().is_some() => {
                s.release.apply(&mut s.gesture, terminal, Some(cell)).ok();
                let options = FormatOptions::new()
                    .with_emit_format(Format::Plain)
                    .with_unwrap(true)
                    .with_trim(true);
                if let Ok(Some(text)) = terminal.format_selection_alloc(None, options)
                    && !text.is_empty()
                {
                    let text = String::from_utf8_lossy(&text).into_owned();
                    self.events.borrow_mut().push(SessionEvent::Copied(text));
                }
            }
            _ => {}
        }
    }

    /// While a held selection is past the top or bottom edge.
    fn autoscrolling(&self) -> bool {
        self.selecting.held.is_some()
            && self
                .selecting
                .gesture
                .autoscroll(&self.terminal)
                .is_ok_and(|autoscroll| autoscroll != Autoscroll::None)
    }

    /// Scroll one row toward the held pointer and extend the selection to
    /// the row now under it.
    fn autoscroll(&mut self, needs_render: &mut bool) {
        let Some((x, y)) = self.selecting.held else {
            return;
        };
        let at = self.viewport(x, y);
        let geometry = self.geometry();
        let (px, py) = surface(x, y);
        let terminal = &*self.terminal;
        let s = &mut self.selecting;
        let selection = s
            .tick
            .set_position(px, py)
            .and_then(|tick| tick.apply(&mut s.gesture, terminal, at, geometry));
        *needs_render = true;
        if let Ok(selection) = selection
            && let Ok(cell) = terminal.grid_ref(Point::Viewport(at))
            && let Some(selection) = cell_span(&s.gesture, terminal, cell, selection)
        {
            terminal.set_selection(Some(&selection)).ok();
        }
    }
    fn input_bottom(&mut self, needs_render: &mut bool) {
        // Usually the view is already at the bottom with nothing selected,
        // so nothing changes and a render would only delay the echo.
        let scrolled = !self.terminal.viewport_active().unwrap_or(false);
        let selected = self.terminal.selection().map_or(true, |s| s.is_some());
        self.terminal.scroll_viewport(ScrollViewport::Bottom);
        self.terminal.set_selection(None).ok();
        *needs_render |= scrolled || selected;
    }

    fn read(&self, read: Read) -> Result<String, libghostty_vt::Error> {
        let total = u32::try_from(self.terminal.total_rows()?)
            .map_err(|_| libghostty_vt::Error::InvalidValue)?;
        let count = read.lines.unwrap_or(u32::from(self.rows)).min(total);
        if count == 0 {
            return Ok(String::new());
        }
        let start = self.terminal.grid_ref(Point::Screen(PointCoordinate {
            x: 0,
            y: total - count,
        }))?;
        let end = self.terminal.grid_ref(Point::Screen(PointCoordinate {
            x: self.cols - 1,
            y: total - 1,
        }))?;
        let selection = Selection::new(start, end, false);
        let options = FormatterOptions::new()
            .with_format(if read.ansi { Format::Vt } else { Format::Plain })
            .with_unwrap(read.unwrap)
            .with_trim(true)
            .with_selection(&selection);
        let mut formatter = Formatter::new(&self.terminal, options)?;
        // Plain and VT output is UTF-8 in Ghostty's formatter (encodeCodepoint),
        // though the API doesn't promise it. Only the extras, which stay off,
        // echo raw program bytes such as OSC 8 URIs.
        Ok(String::from_utf8_lossy(&formatter.format_alloc(None)?).into_owned())
    }

    /// To the program when it tracks the mouse. Otherwise the vertical wheel
    /// scrolls primary-screen history, or sends arrows on the alternate
    /// screen when the program enabled alternate scroll.
    fn mouse(&mut self, input: MouseInput, needs_render: &mut bool) {
        let size = mouse::EncoderSize {
            cell_width: 1,
            cell_height: 1,
            screen_width: self.cols as u32,
            screen_height: self.rows as u32,
            padding_left: 0,
            padding_top: 0,
            padding_right: 0,
            padding_bottom: 0,
        };
        if self.terminal.is_mouse_tracking().unwrap_or(false) {
            if let Ok(data) =
                input::encode_mouse(&mut self.mouse_encoder, &input, &self.terminal, size)
                && !data.is_empty()
            {
                let mut w = self.writer.borrow_mut();
                let _ = w.write_all(&data);
                let _ = w.flush();
            }
            return;
        }
        let (delta, code) = match input.button {
            Some(mouse::Button::Four) => (-self.scroll_delta, KeyCode::Up),
            Some(mouse::Button::Five) => (self.scroll_delta, KeyCode::Down),
            _ => return,
        };
        if self.terminal.active_screen().ok() == Some(Screen::Primary) {
            self.terminal.scroll_viewport(ScrollViewport::Delta(delta));
            *needs_render = true;
        } else if self.terminal.mode(Mode::ALT_SCROLL).unwrap_or(false) {
            let key = KeyEvent::new(code, KeyModifiers::NONE).into_key_input();
            if let Ok(data) = input::encode_key(&mut self.key_encoder, &key, &self.terminal) {
                let mut w = self.writer.borrow_mut();
                for _ in 0..self.scroll_delta.unsigned_abs() {
                    let _ = w.write_all(&data);
                }
                let _ = w.flush();
            }
        }
    }

    fn handle_command(&mut self, cmd: SessionCommand, needs_render: &mut bool) -> bool {
        match cmd {
            SessionCommand::Key(input) => {
                self.input_bottom(needs_render);
                if let Ok(data) = input::encode_key(&mut self.key_encoder, &input, &self.terminal)
                    && !data.is_empty()
                {
                    let mut w = self.writer.borrow_mut();
                    let _ = w.write_all(&data);
                    let _ = w.flush();
                }
            }
            SessionCommand::Pointer(pointer) => {
                let tracking = self.terminal.is_mouse_tracking().unwrap_or(false);
                if let MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left) = pointer.kind
                    && !tracking
                {
                    self.select(pointer.kind, pointer.x, pointer.y, needs_render);
                    return false;
                }
                if tracking && self.selecting.held.take().is_some() {
                    self.selecting.gesture.reset(&self.terminal);
                }
                let event = MouseEvent {
                    kind: pointer.kind,
                    modifiers: pointer.modifiers,
                    column: pointer.x.clamp(0, i32::from(self.cols.saturating_sub(1))) as u16,
                    row: pointer.y.clamp(0, i32::from(self.rows.saturating_sub(1))) as u16,
                };
                if let Some(input) = event.into_mouse_input() {
                    self.mouse(input, needs_render);
                }
            }
            SessionCommand::Mouse(input) => self.mouse(input, needs_render),
            SessionCommand::Focus(gained) => {
                self.focused = gained;
                let data = input::encode_focus(gained);
                let mut w = self.writer.borrow_mut();
                let _ = w.write_all(data);
                let _ = w.flush();
                *needs_render = true;
            }
            SessionCommand::Resize { cols, rows } => {
                let _ = self.terminal.resize(cols, rows, 0, 0);
                let _ = (self.resizer)(cols, rows);
                self.cols = cols;
                self.rows = rows;
                // The render this requests replaces the buffer at the new size.
                self.shared.cols.store(cols, Ordering::Relaxed);
                self.shared.rows.store(rows, Ordering::Relaxed);
                *needs_render = true;
            }
            SessionCommand::Scroll(scroll) => {
                if self.terminal.active_screen().ok() == Some(Screen::Primary) {
                    let to = match scroll {
                        Scroll::Lines(delta) => ScrollViewport::Delta(delta),
                        Scroll::Pages(delta) => {
                            ScrollViewport::Delta(delta.saturating_mul(self.rows as isize))
                        }
                        Scroll::Top => ScrollViewport::Top,
                        Scroll::Bottom => ScrollViewport::Bottom,
                    };
                    self.terminal.scroll_viewport(to);
                    *needs_render = true;
                }
            }
            SessionCommand::Read(read, reply) => {
                if let Ok(text) = self.read(read) {
                    reply(text);
                }
            }
            SessionCommand::Render(reply) => self.renders.push(reply),
            SessionCommand::Paste(data) => {
                self.input_bottom(needs_render);
                let encoded = input::encode_paste(&data, &self.terminal);
                if !encoded.is_empty() {
                    let mut w = self.writer.borrow_mut();
                    let _ = w.write_all(&encoded);
                    let _ = w.flush();
                }
            }
            SessionCommand::Raw(data) => {
                let mut w = self.writer.borrow_mut();
                let _ = w.write_all(&data);
                let _ = w.flush();
            }
            SessionCommand::Shutdown => return true,
        }
        false
    }
}

struct SessionContext<'a> {
    shared: &'a SharedBuffer,
    event_tx: &'a mpsc::Sender<SessionEvent>,
    wake: &'a dyn Fn(),
}

struct ThreadConfig {
    cols: u16,
    rows: u16,
    scrollback_bytes: Option<usize>,
    colors: Option<HostColors>,
    scroll_delta: isize,
    focused: bool,
    read_buffer_size: usize,
    render_on_request: bool,
}

fn session_thread(
    cfg: ThreadConfig,
    writer: Box<dyn Write + Send>,
    resizer: &mut Resizer,
    reader: Box<dyn IoRead + Send>,
    cmd_rx: Receiver<SessionCommand>,
    ctx: SessionContext<'_>,
) -> Result<(), SessionError> {
    let ThreadConfig {
        cols,
        rows,
        scrollback_bytes,
        colors,
        scroll_delta,
        focused,
        read_buffer_size,
        render_on_request,
    } = cfg;

    let SessionContext {
        shared,
        event_tx,
        wake,
    } = ctx;

    let mut terminal = Box::new(Terminal::new(cols, rows)?);
    terminal.set_scrollback_max_lines(None)?;
    if let Some(bytes) = scrollback_bytes {
        terminal.set_scrollback_max_bytes(Some(bytes))?;
    }
    // Ghostty resolves the default cursor (DECSCUSR 0, and the initial state)
    // to a concrete shape and doesn't expose that it's the default. No escape
    // sequence selects a hollow block, so it marks the default here, and the
    // widget reports it as `CursorStyle::Default`.
    terminal.set_default_cursor_style(Some(CursorStyle::BlockHollow))?;

    let events_local: Rc<RefCell<Vec<SessionEvent>>> = Rc::new(RefCell::new(Vec::new()));

    terminal.on_bell({
        let events = Rc::clone(&events_local);
        move |_| {
            events.borrow_mut().push(SessionEvent::Bell);
        }
    })?;

    terminal.on_title_changed({
        let events = Rc::clone(&events_local);
        move |term: &Terminal| {
            if let Ok(t) = term.title() {
                events
                    .borrow_mut()
                    .push(SessionEvent::TitleChanged(t.to_string()));
            }
        }
    })?;

    terminal.on_pwd_changed({
        let events = Rc::clone(&events_local);
        move |term: &Terminal| {
            if let Ok(pwd) = term.pwd() {
                events
                    .borrow_mut()
                    .push(SessionEvent::CwdChanged(pwd.to_string()));
            }
        }
    })?;

    // Only text for the standard clipboard goes on; reads never reach here.
    terminal.on_clipboard_write({
        let events = Rc::clone(&events_local);
        move |_, write| {
            if write.location() == ClipboardLocation::Standard
                && let Some(text) = write
                    .contents()
                    .find(|content| content.mime.starts_with("text/plain"))
            {
                let text = String::from_utf8_lossy(text.data).into_owned();
                events.borrow_mut().push(SessionEvent::ClipboardWrite(text));
            }
            Ok(())
        }
    })?;

    let mut render_state = RenderState::new()?;
    let key_encoder = key::Encoder::new()?;
    let mouse_encoder = mouse::Encoder::new()?;

    let writer: Rc<RefCell<Box<dyn Write + Send>>> = Rc::new(RefCell::new(writer));

    terminal.on_pty_write({
        let writer = Rc::clone(&writer);
        move |_, data| {
            let _ = writer.borrow_mut().write_all(data);
        }
    })?;

    if let Some(ref colors) = colors {
        terminal.on_color_scheme({
            let is_dark = is_dark(colors.background);
            move |_| {
                Some(if is_dark {
                    ColorScheme::Dark
                } else {
                    ColorScheme::Light
                })
            }
        })?;
        init_colors(&mut terminal, colors);
    }

    let pending = Arc::new(Pending::default());
    let _close = ClosePending(Arc::clone(&pending));
    // Carries no data: a token means bytes are waiting, sent when the queue
    // was empty or reaches the cap. Dropped with the reader at EOF.
    let (pty_tx, mut pty_rx) = crossbeam_channel::bounded::<()>(1);

    thread::Builder::new()
        .name("session-reader".into())
        .spawn({
            let pending = Arc::clone(&pending);
            move || {
                let mut buf = vec![0u8; read_buffer_size];
                let mut reader = reader;
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let mut queue = pending.lock();
                            if queue.closed {
                                break;
                            }
                            let was_empty = queue.bytes.is_empty();
                            queue.bytes.extend_from_slice(&buf[..n]);
                            if was_empty || queue.bytes.len() >= PENDING_CAP {
                                pty_tx.try_send(()).ok();
                            }
                            while queue.bytes.len() >= PENDING_CAP && !queue.closed {
                                queue =
                                    pending.taken.wait(queue).unwrap_or_else(|e| e.into_inner());
                            }
                        }
                    }
                }
            }
        })
        .expect("failed to spawn session-reader thread");

    let mut state = SessionState {
        terminal,
        key_encoder,
        mouse_encoder,
        writer,
        resizer,
        shared,
        focused,
        cols,
        rows,
        scroll_delta,
        events: Rc::clone(&events_local),
        selecting: Selecting::new()?,
        renders: Vec::new(),
    };

    // Set by changes that arrive while waiting, rendered on the next pass.
    let mut needs_render = false;
    // With `render_on_request`, the terminal changed, or has output waiting,
    // since the last render.
    let mut stale = false;
    let mut batch = Vec::new();
    let mut woke = None;
    loop {
        // Every pass writes the waiting output first, so a command sees all
        // output read before it, like the modes that change how keys encode.
        pending.take(&mut batch);
        if !batch.is_empty() {
            state.terminal.vt_write(&batch);
            batch.clear();
            needs_render = true;
        }

        let mut shutdown = false;
        let mut should_wake = false;
        match woke.take() {
            Some(Woke::Command(cmd)) => shutdown = state.handle_command(cmd, &mut needs_render),
            Some(Woke::Autoscroll) => state.autoscroll(&mut needs_render),
            Some(Woke::Eof) => {
                pty_rx = crossbeam_channel::never();
                event_tx.send(SessionEvent::Exited).ok();
                should_wake = true;
            }
            Some(Woke::Output) | None => {}
        }
        while !shutdown && let Ok(cmd) = cmd_rx.try_recv() {
            shutdown = state.handle_command(cmd, &mut needs_render);
        }
        if shutdown {
            break;
        }

        if needs_render {
            needs_render = false;
            should_wake = true;
            if render_on_request {
                stale = true;
            } else {
                render_to_shared(&mut state, &mut render_state);
            }
        }
        for ev in events_local.borrow_mut().drain(..) {
            let _ = event_tx.send(ev);
            should_wake = true;
        }

        // Events first, so a caller woken by the reply sees them.
        if !state.renders.is_empty() {
            if stale {
                stale = false;
                render_to_shared(&mut state, &mut render_state);
            }
            state.renders.drain(..).for_each(|reply| reply());
        }

        if should_wake {
            wake();
        }

        // With `render_on_request`, output alone only marks the terminal
        // stale and keeps waiting; the render request writes it.
        woke = Some(loop {
            let autoscroll = if state.autoscrolling() {
                crossbeam_channel::after(AUTOSCROLL)
            } else {
                crossbeam_channel::never()
            };
            crossbeam_channel::select! {
                recv(autoscroll) -> _ => break Woke::Autoscroll,
                recv(cmd_rx) -> msg => match msg {
                    Ok(cmd) => break Woke::Command(cmd),
                    Err(_) => break Woke::Command(SessionCommand::Shutdown),
                },
                recv(pty_rx) -> msg => {
                    if msg.is_err() {
                        break Woke::Eof;
                    }
                    let waiting = pending.len();
                    if !render_on_request || waiting >= PENDING_CAP {
                        break Woke::Output;
                    }
                    if waiting > 0 && !stale {
                        stale = true;
                        wake();
                    }
                },
            }
        });
    }

    Ok(())
}

fn render_to_shared(state: &mut SessionState<'_>, render_state: &mut RenderState<'static>) {
    let area = Rect::new(0, 0, state.cols, state.rows);
    let mut snap = state
        .shared
        .render
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // Clean rows keep their cells from the last render; a new buffer needs them all.
    let resized = snap.buffer.area() != &area;
    if resized {
        snap.buffer = Buffer::empty(area);
    }
    let mut widget = TerminalWidget::new(&mut state.terminal, render_state)
        .focused(state.focused)
        .incremental(!resized);
    (&mut widget).render(area, &mut snap.buffer);
    snap.cursor = widget.cursor().clone();
    snap.mouse = state.terminal.is_mouse_tracking().unwrap_or(false);
    snap.scrolled = state.terminal.scrollbar().ok().and_then(|bar| {
        let history = bar.total.saturating_sub(bar.len);
        let offset = history.saturating_sub(bar.offset);
        (offset > 0).then_some(Scrolled {
            offset: offset.min(u64::from(u32::MAX)) as u32,
            history: history.min(u64::from(u32::MAX)) as u32,
        })
    });
    state.shared.dirty.store(true, Ordering::Relaxed);
}

fn init_colors(terminal: &mut Terminal, colors: &HostColors) {
    let mut buf = Vec::new();
    let fg = colors.foreground;
    let bg = colors.background;
    let _ = write!(
        buf,
        "\x1b]10;rgb:{:02X}/{:02X}/{:02X}\x1b\\",
        fg.r, fg.g, fg.b
    );
    let _ = write!(
        buf,
        "\x1b]11;rgb:{:02X}/{:02X}/{:02X}\x1b\\",
        bg.r, bg.g, bg.b
    );
    for (i, c) in colors.palette.iter().enumerate() {
        let _ = write!(
            buf,
            "\x1b]4;{i};rgb:{:02X}/{:02X}/{:02X}\x1b\\",
            c.r, c.g, c.b
        );
    }
    terminal.vt_write(&buf);
}

fn linearize(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn is_dark(bg: Rgb) -> bool {
    let l = 0.2126 * linearize(bg.r) + 0.7152 * linearize(bg.g) + 0.0722 * linearize(bg.b);
    l < 0.5
}
