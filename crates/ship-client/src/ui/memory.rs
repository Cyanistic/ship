//! What this client selected. Never sent anywhere.

use std::collections::HashMap;

use ship_core::{
    id::IdOf,
    model::{NodeId, Pane, Tab},
    tree::{self, Tabs},
};

#[derive(Default)]
pub(super) struct Memory {
    /// The pane this client last selected in each tab.
    last: HashMap<IdOf<Tab>, IdOf<Pane>>,
    /// Panes in the order this client last selected them, newest first.
    recent: Vec<IdOf<Pane>>,
}

impl Memory {
    /// Records the selection in this client's record. Forgets panes and tabs
    /// no longer in `tabs` first.
    pub fn observe(&mut self, tabs: &Tabs, selection: Option<NodeId>) {
        self.recent.retain(|pane| tree::pane(tabs, *pane).is_ok());
        self.last
            .retain(|tab, pane| tree::tab(tabs, *tab).is_ok_and(|tab| tab.pane(*pane).is_some()));
        if let Some(NodeId::Pane(pane)) = selection
            && let Ok(tab) = tree::pane_owner(tabs, pane)
        {
            self.recent.retain(|recent| *recent != pane);
            self.recent.insert(0, pane);
            self.last.insert(tab, pane);
        }
    }

    /// Where selecting `tab` lands: its zoomed pane, else this client's last
    /// pane there, else its first pane, else the tab.
    pub fn landing(&self, tab: &Tab) -> NodeId {
        tab.zoomed
            .or_else(|| self.last.get(&tab.id).copied())
            .or_else(|| tab.panes().next().map(|pane| pane.id))
            .map_or(NodeId::Tab(tab.id), NodeId::Pane)
    }

    pub fn recent(&self) -> &[IdOf<Pane>] {
        &self.recent
    }
}
