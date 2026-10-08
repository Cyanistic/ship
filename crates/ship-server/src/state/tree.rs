//! Edits on the tab tree. Lookups live in `ship_core::tree` and are
//! re-exported here, so the state actor reaches both through `tree::`.

use std::sync::Arc;

pub(crate) use ship_core::tree::{
    Tabs, not_found, pane, pane_ids, pane_owner, path, tab, viewed_tab,
};
use ship_core::{
    id::IdOf,
    model::{NodeId, Tab},
    prelude::*,
    protocol::MoveTab,
};

/// The map that holds tab `id`, `Arc::make_mut` on each tab on the way down
/// and no other.
fn holder_mut(tabs: &mut Tabs, id: IdOf<Tab>) -> Option<&mut Tabs> {
    if tabs.contains_key(&id) {
        return Some(tabs);
    }
    let next = tabs
        .values()
        .position(|child| tab(&child.tabs, id).is_ok())?;
    holder_mut(&mut Arc::make_mut(&mut tabs[next]).tabs, id)
}

/// Mutable access through `Arc::make_mut` on each tab along the path, top
/// level first, copying only those.
pub(crate) fn tab_mut(tabs: &mut Tabs, id: IdOf<Tab>) -> Result<&mut Tab> {
    holder_mut(tabs, id)
        .and_then(|tabs| tabs.get_mut(&id))
        .map(Arc::make_mut)
        .ok_or_else(|| not_found(NodeId::Tab(id)))
}

/// `None` is the top level.
pub(crate) fn children_mut(tabs: &mut Tabs, parent: Option<IdOf<Tab>>) -> Result<&mut Tabs> {
    match parent {
        None => Ok(tabs),
        Some(parent) => tab_mut(tabs, parent).map(|tab| &mut tab.tabs),
    }
}

/// Detach a tab with its subtree and panes from its parent.
pub(crate) fn take_tab(tabs: &mut Tabs, id: IdOf<Tab>) -> Result<Arc<Tab>> {
    holder_mut(tabs, id)
        .and_then(|tabs| tabs.shift_remove(&id))
        .ok_or_else(|| not_found(NodeId::Tab(id)))
}

/// Insert at the destination: appended under a parent, or next to a sibling
/// under the sibling's parent. A missing parent or sibling is 404.
pub(crate) fn place(tabs: &mut Tabs, tab: Arc<Tab>, to: MoveTab) -> Result<()> {
    let (children, index) = match to {
        MoveTab::Parent(parent) => {
            let children = children_mut(tabs, parent)?;
            let end = children.len();
            (children, end)
        }
        MoveTab::Before(sibling) | MoveTab::After(sibling) => {
            let after = usize::from(matches!(to, MoveTab::After(_)));
            holder_mut(tabs, sibling)
                .and_then(|children| {
                    let index = children.get_index_of(&sibling)?;
                    Some((children, index + after))
                })
                .ok_or_else(|| not_found(NodeId::Tab(sibling)))?
        }
    };
    children.shift_insert(index, tab.id, tab);
    Ok(())
}
