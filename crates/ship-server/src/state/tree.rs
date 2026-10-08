//! Edits on the nested session tree. Lookups live in `ship_core::tree` and
//! are re-exported here, so the state actor reaches both through `tree::`.

use std::sync::Arc;

pub(crate) use ship_core::tree::{
    Sessions, Tabs, first_pane, not_found, pane, pane_ids, pane_owner, path, session_of, tab,
    viewed_tab,
};
use ship_core::{
    id::{IdOf, UntaggedEither},
    model::{NodeId, Tab, TabParent},
    prelude::*,
    protocol::Placement,
};

/// Mutable access through `Arc::make_mut` on the owning session and each tab
/// along the path, copying only those.
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
        children = &mut Arc::make_mut(children.get_mut(ancestor).expect("path entries exist")).tabs;
    }
    Ok(Arc::make_mut(
        children.get_mut(&id).expect("path entries exist"),
    ))
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
pub(crate) fn take_tab(sessions: &mut Sessions, id: IdOf<Tab>) -> Result<Arc<Tab>> {
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
pub(crate) fn place(
    children: &mut Tabs,
    tab: Arc<Tab>,
    placement: Option<Placement>,
) -> Result<()> {
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
