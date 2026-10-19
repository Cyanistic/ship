//! The sidebar's tabs section: client-local folds and the visible tab rows,
//! rendered by tui-tree-widget.

use std::collections::HashSet;

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
};
use ship_core::{
    id::IdOf,
    model::{NodeId, Tab},
    protocol::MoveTab,
    tree::{self, Tabs as TabTree},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use super::{Scroll, agents::icon};
use crate::ui::{agents, attention::Attention, draw::tab_label};

/// Columns left of the tree for rollup icons, drawn in the middle one so
/// they line up with the agents section's icons.
const GUTTER: u16 = 3;
const OPEN: &str = "▾ ";
const CLOSED: &str = "▸ ";
/// tui-tree-widget's fixed indent per level.
const INDENT: u16 = 2;

#[derive(Default)]
pub(in crate::ui) struct Tabs {
    collapsed: HashSet<IdOf<Tab>>,
    /// Kept across frames so the offset survives and wheel scrolling works.
    state: TreeState<IdOf<Tab>>,
    /// Where the tree was last drawn.
    area: Rect,
    /// Rows in the tree as last drawn, scrolled out of view or not.
    rows: usize,
    drawn: Vec<Drawn>,
}

struct Drawn {
    tab: IdOf<Tab>,
    depth: u16,
    y: u16,
}

/// A valid destination and the start of its underlined row.
#[derive(Clone, Copy)]
pub(in crate::ui) struct Gap {
    pub to: MoveTab,
    line: Position,
}

pub(in crate::ui) enum Hit {
    Row(Row),
    /// Below the last row.
    Below,
}

/// A tab row under the pointer.
pub(in crate::ui) struct Row {
    pub tab: IdOf<Tab>,
    /// On the fold marker, which only rows with children have.
    pub marker: bool,
}

impl Tabs {
    /// The sidebar has already checked that `at` is above the split and off
    /// the divider.
    pub fn hit(&self, at: Position) -> Option<Hit> {
        if let Some(row) = self.row_at(at) {
            return Some(Hit::Row(row));
        }
        (at.y >= self.area.y).then_some(Hit::Below)
    }

    pub fn scroll(&mut self, scroll: Scroll) {
        match scroll {
            Scroll::Up => {
                self.state.scroll_up(1);
            }
            // Stop once the last row reaches the bottom.
            Scroll::Down => {
                if self.state.get_offset() + usize::from(self.area.height) < self.rows {
                    self.state.scroll_down(1);
                }
            }
        }
    }

    /// Forget folds for removed tabs or tabs without children.
    pub fn reconcile(&mut self, tabs: &TabTree) {
        self.collapsed
            .retain(|id| tree::tab(tabs, *id).is_ok_and(|tab| !tab.tabs.is_empty()));
    }

    /// Open the selection's ancestors, leaving its own row's fold unchanged.
    pub fn reveal(&mut self, tabs: &TabTree, selection: Option<NodeId>) {
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
    pub fn visible(&self, tabs: &TabTree) -> Vec<IdOf<Tab>> {
        fn walk(section: &Tabs, tabs: &TabTree, rows: &mut Vec<IdOf<Tab>>) {
            for tab in tabs.values() {
                rows.push(tab.id);
                if !section.collapsed.contains(&tab.id) {
                    walk(section, &tab.tabs, rows);
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

    /// The row drawn at `position`, gutter included, if any. The marker's
    /// column repeats the tree widget's layout: the gutter, then the indent.
    fn row_at(&self, position: Position) -> Option<Row> {
        if !self.drawn.iter().any(|row| row.y == position.y) {
            return None;
        }
        let path = self.state.rendered_at(Position {
            x: position.x.max(self.indent()),
            ..position
        })?;
        let tab = *path.last()?;
        let depth = path.len().saturating_sub(1) as u16;
        let start = self.indent().saturating_add(INDENT.saturating_mul(depth));
        let symbol = if self.state.opened().contains(path) {
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
    /// The pointer must be on the section's rows, the divider column included.
    pub fn gap(&self, tabs: &TabTree, tab: IdOf<Tab>, at: Position) -> Option<Gap> {
        let section = Rect {
            width: self.area.width.saturating_add(1),
            ..self.area
        };
        if !section.contains(at) {
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
                    self.area.y.saturating_sub(1),
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
        self.area.x.saturating_add(GUTTER)
    }

    pub fn toggle(&mut self, tab: IdOf<Tab>) {
        self.fold(tab, !self.collapsed.contains(&tab));
    }

    /// The tree in `area`, and the drop line on its rows or the blank row
    /// above them.
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        tabs: &TabTree,
        selection: Option<NodeId>,
        gap: Option<&Gap>,
        attention: &Attention,
    ) {
        self.area = area;
        self.state.close_all();
        let items = self.items(tabs, &mut Vec::new());
        let selected: Vec<_> = selection
            .and_then(|node| tree::viewed_tab(tabs, node))
            .and_then(|id| tree::path(tabs, NodeId::Tab(id)))
            .unwrap_or_default()
            .into_iter()
            .filter_map(|node| match node {
                NodeId::Tab(id) => Some(id),
                NodeId::Pane(_) => None,
            })
            .collect();
        // Selecting asks the widget to bring the row into view, so only a new
        // selection does, and a wheel scroll holds until then.
        if self.state.selected() != selected.as_slice() {
            self.state.select(selected);
        }
        self.rows = self.state.flatten(&items).len();
        let band = Style::new().fg(Color::Reset).on_dark_gray().bold();
        let rows = Rect {
            x: self.indent(),
            width: area.width.saturating_sub(GUTTER),
            ..area
        };
        // Tabs is an ID-keyed map, so sibling identifiers are unique.
        let widget = Tree::new(&items)
            .expect("unique tab IDs")
            .node_open_symbol(OPEN)
            .node_closed_symbol(CLOSED)
            .style(Style::new().gray())
            .highlight_style(band);
        frame.render_stateful_widget(widget, rows, &mut self.state);
        self.drawn = (area.y..area.bottom())
            .filter_map(|y| {
                let path = self.state.rendered_at(Position::new(rows.x, y))?;
                Some(Drawn {
                    tab: *path.last()?,
                    depth: path.len().saturating_sub(1) as u16,
                    y,
                })
            })
            .collect();
        // rendered_at returns the final identifier for blank rows below it.
        self.drawn.dedup_by_key(|row| row.tab);
        for row in &self.drawn {
            let gutter = Rect::new(area.x, row.y, GUTTER, 1).intersection(area);
            if self.state.selected().last() == Some(&row.tab) {
                frame.buffer_mut().set_style(gutter, band);
            }
            let display = tree::tab(tabs, row.tab)
                .ok()
                .and_then(|tab| agents::rollup(tab, attention));
            if let Some(display) = display
                && gutter.width > 1
            {
                let icon = icon(display);
                frame.buffer_mut()[(gutter.x + 1, row.y)]
                    .set_symbol(&icon.content)
                    .set_style(icon.style);
            }
        }
        if let Some(gap) = gap {
            let rows = Rect {
                y: area.y.saturating_sub(1),
                height: area.height.saturating_add(1),
                ..area
            };
            let line = Rect::new(
                gap.line.x,
                gap.line.y,
                area.right().saturating_sub(gap.line.x),
                1,
            )
            .intersection(rows);
            for at in line.positions() {
                frame.buffer_mut()[at].set_style(Style::new().underlined());
            }
        }
    }

    /// Opens unfolded rows in the tree state on the way.
    fn items(
        &mut self,
        tabs: &TabTree,
        path: &mut Vec<IdOf<Tab>>,
    ) -> Vec<TreeItem<'static, IdOf<Tab>>> {
        tabs.values()
            .enumerate()
            .map(|(position, tab)| {
                path.push(tab.id);
                if !tab.tabs.is_empty() && !self.collapsed.contains(&tab.id) {
                    self.state.open(path.clone());
                }
                let children = self.items(&tab.tabs, path);
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
pub(in crate::ui) fn movable(tabs: &TabTree, tab: IdOf<Tab>, to: MoveTab) -> bool {
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
