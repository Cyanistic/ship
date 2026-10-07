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
    sync::{Notify, oneshot, watch},
    time::MissedTickBehavior,
};
use tokio_util::sync::{CancellationToken, DropGuard};

pub(crate) const FRAME: Duration = Duration::from_millis(16);

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
            if let Ok(status) = child.wait() {
                exit_tx
                    .send(ExitStatus {
                        code: status.exit_code(),
                        signal: status.signal().map(str::to_owned),
                    })
                    .ok();
            }
        })
        .map_err(|error| {
            program.kill();
            err!(Io, "cannot watch {} for exit", argv[0], @external: error)
        })?;

    // The blank grid, so a pane that never prints still has a screen.
    let (screen, screen_rx) = watch::channel(Arc::new(capture(&session)));
    let cancel = CancellationToken::new();
    let (done_tx, done) = oneshot::channel();
    let task = PaneTask {
        pane,
        session,
        master,
        program,
        exit: Exit {
            status,
            reaped: false,
        },
        dirty,
        title: None,
        screen,
        bus,
    };
    tokio::spawn(task.run(cancel.clone(), done_tx));
    Ok(Spawned {
        runtime: PaneRuntime {
            handle: PaneHandle { screen: screen_rx },
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
    /// Shared with the wrapper's resizer; teardown reads the foreground group from it.
    master: Master,
    program: Program,
    exit: Exit,
    /// The wrapper's wake callback calls `notify_one`.
    dirty: Arc<Notify>,
    /// Latest title seen this frame; the tree keeps the lasting copy.
    title: Option<String>,
    screen: watch::Sender<Arc<Screen>>,
    bus: ActorRef<RelayBus>,
}

impl PaneTask {
    async fn run(mut self, cancel: CancellationToken, done: oneshot::Sender<()>) {
        let mut frame = tokio::time::interval(FRAME);
        frame.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                () = cancel.cancelled() => break,
                () = self.dirty.notified() => {
                    frame.tick().await;
                    self.publish().await;
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
        tracing::trace!(pane = %self.pane, "screen published");
        self.screen.send_replace(Arc::new(capture(&self.session)));
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

/// The wrapper's rendered buffer and cursor as an owned `Screen`.
fn capture(session: &SessionHandle) -> Screen {
    let (cols, rows) = session.size();
    let area = Rect::new(0, 0, cols, rows);
    let mut buffer = Buffer::empty(area);
    session.blit_to(&mut buffer, area);
    let cursor = session.cursor_state();
    Screen {
        size: Size { cols, rows },
        cells: buffer.content.iter().map(cell).collect(),
        cursor: cursor.position.map(|position| Cursor {
            x: position.x,
            y: position.y,
            shape: match cursor.style {
                CursorStyle::Block => CursorShape::Block,
                CursorStyle::Underline => CursorShape::Underline,
                CursorStyle::Bar => CursorShape::Bar,
            },
            blinking: cursor.blinking,
        }),
    }
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

/// The child's exit status, fed by a std thread blocked in `child.wait()`.
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
        sys::signal::{Signal, killpg},
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
}
