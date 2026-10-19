//! Display states, the tab rollup, the agent list and the jump target.
//! No rendering; `sidebar::agents` draws them.

use ship_core::{
    id::IdOf,
    model::{NodeId, Pane, ReportedState, StatusRecord, Tab},
    tree::Tabs,
};

use super::{
    attention::Attention,
    draw::{pane_label, tab_label},
};

/// Between the tab labels of an agent's path.
const SEPARATOR: &str = " › ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Display {
    Blocked,
    Error,
    Unseen,
    Working,
    Idle,
}

impl Display {
    pub fn label(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Error => "error",
            Self::Unseen => "unseen",
            Self::Working => "working",
            Self::Idle => "idle",
        }
    }

    /// Lower ranks first; blocked and error share the top rank.
    fn rank(self) -> u8 {
        match self {
            Self::Blocked | Self::Error => 0,
            Self::Unseen => 1,
            Self::Working => 2,
            Self::Idle => 3,
        }
    }
}

/// One listed pane, in agent-list order.
pub(super) struct Agent<'a> {
    pub pane: &'a Pane,
    pub root: &'a StatusRecord,
    pub display: Display,
    /// Tab labels from the top level.
    pub path: Vec<String>,
}

impl Agent<'_> {
    /// The record's `app`, else the pane's label.
    pub fn name(&self) -> &str {
        self.root.app.as_deref().unwrap_or(pane_label(self.pane))
    }

    /// The tab labels joined from the top level.
    pub fn tab(&self) -> String {
        self.path.join(SEPARATOR)
    }
}

/// The display state of the pane's root record, if it has one. A finished
/// pane shows as unseen while `attention` says so, else as idle.
pub(super) fn display(pane: &Pane, attention: &Attention) -> Option<Display> {
    Some(match pane.root_status()?.state {
        ReportedState::Blocked => Display::Blocked,
        ReportedState::Error => Display::Error,
        ReportedState::Working => Display::Working,
        ReportedState::Idle | ReportedState::Done if attention.unseen(pane.id) => Display::Unseen,
        ReportedState::Idle | ReportedState::Done => Display::Idle,
    })
}

/// The highest-ranked display among the tab's panes and its descendants'.
pub(super) fn rollup(tab: &Tab, attention: &Attention) -> Option<Display> {
    tab.panes()
        .filter_map(|pane| display(pane, attention))
        .chain(
            tab.tabs
                .values()
                .filter_map(|child| rollup(child, attention)),
        )
        .min_by_key(|display| display.rank())
}

/// By rank, then depth-first tree order.
pub(super) fn list<'a>(tabs: &'a Tabs, attention: &Attention) -> Vec<Agent<'a>> {
    fn walk<'a>(
        tabs: &'a Tabs,
        attention: &Attention,
        path: &mut Vec<String>,
        agents: &mut Vec<Agent<'a>>,
    ) {
        for (position, tab) in tabs.values().enumerate() {
            path.push(tab_label(tab, position));
            agents.extend(tab.panes().filter_map(|pane| {
                Some(Agent {
                    pane,
                    root: pane.root_status()?,
                    display: display(pane, attention)?,
                    path: path.clone(),
                })
            }));
            walk(&tab.tabs, attention, path, agents);
            path.pop();
        }
    }
    let mut agents = Vec::new();
    walk(tabs, attention, &mut Vec::new(), &mut agents);
    agents.sort_by_key(|agent| agent.display.rank());
    agents
}

/// The first pane needing the user after `selection` in `agents`, the
/// list's order: blocked and error, then unseen. With none of those, every
/// agent takes their place. Wraps to the first when the selection is the
/// last of them or isn't one.
pub(super) fn next(agents: &[Agent], selection: Option<NodeId>) -> Option<IdOf<Pane>> {
    let mut needing: Vec<_> = agents
        .iter()
        .filter(|agent| agent.display.rank() <= Display::Unseen.rank())
        .map(|agent| agent.pane.id)
        .collect();
    if needing.is_empty() {
        needing = agents.iter().map(|agent| agent.pane.id).collect();
    }
    let after = needing
        .iter()
        .position(|pane| selection == Some(NodeId::Pane(*pane)))
        .map_or(0, |index| index + 1);
    needing.get(after).or(needing.first()).copied()
}
