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
    protocol::MoveTab,
    tree::{self, Tabs},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use super::draw::tab_label;
use crate::keymap::Side;

/// Drawn before the selected row and blanked to its width on the others,
/// whenever something is selected.
const HIGHLIGHT: &str = " ";
const OPEN: &str = "▾ ";
const CLOSED: &str = "▸ ";
/// tui-tree-widget's fixed indent per level.
const INDENT: u16 = 2;
const MIN_WIDTH: u16 = 10;
const MIN_PANES: u16 = 20;
const BUTTON: &str = "≡";

pub(super) struct Sidebar {
    collapsed: HashSet<IdOf<Tab>>,
    pub shown: bool,
    configured: u16,
    chosen: Option<u16>,
    area: Rect,
    /// The tree as last drawn, for hit-testing.
    rendered: TreeState<IdOf<Tab>>,
    /// Where the tree was last drawn.
    tree: Rect,
    drawn: Vec<Drawn>,
}

struct Drawn {
    tab: IdOf<Tab>,
    depth: u16,
    y: u16,
}

/// A valid destination and the start of its underlined row.
#[derive(Clone, Copy)]
pub(super) struct Gap {
    pub to: MoveTab,
    line: Position,
}

pub(super) enum Hit {
    Row(Row),
    Heading,
    Button,
    Below,
    Divider,
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
            configured: width,
            chosen: None,
            area: Rect::ZERO,
            rendered: TreeState::default(),
            tree: Rect::ZERO,
            drawn: Vec::new(),
        }
    }

    pub fn width(&self) -> u16 {
        if self.shown {
            self.chosen.unwrap_or(self.configured).max(MIN_WIDTH)
        } else {
            0
        }
    }

    pub fn configure(&mut self, width: u16) {
        if self.configured != width {
            self.configured = width;
            self.chosen = None;
        }
    }

    pub fn set_width(&mut self, width: u16, cols: u16) {
        self.chosen = Some(Self::clamp(width, cols));
    }

    pub fn nudge(&mut self, side: Side, amount: u16, cols: u16) {
        if self.shown {
            let width = Self::clamp(self.width(), cols);
            self.set_width(
                match side {
                    Side::Left => width.saturating_sub(amount),
                    Side::Right => width.saturating_add(amount),
                },
                cols,
            );
        }
    }

    /// The pane area's minimum wins when both minimums cannot fit.
    pub fn clamp(width: u16, cols: u16) -> u16 {
        let max = cols.saturating_sub(MIN_PANES);
        width.clamp(MIN_WIDTH.min(max), max)
    }

    pub fn hit(&self, at: Position) -> Option<Hit> {
        if !self.shown || !self.area.contains(at) {
            return None;
        }
        if at.x == self.area.right().saturating_sub(1) {
            return Some(Hit::Divider);
        }
        if at.y == self.area.y {
            return Some(if at.x == self.area.right().saturating_sub(2) {
                Hit::Button
            } else {
                Hit::Heading
            });
        }
        if let Some(row) = self.row_at(at) {
            return Some(Hit::Row(row));
        }
        (at.y >= self.tree.y).then_some(Hit::Below)
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
    fn row_at(&self, position: Position) -> Option<Row> {
        if !self.shown || !self.drawn.iter().any(|row| row.y == position.y) {
            return None;
        }
        let path = self.rendered.rendered_at(position)?;
        let tab = *path.last()?;
        let depth = path.len().saturating_sub(1) as u16;
        let start = self.indent().saturating_add(INDENT.saturating_mul(depth));
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

    /// Resolve the gap above the pointer's row, or after the last drawn row.
    pub fn gap(&self, tabs: &Tabs, tab: IdOf<Tab>, at: Position) -> Option<Gap> {
        if !self.shown || !self.area.contains(at) || at.y < self.tree.y {
            return None;
        }
        let index = self.drawn.partition_point(|row| row.y < at.y);
        let below = self.drawn.get(index);
        let above = index.checked_sub(1).and_then(|index| self.drawn.get(index));
        let indent = self.indent();
        let (to, depth, y) = match above {
            None => {
                let below = below?;
                (
                    MoveTab::Before(below.tab),
                    below.depth,
                    self.tree.y.saturating_sub(1),
                )
            }
            Some(above) => {
                let deepest = above.depth.saturating_add(1);
                let depth = (at.x.saturating_sub(indent) / INDENT)
                    .clamp(below.map_or(0, |row| row.depth), deepest);
                let to = if depth == deepest {
                    match below.filter(|row| row.depth == deepest) {
                        Some(below) => MoveTab::Before(below.tab),
                        None => MoveTab::Parent(Some(above.tab)),
                    }
                } else {
                    let path = tree::path(tabs, NodeId::Tab(above.tab))?;
                    let NodeId::Tab(ancestor) = *path.get(usize::from(depth))? else {
                        return None;
                    };
                    MoveTab::After(ancestor)
                };
                (to, depth, above.y)
            }
        };
        movable(tabs, tab, to).then_some(Gap {
            to,
            line: Position::new(indent.saturating_add(INDENT.saturating_mul(depth)), y),
        })
    }

    fn indent(&self) -> u16 {
        self.tree.x
            + if self.rendered.selected().is_empty() {
                0
            } else {
                Span::raw(HIGHLIGHT).width() as u16
            }
    }

    pub fn toggle(&mut self, tab: IdOf<Tab>) {
        self.fold(tab, !self.collapsed.contains(&tab));
    }

    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        tabs: &Tabs,
        selection: Option<NodeId>,
        gap: Option<&Gap>,
    ) {
        self.rendered = TreeState::default();
        self.drawn.clear();
        self.area = area;
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
        frame.render_widget(
            Paragraph::new(BUTTON).dark_gray(),
            Rect::new(content.right().saturating_sub(1), content.y, 1, 1).intersection(content),
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
        self.drawn = (self.tree.y..self.tree.bottom())
            .filter_map(|y| {
                let path = self.rendered.rendered_at(Position::new(self.tree.x, y))?;
                Some(Drawn {
                    tab: *path.last()?,
                    depth: path.len().saturating_sub(1) as u16,
                    y,
                })
            })
            .collect();
        // rendered_at returns the final identifier for blank rows below it.
        self.drawn.dedup_by_key(|row| row.tab);
        if let Some(gap) = gap {
            let line = Rect::new(
                gap.line.x,
                gap.line.y,
                content.right().saturating_sub(gap.line.x),
                1,
            )
            .intersection(content);
            for at in line.positions() {
                frame.buffer_mut()[at].set_style(Style::new().underlined());
            }
        }
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

/// Shared by drag-time feedback and the lane's turn-time re-check.
pub(super) fn movable(tabs: &Tabs, tab: IdOf<Tab>, to: MoveTab) -> bool {
    let Some(path) = tree::path(tabs, NodeId::Tab(tab)) else {
        return false;
    };
    let target = match to {
        MoveTab::Parent(None) => None,
        MoveTab::Parent(Some(id)) | MoveTab::Before(id) | MoveTab::After(id) => Some(id),
    };
    if let Some(target) = target
        && !tree::path(tabs, NodeId::Tab(target))
            .is_some_and(|path| !path.contains(&NodeId::Tab(tab)))
    {
        return false;
    }
    match to {
        MoveTab::Parent(parent) => {
            let current = path.iter().rev().nth(1).copied();
            current != parent.map(NodeId::Tab)
                || tree::siblings(tabs, tab)
                    .and_then(|tabs| tabs.last())
                    .map(|(id, _)| *id)
                    != Some(tab)
        }
        MoveTab::Before(target) | MoveTab::After(target) => {
            let Some((siblings, at)) = tree::siblings(tabs, target)
                .and_then(|siblings| Some((siblings, siblings.get_index_of(&target)?)))
            else {
                return false;
            };
            let unchanged = match to {
                MoveTab::Before(_) => at.checked_sub(1),
                MoveTab::After(_) => at.checked_add(1),
                MoveTab::Parent(_) => unreachable!(),
            };
            siblings
                .get_index_of(&tab)
                .is_none_or(|index| Some(index) != unchanged)
        }
    }
}
