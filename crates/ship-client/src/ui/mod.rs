//! The full-screen client, opened by bare `ship`.

mod draw;
mod keys;
mod observer;
mod terminal;

use std::{
    future::{Future, pending},
    pin::Pin,
    time::Duration,
};

use crossterm::event::{Event, EventStream};
use futures_util::{Stream, StreamExt};
use ship_core::{
    model::NodeId,
    prelude::*,
    protocol::{AttachRequest, EndReason, Ended, InputFrame, SseEvent, ViewInput},
    screen::Size,
    tree::{self, Tabs},
};
use tokio::{sync::mpsc, time::MissedTickBehavior};
use tokio_stream::wrappers::UnboundedReceiverStream;

use self::{
    keys::{Action, Keys},
    observer::Observer,
    terminal::TerminalGuard,
};
use crate::Client;

const FIRST_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(5);
/// At most one view PUT per frame.
const FRAME: Duration = Duration::from_millis(16);

type Events = Pin<Box<dyn Stream<Item = Result<SseEvent>>>>;
/// A request the loop awaits alongside everything else.
type Pending<'a, T> = Option<Pin<Box<dyn Future<Output = Result<T>> + 'a>>>;

/// Why the client stopped.
enum Exit {
    Detached,
    ServerStopped,
    Signaled,
}

/// Bare `ship`. Opens the attach stream at the terminal's size with nothing
/// selected, then takes over the terminal. On every `Attached` it starts the input POST for that
/// attachment. Navigation keys and resizes mark the view dirty, and the frame
/// tick sends at most one view per frame, latest wins. The screen follows the
/// replica, not the keys. A cut connection keeps the last screen,
/// drops keys and reconnects with backoff (250 ms doubling to 5 s).
///
/// Returns Ok after detach or `Ended(ServerShutdown)`, printing why after
/// the terminal is restored. SIGTERM, SIGHUP and SIGINT restore the terminal
/// and return.
pub async fn run(client: &Client) -> Result<()> {
    let signaled = signaled()?;
    let size = terminal_size()?;
    let request = AttachRequest {
        selection: None,
        size,
    };
    // Fail on an unreachable server before taking the terminal.
    let events = attach_after(client, request.clone(), Duration::ZERO).await?;
    let exit = {
        let mut guard = TerminalGuard::enter()?;
        drive(client, &mut guard, request, events, signaled).await
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
) -> Result<Exit> {
    tokio::pin!(signaled);
    let mut events = Some(events);
    let mut reconnect: Pending<Events> = None;
    let mut backoff = FIRST_BACKOFF;
    let mut observer = Observer::default();
    let mut keys = Keys::default();
    let mut terminal_events = EventStream::new();
    // Frames for the current input POST; `None` while disconnected.
    let mut input: Option<mpsc::UnboundedSender<InputFrame>> = None;
    let mut sending: Pending<()> = None;
    let mut size = request.size;
    let mut view_dirty = false;
    // Where navigation moved the selection, until a view PUT carries it.
    let mut chosen: Option<NodeId> = None;
    let mut putting: Pending<Option<NodeId>> = None;
    let mut tick = tokio::time::interval(FRAME);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    redraw(guard, &observer)?;
    loop {
        let visible = tokio::select! {
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
                    observer.connected = false;
                    if let Some(remembered) = observer.remembered(size) {
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
                    request.size = size;
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
                    size = Size { cols, rows };
                    view_dirty = true;
                    true
                }
                Some(Ok(event)) => {
                    let selected = observer.selected().and_then(|selected| selected.pane).map(|pane| pane.id);
                    match keys.handle(event, selected) {
                        Action::Frame(frame) => {
                            if let Some(input) = &input {
                                input.send(frame).ok();
                            }
                        }
                        Action::Detach => return Ok(Exit::Detached),
                        Action::None => {}
                        action => {
                            if let (Some(replica), Some(record)) = (&observer.replica, observer.record()) {
                                let from = chosen.or(record.selection);
                                if let Some(to) = navigate(&replica.tabs, from, &action)
                                    && Some(to) != from
                                {
                                    chosen = Some(to);
                                    view_dirty = true;
                                }
                            }
                        }
                    }
                    false
                }
                Some(Err(error)) => return Err(err!(Io, "cannot read the terminal", @external: error)),
                None => return Err(err!(Io, "terminal input closed")),
            },
            _ = tick.tick(), if view_dirty && putting.is_none() && observer.connected => {
                view_dirty = false;
                if let (Some(attachment), Some(record)) = (observer.attachment, observer.record()) {
                    let view = ViewInput { selection: chosen.or(record.selection), size };
                    putting = Some(Box::pin(async move {
                        client.set_view(attachment, &view).await.map(|_| view.selection)
                    }));
                }
                false
            }
            () = &mut signaled => return Ok(Exit::Signaled),
        };
        if visible {
            redraw(guard, &observer)?;
        }
    }
}

fn redraw(guard: &mut TerminalGuard, observer: &Observer) -> Result<()> {
    guard
        .terminal
        .draw(|frame| draw::draw(frame, observer))
        .map_err(|error| err!(Io, "cannot draw", @external: error))?;
    let screen = observer
        .selected()
        .and_then(|selected| selected.pane)
        .and_then(|pane| observer.screens.get(&pane.id));
    guard.set_cursor(screen.and_then(|screen| screen.cursor))
}

/// Where an action moves the selection `from`. Top-level tabs cycle in order
/// and land on the tab's first pane, else the tab; from nothing selected they
/// go to the first or last. Panes cycle within their tab; from a tab, to its
/// first or last pane. `None` when there is nowhere to go.
fn navigate(tabs: &Tabs, from: Option<NodeId>, action: &Action) -> Option<NodeId> {
    match action {
        Action::NextTab | Action::PrevTab => {
            let forward = matches!(action, Action::NextTab);
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
            Some(tree::first_pane(tab).map_or(NodeId::Tab(tab.id), NodeId::Pane))
        }
        Action::NextPane | Action::PrevPane => {
            let from = from?;
            let tab = tree::tab(tabs, tree::viewed_tab(tabs, from)?).ok()?;
            let forward = matches!(action, Action::NextPane);
            let index = match from {
                NodeId::Pane(pane) => {
                    step(tab.panes.get_index_of(&pane)?, tab.panes.len(), forward)
                }
                NodeId::Tab(_) if forward => 0,
                NodeId::Tab(_) => tab.panes.len().checked_sub(1)?,
            };
            let (&pane, _) = tab.panes.get_index(index)?;
            Some(NodeId::Pane(pane))
        }
        Action::Frame(_) | Action::Detach | Action::None => None,
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

fn terminal_size() -> Result<Size> {
    let (cols, rows) = crossterm::terminal::size()
        .map_err(|error| err!(Io, "cannot read the terminal size", @external: error))?;
    Ok(Size { cols, rows })
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
