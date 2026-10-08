//! A pane's live resources: its PTY, child process, wrapper session and the
//! task that owns them until teardown.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use kameo::actor::ActorRef;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use ratatui::{
    buffer::{self, Buffer},
    layout::Rect,
    style::{self as tui, Modifier},
};
use ratatui_ghostty::{
    session::{SessionEvent, SessionHandle, SessionIo},
    widget::CursorStyle,
};
use ship_core::{
    id::IdOf,
    model::{ExitStatus, Pane},
    prelude::*,
    relay::{Publish, RelayBus},
    screen::{Attr, Cell, Color, Cursor, CursorShape, Screen, Size},
};
use tokio::{
    sync::{Notify, mpsc, oneshot, watch},
    time::Instant,
};
use tokio_util::sync::{CancellationToken, DropGuard};

/// The shortest gap between two published screens of one pane.
pub(crate) const FRAME: Duration = Duration::from_millis(4);

/// How long a change after a quiet frame waits before it publishes, so a
/// program that draws twice in a row (nvim's scroll) lands in one screen.
const SETTLE: Duration = Duration::from_millis(1);

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

/// What to start. `command: None` is the login shell from portable-pty's
/// `CommandBuilder::new_default_prog`.
pub(crate) struct Launch {
    pub pane: IdOf<Pane>,
    pub command: Option<Vec<String>>,
    pub cwd: PathBuf,
    pub size: Size,
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

pub(crate) enum PaneCommand {
    /// Encoded by the wrapper against the terminal's live modes.
    Key(crossterm::event::KeyEvent),
    /// Bracketed by the wrapper when the program enabled it.
    Paste(String),
    /// Dropped when equal to the pane's current size.
    Resize(Size),
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
    Exited(ExitStatus),
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
/// `TERM=xterm-256color` and `COLORTERM=truecolor`, drops the slave, and
/// starts the exit-watcher thread and the pane task.
///
/// Any failure returns `Err`, and nothing is left running.
pub(crate) fn spawn(launch: Launch, bus: ActorRef<RelayBus>) -> Result<Spawned> {
    let Launch {
        pane,
        command,
        cwd,
        size,
    } = launch;
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
        let master = master.clone();
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
        Default::default(),
        SessionIo {
            reader,
            writer,
            resizer,
        },
        size.cols,
        size.rows,
        {
            let dirty = dirty.clone();
            move || dirty.notify_one()
        },
    )
    .map_err(|error| err!(Io, "cannot start the terminal", @external: error))?;

    let mut builder = match &command {
        None => CommandBuilder::new_default_prog(),
        Some(argv) => CommandBuilder::from_argv(argv.iter().map(Into::into).collect()),
    };
    builder.cwd(&cwd);
    builder.env("TERM", "xterm-256color");
    builder.env("COLORTERM", "truecolor");
    let argv = command.unwrap_or_else(|| vec![builder.get_shell()]);
    let mut child = pty
        .slave
        .spawn_command(builder)
        .map_err(|error| err!(Validation, "cannot start {}", argv[0], @external: error))?;
    drop(pty.slave);
    let mut program = Program::new(&*child).ok_or_else(|| {
        child.kill().ok();
        err!(Internal, "cannot read the pid of {}", argv[0])
    })?;

    let (exit_tx, status) = oneshot::channel();
    std::thread::Builder::new()
        .name("pane-exit".into())
        .spawn(move || {
            if let Some(status) = program::wait(child) {
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
        buffer,
        screen,
        bus,
    };
    tokio::spawn(task.run(commands_rx, cancel.clone(), done_tx));
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
    pane: IdOf<Pane>,
    session: SessionHandle,
    /// The terminal's size as last requested; equal resizes are dropped.
    size: Size,
    /// Shared with the wrapper's resizer; teardown reads the foreground group from it.
    master: Master,
    program: Program,
    exit: Exit,
    /// The wrapper's wake callback calls `notify_one`.
    dirty: Arc<Notify>,
    /// Latest title seen this frame; the tree keeps the lasting copy.
    title: Option<String>,
    /// Kept between captures so each one doesn't allocate a grid.
    buffer: Buffer,
    screen: watch::Sender<Arc<Screen>>,
    bus: ActorRef<RelayBus>,
}

impl PaneTask {
    async fn run(
        mut self,
        mut commands: mpsc::UnboundedReceiver<PaneCommand>,
        cancel: CancellationToken,
        done: oneshot::Sender<()>,
    ) {
        // Waiting for the frame happens in its own arm, so commands keep
        // reaching the program meanwhile. A change after a quiet frame
        // publishes after `SETTLE`, since `next` has passed.
        let mut pending = false;
        let mut next = Instant::now();
        loop {
            tokio::select! {
                () = cancel.cancelled() => break,
                Some(command) = commands.recv() => self.command(command),
                () = self.dirty.notified(), if !pending => {
                    pending = true;
                    next = next.max(Instant::now() + SETTLE);
                }
                () = tokio::time::sleep_until(next), if pending => {
                    pending = false;
                    self.publish().await;
                    next = Instant::now() + FRAME;
                }
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

    /// Hand input and resizes to the wrapper. An exited program's pane keeps
    /// its last screen, so everything is ignored then.
    fn command(&mut self, command: PaneCommand) {
        if self.exit.reaped {
            return;
        }
        match command {
            PaneCommand::Key(key) => self.session.send_key(key),
            PaneCommand::Paste(text) => self.session.send_paste(text.into_bytes()),
            PaneCommand::Resize(size) if size != self.size => {
                self.size = size;
                self.session.send_resize(size.cols, size.rows);
            }
            PaneCommand::Resize(_) => {}
        }
    }

    /// Drain the wrapper's events, publish the screen, then the title if one
    /// arrived.
    async fn publish(&mut self) {
        let mut changed = false;
        while let Some(event) = self.session.poll_event() {
            if let SessionEvent::TitleChanged(title) = event {
                self.title = Some(title).filter(|title| !title.is_empty());
                changed = true;
            }
        }
        if let Some(screen) = capture(&self.session, &mut self.buffer) {
            tracing::trace!(pane = %self.pane, "screen published");
            self.screen.send_replace(Arc::new(screen));
        }
        if changed {
            self.event(PaneChange::Title(self.title.clone())).await;
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

    async fn teardown(mut self) {
        self.program.end(&self.master, &mut self.exit).await;
        drop(self.session);
        tracing::debug!(pane = %self.pane, "pane torn down");
    }
}

/// The wrapper's rendered buffer and cursor as an owned `Screen`, or `None`
/// when nothing was rendered since the last capture. `buffer` is replaced
/// when the size changed, so no cell outlives a resize.
fn capture(session: &SessionHandle, buffer: &mut Buffer) -> Option<Screen> {
    let (cols, rows) = session.size();
    let area = Rect::new(0, 0, cols, rows);
    if buffer.area != area {
        *buffer = Buffer::empty(area);
    }
    let cursor = session.take_if_dirty(buffer, area)?;
    Some(Screen {
        size: Size { cols, rows },
        cells: buffer.content.iter().map(cell).collect(),
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

fn cell(cell: &buffer::Cell) -> Cell {
    Cell {
        symbol: cell.symbol().to_owned(),
        fg: color(cell.fg),
        bg: color(cell.bg),
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
    use nix::{
        errno::Errno,
        sys::{
            signal::{Signal, killpg},
            wait::{WaitStatus, waitpid},
        },
        unistd::Pid,
    };

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
            let groups: Vec<Pid> = std::iter::once(self.group).chain(foreground).collect();
            signal(&groups, Signal::SIGHUP);
            let settle = async {
                exit.wait().await;
                while groups.iter().any(|group| killpg(*group, None).is_ok()) {
                    tokio::time::sleep(POLL).await;
                }
            };
            tokio::time::timeout(GRACE, settle).await.ok();
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
                Err(_) => None,
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
