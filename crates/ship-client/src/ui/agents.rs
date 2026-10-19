//! Display states, the tab rollup and the agent list. No rendering;
//! `sidebar::agents` draws them.

use ship_core::{
    model::{Pane, ReportedState, StatusRecord, Tab},
    tree::Tabs,
};

use super::draw::tab_label;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Display {
    Blocked,
    Error,
    Working,
    Idle,
}

impl Display {
    pub fn label(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Error => "error",
            Self::Working => "working",
            Self::Idle => "idle",
        }
    }

    /// Lower ranks first; blocked and error share the top rank.
    fn rank(self) -> u8 {
        match self {
            Self::Blocked | Self::Error => 0,
            Self::Working => 1,
            Self::Idle => 2,
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

/// The display state of the pane's root record, if it has one. A reported
/// `done` shows as idle.
pub(super) fn display(pane: &Pane) -> Option<Display> {
    Some(match pane.root_status()?.state {
        ReportedState::Blocked => Display::Blocked,
        ReportedState::Error => Display::Error,
        ReportedState::Working => Display::Working,
        ReportedState::Idle | ReportedState::Done => Display::Idle,
    })
}

/// The highest-ranked display among the tab's panes and its descendants'.
pub(super) fn rollup(tab: &Tab) -> Option<Display> {
    tab.panes()
        .filter_map(display)
        .chain(tab.tabs.values().filter_map(|child| rollup(child)))
        .min_by_key(|display| display.rank())
}

/// By rank, then depth-first tree order.
pub(super) fn list(tabs: &Tabs) -> Vec<Agent<'_>> {
    fn walk<'a>(tabs: &'a Tabs, path: &mut Vec<String>, agents: &mut Vec<Agent<'a>>) {
        for (position, tab) in tabs.values().enumerate() {
            path.push(tab_label(tab, position));
            agents.extend(tab.panes().filter_map(|pane| {
                Some(Agent {
                    pane,
                    root: pane.root_status()?,
                    display: display(pane)?,
                    path: path.clone(),
                })
            }));
            walk(&tab.tabs, path, agents);
            path.pop();
        }
    }
    let mut agents = Vec::new();
    walk(tabs, &mut Vec::new(), &mut agents);
    agents.sort_by_key(|agent| agent.display.rank());
    agents
}
