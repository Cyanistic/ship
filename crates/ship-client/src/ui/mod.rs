//! The full-screen client, opened by bare `ship`.

mod draw;
mod keys;
mod memory;
mod mouse;
mod observer;
mod panes;
mod sidebar;
mod terminal;

use std::{
    collections::VecDeque,
    future::{Future, pending},
    path::Path,
    pin::Pin,
    time::Duration,
};

use crossterm::{
    event::{Event, EventStream},
    terminal::size,
};
use futures_util::{Stream, StreamExt};
use ratatui::layout::Rect;
use ship_core::{
    command::{
        Command, Direction, PaneTarget,
        pane::{self, PaneCommand},
        tab::TabCommand,
    },
    id::{Attachment, IdOf},
    model::{NodeId, Pane},
    prelude::*,
    protocol::{AttachRequest, EndReason, Ended, InputFrame, KeyInput, SseEvent, ViewInput},
    screen::Size,
    tree::{self, Tabs},
};
use tokio::{
    sync::mpsc,
    time::{self, MissedTickBehavior},
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use self::{
    draw::Status,
    keys::{Handled, Keys},
    memory::Memory,
    mouse::{Clicked, Mouse},
    observer::Observer,
    sidebar::Sidebar,
    terminal::TerminalGuard,
};
use crate::{
    Client, KeyScope, Outcome, Scope,
    keymap::{
        Action, ClientAction, ClientPane, ClientSettings, ClientTab, ConfigAction,
        Sidebar as SidebarAction,
    },
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

/// Where a navigation key moves the selection.
#[derive(Clone, Copy)]
enum Step {
    /// Around the visible sidebar rows.
    Tab { forward: bool },
    /// Around the viewed tab's panes in layout order.
    Pane { forward: bool },
    /// To the selected pane's neighbor on that side.
    Focus(Direction),
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
    let loaded = match ClientSettings::load(config) {
        Ok(settings) => (settings, None),
        Err(error) => (ClientSettings::defaults(), Some(error.to_string())),
    };
    let (cols, rows) = terminal_size()?;
    let area = draw::tab_area(cols, rows, loaded.0.sidebar_width);
    let selection = match open_first {
        true => navigate(
            &client.tabs().await?,
            &Memory::default(),
            &Sidebar::new(loaded.0.sidebar_width),
            None,
            Step::Tab { forward: true },
        ),
        false => None,
    };
    let request = AttachRequest { selection, area };
    // Fail on an unreachable server before taking the terminal.
    let events = attach_after(client, request.clone(), Duration::ZERO).await?;
    let exit = {
        let mut guard = TerminalGuard::enter()?;
        drive(
            client, &mut guard, request, events, signaled, config, loaded,
        )
        .await
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
    loaded: (ClientSettings, Option<String>),
) -> Result<Exit> {
    tokio::pin!(signaled);
    let mut events = Some(events);
    let mut reconnect: Pending<Events> = None;
    let mut backoff = FIRST_BACKOFF;
    let mut observer = Observer::default();
    let mut memory = Memory::default();
    let (settings, mut message) = loaded;
    guard.set_mouse(settings.mouse)?;
    let mut mouse = Mouse::default();
    let mut sidebar = Sidebar::new(settings.sidebar_width);
    let mut keys = Keys::new(settings.keymap);
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
    let mut terminal = terminal_size()?;
    // Where navigation moved the selection, until a view PUT carries it.
    let mut chosen: Option<NodeId> = None;
    let mut putting: Pending<Option<NodeId>> = None;
    let mut tick = time::interval(FRAME);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    redraw(
        guard,
        &observer,
        &mut sidebar,
        &status(&keys, message.as_deref()),
    )?;
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
                    let screen = matches!(event, SseEvent::Screen(_));
                    let previous = observer.record().and_then(|record| record.selection);
                    let visible = observer.apply(event);
                    if !screen && let Some(replica) = &observer.replica {
                        let selection = observer.record().and_then(|record| record.selection);
                        memory.observe(&replica.tabs, selection);
                        if previous != selection {
                            sidebar.reveal(&replica.tabs, selection);
                        }
                    }
                    visible
                }
                // Cut or failed: keep the screens, drop keys, reconnect.
                Some(Err(_)) | None => {
                    events = None;
                    input = None;
                    sending = None;
                    putting = None;
                    chosen = None;
                    mouse = Mouse::default();
                    client.forget();
                    observer.connected = false;
                    if let Some(remembered) = observer.remembered(view_area(terminal, &sidebar)) {
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
                    request.area = view_area(terminal, &sidebar);
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
                    Ok(Some(created)) => chosen = Some(created),
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
                    Some(()) => {
                        message = reload(&mut keys, &mut sidebar, guard, &mut mouse, config).err().map(|error| error.to_string());
                    }
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
                    terminal = (cols, rows);
                    true
                }
                Some(Ok(Event::Mouse(event))) if guard.mouse() && observer.connected => {
                    let selected = observer.selected();
                    let (_, panes) = draw::split(Rect::new(0, 0, terminal.0, terminal.1), sidebar.width());
                    let clicked = mouse.route(
                        event,
                        panes,
                        selected.as_ref().and_then(|selected| selected.tab.geometry.as_deref()),
                        selected.and_then(|selected| selected.pane).map(|pane| pane.id),
                        &sidebar,
                    );
                    match clicked {
                        Some(Clicked::Select(NodeId::Tab(id))) => {
                            if let Some(replica) = &observer.replica
                                && let Ok(tab) = tree::tab(&replica.tabs, id) {
                                chosen = Some(memory.landing(tab));
                            }
                            true
                        }
                        Some(Clicked::Select(node)) => {
                            chosen = Some(node);
                            true
                        }
                        Some(Clicked::Fold(tab)) => {
                            sidebar.toggle(tab);
                            true
                        }
                        Some(Clicked::Frame(frame)) => {
                            if let Some(input) = &input { input.send(frame).ok(); }
                            false
                        }
                        None => false,
                    }
                }
                // Decoded before capture went off.
                Some(Ok(Event::Mouse(_))) => false,
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
                            queued.push_back(binding.actions.into());
                        }
                        Handled::None => {}
                    }
                    keys.shown_mode() != before.as_ref()
                }
                Some(Err(error)) => return Err(err!(Io, "cannot read the terminal", @external: error)),
                None => return Err(err!(Io, "terminal input closed")),
            },
            // At most one view PUT per frame, and only once the replica shows
            // the last one, so a record that hasn't caught up isn't resent.
            _ = tick.tick(), if putting.is_none()
                && observer.connected
                && observer.caught_up(client.written())
                && unsent(&observer, chosen, view_area(terminal, &sidebar)).is_some() =>
            {
                if let Some((attachment, view)) = unsent(&observer, chosen, view_area(terminal, &sidebar)) {
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
            let selection = chosen.or(observer.record().and_then(|record| record.selection));
            // Leave zoom for everyone first. The queue waits for the unzoom's
            // revision, then runs the action again on the unzoomed geometry.
            if matches!(action, Action::Client(ClientAction::Pane(_)))
                && let Some(zoomed) = zoomed_pane(&observer, selection)
            {
                actions.push_front(action);
                actions.push_front(Action::Server(Command::Pane(PaneCommand::Zoom(
                    pane::Zoom {
                        pane: PaneTarget { id: Some(zoomed) },
                        zoomed: Some(false),
                    },
                ))));
                continue;
            }
            let done = match action {
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
                        moved(
                            &observer,
                            &memory,
                            &sidebar,
                            &mut chosen,
                            Step::Tab { forward },
                        );
                        Ok(())
                    }
                    ClientAction::Pane(pane @ (ClientPane::Next {} | ClientPane::Prev {})) => {
                        let forward = matches!(pane, ClientPane::Next {});
                        moved(
                            &observer,
                            &memory,
                            &sidebar,
                            &mut chosen,
                            Step::Pane { forward },
                        );
                        Ok(())
                    }
                    ClientAction::Pane(ClientPane::Focus { direction }) => {
                        moved(
                            &observer,
                            &memory,
                            &sidebar,
                            &mut chosen,
                            Step::Focus(direction),
                        );
                        Ok(())
                    }
                    ClientAction::Pane(ClientPane::Swap { direction }) => {
                        // Only the resolved pair goes to the server.
                        if let Some(replica) = &observer.replica
                            && let Some(NodeId::Pane(from)) = selection
                            && let Some(other) = navigate(
                                &replica.tabs,
                                &memory,
                                &sidebar,
                                selection,
                                Step::Focus(direction),
                            )
                            .and_then(|to| match to {
                                NodeId::Pane(other) => Some(other),
                                NodeId::Tab(_) => None,
                            })
                        {
                            actions.push_front(Action::Server(Command::Pane(PaneCommand::Swap(
                                pane::Swap {
                                    pane: PaneTarget { id: Some(from) },
                                    other,
                                },
                            ))));
                        }
                        Ok(())
                    }
                    ClientAction::Tab(ClientTab::Select { .. }) => unavailable("tab select"),
                    ClientAction::Tab(action @ (ClientTab::Expand {} | ClientTab::Collapse {})) => {
                        if let Some(replica) = &observer.replica
                            && let Some(tab) = chosen
                                .or(observer.record().and_then(|record| record.selection))
                                .and_then(|node| tree::viewed_tab(&replica.tabs, node))
                        {
                            sidebar.fold(tab, matches!(action, ClientTab::Collapse {}));
                        }
                        Ok(())
                    }
                    ClientAction::Sidebar(SidebarAction::Toggle {}) => {
                        sidebar.shown = !sidebar.shown;
                        Ok(())
                    }
                    ClientAction::Mode(mode) => keys.enter(mode),
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
                    ClientAction::Config(ConfigAction::Reload {}) => {
                        reload(&mut keys, &mut sidebar, guard, &mut mouse, config)
                    }
                },
            };
            if let Err(error) = done {
                message = Some(error.to_string());
                queued.pop_front();
            }
        }
        if let Some(replica) = &observer.replica {
            sidebar.reconcile(&replica.tabs);
        }
        if visible {
            redraw(
                guard,
                &observer,
                &mut sidebar,
                &status(&keys, message.as_deref()),
            )?;
        }
    }
}

fn redraw(
    guard: &mut TerminalGuard,
    observer: &Observer,
    sidebar: &mut Sidebar,
    status: &Status,
) -> Result<()> {
    let mut cursor = None;
    guard
        .terminal
        .draw(|frame| cursor = draw::draw(frame, observer, sidebar, status))
        .map_err(|error| err!(Io, "cannot draw", @external: error))?;
    guard.set_cursor(cursor)
}

fn status<'a>(keys: &'a Keys, message: Option<&'a str>) -> Status<'a> {
    Status {
        mode: keys.shown_mode(),
        message,
        hints: keys
            .shown_mode()
            .and_then(|mode| keys.keymap.modes.get(mode))
            .map(|mode| mode.hints().collect())
            .unwrap_or_default(),
    }
}

/// Loads `config` into `keys`, the sidebar and mouse capture. A bad file
/// keeps the running settings. A press held across the reload is dropped,
/// since its release may go to the outer terminal.
fn reload(
    keys: &mut Keys,
    sidebar: &mut Sidebar,
    guard: &mut TerminalGuard,
    mouse: &mut Mouse,
    config: &Path,
) -> Result<()> {
    let settings = ClientSettings::load(config)?;
    guard.set_mouse(settings.mouse)?;
    *mouse = Mouse::default();
    sidebar.width = settings.sidebar_width;
    keys.replace(settings.keymap);
    Ok(())
}

fn unavailable(action: &str) -> Result<()> {
    Err(err!(Unavailable, "{} is not available yet", action))
}

/// The selected pane, when its tab is zoomed onto it.
fn zoomed_pane(observer: &Observer, selection: Option<NodeId>) -> Option<IdOf<Pane>> {
    let Some(NodeId::Pane(pane)) = selection else {
        return None;
    };
    let tabs = &observer.replica.as_ref()?.tabs;
    let tab = tree::tab(tabs, tree::pane_owner(tabs, pane).ok()?).ok()?;
    (tab.zoomed == Some(pane)).then_some(pane)
}

/// Moves the pending selection one `step`.
fn moved(
    observer: &Observer,
    memory: &Memory,
    sidebar: &Sidebar,
    chosen: &mut Option<NodeId>,
    step: Step,
) {
    if let (Some(replica), Some(record)) = (&observer.replica, observer.record()) {
        let from = chosen.or(record.selection);
        if let Some(to) = navigate(&replica.tabs, memory, sidebar, from, step)
            && Some(to) != from
        {
            *chosen = Some(to);
        }
    }
}

/// Where `step` moves the selection `from`. Visible tab rows cycle in order and
/// land where `memory` says; from nothing selected they go to the first or
/// last. Panes cycle within their tab; from a tab, to its first or last pane.
/// Focus goes to the neighbor in the tab's published geometry, ties to the
/// panes `memory` selected most recently. `None` when there is nowhere to go.
fn navigate(
    tabs: &Tabs,
    memory: &Memory,
    sidebar: &Sidebar,
    from: Option<NodeId>,
    step: Step,
) -> Option<NodeId> {
    match step {
        Step::Tab { forward } => {
            let rows = sidebar.visible(tabs);
            let index = match from {
                Some(node) => {
                    let tab = tree::viewed_tab(tabs, node)?;
                    self::step(rows.iter().position(|id| *id == tab)?, rows.len(), forward)
                }
                None if forward => 0,
                None => rows.len().checked_sub(1)?,
            };
            let tab = tree::tab(tabs, *rows.get(index)?).ok()?;
            Some(memory.landing(tab))
        }
        Step::Pane { forward } => {
            let from = from?;
            let tab = tree::tab(tabs, tree::viewed_tab(tabs, from)?).ok()?;
            let panes: Vec<_> = tab.panes().map(|pane| pane.id).collect();
            let index = match from {
                NodeId::Pane(pane) => self::step(
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
        Step::Focus(direction) => {
            let Some(NodeId::Pane(from)) = from else {
                return None;
            };
            let tab = tree::tab(tabs, tree::pane_owner(tabs, from).ok()?).ok()?;
            let to = tab
                .geometry
                .as_ref()?
                .neighbor(from, direction, memory.recent())?;
            Some(NodeId::Pane(to))
        }
    }
}

/// The index after or before `index` among `len`, wrapping around.
fn step(index: usize, len: usize, forward: bool) -> usize {
    (index + if forward { 1 } else { len - 1 }) % len
}

/// Attach after `delay`; zero for the first attach.
async fn attach_after(client: &Client, request: AttachRequest, delay: Duration) -> Result<Events> {
    time::sleep(delay).await;
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

fn terminal_size() -> Result<(u16, u16)> {
    size().map_err(|error| err!(Io, "cannot read the terminal size", @external: error))
}

/// The tab area a `terminal` of (cols, rows) leaves beside `sidebar`.
fn view_area((cols, rows): (u16, u16), sidebar: &Sidebar) -> Size {
    draw::tab_area(cols, rows, sidebar.width())
}

/// The view this client wants the server to hold, with its attachment, when
/// the server's record of it differs: the pending selection, else the
/// recorded one, at `area`. `None` before the first `Attached` event.
fn unsent(
    observer: &Observer,
    chosen: Option<NodeId>,
    area: Size,
) -> Option<(IdOf<Attachment>, ViewInput)> {
    let record = observer.record()?;
    let view = ViewInput {
        selection: chosen.or(record.selection),
        area,
    };
    (view.selection != record.selection || view.area != record.area)
        .then_some((observer.attachment?, view))
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
