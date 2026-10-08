//! Read-only lookups on the nested session tree, shared by the server and
//! clients. Targets are found by traversal; there is no parent index to keep
//! in sync.

use std::sync::Arc;

use indexmap::IndexMap;

use crate::{
    id::IdOf,
    model::{NodeId, Pane, Session, Tab},
    prelude::*,
};

/// Sessions keyed by ID, in creation order, as the replica carries them.
pub type Sessions = IndexMap<IdOf<Session>, Arc<Session>>;
/// A session's or tab's child tabs, in order. `Arc` so a commit copies
/// only the tabs on an edited path.
pub type Tabs = IndexMap<IdOf<Tab>, Arc<Tab>>;

/// Ancestry of `node`, session first and `node` last. `None` if absent.
pub fn path(sessions: &Sessions, node: NodeId) -> Option<Vec<NodeId>> {
    match node {
        NodeId::Session(session) => sessions.contains_key(&session).then(|| vec![node]),
        NodeId::Tab(tab) => sessions.values().find_map(|session| {
            let mut tabs = Vec::new();
            tab_path(&session.tabs, tab, &mut tabs).then(|| {
                std::iter::once(NodeId::Session(session.id))
                    .chain(tabs.into_iter().map(NodeId::Tab))
                    .collect()
            })
        }),
        NodeId::Pane(pane) => {
            let mut path = path(sessions, NodeId::Tab(pane_owner(sessions, pane).ok()?))?;
            path.push(node);
            Some(path)
        }
    }
}

/// The session `node` belongs to. `None` if absent.
pub fn session_of(sessions: &Sessions, node: NodeId) -> Option<IdOf<Session>> {
    let NodeId::Session(session) = path(sessions, node)?[0] else {
        unreachable!("a path starts at its session");
    };
    Some(session)
}

/// The first pane of `session` in tree order: each tab's own panes before
/// its child tabs'. `None` if it has none or is absent.
pub fn first_pane(sessions: &Sessions, session: IdOf<Session>) -> Option<IdOf<Pane>> {
    fn find(tabs: &Tabs) -> Option<IdOf<Pane>> {
        tabs.values()
            .find_map(|tab| tab.panes.keys().next().copied().or_else(|| find(&tab.tabs)))
    }
    find(&sessions.get(&session)?.tabs)
}

/// Pushes the IDs from `tabs` down to `id` onto `path`; false if absent.
fn tab_path(tabs: &Tabs, id: IdOf<Tab>, path: &mut Vec<IdOf<Tab>>) -> bool {
    for tab in tabs.values() {
        path.push(tab.id);
        if tab.id == id || tab_path(&tab.tabs, id, path) {
            return true;
        }
        path.pop();
    }
    false
}

pub fn tab(sessions: &Sessions, id: IdOf<Tab>) -> Result<&Tab> {
    fn find(tabs: &Tabs, id: IdOf<Tab>) -> Option<&Tab> {
        tabs.get(&id)
            .map(Arc::as_ref)
            .or_else(|| tabs.values().find_map(|tab| find(&tab.tabs, id)))
    }
    sessions
        .values()
        .find_map(|session| find(&session.tabs, id))
        .ok_or_else(|| not_found(NodeId::Tab(id)))
}

pub fn pane(sessions: &Sessions, id: IdOf<Pane>) -> Result<&Pane> {
    fn find(tabs: &Tabs, id: IdOf<Pane>) -> Option<&Pane> {
        tabs.values()
            .find_map(|tab| tab.panes.get(&id).or_else(|| find(&tab.tabs, id)))
    }
    sessions
        .values()
        .find_map(|session| find(&session.tabs, id))
        .ok_or_else(|| not_found(NodeId::Pane(id)))
}

/// Every pane ID in the tree, in no particular order.
pub fn pane_ids(sessions: &Sessions) -> impl Iterator<Item = IdOf<Pane>> + '_ {
    fn walk(tabs: &Tabs) -> Box<dyn Iterator<Item = IdOf<Pane>> + '_> {
        Box::new(
            tabs.values()
                .flat_map(|tab| tab.panes.keys().copied().chain(walk(&tab.tabs))),
        )
    }
    sessions.values().flat_map(|session| walk(&session.tabs))
}

/// The tab that owns pane `id`.
pub fn pane_owner(sessions: &Sessions, id: IdOf<Pane>) -> Result<IdOf<Tab>> {
    fn find(tabs: &Tabs, id: IdOf<Pane>) -> Option<IdOf<Tab>> {
        tabs.values().find_map(|tab| {
            tab.panes
                .contains_key(&id)
                .then_some(tab.id)
                .or_else(|| find(&tab.tabs, id))
        })
    }
    sessions
        .values()
        .find_map(|session| find(&session.tabs, id))
        .ok_or_else(|| not_found(NodeId::Pane(id)))
}

/// The tab a selection views: the tab itself, or a pane's tab. `None` for a
/// session or a node that isn't in the tree.
pub fn viewed_tab(sessions: &Sessions, selection: NodeId) -> Option<IdOf<Tab>> {
    match selection {
        NodeId::Session(_) => None,
        NodeId::Tab(tab) => Some(tab),
        NodeId::Pane(pane) => pane_owner(sessions, pane).ok(),
    }
}

pub fn not_found(node: NodeId) -> AppError {
    match node {
        NodeId::Session(id) => err!(NotFound, "session {} not found", id),
        NodeId::Tab(id) => err!(NotFound, "tab {} not found", id),
        NodeId::Pane(id) => err!(NotFound, "pane {} not found", id),
    }
}
