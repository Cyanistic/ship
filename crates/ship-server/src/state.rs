use std::sync::Arc;

use indexmap::IndexMap;
use kameo::prelude::*;
use ship_core::{
    id::*,
    model::*,
    prelude::*,
    protocol::{AttachRequest, Attached, Create, MoveTab, Replica, ViewingRecord},
};
use ship_macros::Actor;
use uuid::Uuid;

use ship_core::relay::{Publish, RelayBus};

mod tree;

pub(crate) type Sessions = IndexMap<IdOf<Session>, Arc<Session>>;
pub(crate) type Viewers = IndexMap<IdOf<Attachment>, ViewingRecord>;

/// Sole owner of the tree and of every active attachment's viewing record.
#[derive(Actor)]
pub struct ServerState {
    incarnation: Uuid,
    revision: u64,
    sessions: Sessions,
    viewers: Viewers,
    bus: ActorRef<RelayBus>,
}

impl ServerState {
    pub fn new(bus: ActorRef<RelayBus>) -> Self {
        Self {
            incarnation: Uuid::now_v7(),
            revision: 0,
            sessions: Sessions::new(),
            viewers: Viewers::new(),
            bus,
        }
    }

    /// Clone `sessions` and `viewers`, run `edit` on the clones, repair every
    /// viewing record (deleting those whose session is gone), swap the clones
    /// in, bump `revision` and publish. An `Err` from `edit` discards the clones,
    /// leaving state unchanged. Cloning copies only `Arc`s; `Arc::make_mut`
    /// copies the sessions an edit touches. A publish failure after the swap
    /// returns `Unavailable`; the edit stays committed.
    async fn commit<R>(
        &mut self,
        edit: impl FnOnce(&mut Sessions, &mut Viewers) -> Result<R>,
    ) -> Result<R> {
        let mut sessions = self.sessions.clone();
        let mut viewers = self.viewers.clone();
        let result = edit(&mut sessions, &mut viewers)?;
        self.viewers = viewers
            .into_iter()
            .filter_map(|(attachment, record)| {
                Self::repair(&self.sessions, &sessions, record).map(|record| (attachment, record))
            })
            .collect();
        self.sessions = sessions;
        self.revision += 1;
        tracing::debug!(revision = self.revision, "publishing replica");
        self.bus
            .tell(Publish(self.replica()))
            .await
            .map_err(|error| err!(Unavailable, "cannot publish state", @external: error))?;
        Ok(result)
    }

    /// Keep `record.selection` if it is still inside `record.session` in `new`.
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
        let selection = tree::path(old, record.selection)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .find(|node| inside(new, session, *node))
            .unwrap_or(NodeId::Session(session));
        Some(ViewingRecord { session, selection })
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

fn viewer(viewers: &mut Viewers, attachment: IdOf<Attachment>) -> Result<&mut ViewingRecord> {
    viewers
        .get_mut(&attachment)
        .ok_or_else(|| err!(NotFound, "attachment {} not found", attachment))
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
pub struct Rename<T: Identified> {
    pub id: IdOf<T>,
    pub name: String,
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
pub struct Select {
    pub attachment: IdOf<Attachment>,
    pub selection: NodeId,
}
pub struct SwitchSession {
    pub attachment: IdOf<Attachment>,
    pub session: IdOf<Session>,
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
        self.commit(|sessions, _| {
            ensure_unique(sessions, &create.input.name, None)?;
            let session = Session {
                id: Id::new(),
                name: create.input.name,
                tabs: IndexMap::new(),
            };
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

impl Message<Rename<Session>> for ServerState {
    type Reply = Result<Session>;

    async fn handle(
        &mut self,
        rename: Rename<Session>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let name: SessionName = rename.name.parse()?;
        self.commit(|sessions, _| {
            session(sessions, rename.id)?;
            ensure_unique(sessions, &name, Some(rename.id))?;
            let session = Arc::make_mut(&mut sessions[&rename.id]);
            session.name = name;
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
        self.commit(|sessions, _| {
            let pane = Pane {
                id: Id::new(),
                name: create.input.name,
            };
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
    /// requested session now; otherwise the record starts at the session root.
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
                .unwrap_or(NodeId::Session(request.session));
            let record = ViewingRecord {
                session: request.session,
                selection,
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

impl Message<Select> for ServerState {
    type Reply = Result<ViewingRecord>;

    /// The selection must exist and belong to the attachment's session.
    async fn handle(
        &mut self,
        Select {
            attachment,
            selection,
        }: Select,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, viewers| {
            let record = viewer(viewers, attachment)?;
            tree::path(sessions, selection).ok_or_else(|| tree::not_found(selection))?;
            if !inside(sessions, record.session, selection) {
                return Err(err!(
                    InvalidStructure,
                    "{} is not in attached session {}",
                    selection,
                    record.session
                ));
            }
            record.selection = selection;
            Ok(record.clone())
        })
        .await
    }
}

impl Message<SwitchSession> for ServerState {
    type Reply = Result<ViewingRecord>;

    /// Attach to another session, selecting the session itself.
    async fn handle(
        &mut self,
        SwitchSession {
            attachment,
            session: id,
        }: SwitchSession,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions, viewers| {
            let record = viewer(viewers, attachment)?;
            session(sessions, id)?;
            *record = ViewingRecord {
                session: id,
                selection: NodeId::Session(id),
            };
            Ok(record.clone())
        })
        .await
    }
}
