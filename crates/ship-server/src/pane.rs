//! A pane's live resources: its PTY, child process, wrapper session and the
//! task that owns them until teardown.

use std::{
    collections::HashMap,
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crossterm::event::KeyEvent;
use indexmap::IndexMap;
use kameo::actor::ActorRef;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use ratatui::{
    buffer::{self, Buffer},
    layout::Rect,
    style::{self as tui, Modifier},
};
use ratatui_ghostty::{
    session::{Pointer, Read, Scroll, SessionConfig, SessionEvent, SessionHandle, SessionIo},
    widget::CursorStyle,
};
use ship_core::{
    id::{Attachment, IdOf},
    model::{ExitStatus, Pane, StatusRecord},
    prelude::*,
    protocol::{MouseInput, ReadPane, ScrollDirection, ScrollPane},
    relay::{Publish, RelayBus},
    screen::{Attr, Color, Cursor, CursorShape, Multi, Row, Run, Screen, Scrolled, Size},
};
use ship_macros::traced;
use tokio::{
    sync::{Notify, mpsc, oneshot, watch},
    time::{Instant, sleep_until},
};
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::Instrument;
use url::Url;

use crate::{records::Records, settings::ServerSettings};

/// The shortest gap between two published screens of one pane.
pub(crate) const FRAME: Duration = Duration::from_millis(4);

/// How long a change after a quiet frame waits before it publishes, so a
/// program that draws twice in a row (nvim's scroll) lands in one screen.
const SETTLE: Duration = Duration::from_millis(1);

/// How long after a keypress the pane's changes publish at once, so the echo
/// and the redraws it causes skip `SETTLE` and `FRAME`.
const TYPED: Duration = Duration::from_millis(32);

/// At most `BURST` immediate publishes per `BURST_WINDOW`, so a flood right
/// after a keypress still paces.
const BURST: u32 = 16;
const BURST_WINDOW: Duration = Duration::from_millis(16);

/// Owned by the state actor. Dropping it cancels the pane task, which runs
/// teardown in the background. `stop` does the same and waits for it.
pub(crate) struct PaneRuntime {
    pub handle: PaneHandle,
    cancel: DropGuard,
    /// The pane task sends `()` after teardown.
    done: oneshot::Receiver<()>,
}

impl PaneRuntime {
    /// Cancel, then await `done`. An `Err` (task already gone) also counts as done.
    pub async fn stop(self) {
        let Self { cancel, done, .. } = self;
        drop(cancel);
        done.await.ok();
    }
}

/// What to start. `command: None` is the configured shell, else the login
/// shell, from portable-pty's `CommandBuilder::new_default_prog`.
pub(crate) struct Launch {
    pub pane: IdOf<Pane>,
    pub command: Option<Vec<String>>,
    pub cwd: PathBuf,
    pub size: Size,
}

/// The environment every pane's program starts with, on top of what the
/// server inherited from the client that launched it. Resolved once at
/// startup, so a rebuilt binary at the same path is still found.
pub(crate) struct PaneEnv {
    server_url: String,
    bin: PathBuf,
    /// Re-read for shell and history settings on every pane spawn.
    config: PathBuf,
}

impl PaneEnv {
    pub fn new(address: SocketAddr, config: PathBuf) -> Result<Self> {
        let bin = env::current_exe()
            .map_err(|error| err!(Io, "cannot resolve the server executable", @external: error))?;
        Ok(Self {
            server_url: format!("http://{address}"),
            bin,
            config,
        })
    }

    /// A bad file leaves both the login shell and default history cap.
    fn settings(&self) -> ServerSettings {
        ServerSettings::load(&self.config).unwrap_or_else(|error| {
            tracing::warn!(%error, "using default server settings");
            ServerSettings::default()
        })
    }

    /// Advertises Ship as the terminal, since Ship's emulator is what the
    /// program talks to, and drops the launching client's terminal,
    /// multiplexer and agent identities. Then tells the program how to reach
    /// this server about its own pane.
    fn apply(&self, builder: &mut CommandBuilder, pane: IdOf<Pane>) {
        builder.env("TERM", "xterm-256color");
        builder.env("COLORTERM", "truecolor");
        builder.env("TERM_PROGRAM", "ship");
        builder.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
        for key in [
            "ITERM_SESSION_ID",
            "LC_TERMINAL",
            "LC_TERMINAL_VERSION",
            "WEZTERM_PANE",
            "KITTY_WINDOW_ID",
            "WT_SESSION",
            "TMUX",
            "TMUX_PANE",
            "STY",
            "ZELLIJ",
            "ZELLIJ_SESSION_NAME",
            "ZELLIJ_PANE_ID",
            // A pane is not a child of whatever agent started the server.
            "CLAUDECODE",
            "CLAUDE_CODE_CHILD_SESSION",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_CODE_MESSAGING_TOKEN",
            "CODEX_THREAD_ID",
            "OMPCODE",
        ] {
            builder.env_remove(key);
        }
        builder.env("SHIP_SERVER_URL", &self.server_url);
        builder.env("SHIP_BIN", &self.bin);
        builder.env("SHIP_PANE_ID", pane.to_string());
    }
}

/// What the rest of the server may hold of a running pane.
#[derive(Clone)]
pub(crate) struct PaneHandle {
    /// The pane's latest screen. Ends when the pane task does.
    pub screen: watch::Receiver<Arc<Screen>>,
    /// Input and resizes. Sending never waits on the terminal; a send after
    /// the pane task ended fails and is ignored.
    pub commands: mpsc::UnboundedSender<PaneCommand>,
}

/// Pointer input tagged with the attachment that sent it.
pub(crate) struct PaneMouse {
    pub input: MouseInput,
    pub from: IdOf<Attachment>,
}

pub(crate) enum PaneCommand {
    Mouse(PaneMouse),
    /// Encoded by the wrapper against the terminal's live modes.
    Key(KeyEvent),
    /// Bracketed by the wrapper when the program enabled it.
    Paste(String),
    /// Dropped when equal to the pane's current size.
    Resize(Size),
    Scroll(ScrollPane),
    Read(ReadPane, oneshot::Sender<String>),
}

/// Every running pane, published by the state actor.
pub(crate) type LivePanes = HashMap<IdOf<Pane>, PaneHandle>;

/// Bus publication for the state actor.
#[derive(Clone)]
pub(crate) struct PaneEvent {
    pub pane: IdOf<Pane>,
    pub change: PaneChange,
}

#[derive(Clone)]
pub(crate) enum PaneChange {
    Title(Option<String>),
    /// From OSC 7, else the program's working directory.
    Dir(String),
    Exited(ExitStatus),
    Program(IndexMap<String, StatusRecord>),
}

/// Text for clients' clipboards, on the bus apart from `PaneEvent`, since it
/// changes no shared state. `to: None` is every viewer of `pane`'s tab.
#[derive(Clone)]
pub(crate) struct Clipboard {
    pub pane: IdOf<Pane>,
    pub to: Option<IdOf<Attachment>>,
    pub text: String,
}

/// Shared by the wrapper's resizer and teardown, which on Unix reads the
/// terminal's foreground group from it.
type Master = Arc<Mutex<Box<dyn MasterPty + Send>>>;

/// A started pane: its runtime, and the resolved argv for `Pane::command`.
pub(crate) struct Spawned {
    pub runtime: PaneRuntime,
    pub command: Vec<String>,
}

/// Synchronous; the state actor calls it just before the commit that inserts
/// the pane. Opens the PTY, starts the wrapper session, spawns the child with
/// `env` applied, drops the slave, and starts the exit-watcher thread and the
/// pane task.
///
/// Any failure returns `Err`, and nothing is left running.
#[traced(fields(pane = %launch.pane))]
pub(crate) fn spawn(launch: Launch, env: &PaneEnv, bus: ActorRef<RelayBus>) -> Result<Spawned> {
    let Launch {
        pane,
        command,
        cwd,
        size,
    } = launch;
    let size = terminal_size(size);
    let settings = env.settings();
    let pty = native_pty_system()
        .openpty(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| err!(Io, "cannot open a terminal", @external: error))?;
    let reader = pty
        .master
        .try_clone_reader()
        .map_err(|error| err!(Io, "cannot read the terminal", @external: error))?;
    let writer = pty
        .master
        .take_writer()
        .map_err(|error| err!(Io, "cannot write the terminal", @external: error))?;
    let master: Master = Arc::new(Mutex::new(pty.master));
    let resizer = {
        let master = Arc::clone(&master);
        Box::new(move |cols, rows| {
            master
                .lock()
                .map_err(|_| "terminal lock poisoned")?
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(Into::into)
        })
    };
    let dirty = Arc::new(Notify::new());
    let session = SessionHandle::spawn(
        SessionConfig {
            scrollback_bytes: Some(settings.scrollback_bytes),
            render_on_request: true,
            ..Default::default()
        },
        SessionIo {
            reader,
            writer,
            resizer,
        },
        size.cols,
        size.rows,
        {
            let dirty = Arc::clone(&dirty);
            move || dirty.notify_one()
        },
    )
    .map_err(|error| err!(Io, "cannot start the terminal", @external: error))?;

    let mut builder = match &command {
        None => match settings.shell {
            Some(shell) => {
                // An explicit shell must fail rather than silently fall back
                // through portable-pty's default-program resolution.
                let mut builder = CommandBuilder::new(&shell);
                builder.env("SHELL", shell);
                builder
            }
            None => CommandBuilder::new_default_prog(),
        },
        Some(argv) => CommandBuilder::from_argv(argv.iter().map(Into::into).collect()),
    };
    builder.cwd(&cwd);
    // portable-pty leaves PWD as the server's; programs that read it see that.
    builder.env("PWD", &cwd);
    env.apply(&mut builder, pane);
    let argv = if builder.is_default_prog() {
        vec![builder.get_shell()]
    } else {
        builder
            .get_argv()
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    };
    let mut child = pty
        .slave
        .spawn_command(builder)
        .map_err(|error| err!(Validation, "cannot start {}", argv[0], @external: error))?;
    drop(pty.slave);
    let pid = child.process_id();
    let mut program = Program::new(&*child).ok_or_else(|| {
        child.kill().ok();
        err!(Internal, "cannot read the pid of {}", argv[0])
    })?;

    let (exit_tx, status) = oneshot::channel();
    let exit_span = tracing::debug_span!(parent: None, "pane_exit", pane = %pane);
    thread::Builder::new()
        .name("pane-exit".into())
        .spawn(move || {
            let _entered = exit_span.enter();
            if let Some(status) = program::wait(child) {
                tracing::info!(%pane, pid, code = status.code, signal = status.signal.as_deref(), "pane program exited");
                exit_tx.send(status).ok();
            }
        })
        .map_err(|error| {
            program.kill();
            err!(Io, "cannot watch {} for exit", argv[0], @external: error)
        })?;

    // The blank grid, so a pane that never prints still has a screen.
    let mut buffer = Buffer::default();
    let first = capture(&session, &mut buffer).expect("a new session starts dirty");
    let (screen, screen_rx) = watch::channel(Arc::new(first));
    let cancel = CancellationToken::new();
    let (done_tx, done) = oneshot::channel();
    let (commands, commands_rx) = mpsc::unbounded_channel();
    let task = PaneTask {
        pane,
        session,
        size,
        master,
        program,
        exit: Exit {
            status,
            reaped: false,
        },
        dirty,
        title: None,
        dir: None,
        reported: false,
        pid,
        pointer: None,
        buffer,
        screen,
        bus,
        pacing: Pacing::default(),
        records: Records::default(),
    };
    tokio::spawn(
        task.run(commands_rx, cancel.clone(), done_tx)
            .instrument(tracing::info_span!(parent: None, "pane", pane = %pane)),
    );
    // Only the program, not its arguments, which can carry secrets.
    tracing::info!(%pane, pid, program = %argv[0], cwd = %cwd.display(), "pane program started");
    Ok(Spawned {
        runtime: PaneRuntime {
            handle: PaneHandle {
                screen: screen_rx,
                commands,
            },
            cancel: cancel.drop_guard(),
            done,
        },
        command: argv,
    })
}

/// The pane task's state.
struct PaneTask {
    records: Records,
    pane: IdOf<Pane>,
    session: SessionHandle,
    /// The terminal's size as last requested; equal resizes are dropped.
    size: Size,
    /// Shared with the wrapper's resizer; teardown reads the foreground group from it.
    master: Master,
    program: Program,
    exit: Exit,
    /// The wrapper's wake callback calls `notify_one` when the terminal
    /// changed; it renders only when `publish` asks, once per frame.
    dirty: Arc<Notify>,
    /// Latest title seen this frame; the tree keeps the lasting copy.
    title: Option<String>,
    /// Last directory published, so a repeated report sends nothing.
    dir: Option<String>,
    /// Whether the shell reported a directory through OSC 7, which then ends
    /// the lookups of `pid`'s working directory.
    reported: bool,
    pid: Option<u32>,
    /// The attachment that last sent pointer input, which a mouse
    /// selection's copy goes to.
    pointer: Option<IdOf<Attachment>>,
    /// Kept between captures so each one doesn't allocate a grid.
    buffer: Buffer,
    screen: watch::Sender<Arc<Screen>>,
    bus: ActorRef<RelayBus>,
    pacing: Pacing,
}

/// When a change publishes: after `SETTLE` and at least `FRAME` after the
/// last publish, or at once within `TYPED` of a keypress.
#[derive(Default)]
struct Pacing {
    typed: Option<Instant>,
    /// When the current burst window opened, and its immediate publishes.
    burst: Option<(Instant, u32)>,
}

impl Pacing {
    fn typed(&mut self) {
        self.typed = Some(Instant::now());
    }

    /// When a change arriving now publishes, given the paced time `next`.
    fn due(&mut self, next: Instant) -> Instant {
        let now = Instant::now();
        if self.typed.is_some_and(|at| now - at < TYPED) {
            let (opened, used) = self.burst.get_or_insert((now, 0));
            if now - *opened >= BURST_WINDOW {
                *opened = now;
                *used = 0;
            }
            if *used < BURST {
                *used += 1;
                return now;
            }
        }
        next.max(now + SETTLE)
    }
}

impl PaneTask {
    async fn run(
        mut self,
        mut commands: mpsc::UnboundedReceiver<PaneCommand>,
        cancel: CancellationToken,
        done: oneshot::Sender<()>,
    ) {
        // Waiting for the frame happens in its own arm, so commands keep
        // reaching the program meanwhile. `pacing` picks when a change
        // publishes; the check runs here rather than in the timer arm since
        // a timer, even one already due, waits for tokio's next 1 ms tick.
        let mut pending = false;
        let mut next = Instant::now();
        loop {
            if pending && Instant::now() >= next {
                pending = false;
                self.publish().await;
                next = Instant::now() + FRAME;
                continue;
            }
            tokio::select! {
                () = cancel.cancelled() => break,
                Some(command) = commands.recv() => self.command(command),
                () = self.dirty.notified(), if !pending => {
                    pending = true;
                    next = self.pacing.due(next);
                }
                () = sleep_until(next), if pending => {}
                status = &mut self.exit.status, if !self.exit.reaped => {
                    self.exit.reaped = true;
                    self.publish().await;
                    if let Ok(status) = status {
                        self.event(PaneChange::Exited(status)).await;
                    }
                }
            }
        }
        self.teardown().await;
        done.send(()).ok();
    }

    /// Exited programs ignore keys, pastes and resizes, but keep history input.
    #[traced]
    fn command(&mut self, command: PaneCommand) {
        if self.exit.reaped
            && !matches!(
                command,
                PaneCommand::Scroll(_) | PaneCommand::Read(..) | PaneCommand::Mouse(_)
            )
        {
            return;
        }
        match command {
            PaneCommand::Scroll(ScrollPane { direction, amount }) => {
                let scroll = match direction {
                    ScrollDirection::Top => Scroll::Top,
                    ScrollDirection::Bottom => Scroll::Bottom,
                    ScrollDirection::Up | ScrollDirection::Down => {
                        let sign = if matches!(direction, ScrollDirection::Up) {
                            -1
                        } else {
                            1
                        };
                        match amount {
                            Some(lines) => {
                                Scroll::Lines(sign * isize::try_from(lines).unwrap_or(isize::MAX))
                            }
                            None => Scroll::Pages(sign),
                        }
                    }
                };
                self.session.send_scroll_to(scroll);
            }
            PaneCommand::Read(
                ReadPane {
                    lines,
                    unwrap,
                    ansi,
                },
                reply,
            ) => {
                self.session.read(
                    Read {
                        lines,
                        unwrap,
                        ansi,
                    },
                    Box::new(move |text| {
                        reply.send(text).ok();
                    }),
                );
            }
            PaneCommand::Mouse(PaneMouse { input, from }) => {
                self.pointer = Some(from);
                self.session.send_pointer(Pointer {
                    kind: input.kind,
                    modifiers: input.modifiers,
                    x: input.x,
                    y: input.y,
                });
            }
            PaneCommand::Key(key) => {
                self.pacing.typed();
                self.session.send_key(key);
            }
            PaneCommand::Paste(text) => self.session.send_paste(text.into_bytes()),
            PaneCommand::Resize(size) => {
                let size = terminal_size(size);
                if size != self.size {
                    self.size = size;
                    self.session.send_resize(size.cols, size.rows);
                }
            }
        }
    }

    /// Drain the wrapper's events, publish the screen, then the title and
    /// directory if they changed. A shell that hasn't reported OSC 7, such as
    /// zsh under oh-my-zsh with SSH variables set, has its working directory
    /// read instead; its prompt after a `cd` is output, so it lands here.
    #[traced]
    async fn publish(&mut self) {
        let (rendered, render) = oneshot::channel();
        self.session.render(Box::new(move || {
            rendered.send(()).ok();
        }));
        render.await.ok();
        let mut changed = false;
        let mut program_changed = false;
        let mut dir = None;
        let mut copies = Vec::new();
        while let Some(event) = self.session.poll_event() {
            match event {
                SessionEvent::ProgramStatus(report) => {
                    program_changed |= self.records.apply(report)
                }
                SessionEvent::PromptStart => program_changed |= self.records.prompt(),
                SessionEvent::Copied(text) => copies.push((self.pointer, text)),
                SessionEvent::ClipboardWrite(text) => copies.push((None, text)),
                SessionEvent::TitleChanged(title) => {
                    self.title = Some(title).filter(|title| !title.is_empty());
                    changed = true;
                }
                SessionEvent::CwdChanged(uri) => {
                    dir = local_path(&uri).or(dir);
                    self.reported |= dir.is_some();
                }
                _ => {}
            }
        }
        if !self.reported && !self.exit.reaped {
            dir = self.pid.and_then(process_cwd);
        }
        if let Some(screen) = capture(&self.session, &mut self.buffer) {
            tracing::trace!(pane = %self.pane, "screen published");
            self.screen.send_replace(Arc::new(screen));
        }
        for (to, text) in copies {
            let pane = self.pane;
            self.bus
                .tell(Publish(Clipboard { pane, to, text }))
                .await
                .ok();
        }
        if changed {
            self.event(PaneChange::Title(self.title.clone())).await;
        }
        if program_changed && !self.exit.reaped {
            self.event(PaneChange::Program(self.records.snapshot()))
                .await;
        }
        self.set_dir(dir).await;
    }

    async fn set_dir(&mut self, dir: Option<String>) {
        if let Some(dir) = dir.filter(|dir| self.dir.as_ref() != Some(dir)) {
            self.dir = Some(dir.clone());
            self.event(PaneChange::Dir(dir)).await;
        }
    }

    /// `&mut`, because `SessionHandle` isn't `Sync` and a shared borrow held
    /// across the await would make the task `!Send`.
    async fn event(&mut self, change: PaneChange) {
        let event = PaneEvent {
            pane: self.pane,
            change,
        };
        self.bus.tell(Publish(event)).await.ok();
    }

    #[traced]
    async fn teardown(mut self) {
        self.program.end(&self.master, &mut self.exit).await;
        drop(self.session);
        tracing::debug!(pane = %self.pane, "pane torn down");
    }
}

/// OSC 7's `file://HOST/PATH` as a path. Shells put their hostname in
/// HOST, which `to_file_path` refuses, so it becomes `localhost` first.
/// Anything else is ignored.
fn local_path(uri: &str) -> Option<String> {
    let mut url = Url::parse(uri).ok().filter(|url| url.scheme() == "file")?;
    url.set_host(Some("localhost")).ok()?;
    Some(url.to_file_path().ok()?.display().to_string())
}

/// The working directory of the process `pid`, with symlinks resolved, so
/// `/tmp` reads as `/private/tmp` on macOS.
#[cfg(target_os = "linux")]
fn process_cwd(pid: u32) -> Option<String> {
    let dir = std::fs::read_link(format!("/proc/{pid}/cwd")).ok()?;
    Some(dir.display().to_string())
}

#[cfg(target_os = "macos")]
fn process_cwd(pid: u32) -> Option<String> {
    use std::mem::MaybeUninit;

    let mut info = MaybeUninit::<libc::proc_vnodepathinfo>::zeroed();
    let size = i32::try_from(size_of::<libc::proc_vnodepathinfo>()).ok()?;
    // SAFETY: `info` is writable for `size` bytes, the most
    // PROC_PIDVNODEPATHINFO writes.
    let written = unsafe {
        libc::proc_pidinfo(
            i32::try_from(pid).ok()?,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if written != size {
        return None;
    }
    // SAFETY: all zeroes is a valid `proc_vnodepathinfo`, and the call wrote
    // the rest.
    let info = unsafe { info.assume_init() };
    // `libc` splits the MAXPATHLEN path into rows for old compilers.
    let path = info.pvi_cdir.vip_path.as_flattened();
    let path = &path[..path.iter().position(|&byte| byte == 0)?];
    String::from_utf8(path.iter().map(|&byte| byte as u8).collect()).ok()
}

/// Elsewhere only OSC 7 reports a directory.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn process_cwd(_pid: u32) -> Option<String> {
    None
}

/// The wrapper's rendered buffer and cursor as an owned `Screen`, or `None`
/// when nothing was rendered since the last capture. `buffer` is replaced
/// when the size changed, so no cell outlives a resize.
#[traced(level = "trace")]
fn capture(session: &SessionHandle, buffer: &mut Buffer) -> Option<Screen> {
    let (cols, rows) = session.size();
    let area = Rect::new(0, 0, cols, rows);
    if buffer.area != area {
        *buffer = Buffer::empty(area);
    }
    let rendered = session.take_if_dirty(buffer, area)?;
    let cursor = rendered.cursor;
    Some(Screen {
        mouse: rendered.mouse,
        scrolled: rendered.scrolled.map(|value| Scrolled {
            offset: value.offset,
            history: value.history,
        }),
        size: Size { cols, rows },
        rows: buffer
            .content
            .chunks(usize::from(cols.max(1)))
            .map(row)
            .collect(),
        cursor: cursor.position.map(|position| Cursor {
            x: position.x,
            y: position.y,
            shape: match cursor.style {
                CursorStyle::Block => CursorShape::Block,
                CursorStyle::Underline => CursorShape::Underline,
                CursorStyle::Bar => CursorShape::Bar,
                CursorStyle::Default => CursorShape::Default,
            },
            blinking: cursor.blinking,
        }),
    })
}

const ATTRS: [(Modifier, Attr); 9] = [
    (Modifier::BOLD, Attr::Bold),
    (Modifier::DIM, Attr::Dim),
    (Modifier::ITALIC, Attr::Italic),
    (Modifier::UNDERLINED, Attr::Underlined),
    (Modifier::SLOW_BLINK, Attr::SlowBlink),
    (Modifier::RAPID_BLINK, Attr::RapidBlink),
    (Modifier::REVERSED, Attr::Reversed),
    (Modifier::HIDDEN, Attr::Hidden),
    (Modifier::CROSSED_OUT, Attr::CrossedOut),
];

/// One buffer row: text through its last non-blank cell, adjacent cells with
/// equal styles merged into one run, and no trailing default-style run.
#[traced(level = "trace")]
fn row(cells: &[buffer::Cell]) -> Row {
    let end = cells
        .iter()
        .rposition(|cell| cell.symbol() != " " || cell.style() != buffer::Cell::EMPTY.style())
        .map_or(0, |last| last + 1);
    let mut row = Row::default();
    let mut previous = None;
    for (col, cell) in (0..).zip(&cells[..end]) {
        let mut chars = cell.symbol().chars();
        match (chars.next(), chars.next()) {
            (Some(only), None) => row.text.push(only),
            _ => {
                row.text.push(' ');
                row.multi.push(Multi {
                    col,
                    symbol: cell.symbol().to_owned(),
                });
            }
        }
        let style = cell.style();
        match row.runs.last_mut() {
            Some(run) if previous == Some(style) => run.cells += 1,
            _ => row.runs.push(run_of(cell)),
        }
        previous = Some(style);
    }
    if row.runs.last().is_some_and(Run::is_default) {
        row.runs.pop();
    }
    row
}

/// A one-cell run with `cell`'s style.
fn run_of(cell: &buffer::Cell) -> Run {
    Run {
        cells: 1,
        fg: color(cell.fg),
        bg: color(cell.bg),
        underline_color: color(cell.underline_color),
        attrs: ATTRS
            .iter()
            .filter(|(modifier, _)| cell.modifier.contains(*modifier))
            .map(|(_, attr)| *attr)
            .collect(),
    }
}

/// Named colors are the first sixteen palette entries.
fn color(color: tui::Color) -> Color {
    use tui::Color::*;
    let indexed = match color {
        Reset => return Color::Default,
        Rgb(r, g, b) => return Color::Rgb(r, g, b),
        Indexed(index) => index,
        Black => 0,
        Red => 1,
        Green => 2,
        Yellow => 3,
        Blue => 4,
        Magenta => 5,
        Cyan => 6,
        Gray => 7,
        DarkGray => 8,
        LightRed => 9,
        LightGreen => 10,
        LightYellow => 11,
        LightBlue => 12,
        LightMagenta => 13,
        LightCyan => 14,
        White => 15,
    };
    Color::Indexed(indexed)
}

/// The child's exit status, fed by a std thread blocked in `program::wait`.
struct Exit {
    status: oneshot::Receiver<ExitStatus>,
    reaped: bool,
}

impl Exit {
    /// Await the reap. Returns at once if it already happened; a receiver
    /// polled after completing would panic.
    async fn wait(&mut self) {
        if !self.reaped {
            (&mut self.status).await.ok();
            self.reaped = true;
        }
    }
}

use program::Program;

/// Ends the pane's program and everything it started: its group and the
/// terminal's foreground group get SIGHUP, up to GRACE to exit, then SIGKILL.
#[cfg(unix)]
mod program {
    use std::iter;

    use nix::{
        errno::Errno,
        sys::{
            signal::{Signal, killpg},
            wait::{WaitStatus, waitpid},
        },
        unistd::Pid,
    };
    use tokio::time::{sleep, timeout};

    use super::*;

    const GRACE: Duration = Duration::from_secs(2);
    /// How often teardown checks whether the hung-up groups are gone.
    const POLL: Duration = Duration::from_millis(20);

    pub(super) struct Program {
        /// portable-pty runs the child in a new session, so its pid is its group.
        group: Pid,
    }

    impl Program {
        pub fn new(child: &dyn Child) -> Option<Self> {
            let pid = i32::try_from(child.process_id()?).ok()?;
            Some(Self {
                group: Pid::from_raw(pid),
            })
        }

        /// SIGKILL the child's group without waiting.
        pub fn kill(&mut self) {
            killpg(self.group, Signal::SIGKILL).ok();
        }

        /// Give the child up to GRACE to be reaped and both groups to empty.
        pub async fn end(&mut self, master: &Master, exit: &mut Exit) {
            let foreground = master
                .lock()
                .ok()
                .and_then(|master| master.process_group_leader())
                .map(Pid::from_raw)
                .filter(|group| group.as_raw() > 0 && *group != self.group);
            let groups: Vec<Pid> = iter::once(self.group).chain(foreground).collect();
            signal(&groups, Signal::SIGHUP);
            let settle = async {
                exit.wait().await;
                while groups.iter().any(|group| killpg(*group, None).is_ok()) {
                    sleep(POLL).await;
                }
            };
            if timeout(GRACE, settle).await.is_err() {
                tracing::info!(
                    group = self.group.as_raw(),
                    "pane program outlived SIGHUP; killing"
                );
            }
            signal(&groups, Signal::SIGKILL);
            exit.wait().await;
        }
    }

    /// Block until the child is reaped. A signal death reports its name, such
    /// as `SIGTERM`, with code 1. `None` if the child can't be waited for.
    pub fn wait(child: Box<dyn Child + Send + Sync>) -> Option<ExitStatus> {
        let pid = Pid::from_raw(i32::try_from(child.process_id()?).ok()?);
        loop {
            return match waitpid(pid, None) {
                Ok(WaitStatus::Exited(_, code)) => Some(ExitStatus {
                    code: code.unsigned_abs(),
                    signal: None,
                }),
                Ok(WaitStatus::Signaled(_, signal, _)) => Some(ExitStatus {
                    code: 1,
                    signal: Some(signal.as_str().to_owned()),
                }),
                Ok(_) | Err(Errno::EINTR) => continue,
                Err(error) => {
                    tracing::warn!(pid = pid.as_raw(), %error, "cannot wait for pane program");
                    None
                }
            };
        }
    }

    /// Signal every group; one that is already gone is fine.
    fn signal(groups: &[Pid], signal: Signal) {
        for group in groups {
            killpg(*group, signal).ok();
        }
    }
}

/// Ends only the pane's program, through portable-pty; processes it started
/// may outlive it.
#[cfg(not(unix))]
mod program {
    use portable_pty::ChildKiller;

    use super::*;

    pub(super) struct Program {
        killer: Box<dyn ChildKiller + Send + Sync>,
    }

    impl Program {
        pub fn new(child: &dyn Child) -> Option<Self> {
            Some(Self {
                killer: child.clone_killer(),
            })
        }

        pub fn kill(&mut self) {
            self.killer.kill().ok();
        }

        pub async fn end(&mut self, _master: &Master, exit: &mut Exit) {
            self.kill();
            exit.wait().await;
        }
    }

    /// Block until the child exits, with portable-pty's status.
    pub fn wait(mut child: Box<dyn Child + Send + Sync>) -> Option<ExitStatus> {
        let status = child.wait().ok()?;
        Some(ExitStatus {
            code: status.exit_code(),
            signal: status.signal().map(str::to_owned),
        })
    }
}

/// Both PTY and emulator need a positive grid; published content stays actual.
fn terminal_size(size: Size) -> Size {
    Size {
        cols: size.cols.max(1),
        rows: size.rows.max(1),
    }
}
