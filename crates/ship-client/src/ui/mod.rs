//! `ship attach`: the full-screen client.

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
    id::IdOf,
    model::Session,
    prelude::*,
    protocol::{AttachRequest, EndReason, Ended, InputFrame, SseEvent, ViewInput, ViewingRecord},
    screen::Size,
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
    SessionRemoved,
    ServerStopped,
    Signaled,
}

/// `ship attach`. Opens the attach stream at the terminal's size, then takes
/// over the terminal. On every `Attached` it starts the input POST for that
/// attachment. Resizes mark the view dirty, and the frame tick sends at most
/// one view per frame, latest wins. A cut connection keeps the last screen,
/// drops keys and reconnects with backoff (250 ms doubling to 5 s).
///
/// Returns Ok after detach, `Ended(SessionRemoved)`, `Ended(ServerShutdown)`
/// or a 404 on reattach, printing why after the terminal is restored. SIGTERM,
/// SIGHUP and SIGINT restore the terminal and return.
pub async fn run(client: &Client, session: IdOf<Session>) -> Result<()> {
    let signaled = signaled()?;
    let size = terminal_size()?;
    let request = AttachRequest {
        session,
        selection: None,
        size,
    };
    // Fail on an unknown session before taking the terminal.
    let events = attach_after(client, request.clone(), Duration::ZERO).await?;
    let exit = {
        let mut guard = TerminalGuard::enter()?;
        drive(client, &mut guard, request, events, signaled).await
    }?;
    match exit {
        Exit::Detached => eprintln!("detached"),
        Exit::SessionRemoved => eprintln!("session removed"),
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
    let mut putting: Pending<ViewingRecord> = None;
    let mut tick = tokio::time::interval(FRAME);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    redraw(guard, &observer)?;
    loop {
        let visible = tokio::select! {
            event = next(&mut events) => match event {
                Some(Ok(SseEvent::Ended(Ended { reason }))) => {
                    return Ok(match reason {
                        EndReason::SessionRemoved => Exit::SessionRemoved,
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
                Err(error) if *error.code() == ErrorCode::NotFound => return Ok(Exit::SessionRemoved),
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
            _ = finish(&mut putting) => {
                putting = None;
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
                    }
                    false
                }
                Some(Err(error)) => return Err(err!(Io, "cannot read the terminal", @external: error)),
                None => return Err(err!(Io, "terminal input closed")),
            },
            _ = tick.tick(), if view_dirty && putting.is_none() && observer.connected => {
                view_dirty = false;
                if let (Some(attachment), Some(record)) = (observer.attachment, observer.record()) {
                    let view = ViewInput { selection: record.selection, size };
                    putting = Some(Box::pin(async move { client.set_view(attachment, &view).await }));
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
