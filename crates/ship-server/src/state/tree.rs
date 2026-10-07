//! Lookups and edits on the nested session tree. Targets are found by
//! traversal; there is no parent index to keep in sync.

use std::sync::Arc;

use indexmap::IndexMap;
use ship_core::{
    id::{IdOf, UntaggedEither},
    model::{NodeId, Pane, Tab, TabParent},
    prelude::*,
    protocol::Placement,
};

use super::Sessions;

type Tabs = IndexMap<IdOf<Tab>, Tab>;

/// Ancestry of `node`, session first and `node` last. `None` if absent.
pub(crate) fn path(sessions: &Sessions, node: NodeId) -> Option<Vec<NodeId>> {
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

pub(crate) fn tab(sessions: &Sessions, id: IdOf<Tab>) -> Result<&Tab> {
    fn find(tabs: &Tabs, id: IdOf<Tab>) -> Option<&Tab> {
        tabs.get(&id)
            .or_else(|| tabs.values().find_map(|tab| find(&tab.tabs, id)))
    }
    sessions
        .values()
        .find_map(|session| find(&session.tabs, id))
        .ok_or_else(|| not_found(NodeId::Tab(id)))
}

pub(crate) fn pane(sessions: &Sessions, id: IdOf<Pane>) -> Result<&Pane> {
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
pub(crate) fn pane_ids(sessions: &Sessions) -> impl Iterator<Item = IdOf<Pane>> + '_ {
    fn walk(tabs: &Tabs) -> Box<dyn Iterator<Item = IdOf<Pane>> + '_> {
        Box::new(
            tabs.values()
                .flat_map(|tab| tab.panes.keys().copied().chain(walk(&tab.tabs))),
        )
    }
    sessions.values().flat_map(|session| walk(&session.tabs))
}

/// The tab that owns pane `id`.
pub(crate) fn pane_owner(sessions: &Sessions, id: IdOf<Pane>) -> Result<IdOf<Tab>> {
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

/// Mutable access through `Arc::make_mut`, copying only the owning session.
pub(crate) fn tab_mut(sessions: &mut Sessions, id: IdOf<Tab>) -> Result<&mut Tab> {
    let node = NodeId::Tab(id);
    let path = path(sessions, node).ok_or_else(|| not_found(node))?;
    let [NodeId::Session(session), ancestors @ .., _] = path.as_slice() else {
        unreachable!("a tab path is its session, its ancestors and the tab");
    };
    let mut children = &mut Arc::make_mut(&mut sessions[session]).tabs;
    for ancestor in ancestors {
        let NodeId::Tab(ancestor) = ancestor else {
            unreachable!("only the first path entry is a session");
        };
        children = &mut children.get_mut(ancestor).expect("path entries exist").tabs;
    }
    Ok(children.get_mut(&id).expect("path entries exist"))
}

pub(crate) fn children_mut(sessions: &mut Sessions, parent: IdOf<TabParent>) -> Result<&mut Tabs> {
    match parent {
        UntaggedEither::Left(session) => sessions
            .get_mut(&session)
            .map(|session| &mut Arc::make_mut(session).tabs)
            .ok_or_else(|| not_found(parent.into())),
        UntaggedEither::Right(tab) => tab_mut(sessions, tab).map(|tab| &mut tab.tabs),
    }
}

/// Detach a tab with its subtree and panes from its parent.
pub(crate) fn take_tab(sessions: &mut Sessions, id: IdOf<Tab>) -> Result<Tab> {
    let node = NodeId::Tab(id);
    let path = path(sessions, node).ok_or_else(|| not_found(node))?;
    let parent = match path[path.len() - 2] {
        NodeId::Session(session) => UntaggedEither::Left(session),
        NodeId::Tab(tab) => UntaggedEither::Right(tab),
        NodeId::Pane(_) => unreachable!("a pane has no child tabs"),
    };
    Ok(children_mut(sessions, parent)?
        .shift_remove(&id)
        .expect("a tab is among its parent's children"))
}

/// Insert at the placement sibling, or append when `None`.
pub(crate) fn place(children: &mut Tabs, tab: Tab, placement: Option<Placement>) -> Result<()> {
    let index = match placement {
        None => children.len(),
        Some(Placement::Before(sibling)) => sibling_index(children, sibling)?,
        Some(Placement::After(sibling)) => sibling_index(children, sibling)? + 1,
    };
    children.shift_insert(index, tab.id, tab);
    Ok(())
}

fn sibling_index(children: &Tabs, sibling: IdOf<Tab>) -> Result<usize> {
    children.get_index_of(&sibling).ok_or_else(|| {
        err!(
            InvalidStructure,
            "tab {} is not a child of the destination",
            sibling
        )
    })
}

pub(crate) fn not_found(node: NodeId) -> AppError {
    match node {
        NodeId::Session(id) => err!(NotFound, "session {} not found", id),
        NodeId::Tab(id) => err!(NotFound, "tab {} not found", id),
        NodeId::Pane(id) => err!(NotFound, "pane {} not found", id),
    }
}
