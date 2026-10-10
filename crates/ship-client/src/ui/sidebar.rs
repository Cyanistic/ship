//! Client-local folds and the visible tab rows, rendered by tui-tree-widget.

use std::collections::HashSet;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Paragraph},
};
use ship_core::{
    id::IdOf,
    model::{NodeId, Tab},
    tree::{self, Tabs},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use super::draw::tab_label;

pub(super) struct Sidebar {
    collapsed: HashSet<IdOf<Tab>>,
    pub shown: bool,
    pub width: u16,
}

impl Sidebar {
    pub fn new(width: u16) -> Self {
        Self {
            collapsed: HashSet::new(),
            shown: true,
            width,
        }
    }

    pub fn width(&self) -> u16 {
        if self.shown { self.width } else { 0 }
    }

    /// Forget removed tabs and open ancestors of the authoritative selection
    /// (or a pending local navigation). The selected row itself may be folded.
    pub fn reconcile(&mut self, tabs: &Tabs, selection: Option<NodeId>) {
        self.collapsed.retain(|id| tree::tab(tabs, *id).is_ok());
        let path = selection
            .and_then(|node| tree::path(tabs, node))
            .unwrap_or_default();
        let selected = selection.and_then(|node| tree::viewed_tab(tabs, node));
        for node in path {
            if let NodeId::Tab(id) = node
                && Some(id) != selected
            {
                self.collapsed.remove(&id);
            }
        }
    }

    /// Tree order under unfolded ancestors, even while the sidebar is hidden.
    pub fn visible(&self, tabs: &Tabs) -> Vec<IdOf<Tab>> {
        fn walk(sidebar: &Sidebar, tabs: &Tabs, rows: &mut Vec<IdOf<Tab>>) {
            for tab in tabs.values() {
                rows.push(tab.id);
                if !sidebar.collapsed.contains(&tab.id) {
                    walk(sidebar, &tab.tabs, rows);
                }
            }
        }
        let mut rows = Vec::new();
        walk(self, tabs, &mut rows);
        rows
    }

    pub fn fold(&mut self, tab: IdOf<Tab>, collapsed: bool) {
        if collapsed {
            self.collapsed.insert(tab);
        } else {
            self.collapsed.remove(&tab);
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, tabs: &Tabs, selection: Option<NodeId>) {
        if area.is_empty() {
            return;
        }
        // Herdr's ANSI `terminal` palette: a dark-gray divider and bold header,
        // gray rows and a dark-gray band behind the selected row.
        let divider = Block::new()
            .borders(Borders::RIGHT)
            .border_style(Style::new().dark_gray());
        let content = divider.inner(area);
        frame.render_widget(divider, area);
        frame.render_widget(
            Paragraph::new(" tabs").dark_gray().bold(),
            Rect {
                height: 1,
                ..content
            },
        );
        let mut state = TreeState::default();
        let items = self.items(tabs, &mut Vec::new(), &mut state);
        let selected = selection.and_then(|node| tree::viewed_tab(tabs, node));
        if let Some(path) = selected.and_then(|id| tree::path(tabs, NodeId::Tab(id))) {
            state.select(
                path.into_iter()
                    .filter_map(|node| match node {
                        NodeId::Tab(id) => Some(id),
                        NodeId::Pane(_) => None,
                    })
                    .collect(),
            );
        }
        // Tabs is an ID-keyed map, so sibling identifiers are unique.
        let widget = Tree::new(&items)
            .expect("unique tab IDs")
            .node_open_symbol("▾ ")
            .node_closed_symbol("▸ ")
            .highlight_symbol(" ")
            .style(Style::new().gray())
            .highlight_style(Style::new().fg(Color::Reset).on_dark_gray().bold());
        frame.render_stateful_widget(
            widget,
            Rect {
                y: content.y.saturating_add(content.height.min(2)),
                height: content.height.saturating_sub(2),
                ..content
            },
            &mut state,
        );
    }

    fn items(
        &self,
        tabs: &Tabs,
        path: &mut Vec<IdOf<Tab>>,
        state: &mut TreeState<IdOf<Tab>>,
    ) -> Vec<TreeItem<'static, IdOf<Tab>>> {
        tabs.values()
            .enumerate()
            .map(|(position, tab)| {
                path.push(tab.id);
                if !self.collapsed.contains(&tab.id) {
                    state.open(path.clone());
                }
                let children = self.items(&tab.tabs, path, state);
                path.pop();
                TreeItem::new(tab.id, tab_label(tab, position), children).expect("unique tab IDs")
            })
            .collect()
    }
}
