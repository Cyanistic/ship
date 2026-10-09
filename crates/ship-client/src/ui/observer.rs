use std::{collections::HashMap, sync::Arc};

use ship_core::{
    id::{Attachment, IdOf},
    model::{NodeId, Pane, Tab},
    protocol::{AttachRequest, PaneScreen, Replica, SseEvent, ViewingRecord},
    screen::{Screen, Size},
    tree,
};

/// The client's copy of server-owned state: the replica and the latest screen
/// of each pane it has been sent.
#[derive(Default)]
pub(super) struct Observer {
    pub attachment: Option<IdOf<Attachment>>,
    pub replica: Option<Arc<Replica>>,
    pub screens: HashMap<IdOf<Pane>, Arc<Screen>>,
    /// Whether an attach stream is open. Screens stay while it isn't.
    pub connected: bool,
}

/// The record's selection resolved against the replica.
pub(super) struct Selected<'a> {
    /// The selected tab, or the selected pane's tab.
    pub tab: &'a Tab,
    pub pane: Option<&'a Pane>,
}

impl Observer {
    /// The only writer of server-owned state. `Attached` replaces the
    /// replica, `State` applies only for the same incarnation and a higher
    /// revision, and `Screen` replaces that pane's screen. Returns whether
    /// anything visible changed.
    pub fn apply(&mut self, event: SseEvent) -> bool {
        match event {
            SseEvent::Attached(attached) => {
                self.attachment = Some(attached.attachment);
                self.replica = Some(attached.replica);
                self.connected = true;
            }
            SseEvent::State(replica) => match &self.replica {
                Some(current)
                    if current.incarnation == replica.incarnation
                        && current.revision < replica.revision =>
                {
                    self.replica = Some(replica);
                }
                _ => return false,
            },
            SseEvent::Screen(PaneScreen { pane, screen }) => {
                self.screens.insert(pane, screen);
                return self
                    .selected()
                    .and_then(|selected| selected.pane)
                    .is_some_and(|selected| selected.id == pane);
            }
            SseEvent::Ended(_) => return false,
        }
        self.reconcile();
        true
    }

    /// Drop screens for panes no longer in the replica.
    fn reconcile(&mut self) {
        let Some(replica) = &self.replica else {
            return;
        };
        self.screens
            .retain(|pane, _| tree::pane(&replica.tabs, *pane).is_ok());
    }

    /// Whether the replica has reached `revision`. Nothing to wait for at 0.
    pub fn caught_up(&self, revision: u64) -> bool {
        self.replica
            .as_ref()
            .map_or(revision == 0, |replica| replica.revision >= revision)
    }

    /// This client's record, matched by attachment ID.
    pub fn record(&self) -> Option<&ViewingRecord> {
        self.replica.as_ref()?.viewers.get(&self.attachment?)
    }

    /// What to send on reattach: the last record's selection at the
    /// terminal's current tab area. `None` before the first `Attached` event.
    pub fn remembered(&self, area: Size) -> Option<AttachRequest> {
        let record = self.record()?;
        Some(AttachRequest {
            selection: record.selection,
            area,
        })
    }

    /// `None` when nothing is selected or before the first `Attached`.
    pub fn selected(&self) -> Option<Selected<'_>> {
        let selection = self.record()?.selection?;
        let tabs = &self.replica.as_ref()?.tabs;
        let tab = tree::tab(tabs, tree::viewed_tab(tabs, selection)?).ok()?;
        let pane = match selection {
            NodeId::Pane(id) => tab.panes.get(&id),
            NodeId::Tab(_) => None,
        };
        Some(Selected { tab, pane })
    }
}
