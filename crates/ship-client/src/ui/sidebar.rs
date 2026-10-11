//! Client-local folds and the visible tab rows, rendered by tui-tree-widget.

use std::collections::HashSet;

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use ship_core::{
    id::IdOf,
    model::{NodeId, Tab},
    tree::{self, Tabs},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use super::draw::tab_label;

/// Drawn before the selected row and blanked to its width on the others,
/// whenever something is selected.
const HIGHLIGHT: &str = " ";
const OPEN: &str = "▾ ";
const CLOSED: &str = "▸ ";
/// tui-tree-widget's fixed indent per level.
const INDENT: u16 = 2;

pub(super) struct Sidebar {
    collapsed: HashSet<IdOf<Tab>>,
    pub shown: bool,
    pub width: u16,
    /// The tree as last drawn, for hit-testing.
    rendered: TreeState<IdOf<Tab>>,
    /// Where the tree was last drawn.
    tree: Rect,
}

/// A sidebar row under the pointer.
pub(super) struct Row {
    pub tab: IdOf<Tab>,
    /// On the fold marker, which only rows with children have.
    pub marker: bool,
}

impl Sidebar {
    pub fn new(width: u16) -> Self {
        Self {
            collapsed: HashSet::new(),
            shown: true,
            width,
            rendered: TreeState::default(),
            tree: Rect::ZERO,
        }
    }

    pub fn width(&self) -> u16 {
        if self.shown { self.width } else { 0 }
    }

    /// Forget folds for removed tabs or tabs without children.
    pub fn reconcile(&mut self, tabs: &Tabs) {
        self.collapsed
            .retain(|id| tree::tab(tabs, *id).is_ok_and(|tab| !tab.tabs.is_empty()));
    }

    /// Open the selection's ancestors, leaving its own row's fold unchanged.
    pub fn reveal(&mut self, tabs: &Tabs, selection: Option<NodeId>) {
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

    /// The row drawn at `position`, if any. The marker's column repeats the
    /// tree widget's layout: the highlight symbol, then the indent.
    pub fn row_at(&self, position: Position) -> Option<Row> {
        if !self.shown {
            return None;
        }
        let path = self.rendered.rendered_at(position)?;
        let tab = *path.last()?;
        let highlight = if self.rendered.selected().is_empty() {
            0
        } else {
            Span::raw(HIGHLIGHT).width() as u16
        };
        let depth = path.len().saturating_sub(1) as u16;
        let start = self.tree.x + highlight + INDENT * depth;
        let symbol = if self.rendered.opened().contains(path) {
            Some(OPEN)
        } else {
            self.collapsed.contains(&tab).then_some(CLOSED)
        };
        Some(Row {
            tab,
            marker: symbol.is_some_and(|symbol| {
                (start..start.saturating_add(Span::raw(symbol).width() as u16))
                    .contains(&position.x)
            }),
        })
    }

    pub fn toggle(&mut self, tab: IdOf<Tab>) {
        self.fold(tab, !self.collapsed.contains(&tab));
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, tabs: &Tabs, selection: Option<NodeId>) {
        self.rendered = TreeState::default();
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
            .node_open_symbol(OPEN)
            .node_closed_symbol(CLOSED)
            .highlight_symbol(HIGHLIGHT)
            .style(Style::new().gray())
            .highlight_style(Style::new().fg(Color::Reset).on_dark_gray().bold());
        self.tree = Rect {
            y: content.y.saturating_add(content.height.min(2)),
            height: content.height.saturating_sub(2),
            ..content
        };
        frame.render_stateful_widget(widget, self.tree, &mut state);
        self.rendered = state;
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
                if !tab.tabs.is_empty() && !self.collapsed.contains(&tab.id) {
                    state.open(path.clone());
                }
                let children = self.items(&tab.tabs, path, state);
                path.pop();
                // tmux's zoom flag, since a zoomed pane has no border to carry one.
                let mut row = Line::from(tab_label(tab, position));
                if tab.zoomed.is_some() {
                    row.push_span(Span::from(" Z").cyan().bold());
                }
                TreeItem::new(tab.id, row, children).expect("unique tab IDs")
            })
            .collect()
    }
}
