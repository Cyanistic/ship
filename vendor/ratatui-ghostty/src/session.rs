//! A more opiniated wrapper around ghostty types.
//!
//! For full control use the conversion and render helpers with raw ghostty
//! types.
//!
//! This implements a more convenient API with the following tradeoffs
//! - Spawns a background thread to handle all terminal related processing.
//! - Notifies the a rerender is possible with the wake callback.
//! - As ghostty types dont implement Sync we keep a ratatui Buffer behind a mutex, and thus storing the current viewport twice.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use crate::colors::HostColors;
use crate::input::{self, IntoKeyInput, IntoMouseInput, KeyInput, MouseInput};
use crate::widget::{CursorState, TerminalWidget};
use crossbeam_channel::{Receiver, Sender};
use libghostty_vt::render::RenderState;
use libghostty_vt::terminal::{ColorScheme, ScrollViewport, Terminal};
use libghostty_vt::{key, mouse};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

/// Longest the session thread drains queued output before rendering. Without
/// it, sustained output keeps the queue full and nothing renders until it stops.
const DRAIN_BUDGET: Duration = Duration::from_millis(8);

/// Callback used to resize the PTY when the terminal dimensions change.
pub type Resizer =
    Box<dyn FnMut(u16, u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send>;

/// I/O handles for the child process.
///
/// The session thread reads PTY output from `reader`, writes encoded input to
/// `writer`, and calls `resizer` when the terminal dimensions change.
pub struct SessionIo {
    pub reader: Box<dyn Read + Send>,
    pub writer: Box<dyn Write + Send>,
    pub resizer: Resizer,
}

/// Configuration for spawning a [`SessionHandle`].
pub struct SessionConfig {
    /// Host terminal colors forwarded into the child process. `None` skips color initialization.
    pub colors: Option<HostColors>,
    /// Maximum number of scrollback lines.
    pub scrollback: usize,
    /// Number of lines scrolled per mouse wheel tick.
    pub scroll_delta: isize,
    /// Whether the session starts with keyboard focus. Defaults to `true`.
    pub focused: bool,
    /// Size of the buffer used to read PTY output, in bytes.
    pub read_buffer_size: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            colors: None,
            scrollback: 1000,
            scroll_delta: 3,
            focused: true,
            read_buffer_size: 4096,
        }
    }
}

/// Error returned by session operations.
#[derive(Debug)]
pub enum SessionError {
    /// Failed to initialize or operate the terminal emulator.
    Terminal(libghostty_vt::Error),
    /// An I/O error occurred (e.g. broken PTY pipe).
    Io(std::io::Error),
    /// The PTY resize callback failed.
    Resize(Box<dyn std::error::Error + Send + Sync>),
    /// The session thread panicked.
    Panicked,
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Terminal(e) => write!(f, "terminal error: {e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Resize(e) => write!(f, "resize error: {e}"),
            Self::Panicked => write!(f, "session thread panicked"),
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
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

impl From<std::io::Error> for SessionError {
    fn from(e: std::io::Error) -> Self {
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

    /// Set the maximum number of scrollback lines.
    pub fn scrollback(mut self, lines: usize) -> Self {
        self.config.scrollback = lines;
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
            scrollback,
            scroll_delta,
            focused,
            read_buffer_size,
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
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    session_thread(
                        ThreadConfig {
                            cols,
                            rows,
                            scrollback,
                            colors,
                            scroll_delta,
                            focused,
                            read_buffer_size,
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
        let _ = self.cmd_tx.send(SessionCommand::Scroll(delta));
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
        let w = snap.buffer.area().width.min(dest_area.width);
        let h = snap.buffer.area().height.min(dest_area.height);
        for y in 0..h {
            for x in 0..w {
                let dx = dest_area.x + x;
                let dy = dest_area.y + y;
                if dx < dest.area().right() && dy < dest.area().bottom() {
                    dest[(dx, dy)] = snap.buffer[(x, y)].clone();
                }
            }
        }
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

enum SessionCommand {
    Key(KeyInput),
    Mouse(MouseInput),
    Focus(bool),
    Resize { cols: u16, rows: u16 },
    Scroll(isize),
    Paste(Vec<u8>),
    Raw(Vec<u8>),
    Shutdown,
}

struct RenderSnapshot {
    buffer: Buffer,
    cursor: CursorState,
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
    writer: std::rc::Rc<std::cell::RefCell<Box<dyn Write + Send>>>,
    resizer: &'a mut Resizer,
    shared: &'a SharedBuffer,
    focused: bool,
    cols: u16,
    rows: u16,
    scroll_delta: isize,
}

impl<'a> SessionState<'a> {
    fn handle_command(&mut self, cmd: SessionCommand, needs_render: &mut bool) -> bool {
        match cmd {
            SessionCommand::Key(input) => {
                if let Ok(data) = input::encode_key(&mut self.key_encoder, &input, &self.terminal)
                    && !data.is_empty()
                {
                    let mut w = self.writer.borrow_mut();
                    let _ = w.write_all(&data);
                    let _ = w.flush();
                }
            }
            SessionCommand::Mouse(input) => {
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
                if let Ok(data) =
                    input::encode_mouse(&mut self.mouse_encoder, &input, &self.terminal, size)
                    && !data.is_empty()
                {
                    let mut w = self.writer.borrow_mut();
                    let _ = w.write_all(&data);
                    let _ = w.flush();
                } else if input.button == Some(mouse::Button::Four) {
                    self.terminal
                        .scroll_viewport(ScrollViewport::Delta(-self.scroll_delta));
                    *needs_render = true;
                } else if input.button == Some(mouse::Button::Five) {
                    self.terminal
                        .scroll_viewport(ScrollViewport::Delta(self.scroll_delta));
                    *needs_render = true;
                }
            }
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
                if let Ok(mut snap) = self.shared.render.lock() {
                    snap.buffer = Buffer::empty(Rect::new(0, 0, cols, rows));
                }
                self.shared.cols.store(cols, Ordering::Relaxed);
                self.shared.rows.store(rows, Ordering::Relaxed);
                *needs_render = true;
            }
            SessionCommand::Scroll(delta) => {
                self.terminal.scroll_viewport(ScrollViewport::Delta(delta));
                *needs_render = true;
            }
            SessionCommand::Paste(data) => {
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
    scrollback: usize,
    colors: Option<HostColors>,
    scroll_delta: isize,
    focused: bool,
    read_buffer_size: usize,
}

fn session_thread(
    cfg: ThreadConfig,
    writer: Box<dyn Write + Send>,
    resizer: &mut Resizer,
    reader: Box<dyn Read + Send>,
    cmd_rx: Receiver<SessionCommand>,
    ctx: SessionContext<'_>,
) -> Result<(), SessionError> {
    let ThreadConfig {
        cols,
        rows,
        scrollback,
        colors,
        scroll_delta,
        focused,
        read_buffer_size,
    } = cfg;
    use std::cell::RefCell;
    use std::rc::Rc;

    let SessionContext {
        shared,
        event_tx,
        wake,
    } = ctx;

    let mut terminal = Box::new(Terminal::new(cols, rows)?);
    // Preserve the wrapper's documented line-count setting in this probe.
    terminal.set_scrollback_max_lines(Some(scrollback))?;

    let events_local: Rc<RefCell<Vec<SessionEvent>>> = Rc::new(RefCell::new(Vec::new()));

    terminal.on_bell({
        let events = events_local.clone();
        move |_| {
            events.borrow_mut().push(SessionEvent::Bell);
        }
    })?;

    terminal.on_title_changed({
        let events = events_local.clone();
        move |term: &Terminal| {
            if let Ok(t) = term.title() {
                events
                    .borrow_mut()
                    .push(SessionEvent::TitleChanged(t.to_string()));
            }
        }
    })?;

    let mut render_state = RenderState::new()?;
    let key_encoder = key::Encoder::new()?;
    let mouse_encoder = mouse::Encoder::new()?;

    let writer: Rc<RefCell<Box<dyn Write + Send>>> = Rc::new(RefCell::new(writer));

    terminal.on_pty_write({
        let writer = writer.clone();
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

    let (pty_tx, pty_rx) = crossbeam_channel::bounded::<Vec<u8>>(64);

    thread::Builder::new()
        .name("session-reader".into())
        .spawn(move || {
            let mut buf = vec![0u8; read_buffer_size];
            let mut reader = reader;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if pty_tx.send(buf[..n].to_vec()).is_err() {
                            break;
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
    };

    let mut last_pwd = String::new();

    loop {
        let mut needs_render = false;
        let mut shutdown = false;

        while let Ok(cmd) = cmd_rx.try_recv() {
            if state.handle_command(cmd, &mut needs_render) {
                shutdown = true;
                break;
            }
        }
        if shutdown {
            break;
        }

        let mut had_pty = false;
        let drain_start = Instant::now();
        while let Ok(data) = pty_rx.try_recv() {
            state.terminal.vt_write(&data);
            had_pty = true;
            while let Ok(cmd) = cmd_rx.try_recv() {
                if state.handle_command(cmd, &mut needs_render) {
                    shutdown = true;
                    break;
                }
            }
            if shutdown || drain_start.elapsed() >= DRAIN_BUDGET {
                break;
            }
        }
        if shutdown {
            break;
        }
        if had_pty {
            needs_render = true;

            if let Ok(pwd) = state.terminal.pwd()
                && pwd != last_pwd
            {
                last_pwd = pwd.to_string();
                let _ = event_tx.send(SessionEvent::CwdChanged(last_pwd.clone()));
            }
        }

        let mut should_wake = false;
        if needs_render {
            render_to_shared(&mut state, &mut render_state);
            should_wake = true;
        }

        for ev in events_local.borrow_mut().drain(..) {
            let _ = event_tx.send(ev);
            should_wake = true;
        }

        if should_wake {
            wake();
        }

        let mut cmd_needs_render = false;
        crossbeam_channel::select! {
            recv(cmd_rx) -> msg => {
                if let Ok(cmd) = msg && state.handle_command(cmd, &mut cmd_needs_render) {
                    break;
                }
            },
            recv(pty_rx) -> msg => {
                match msg {
                    Ok(data) => {
                        state.terminal.vt_write(&data);
                        render_to_shared(&mut state, &mut render_state);
                        wake();
                    }
                    Err(_) => break,
                }
            },
        }
        if cmd_needs_render {
            render_to_shared(&mut state, &mut render_state);
            wake();
        }
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
    if snap.buffer.area() != &area {
        snap.buffer = Buffer::empty(area);
    }
    snap.buffer.reset();
    let mut widget = TerminalWidget::new(&mut state.terminal, render_state).focused(state.focused);
    (&mut widget).render(area, &mut snap.buffer);
    snap.cursor = widget.cursor().clone();
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

fn is_dark(bg: crate::colors::Rgb) -> bool {
    let l = 0.2126 * linearize(bg.r) + 0.7152 * linearize(bg.g) + 0.0722 * linearize(bg.b);
    l < 0.5
}
