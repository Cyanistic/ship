use std::sync::Arc;

use indexmap::IndexMap;
use kameo::prelude::*;
use ship_core::{id::*, model::*, prelude::*, protocol::Create};

pub(crate) type Sessions = IndexMap<IdOf<Session>, Arc<Session>>;

/// Sole owner of the tree.
#[derive(Actor, Default)]
pub struct ServerState {
    sessions: Sessions,
}

impl ServerState {
    /// Clone `sessions`, run `edit` on the clone and swap it in. An `Err` from
    /// `edit` discards the clone, leaving state unchanged. Cloning copies only
    /// `Arc`s; `Arc::make_mut` copies the sessions an edit touches.
    fn commit<R>(&mut self, edit: impl FnOnce(&mut Sessions) -> Result<R>) -> Result<R> {
        let mut sessions = self.sessions.clone();
        let result = edit(&mut sessions)?;
        self.sessions = sessions;
        Ok(result)
    }
}

fn session(sessions: &Sessions, id: IdOf<Session>) -> Result<&Arc<Session>> {
    sessions
        .get(&id)
        .ok_or_else(|| err!(NotFound, "session {} not found", id))
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
        self.commit(|sessions| {
            ensure_unique(sessions, &create.input.name, None)?;
            let session = Session {
                id: Id::new(),
                name: create.input.name,
            };
            sessions.insert(session.id, Arc::new(session.clone()));
            Ok(session)
        })
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
        self.commit(|sessions| {
            session(sessions, rename.id)?;
            ensure_unique(sessions, &name, Some(rename.id))?;
            let session = Arc::make_mut(&mut sessions[&rename.id]);
            session.name = name;
            Ok(session.clone())
        })
    }
}

impl Message<Remove<Session>> for ServerState {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        Remove(id): Remove<Session>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|sessions| {
            sessions
                .shift_remove(&id)
                .map(drop)
                .ok_or_else(|| err!(NotFound, "session {} not found", id))
        })
    }
}
