use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use futures_util::future::join_all;
use indexmap::IndexMap;
use kameo::prelude::*;
use ship_core::{
    id::*,
    model::*,
    prelude::*,
    protocol::{
        AttachRequest, Attached, Create, CreateSession, MoveTab, PaneInput, Replica, ViewInput,
        ViewingRecord,
    },
    screen::Size,
};
use ship_macros::Actor;
use tokio::sync::watch;
use uuid::Uuid;

use ship_core::relay::{Publish, RelayBus};

use crate::pane::{
    self, Launch, LivePanes, PaneChange, PaneCommand, PaneEvent, PaneRuntime, Spawned,
};

pub(crate) mod tree;

use tree::Sessions;

pub(crate) type Viewers = IndexMap<IdOf<Attachment>, ViewingRecord>;

/// Sole owner of the tree, of every active attachment's viewing record and of
/// every pane's running program.
#[derive(Actor)]
#[actor(on_stop = Self::stop_panes)]
pub struct ServerState {
    incarnation: Uuid,
    revision: u64,
    sessions: Sessions,
    viewers: Viewers,
    /// Exactly the panes in `sessions` once each `commit` returns.
    runtimes: HashMap<IdOf<Pane>, PaneRuntime>,
    /// The runtimes' handles, written directly rather than through the bus,
    /// so streams find every pane of a replica they hold.
    live: watch::Sender<LivePanes>,
    bus: ActorRef<RelayBus>,
}

impl ServerState {
    pub(crate) fn new(bus: ActorRef<RelayBus>, live: watch::Sender<LivePanes>) -> Self {
        Self {
            incarnation: Uuid::now_v7(),
            revision: 0,
            sessions: Sessions::new(),
            viewers: Viewers::new(),
            runtimes: HashMap::new(),
            live,
            bus,
        }
    }

    /// Clone `sessions` and `viewers`, run `edit` on the clones, repair every
    /// viewing record (deleting those whose session is gone), swap the clones
    /// in, bump `revision` and publish. An `Err` from `edit` discards the clones,
    /// leaving state unchanged. Cloning copies only `Arc`s; `Arc::make_mut`
    /// copies the sessions an edit touches. A publish failure after the swap
    /// returns `Unavailable`; the edit stays committed. After publishing, each
    /// viewed tab's panes are sent its size.
    ///
    /// On both paths, runtimes whose pane is not in the resulting tree are
    /// dropped, which tears their programs down. That covers removed panes and
    /// a runtime started for an edit that failed.
    async fn commit<R>(
        &mut self,
        edit: impl FnOnce(&mut Sessions, &mut Viewers) -> Result<R>,
    ) -> Result<R> {
        let mut sessions = self.sessions.clone();
        let mut viewers = self.viewers.clone();
        let result = edit(&mut sessions, &mut viewers);
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.retain_runtimes();
                return Err(error);
            }
        };
        self.viewers = viewers
            .into_iter()
            .filter_map(|(attachment, record)| {
                Self::repair(&self.sessions, &sessions, record).map(|record| (attachment, record))
            })
            .collect();
        self.sessions = sessions;
        self.retain_runtimes();
        self.revision += 1;
        tracing::debug!(revision = self.revision, "publishing replica");
        self.bus
            .tell(Publish(self.replica()))
            .await
            .map_err(|error| err!(Unavailable, "cannot publish state", @external: error))?;
        self.apply_sizes();
        Ok(result)
    }

    /// Keep `record.selection` if it is still inside `record.session` in `new`.
    /// A removed pane goes to the next pane of its tab, else the previous one,
    /// while the tab stays in the session (FR-017).
    /// Otherwise walk its ancestors in `old`, nearest first, and pick the first
    /// one still inside that session in `new`. `None` when the session is gone,
    /// which deletes the record and ends its stream.
    fn repair(old: &Sessions, new: &Sessions, record: ViewingRecord) -> Option<ViewingRecord> {
        let session = record.session;
        if !new.contains_key(&session) {
            return None;
        }
        if inside(new, session, record.selection) {
            return Some(record);
        }
        // A removed pane whose tab stayed: the pane that took its place, else
        // the new last one, which was its predecessor.
        if let NodeId::Pane(pane) = record.selection
            && let Ok(owner) = tree::pane_owner(old, pane)
            && inside(new, session, NodeId::Tab(owner))
            && let (Ok(before), Ok(after)) = (tree::tab(old, owner), tree::tab(new, owner))
            && let Some(index) = before.panes.get_index_of(&pane)
            && let Some((&next, _)) = after.panes.get_index(index).or_else(|| after.panes.last())
        {
            return Some(ViewingRecord {
                selection: NodeId::Pane(next),
                ..record
            });
        }
        let selection = tree::path(old, record.selection)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .find(|node| inside(new, session, *node))
            .unwrap_or(NodeId::Session(session));
        Some(ViewingRecord {
            selection,
            ..record
        })
    }

    /// Drop runtimes whose panes are gone and publish the rest's handles,
    /// before `commit` publishes the replica.
    fn retain_runtimes(&mut self) {
        let panes: HashSet<_> = tree::pane_ids(&self.sessions).collect();
        self.runtimes.retain(|pane, _| panes.contains(pane));
        self.live.send_replace(
            self.runtimes
                .iter()
                .map(|(pane, runtime)| (*pane, runtime.handle.clone()))
                .collect(),
        );
    }

    /// Each viewed tab's size: the smallest of its viewers' terminals in each
    /// dimension, one row less for the status line. Derived, never stored.
    fn tab_sizes(&self) -> HashMap<IdOf<Tab>, Size> {
        let mut sizes: HashMap<IdOf<Tab>, Size> = HashMap::new();
        for record in self.viewers.values() {
            let Some(tab) = tree::viewed_tab(&self.sessions, record.selection) else {
                continue;
            };
            let size = Size {
                cols: record.size.cols.max(1),
                rows: record.size.rows.saturating_sub(1).max(1),
            };
            sizes
                .entry(tab)
                .and_modify(|smallest| {
                    smallest.cols = smallest.cols.min(size.cols);
                    smallest.rows = smallest.rows.min(size.rows);
                })
                .or_insert(size);
        }
        sizes
    }

    /// Send each viewed tab's size to its own panes; pane tasks drop resizes
    /// that change nothing. A tab nobody views gets nothing and keeps its size.
    fn apply_sizes(&self) {
        for (tab, size) in self.tab_sizes() {
            let Ok(tab) = tree::tab(&self.sessions, tab) else {
                continue;
            };
            for pane in tab.panes.keys() {
                if let Some(runtime) = self.runtimes.get(pane) {
                    runtime.handle.commands.send(PaneCommand::Resize(size)).ok();
                }
            }
        }
    }

    /// Start a pane's program before the commit that inserts it into `tab`,
    /// and keep its runtime. It starts at the tab's size, else 80x24. Returns
    /// the pane, running, for the edit to insert.
    fn start_pane(&mut self, tab: IdOf<Tab>, input: PaneInput) -> Result<Pane> {
        let cwd = match input.spec.cwd {
            Some(cwd) => PathBuf::from(cwd),
            None => std::env::home_dir()
                .ok_or_else(|| err!(Configuration, "the server has no home directory"))?,
        };
        if !cwd.is_absolute() || !cwd.is_dir() {
            return Err(err!(
                Validation,
                "'{}' is not an existing absolute directory",
                cwd.display()
            ));
        }
        let id = Id::new();
        let Spawned { runtime, command } = pane::spawn(
            Launch {
                pane: id,
                command: input.spec.command,
                cwd: cwd.clone(),
                size: self
                    .tab_sizes()
                    .get(&tab)
                    .copied()
                    .unwrap_or(Size::FALLBACK),
            },
            self.bus.clone(),
        )?;
        self.runtimes.insert(id, runtime);
        Ok(Pane {
            id,
            name: input.name,
            command,
            cwd: cwd.display().to_string(),
            title: None,
            status: PaneStatus::Running,
        })
    }

    /// kameo's `on_stop`. Every pane in the tree gets the full hangup, grace
    /// and kill before the actor finishes stopping.
    async fn stop_panes(
        &mut self,
        _: WeakActorRef<Self>,
        _: ActorStopReason,
    ) -> std::result::Result<(), kameo::error::Infallible> {
        join_all(self.runtimes.drain().map(|(_, runtime)| runtime.stop())).await;
        Ok(())
    }

    pub fn replica(&self) -> Arc<Replica> {
        Arc::new(Replica {
            incarnation: self.incarnation,
            revision: self.revision,
            sessions: self.sessions.clone(),
            viewers: self.viewers.clone(),
        })
    }
}

fn session(sessions: &Sessions, id: IdOf<Session>) -> Result<&Arc<Session>> {
    sessions
        .get(&id)
        .ok_or_else(|| err!(NotFound, "session {} not found", id))
}

/// Whether `node` exists in `sessions` and belongs to `session`.
fn inside(sessions: &Sessions, session: IdOf<Session>, node: NodeId) -> bool {
    tree::path(sessions, node).is_some_and(|path| path[0] == NodeId::Session(session))
}

/// Session names are unique; `except` is the session being renamed.
fn ensure_unique(
    sessions: &Sessions,
    name: &SessionName,
    except: Option<IdOf<Session>>,
) -> Result<()> {
    if sessions
        .values()
        .any(|session| &session.name == name && Some(session.id) != except)
    {
        return Err(err!(Conflict, "session name '{}' already exists", name));
    }
    Ok(())
}

pub struct ListSessions;
pub struct Get<T: Identified>(pub IdOf<T>);
/// Sessions rename to a `SessionName`; tabs and panes to an `OptionalName`.
pub struct Rename<T: Identified, N = OptionalName> {
    pub id: IdOf<T>,
    pub name: N,
}
pub struct Remove<T: Identified>(pub IdOf<T>);
pub struct Move {
    pub id: IdOf<Tab>,
    pub to: MoveTab,
}
pub struct Attach {
    pub attachment: IdOf<Attachment>,
    pub request: AttachRequest,
}
pub struct Detach(pub IdOf<Attachment>);
/// Whether an attachment is active, before the input route reads its body.
pub struct CheckAttachment(pub IdOf<Attachment>);
/// Replace an attachment's view.
pub struct SetView {
    pub attachment: IdOf<Attachment>,
    pub view: ViewInput,
}

impl Message<ListSessions> for ServerState {
    type Reply = Result<Sessions>;

    async fn handle(&mut self, _: ListSessions, _: &mut Context<Self, Self::Reply>) -> Self::Reply {
        Ok(self.sessions.clone())
    }
}

impl Message<Create<Session>> for ServerState {
    type Reply = Result<Session>;

    async fn handle(
        &mut self,
        create: Create<Session>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let CreateSession { name, starter } = create.input;
        ensure_unique(&self.sessions, &name, None)?;
        let mut tabs = IndexMap::new();
        if let Some(spec) = starter {
            let tab = Id::new();
            let pane = self.start_pane(
                tab,
                PaneInput {
                    name: OptionalName::default(),
                    spec,
                },
            )?;
            let tab = Tab {
                id: tab,
                name: OptionalName::default(),
                tabs: IndexMap::new(),
                panes: IndexMap::from([(pane.id, pane)]),
            };
            tabs.insert(tab.id, tab);
        }
        let session = Session {
            id: Id::new(),
            name,
            tabs,
        };
        self.commit(|sessions, _| {
            sessions.insert(session.id, Arc::new(session.clone()));
            Ok(session)
        })
        .await
    }
}

impl Message<Get<Session>> for ServerState {
    type Reply = Result<Session>;

    async fn handle(
        &mut self,
        Get(id): Get<Session>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        session(&self.sessions, id).map(|session| (**session).clone())
    }
}

impl Message<Rename<Session, SessionName>> for ServerState {
    type Reply = Result<Session>;

    async fn handle(
        &mut self,
        rename: Rename<Session, SessionName>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, _| {
            session(sessions, rename.id)?;
            ensure_unique(sessions, &rename.name, Some(rename.id))?;
            let session = Arc::make_mut(&mut sessions[&rename.id]);
            session.name = rename.name;
            Ok(session.clone())
        })
        .await
    }
}

impl Message<Remove<Session>> for ServerState {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        Remove(id): Remove<Session>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, _| {
            sessions
                .shift_remove(&id)
                .map(drop)
                .ok_or_else(|| err!(NotFound, "session {} not found", id))
        })
        .await
    }
}

impl Message<Create<Tab>> for ServerState {
    type Reply = Result<Tab>;

    async fn handle(
        &mut self,
        create: Create<Tab>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, _| {
            let tab = Tab {
                id: Id::new(),
                name: create.input.name,
                tabs: IndexMap::new(),
                panes: IndexMap::new(),
            };
            tree::children_mut(sessions, create.parent)?.insert(tab.id, tab.clone());
            Ok(tab)
        })
        .await
    }
}

impl Message<Get<Tab>> for ServerState {
    type Reply = Result<Tab>;

    async fn handle(
        &mut self,
        Get(id): Get<Tab>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        tree::tab(&self.sessions, id).cloned()
    }
}

impl Message<Rename<Tab>> for ServerState {
    type Reply = Result<Tab>;

    async fn handle(
        &mut self,
        rename: Rename<Tab>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, _| {
            let tab = tree::tab_mut(sessions, rename.id)?;
            tab.name = rename.name;
            Ok(tab.clone())
        })
        .await
    }
}

impl Message<Remove<Tab>> for ServerState {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        Remove(id): Remove<Tab>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, _| tree::take_tab(sessions, id).map(drop))
            .await
    }
}

impl Message<Move> for ServerState {
    type Reply = Result<Tab>;

    /// A placement sibling equal to the moving tab is gone after `take_tab`,
    /// so it fails as a missing sibling.
    async fn handle(
        &mut self,
        Move { id, to }: Move,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, _| {
            let node = NodeId::Tab(id);
            tree::path(sessions, node).ok_or_else(|| tree::not_found(node))?;
            let parent = NodeId::from(to.parent);
            let destination =
                tree::path(sessions, parent).ok_or_else(|| tree::not_found(parent))?;
            if destination.contains(&node) {
                return Err(err!(
                    InvalidStructure,
                    "cannot move tab {} into itself or its own descendant",
                    id
                ));
            }
            let tab = tree::take_tab(sessions, id)?;
            let moved = tab.clone();
            tree::place(tree::children_mut(sessions, to.parent)?, tab, to.placement)?;
            Ok(moved)
        })
        .await
    }
}

impl Message<Create<Pane>> for ServerState {
    type Reply = Result<Pane>;

    async fn handle(
        &mut self,
        create: Create<Pane>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        tree::tab(&self.sessions, create.parent)?;
        let pane = self.start_pane(create.parent, create.input)?;
        self.commit(|sessions, _| {
            tree::tab_mut(sessions, create.parent)?
                .panes
                .insert(pane.id, pane.clone());
            Ok(pane)
        })
        .await
    }
}

impl Message<Get<Pane>> for ServerState {
    type Reply = Result<Pane>;

    async fn handle(
        &mut self,
        Get(id): Get<Pane>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        tree::pane(&self.sessions, id).cloned()
    }
}

impl Message<Rename<Pane>> for ServerState {
    type Reply = Result<Pane>;

    async fn handle(
        &mut self,
        rename: Rename<Pane>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, _| {
            let owner = tree::pane_owner(sessions, rename.id)?;
            let pane = &mut tree::tab_mut(sessions, owner)?.panes[&rename.id];
            pane.name = rename.name;
            Ok(pane.clone())
        })
        .await
    }
}

impl Message<Remove<Pane>> for ServerState {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        Remove(id): Remove<Pane>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, _| {
            let owner = tree::pane_owner(sessions, id)?;
            tree::tab_mut(sessions, owner)?.panes.shift_remove(&id);
            Ok(())
        })
        .await
    }
}

impl Message<Attach> for ServerState {
    type Reply = Result<Attached>;

    /// Commit the new record and return it with the committed replica as the
    /// stream's seed. A requested selection is kept only if it is inside the
    /// requested session now; otherwise the record starts at the session's
    /// first pane, else the session itself.
    async fn handle(
        &mut self,
        Attach {
            attachment,
            request,
        }: Attach,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, viewers| {
            session(sessions, request.session)?;
            if viewers.contains_key(&attachment) {
                return Err(err!(Conflict, "attachment {} already exists", attachment));
            }
            let selection = request
                .selection
                .filter(|node| inside(sessions, request.session, *node))
                .or_else(|| tree::first_pane(sessions, request.session).map(NodeId::Pane))
                .unwrap_or(NodeId::Session(request.session));
            let record = ViewingRecord {
                session: request.session,
                selection,
                size: request.size,
            };
            viewers.insert(attachment, record);
            Ok(())
        })
        .await?;
        Ok(Attached {
            attachment,
            replica: self.replica(),
        })
    }
}

impl Message<Detach> for ServerState {
    type Reply = Result<()>;

    /// Idempotent: an attachment already gone, for example with its session,
    /// commits nothing.
    async fn handle(
        &mut self,
        Detach(attachment): Detach,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if !self.viewers.contains_key(&attachment) {
            return Ok(());
        }
        self.commit(|_, viewers| {
            viewers.shift_remove(&attachment);
            Ok(())
        })
        .await
    }
}

impl Message<CheckAttachment> for ServerState {
    type Reply = Result<u64>;

    /// The current revision, whose replica holds the attachment; 404 if it
    /// isn't active.
    async fn handle(
        &mut self,
        CheckAttachment(attachment): CheckAttachment,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if !self.viewers.contains_key(&attachment) {
            return Err(err!(NotFound, "attachment {} not found", attachment));
        }
        Ok(self.revision)
    }
}

impl Message<SetView> for ServerState {
    type Reply = Result<ViewingRecord>;

    /// The record becomes the view, in the selection's session, and is
    /// returned as stored. A selection no longer in the tree, such as a pane
    /// removed a moment ago, leaves the record as it was. 404 if the
    /// attachment isn't active.
    async fn handle(
        &mut self,
        SetView { attachment, view }: SetView,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = self
            .viewers
            .get(&attachment)
            .cloned()
            .ok_or_else(|| err!(NotFound, "attachment {} not found", attachment))?;
        let Some(session) = tree::session_of(&self.sessions, view.selection) else {
            return Ok(record);
        };
        let record = ViewingRecord {
            session,
            selection: view.selection,
            size: view.size,
        };
        self.commit(|_, viewers| {
            viewers.insert(attachment, record.clone());
            Ok(record)
        })
        .await
    }
}

impl Message<PaneEvent> for ServerState {
    type Reply = Result<()>;

    /// Title or exit. Ignored for panes no longer in the tree.
    async fn handle(
        &mut self,
        PaneEvent { pane: id, change }: PaneEvent,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if tree::pane(&self.sessions, id).is_err() {
            return Ok(());
        }
        self.commit(|sessions, _| {
            let owner = tree::pane_owner(sessions, id)?;
            let pane = &mut tree::tab_mut(sessions, owner)?.panes[&id];
            match change {
                PaneChange::Title(title) => pane.title = title,
                PaneChange::Exited(status) => pane.status = PaneStatus::Exited(status),
            }
            Ok(())
        })
        .await
    }
}
