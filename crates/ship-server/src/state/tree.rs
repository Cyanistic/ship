//! Lookups and edits on the nested session tree. Targets are found by
//! traversal; there is no parent index to keep in sync.

use std::sync::Arc;

use indexmap::IndexMap;
use ship_core::{
    id::{IdOf, UntaggedEither},
    model::{Tab, TabParent},
    prelude::*,
    protocol::Placement,
};

use super::Sessions;

type Tabs = IndexMap<IdOf<Tab>, Tab>;

/// Ancestry of `node`, session first and `node` last. `None` if absent.
pub(crate) fn path(sessions: &Sessions, node: IdOf<TabParent>) -> Option<Vec<IdOf<TabParent>>> {
    match node {
        UntaggedEither::Left(session) => sessions
            .contains_key(&session)
            .then(|| vec![UntaggedEither::Left(session)]),
        UntaggedEither::Right(tab) => sessions.values().find_map(|session| {
            let mut tabs = Vec::new();
            tab_path(&session.tabs, tab, &mut tabs).then(|| {
                std::iter::once(UntaggedEither::Left(session.id))
                    .chain(tabs.into_iter().map(UntaggedEither::Right))
                    .collect()
            })
        }),
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
        .ok_or_else(|| not_found(UntaggedEither::Right(id)))
}

/// Mutable access through `Arc::make_mut`, copying only the owning session.
pub(crate) fn tab_mut(sessions: &mut Sessions, id: IdOf<Tab>) -> Result<&mut Tab> {
    let node = UntaggedEither::Right(id);
    let path = path(sessions, node).ok_or_else(|| not_found(node))?;
    let [UntaggedEither::Left(session), ancestors @ .., _] = path.as_slice() else {
        unreachable!("a tab path is its session, its ancestors and the tab");
    };
    let mut children = &mut Arc::make_mut(&mut sessions[session]).tabs;
    for ancestor in ancestors {
        let UntaggedEither::Right(ancestor) = ancestor else {
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
            .ok_or_else(|| not_found(parent)),
        UntaggedEither::Right(tab) => tab_mut(sessions, tab).map(|tab| &mut tab.tabs),
    }
}

/// Detach a tab with its subtree from its parent.
pub(crate) fn take_tab(sessions: &mut Sessions, id: IdOf<Tab>) -> Result<Tab> {
    let node = UntaggedEither::Right(id);
    let path = path(sessions, node).ok_or_else(|| not_found(node))?;
    let parent = path[path.len() - 2];
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

pub(crate) fn not_found(node: IdOf<TabParent>) -> AppError {
    match node {
        UntaggedEither::Left(id) => err!(NotFound, "session {} not found", id),
        UntaggedEither::Right(id) => err!(NotFound, "tab {} not found", id),
    }
}
