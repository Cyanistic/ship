//! The full-screen client, opened by bare `ship`.

mod draw;
mod keys;
mod observer;
mod terminal;

use std::{
    collections::VecDeque,
    future::{Future, pending},
    path::Path,
    pin::Pin,
    time::Duration,
};

use crossterm::event::{Event, EventStream};
use futures_util::{Stream, StreamExt};
use ship_core::{
    command::{Command, pane::PaneCommand, tab::TabCommand},
    model::NodeId,
    prelude::*,
    protocol::{AttachRequest, EndReason, Ended, InputFrame, KeyInput, SseEvent, ViewInput},
    screen::Size,
    tree::{self, Tabs},
};
use tokio::{sync::mpsc, time::MissedTickBehavior};
use tokio_stream::wrappers::UnboundedReceiverStream;

use self::{
    draw::Status,
    keys::{Handled, Keys},
    observer::Observer,
    terminal::TerminalGuard,
};
use crate::{
    Client, KeyScope, Outcome, Scope,
    keymap::{Action, ClientAction, ClientPane, ClientTab, ConfigAction, Keymap, Sidebar},
    watch,
};

const FIRST_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(5);
/// At most one view PUT per frame.
const FRAME: Duration = Duration::from_millis(16);

type Events = Pin<Box<dyn Stream<Item = Result<SseEvent>>>>;
/// One item per save of the config file.
type Saves = Pin<Box<dyn Stream<Item = ()>>>;
/// A request the loop awaits alongside everything else.
type Pending<'a, T> = Option<Pin<Box<dyn Future<Output = Result<T>> + 'a>>>;

/// Which ring a navigation key moves through.
#[derive(Clone, Copy)]
enum Step {
    Tab,
    Pane,
}

/// Why the client stopped.
enum Exit {
    Detached,
    ServerStopped,
    Signaled,
}

/// Bare `ship`. Opens the attach stream at the terminal's tab area, then takes
/// over the terminal. Nothing is selected unless `open_first` says this `ship`
/// launched the server: then the attach asks for the first tab's pane. On
/// every `Attached` it starts the input POST for that attachment. Keys go
/// through the keymap loaded from `config`, or the defaults with the error on
/// the status line, and reloaded when the file is saved; a bad save keeps the
/// running keymap and shows the error. Bound keys queue their actions, which
/// run one at a time.
/// Navigation, created tabs and panes, and resizes mark the view dirty, and
/// the frame tick sends at most one view per frame, latest wins. The screen
/// follows the replica, not the keys. A cut connection keeps the last screen,
/// drops keys and reconnects with backoff (250 ms doubling to 5 s).
///
/// Returns Ok after detach or `Ended(ServerShutdown)`, printing why after
/// the terminal is restored. SIGTERM, SIGHUP and SIGINT restore the terminal
/// and return.
pub async fn run(client: &Client, open_first: bool, config: &Path) -> Result<()> {
    let signaled = signaled()?;
    let area = tab_area()?;
    let selection = match open_first {
        true => navigate(&client.tabs().await?, None, Step::Tab, true),
        false => None,
    };
    let request = AttachRequest { selection, area };
    // Fail on an unreachable server before taking the terminal.
    let events = attach_after(client, request.clone(), Duration::ZERO).await?;
    let exit = {
        let mut guard = TerminalGuard::enter()?;
        drive(client, &mut guard, request, events, signaled, config).await
    }?;
    match exit {
        Exit::Detached => eprintln!("detached"),
        Exit::ServerStopped => eprintln!("server stopped"),
        Exit::Signaled => {}
    }
    Ok(())
}

async fn drive(
    client: &Client,
    guard: &mut TerminalGuard,
    mut request: AttachRequest,
    events: Events,
    signaled: impl Future<Output = ()>,
    config: &Path,
) -> Result<Exit> {
    tokio::pin!(signaled);
    let mut events = Some(events);
    let mut reconnect: Pending<Events> = None;
    let mut backoff = FIRST_BACKOFF;
    let mut observer = Observer::default();
    let (keymap, mut message) = match Keymap::load(config) {
        Ok(keymap) => (keymap, None),
        Err(errors) => (Keymap::defaults(), Some(summary(&errors))),
    };
    let mut keys = Keys::new(keymap);
    let mut saves: Option<Saves> = match watch::changes(config) {
        Ok(saves) => Some(Box::pin(saves)),
        Err(error) => {
            message = message.or(Some(error.to_string()));
            None
        }
    };
    // Each bound key's actions still to run, oldest first. A failure drops
    // the rest of its key's actions.
    let mut queued: VecDeque<VecDeque<Action>> = VecDeque::new();
    // The server action in flight, resolving to what it created.
    let mut running: Pending<Option<NodeId>> = None;
    let mut terminal_events = EventStream::new();
    // Frames for the current input POST; `None` while disconnected.
    let mut input: Option<mpsc::UnboundedSender<InputFrame>> = None;
    let mut sending: Pending<()> = None;
    let mut area = request.area;
    let mut view_dirty = false;
    // Where navigation moved the selection, until a view PUT carries it.
    let mut chosen: Option<NodeId> = None;
    let mut putting: Pending<Option<NodeId>> = None;
    let mut tick = tokio::time::interval(FRAME);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    redraw(guard, &observer, &status(&keys, message.as_deref()))?;
    loop {
        let mut visible = tokio::select! {
            event = next(&mut events) => match event {
                Some(Ok(SseEvent::Ended(Ended { reason }))) => {
                    return Ok(match reason {
                        EndReason::ServerShutdown => Exit::ServerStopped,
                    });
                }
                Some(Ok(event)) => {
                    if let SseEvent::Attached(attached) = &event {
                        let (frames, received) = mpsc::unbounded_channel();
                        input = Some(frames);
                        sending = Some(Box::pin(client.input(attached.attachment, UnboundedReceiverStream::new(received))));
                        backoff = FIRST_BACKOFF;
                    }
                    observer.apply(event)
                }
                // Cut or failed: keep the screens, drop keys, reconnect.
                Some(Err(_)) | None => {
                    events = None;
                    input = None;
                    sending = None;
                    putting = None;
                    chosen = None;
                    client.forget();
                    observer.connected = false;
                    if let Some(remembered) = observer.remembered(area) {
                        request = remembered;
                    }
                    reconnect = Some(Box::pin(attach_after(client, request.clone(), backoff)));
                    true
                }
            },
            result = finish(&mut reconnect) => match result {
                Ok(stream) => {
                    reconnect = None;
                    events = Some(stream);
                    false
                }
                Err(_) => {
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                    request.area = area;
                    reconnect = Some(Box::pin(attach_after(client, request.clone(), backoff)));
                    false
                }
            },
            // The input POST ended with its attachment; the next `Attached`
            // starts another.
            _ = finish(&mut sending) => {
                sending = None;
                input = None;
                false
            }
            result = finish(&mut running) => {
                running = None;
                match result {
                    Ok(Some(created)) => {
                        chosen = Some(created);
                        view_dirty = true;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        message = Some(error.to_string());
                        queued.pop_front();
                    }
                }
                true
            }
            save = next(&mut saves) => {
                match save {
                    Some(()) => message = reload(&mut keys, config).err().map(|error| error.to_string()),
                    None => saves = None,
                }
                true
            }
            result = finish(&mut putting) => {
                putting = None;
                match result {
                    // Moved again since; that view goes out next.
                    Ok(sent) if chosen != sent => {}
                    _ => chosen = None,
                }
                false
            }
            event = terminal_events.next() => match event {
                Some(Ok(Event::Resize(cols, rows))) => {
                    area = draw::tab_area(cols, rows);
                    view_dirty = true;
                    true
                }
                Some(Ok(event)) => {
                    let selected = observer.selected().and_then(|selected| selected.pane).map(|pane| pane.id);
                    let before = keys.shown_mode().cloned();
                    match keys.handle(event, selected) {
                        Handled::Frame(frame) => {
                            if let Some(input) = &input {
                                input.send(frame).ok();
                            }
                        }
                        Handled::Run(binding) => {
                            message = None;
                            queued.push_back(binding.0.into());
                        }
                        Handled::None => {}
                    }
                    keys.shown_mode() != before.as_ref()
                }
                Some(Err(error)) => return Err(err!(Io, "cannot read the terminal", @external: error)),
                None => return Err(err!(Io, "terminal input closed")),
            },
            _ = tick.tick(), if view_dirty && putting.is_none() && observer.connected => {
                view_dirty = false;
                if let (Some(attachment), Some(record)) = (observer.attachment, observer.record()) {
                    let view = ViewInput { selection: chosen.or(record.selection), area };
                    putting = Some(Box::pin(async move {
                        client.set_view(attachment, &view).await.map(|_| view.selection)
                    }));
                }
                false
            }
            () = &mut signaled => return Ok(Exit::Signaled),
        };
        // Run queued actions up to the next server action, which waits for
        // the one in flight and then for the replica to show what it did.
        while running.is_none()
            && observer.caught_up(client.written())
            && let Some(actions) = queued.front_mut()
        {
            let Some(action) = actions.pop_front() else {
                queued.pop_front();
                continue;
            };
            visible = true;
            let done = match action {
                Action::None => Ok(()),
                Action::Server(command) => {
                    let creates = matches!(
                        command,
                        Command::Tab(TabCommand::Create(_)) | Command::Pane(PaneCommand::Create(_))
                    );
                    let scope = Scope::Keys(KeyScope {
                        selection: chosen.or(observer.record().and_then(|record| record.selection)),
                        tabs: observer
                            .replica
                            .as_ref()
                            .map(|replica| replica.tabs.clone())
                            .unwrap_or_default(),
                    });
                    running = Some(Box::pin(async move {
                        Ok(match client.execute(command, &scope).await? {
                            Outcome::Tab(tab) if creates => Some(
                                tab.panes()
                                    .next()
                                    .map(|pane| pane.id)
                                    .map_or(NodeId::Tab(tab.id), NodeId::Pane),
                            ),
                            Outcome::Pane(pane) if creates => Some(NodeId::Pane(pane.id)),
                            _ => None,
                        })
                    }));
                    Ok(())
                }
                Action::Client(action) => match action {
                    ClientAction::Tab(tab @ (ClientTab::Next {} | ClientTab::Prev {})) => {
                        let forward = matches!(tab, ClientTab::Next {});
                        moved(&observer, &mut chosen, &mut view_dirty, Step::Tab, forward);
                        Ok(())
                    }
                    ClientAction::Pane(pane @ (ClientPane::Next {} | ClientPane::Prev {})) => {
                        let forward = matches!(pane, ClientPane::Next {});
                        moved(&observer, &mut chosen, &mut view_dirty, Step::Pane, forward);
                        Ok(())
                    }
                    ClientAction::Tab(ClientTab::Select { .. }) => unavailable("tab select"),
                    ClientAction::Tab(ClientTab::Expand {}) => unavailable("tab expand"),
                    ClientAction::Tab(ClientTab::Collapse {}) => unavailable("tab collapse"),
                    ClientAction::Pane(ClientPane::Focus { .. }) => unavailable("pane focus"),
                    ClientAction::Sidebar(Sidebar::Toggle {}) => unavailable("sidebar toggle"),
                    ClientAction::Mode(mode) => {
                        keys.enter(mode);
                        Ok(())
                    }
                    ClientAction::Send(chord) => {
                        match observer.selected().and_then(|selected| selected.pane) {
                            Some(pane) => {
                                if let Some(input) = &input {
                                    input
                                        .send(InputFrame::Key(KeyInput {
                                            pane: pane.id,
                                            key: chord.into(),
                                        }))
                                        .ok();
                                }
                                Ok(())
                            }
                            None => Err(err!(Validation, "no pane selected")),
                        }
                    }
                    ClientAction::Detach {} => return Ok(Exit::Detached),
                    ClientAction::Config(ConfigAction::Reload {}) => reload(&mut keys, config),
                },
            };
            if let Err(error) = done {
                message = Some(error.to_string());
                queued.pop_front();
            }
        }
        if visible {
            redraw(guard, &observer, &status(&keys, message.as_deref()))?;
        }
    }
}

fn redraw(guard: &mut TerminalGuard, observer: &Observer, status: &Status) -> Result<()> {
    let mut cursor = None;
    guard
        .terminal
        .draw(|frame| cursor = draw::draw(frame, observer, status))
        .map_err(|error| err!(Io, "cannot draw", @external: error))?;
    guard.set_cursor(cursor)
}

fn status<'a>(keys: &'a Keys, message: Option<&'a str>) -> Status<'a> {
    Status {
        mode: keys.shown_mode(),
        message,
    }
}

/// Loads `config` into `keys`. A bad file keeps the running keymap.
fn reload(keys: &mut Keys, config: &Path) -> Result<()> {
    let keymap =
        Keymap::load(config).map_err(|errors| err!(Configuration, "{}", summary(&errors)))?;
    keys.replace(keymap);
    Ok(())
}

/// The first error of a keymap that failed to load, and how many more.
fn summary(errors: &[AppError]) -> String {
    match errors {
        [] => String::new(),
        [only] => only.to_string(),
        [first, rest @ ..] => format!("{first} (and {} more; ship config check)", rest.len()),
    }
}

fn unavailable(action: &str) -> Result<()> {
    Err(err!(Unavailable, "{} is not available yet", action))
}

/// Moves the pending selection one step around `ring`, marking the view
/// dirty when it changed.
fn moved(
    observer: &Observer,
    chosen: &mut Option<NodeId>,
    view_dirty: &mut bool,
    ring: Step,
    forward: bool,
) {
    if let (Some(replica), Some(record)) = (&observer.replica, observer.record()) {
        let from = chosen.or(record.selection);
        if let Some(to) = navigate(&replica.tabs, from, ring, forward)
            && Some(to) != from
        {
            *chosen = Some(to);
            *view_dirty = true;
        }
    }
}

/// Where one step around `ring` moves the selection `from`. Top-level tabs
/// cycle in order and land on the tab's first pane, else the tab; from
/// nothing selected they go to the first or last. Panes cycle within their
/// tab; from a tab, to its first or last pane. `None` when there is nowhere
/// to go.
fn navigate(tabs: &Tabs, from: Option<NodeId>, ring: Step, forward: bool) -> Option<NodeId> {
    match ring {
        Step::Tab => {
            let index = match from {
                Some(node) => {
                    let Some(&NodeId::Tab(top)) = tree::path(tabs, node)?.first() else {
                        return None;
                    };
                    step(tabs.get_index_of(&top)?, tabs.len(), forward)
                }
                None if forward => 0,
                None => tabs.len().checked_sub(1)?,
            };
            let (_, tab) = tabs.get_index(index)?;
            Some(
                tab.panes()
                    .next()
                    .map(|pane| pane.id)
                    .map_or(NodeId::Tab(tab.id), NodeId::Pane),
            )
        }
        Step::Pane => {
            let from = from?;
            let tab = tree::tab(tabs, tree::viewed_tab(tabs, from)?).ok()?;
            let panes: Vec<_> = tab.panes().map(|pane| pane.id).collect();
            let index = match from {
                NodeId::Pane(pane) => step(
                    panes.iter().position(|id| *id == pane)?,
                    panes.len(),
                    forward,
                ),
                NodeId::Tab(_) if forward => 0,
                NodeId::Tab(_) => panes.len().checked_sub(1)?,
            };
            let pane = *panes.get(index)?;
            Some(NodeId::Pane(pane))
        }
    }
}

/// The index after or before `index` among `len`, wrapping around.
fn step(index: usize, len: usize, forward: bool) -> usize {
    (index + if forward { 1 } else { len - 1 }) % len
}

/// Attach after `delay`; zero for the first attach.
async fn attach_after(client: &Client, request: AttachRequest, delay: Duration) -> Result<Events> {
    tokio::time::sleep(delay).await;
    Ok(Box::pin(client.attach(&request).await?))
}

// `select!` evaluates a branch's expression even when its `if` is false, so
// `events.as_mut().unwrap().next(), if events.is_some()` would panic. These
// two turn "nothing there" into "never ready" instead.

/// The next event, or never while disconnected.
async fn next<S: Stream + Unpin>(events: &mut Option<S>) -> Option<S::Item> {
    match events {
        Some(events) => events.next().await,
        None => pending().await,
    }
}

/// The pending request's result, or never while none is pending.
async fn finish<F: Future + Unpin>(request: &mut Option<F>) -> F::Output {
    match request {
        Some(request) => request.await,
        None => pending().await,
    }
}

fn tab_area() -> Result<Size> {
    let (cols, rows) = crossterm::terminal::size()
        .map_err(|error| err!(Io, "cannot read the terminal size", @external: error))?;
    Ok(draw::tab_area(cols, rows))
}

/// Resolves on SIGTERM, SIGHUP or SIGINT, so the terminal is restored before
/// exit. Raw mode turns `C-c` into a key, so SIGINT only comes from `kill`.
#[cfg(unix)]
fn signaled() -> Result<impl Future<Output = ()>> {
    use tokio::signal::unix::{SignalKind, signal};

    let register = |kind| {
        signal(kind).map_err(|error| err!(Io, "cannot register a signal handler", @external: error))
    };
    let mut terminate = register(SignalKind::terminate())?;
    let mut hangup = register(SignalKind::hangup())?;
    let mut interrupt = register(SignalKind::interrupt())?;
    Ok(async move {
        tokio::select! {
            _ = terminate.recv() => {}
            _ = hangup.recv() => {}
            _ = interrupt.recv() => {}
        }
    })
}

#[cfg(not(unix))]
fn signaled() -> Result<impl Future<Output = ()>> {
    Ok(pending())
}
