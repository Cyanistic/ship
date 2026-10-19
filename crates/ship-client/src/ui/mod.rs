//! The full-screen client, opened by bare `ship`.

mod agents;
mod draw;
mod keys;
mod lane;
mod memory;
mod menu;
mod mouse;
mod observer;
mod panes;
mod sidebar;
mod terminal;

use std::{
    collections::HashMap,
    future::{Future, pending},
    mem,
    path::Path,
    pin::Pin,
    sync::Arc,
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
    },
    geometry::TabGeometry,
    id::{Attachment, IdOf},
    layout::Axis,
    model::{NodeId, Pane, Tab},
    prelude::*,
    protocol::{
        AttachRequest, ClipboardText, EndReason, Ended, InputFrame, KeyInput, MoveTab, PasteInput,
        ResizePane, SelectView, SseEvent, ViewInput,
    },
    screen::{Screen, Size},
    tree::{self, Tabs},
};
use ship_macros::traced;
use tokio::{
    sync::mpsc,
    time::{self, MissedTickBehavior},
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use self::{
    draw::Status,
    keys::{Handled, Keys},
    lane::{Item, Lane, Turn},
    memory::Memory,
    menu::{Menu, Picked},
    mouse::{Clicked, Mouse},
    observer::Observer,
    sidebar::{Sidebar, movable},
    terminal::TerminalGuard,
};
use crate::{
    Client, KeyScope, Scope,
    keymap::{
        Action, AgentSettings, ClientAction, ClientPane, ClientSettings, ClientTab, ConfigAction,
        Menus, Sidebar as SidebarAction,
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
type Request<'a, T = ()> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

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
/// Keys, clicks and actions share one ordered lane. Server requests wait for
/// their reply and the replica before the next turn. The frame tick sends
/// area only. A cut connection keeps the last screen, drops the lane and
/// reconnects with backoff (250 ms doubling to 5 s).
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
    let mut reconnect: Option<Request<Events>> = None;
    let mut backoff = FIRST_BACKOFF;
    let (settings, message) = loaded;
    guard.set_mouse(settings.mouse)?;
    let mut local = Local {
        observer: Observer::default(),
        memory: Memory::default(),
        sidebar: Sidebar::new(settings.sidebar_width),
        keys: Keys::new(settings.keymap),
        mouse: Mouse::default(),
        menu: None,
        input: None,
        message,
        agents: settings.agents,
    };
    let mut saves: Option<Saves> = match watch::changes(config) {
        Ok(saves) => Some(Box::pin(saves)),
        Err(error) => {
            local.message = local.message.or(Some(error.to_string()));
            None
        }
    };
    let mut lane = Lane::default();
    let mut terminal_events = EventStream::new();
    let mut sending: Option<Request> = None;
    let mut terminal = terminal_size()?;
    guard.resize(terminal)?;
    let mut putting: Option<Request> = None;
    let mut tick = time::interval(FRAME);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    redraw(guard, &mut local)?;
    loop {
        let mut visible = tokio::select! {
            event = next(&mut events) => match event {
                Some(Ok(SseEvent::Ended(Ended { reason }))) => {
                    return Ok(match reason {
                        EndReason::ServerShutdown => Exit::ServerStopped,
                    });
                }
                Some(Ok(SseEvent::Clipboard(ClipboardText { text }))) => {
                    guard.copy(&text)?;
                    false
                }
                Some(Ok(event)) => {
                    if let SseEvent::Attached(attached) = &event {
                        let (frames, received) = mpsc::unbounded_channel();
                        local.input = Some(frames);
                        sending = Some(Box::pin(client.input(attached.attachment, UnboundedReceiverStream::new(received))));
                        backoff = FIRST_BACKOFF;
                    }
                    let screen = matches!(event, SseEvent::Screen(_));
                    let previous = local.observer.record().and_then(|record| record.selection);
                    let visible = local.observer.apply(event);
                    if !screen && let Some(replica) = &local.observer.replica {
                        let selection = local.observer.record().and_then(|record| record.selection);
                        local.memory.observe(&replica.tabs, selection);
                        if previous != selection {
                            local.sidebar.tabs.reveal(&replica.tabs, selection);
                        }
                    }
                    visible
                }
                // Keep the screens, but abandon all work from this connection.
                Some(Err(_)) | None => {
                    events = None;
                    local.input = None;
                    sending = None;
                    putting = None;
                    lane.clear();
                    local.mouse = Mouse::default();
                    local.menu = None;
                    client.forget();
                    local.observer.connected = false;
                    if let Some(remembered) = local.observer.remembered(view_area(terminal, &local.sidebar)) {
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
                    request.area = view_area(terminal, &local.sidebar);
                    reconnect = Some(Box::pin(attach_after(client, request.clone(), backoff)));
                    false
                }
            },
            _ = finish(&mut sending) => {
                sending = None;
                local.input = None;
                false
            }
            result = lane.finished() => {
                if let Err(error) = result {
                    local.message = Some(error.to_string());
                }
                true
            }
            save = next(&mut saves) => {
                match save {
                    Some(()) => {
                        local.message = local.reload(guard, config).err().map(|error| error.to_string());
                    }
                    None => saves = None,
                }
                true
            }
            result = finish(&mut putting) => {
                putting = None;
                if let Err(error) = result {
                    local.message = Some(error.to_string());
                }
                false
            }
            event = terminal_events.next() => match event {
                Some(Ok(Event::Resize(cols, rows))) => {
                    terminal = (cols, rows);
                    guard.resize(terminal)?;
                    local.menu = None;
                    true
                }
                Some(Ok(Event::Mouse(event))) if local.menu.is_none() && guard.mouse() && local.observer.connected => {
                    let mut mouse = mem::take(&mut local.mouse);
                    let was_dragging = mouse.dragging();
                    let clicked = mouse.route(event, &local.scene(Rect::new(0, 0, terminal.0, terminal.1)), lane.busy());
                    local.mouse = mouse;
                    let mut visible = was_dragging || local.mouse.dragging();
                    if let Some(clicked) = clicked {
                        match clicked {
                            Clicked::Select(node) => lane.push(Item::Select(node)),
                            Clicked::Fold(tab) => lane.push(Item::Fold(tab)),
                            Clicked::Move(tab) => lane.push(Item::Move(tab)),
                            Clicked::Menu(target, at) => {
                                local.menu = Menu::open(menu::items(&local.keys.keymap.menus, target), at, Rect::new(0, 0, terminal.0, terminal.1));
                                if local.menu.is_some() && let Some(node) = target.node() {
                                    lane.push(Item::Select(node));
                                }
                                visible = true;
                            }
                            Clicked::Frame(frame) => lane.push(Item::Mouse(frame)),
                            Clicked::Resize(positions) => {
                                for position in positions { lane.push(Item::Resize(position)); }
                            }
                            Clicked::Width(width) => {
                                local.sidebar.set_width(width, terminal.0);
                                visible = true;
                            }
                            Clicked::Split(row) => {
                                local.sidebar.set_split(row);
                                visible = true;
                            }
                            Clicked::SidebarScroll { at, scroll } => {
                                local.sidebar.scroll(at, scroll);
                                visible = true;
                            }
                        }
                    }
                    visible
                }
                Some(Ok(Event::Mouse(_))) if local.menu.is_none() => false,
                Some(Ok(event)) => {
                    let before = local.keys.shown_mode().cloned();
                    let mut now = false;
                    let handled = if let Some(menu) = &mut local.menu {
                        now = true;
                        match menu.handle(&event) {
                            Picked::Run(action) => {
                                local.menu = None;
                                keys::handled(action)
                            }
                            Picked::Close => {
                                local.menu = None;
                                Handled::None
                            }
                            Picked::Open => Handled::None,
                        }
                    } else {
                        local.keys.handle(event)
                    };
                    match handled {
                        Handled::Queue(item) => {
                            if matches!(item, Item::Action(_)) {
                                local.message = None;
                            }
                            // Detach is still available while disconnected, but
                            // no other waiting input survives to a new connection.
                            if local.observer.connected {
                                lane.push(item);
                            } else if matches!(item, Item::Action(Action::Client(ClientAction::Detach {}))) {
                                return Ok(Exit::Detached);
                            }
                        }
                        Handled::Now(action) => {
                            local.message = local.now(action, guard, config, terminal.0).err().map(|error| error.to_string());
                            now = true;
                        }
                        Handled::None => {}
                    }
                    now || local.keys.shown_mode() != before.as_ref()
                }
                Some(Err(error)) => return Err(err!(Io, "cannot read the terminal", @external: error)),
                None => return Err(err!(Io, "terminal input closed")),
            },
            _ = tick.tick(), if putting.is_none()
                && local.observer.connected
                && local.observer.caught_up(client.written())
                && unsent(&local.observer, view_area(terminal, &local.sidebar)).is_some() =>
            {
                if let Some((attachment, view)) = unsent(&local.observer, view_area(terminal, &local.sidebar)) {
                    putting = Some(Box::pin(async move {
                        client.set_view(attachment, &view).await.map(|_| ())
                    }));
                }
                false
            }
            () = &mut signaled => return Ok(Exit::Signaled),
        };
        while let Some(item) = lane.next(&local.observer, client.written()) {
            // Input frames change only the server's screen; redraw on its
            // publication, not on each keystroke or mouse frame.
            visible |= matches!(
                item,
                Item::Action(_) | Item::Select(_) | Item::Fold(_) | Item::Move(_)
            );
            match local.turn(client, item) {
                Turn::Done => {}
                Turn::Wait(request) => lane.wait(request),
                Turn::First(items) => lane.first(items),
                Turn::Detach => {
                    // End the body and let the existing input POST drain before
                    // dropping the attachment, otherwise queued frames are lost.
                    local.input = None;
                    if let Some(sending) = sending {
                        tokio::select! {
                            result = sending => result?,
                            () = &mut signaled => return Ok(Exit::Signaled),
                        }
                    }
                    return Ok(Exit::Detached);
                }
            }
        }
        if let Some(replica) = &local.observer.replica {
            local.sidebar.tabs.reconcile(&replica.tabs);
            local.mouse.reconcile(&replica.tabs);
        }
        if visible {
            redraw(guard, &mut local)?;
        }
    }
}

/// Client-local state used to resolve each input at its turn.
struct Local {
    observer: Observer,
    memory: Memory,
    sidebar: Sidebar,
    keys: Keys,
    mouse: Mouse,
    menu: Option<Menu>,
    input: Option<mpsc::UnboundedSender<InputFrame>>,
    message: Option<String>,
    /// Replaced whole on reload; readers take the piece they need.
    agents: AgentSettings,
}

/// Published geometry shared by the router and pane painter.
struct Scene<'a> {
    panes: Rect,
    tabs: Option<&'a Tabs>,
    tab: Option<&'a Tab>,
    geometry: Option<&'a TabGeometry>,
    selected: Option<IdOf<Pane>>,
    screens: &'a HashMap<IdOf<Pane>, Arc<Screen>>,
    sidebar: &'a Sidebar,
    menus: &'a Menus,
}

impl Local {
    fn scene(&self, screen: Rect) -> Scene<'_> {
        let selected = self.observer.selected();
        let tab = selected.as_ref().map(|selected| selected.tab);
        Scene {
            panes: draw::split(screen, self.sidebar.width()).1,
            tabs: self.observer.replica.as_ref().map(|replica| &replica.tabs),
            tab,
            geometry: tab.and_then(|tab| tab.geometry.as_deref()),
            selected: selected
                .and_then(|selected| selected.pane)
                .map(|pane| pane.id),
            screens: &self.observer.screens,
            sidebar: &self.sidebar,
            menus: &self.keys.keymap.menus,
        }
    }

    fn now(
        &mut self,
        action: ClientAction,
        guard: &mut TerminalGuard,
        config: &Path,
        cols: u16,
    ) -> Result<()> {
        match action {
            ClientAction::Mode(mode) => self.keys.enter(mode),
            ClientAction::Sidebar(SidebarAction::Toggle {}) => {
                self.sidebar.shown = !self.sidebar.shown;
                Ok(())
            }
            ClientAction::Sidebar(SidebarAction::Resize { direction, amount }) => {
                let amount = amount.unwrap_or(1);
                match direction {
                    Direction::Left | Direction::Right => {
                        self.sidebar.nudge(direction, amount, cols);
                    }
                    Direction::Up | Direction::Down => self.sidebar.nudge_split(direction, amount),
                }
                Ok(())
            }
            ClientAction::Config(ConfigAction::Reload {}) => self.reload(guard, config),
            _ => unreachable!("Keys only returns press-time actions"),
        }
    }

    /// A bad reload keeps the running settings; a valid one closes overlays
    /// and drops presses whose releases may now go to the outer terminal.
    fn reload(&mut self, guard: &mut TerminalGuard, config: &Path) -> Result<()> {
        let settings = ClientSettings::load(config)?;
        guard.set_mouse(settings.mouse)?;
        self.mouse = Mouse::default();
        self.menu = None;
        self.sidebar.configure(settings.sidebar_width);
        self.keys.replace(settings.keymap);
        self.agents = settings.agents;
        Ok(())
    }

    #[traced]
    fn turn<'a>(&mut self, client: &'a Client, item: Item) -> Turn<'a> {
        match item {
            Item::Action(action) => self.act(client, action),
            Item::Key(key) => {
                if let Some(pane) = typing_pane(&self.observer, &self.memory) {
                    self.send(InputFrame::Key(KeyInput { pane, key }));
                }
                Turn::Done
            }
            Item::Paste(text) => {
                if let Some(pane) = typing_pane(&self.observer, &self.memory) {
                    self.send(InputFrame::Paste(PasteInput { pane, text }));
                }
                Turn::Done
            }
            Item::Select(node) => {
                let Some(replica) = &self.observer.replica else {
                    return Turn::Done;
                };
                if tree::path(&replica.tabs, node).is_none() {
                    return Turn::Done;
                }
                let node = match node {
                    NodeId::Tab(id) => match tree::tab(&replica.tabs, id) {
                        Ok(tab) => self.memory.landing(tab),
                        Err(_) => return Turn::Done,
                    },
                    pane => pane,
                };
                self.select(client, node)
            }
            Item::Fold(tab) => {
                if self
                    .observer
                    .replica
                    .as_ref()
                    .is_some_and(|replica| tree::tab(&replica.tabs, tab).is_ok())
                {
                    self.sidebar.tabs.toggle(tab);
                }
                Turn::Done
            }
            Item::Mouse(frame) => {
                self.send(frame);
                Turn::Done
            }
            Item::Move(moving) => {
                if !self
                    .observer
                    .replica
                    .as_ref()
                    .is_some_and(|replica| movable(&replica.tabs, moving.tab, moving.to))
                {
                    return Turn::Done;
                }
                if let MoveTab::Parent(Some(parent)) = moving.to {
                    self.sidebar.tabs.fold(parent, false);
                }
                Turn::Wait(Box::pin(async move {
                    match client.move_tab(moving.tab, &moving.to).await {
                        Err(error)
                            if matches!(
                                error.code(),
                                ErrorCode::NotFound | ErrorCode::InvalidStructure
                            ) =>
                        {
                            Ok(())
                        }
                        result => result.map(|_| ()),
                    }
                }))
            }
            Item::Resize(position) => {
                if !self.observer.replica.as_ref().is_some_and(|replica| {
                    tree::pane_owner(&replica.tabs, position.edge.pane).is_ok()
                }) {
                    return Turn::Done;
                }
                Turn::Wait(Box::pin(async move {
                    match client
                        .resize(
                            position.edge.pane,
                            &ResizePane {
                                direction: match position.edge.axis {
                                    Axis::Horizontal => Direction::Right,
                                    Axis::Vertical => Direction::Down,
                                },
                                amount: None,
                                to: Some(position.to),
                            },
                        )
                        .await
                    {
                        Err(error) if matches!(error.code(), ErrorCode::NotFound) => Ok(()),
                        result => result.map(|_| ()),
                    }
                }))
            }
        }
    }

    #[traced]
    fn send(&mut self, frame: InputFrame) {
        if let Some(input) = &self.input {
            input.send(frame).ok();
        }
    }

    fn select<'a>(&mut self, client: &'a Client, node: NodeId) -> Turn<'a> {
        let Some(attachment) = self.observer.attachment else {
            return Turn::Done;
        };
        Turn::Wait(Box::pin(async move {
            match client
                .select(
                    attachment,
                    &SelectView {
                        selection: Some(node),
                    },
                )
                .await
            {
                Err(error) if matches!(error.code(), ErrorCode::NotFound) => Ok(()),
                result => result.map(|_| ()),
            }
        }))
    }

    fn act<'a>(&mut self, client: &'a Client, action: Action) -> Turn<'a> {
        let selection = self.observer.record().and_then(|record| record.selection);
        if matches!(action, Action::Client(ClientAction::Pane(_)))
            && let Some(zoomed) = zoomed_pane(&self.observer, selection)
        {
            return Turn::First(vec![
                Item::Action(Action::Server(Command::Pane(PaneCommand::Zoom(
                    pane::Zoom {
                        pane: PaneTarget { id: Some(zoomed) },
                        zoomed: Some(false),
                    },
                )))),
                Item::Action(action),
            ]);
        }
        let step = match &action {
            Action::Client(ClientAction::Tab(ClientTab::Next {})) => {
                Some(Step::Tab { forward: true })
            }
            Action::Client(ClientAction::Tab(ClientTab::Prev {})) => {
                Some(Step::Tab { forward: false })
            }
            Action::Client(ClientAction::Pane(ClientPane::Next {})) => {
                Some(Step::Pane { forward: true })
            }
            Action::Client(ClientAction::Pane(ClientPane::Prev {})) => {
                Some(Step::Pane { forward: false })
            }
            Action::Client(ClientAction::Pane(ClientPane::Focus { direction })) => {
                Some(Step::Focus(*direction))
            }
            _ => None,
        };
        if let Some(step) = step {
            if let Some(replica) = &self.observer.replica
                && let Some(to) =
                    navigate(&replica.tabs, &self.memory, &self.sidebar, selection, step)
                && Some(to) != selection
            {
                return self.select(client, to);
            }
            return Turn::Done;
        }
        let done = match action {
            Action::Server(command) => {
                let scope = Scope::Keys(KeyScope {
                    attachment: self.observer.attachment,
                    selection,
                    tabs: self
                        .observer
                        .replica
                        .as_ref()
                        .map(|replica| replica.tabs.clone())
                        .unwrap_or_default(),
                });
                return Turn::Wait(Box::pin(async move {
                    client.execute(command, &scope).await.map(|_| ())
                }));
            }
            Action::Client(action) => match action {
                ClientAction::Pane(ClientPane::Swap { direction }) => {
                    if let Some(replica) = &self.observer.replica
                        && let Some(NodeId::Pane(from)) = selection
                        && let Some(NodeId::Pane(other)) = navigate(
                            &replica.tabs,
                            &self.memory,
                            &self.sidebar,
                            selection,
                            Step::Focus(direction),
                        )
                    {
                        return Turn::First(vec![Item::Action(Action::Server(Command::Pane(
                            PaneCommand::Swap(pane::Swap {
                                pane: PaneTarget { id: Some(from) },
                                other,
                            }),
                        )))]);
                    }
                    Ok(())
                }
                ClientAction::Tab(ClientTab::Select { .. }) => unavailable("tab select"),
                ClientAction::Tab(action @ (ClientTab::Expand {} | ClientTab::Collapse {})) => {
                    if let Some(replica) = &self.observer.replica
                        && let Some(tab) =
                            selection.and_then(|node| tree::viewed_tab(&replica.tabs, node))
                    {
                        self.sidebar
                            .tabs
                            .fold(tab, matches!(action, ClientTab::Collapse {}));
                    }
                    Ok(())
                }
                ClientAction::Send(chord) => match typing_pane(&self.observer, &self.memory) {
                    Some(pane) => {
                        self.send(InputFrame::Key(KeyInput {
                            pane,
                            key: chord.into(),
                        }));
                        Ok(())
                    }
                    None => Err(err!(Validation, "no pane selected")),
                },
                ClientAction::Detach {} => return Turn::Detach,
                _ => unreachable!("press-time actions and navigation handled before the lane"),
            },
        };
        if let Err(error) = done {
            self.message = Some(error.to_string());
        }
        Turn::Done
    }
}

/// Tab typing uses its landing pane without changing the tab selection.
fn typing_pane(observer: &Observer, memory: &Memory) -> Option<IdOf<Pane>> {
    let selected = observer.selected()?;
    match selected.pane {
        Some(pane) => Some(pane.id),
        None => match memory.landing(selected.tab) {
            NodeId::Pane(pane) => Some(pane),
            NodeId::Tab(_) => None,
        },
    }
}

#[traced]
fn redraw(guard: &mut TerminalGuard, local: &mut Local) -> Result<()> {
    let mut cursor = None;
    guard
        .terminal
        .draw(|frame| cursor = draw::draw(frame, local))
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
            let rows = sidebar.tabs.visible(tabs);
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

/// Send only the area, never a selection that could overwrite a script's edit.
fn unsent(observer: &Observer, area: Size) -> Option<(IdOf<Attachment>, ViewInput)> {
    (observer.record()?.area != area).then_some((observer.attachment?, ViewInput { area }))
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
