//! Layout geometry has one owner: the server. Terminal shrink ignores edit minima.

use std::{collections::HashMap, sync::Arc};

use indexmap::IndexMap;
use ratatui::{
    layout::{Constraint, Direction, Layout as TuiLayout, Rect, Spacing},
    widgets::Block,
};
use ship_core::{
    geometry::{PaneGeometry, TabGeometry},
    id::IdOf,
    layout::{Axis, Layout, Ratio},
    model::{Pane, Tab},
    screen::Size,
    tree::Tabs,
};

pub(crate) const MIN_CONTENT: Size = Size { cols: 1, rows: 1 };

/// Project source tabs into published tabs. Source geometry stays absent;
/// copy only viewed tabs and their ancestors, sharing all other branches.
/// One recursive walk avoids a separate geometry map or repeated ID lookups.
pub(crate) fn publish(tabs: &Tabs, sizes: &HashMap<IdOf<Tab>, Size>) -> Tabs {
    fn decorate(tabs: &Tabs, sizes: &HashMap<IdOf<Tab>, Size>) -> Option<Tabs> {
        let mut published: Option<Tabs> = None;
        for (id, source) in tabs {
            let children = decorate(&source.tabs, sizes);
            let size = sizes.get(id);
            if children.is_none() && size.is_none() {
                continue;
            }
            let mut projected = Tab::clone(source);
            projected.geometry = size.map(|size| Box::new(tab(source, *size)));
            if let Some(children) = children {
                projected.tabs = children;
            }
            published
                .get_or_insert_with(|| tabs.clone())
                .insert(*id, Arc::new(projected));
        }
        published
    }
    decorate(tabs, sizes).unwrap_or_else(|| tabs.clone())
}

/// Fill weights cover the whole area with overlap; Ratio constraints leave
/// a trailing cell unused. Quantize only for ratatui, never the stored share.
pub(crate) fn halves(area: Rect, axis: Axis, ratio: Ratio) -> [Rect; 2] {
    const TOTAL: u16 = 1 << 15;
    let first = (f32::from(ratio) * f32::from(TOTAL)).round() as u16;
    let first = first.clamp(1, TOTAL - 1);
    TuiLayout::default()
        .direction(match axis {
            Axis::Horizontal => Direction::Horizontal,
            Axis::Vertical => Direction::Vertical,
        })
        .constraints([Constraint::Fill(first), Constraint::Fill(TOTAL - first)])
        .spacing(Spacing::Overlap(1))
        .areas(area)
}

pub(crate) fn tab(tab: &Tab, size: Size) -> TabGeometry {
    fn walk(
        layout: &Layout,
        frame: Rect,
        bordered: bool,
        panes: &mut IndexMap<IdOf<Pane>, PaneGeometry>,
    ) {
        match layout {
            Layout::Pane(pane) => {
                let content = if bordered {
                    Block::bordered().inner(frame)
                } else {
                    frame
                };
                panes.insert(pane.id, PaneGeometry { frame, content });
            }
            Layout::Split(split) => {
                let [first, second] = halves(frame, split.axis, split.ratio);
                walk(&split.first, first, bordered, panes);
                walk(&split.second, second, bordered, panes);
            }
        }
    }
    let mut panes = IndexMap::new();
    let frame = Rect::new(0, 0, size.cols, size.rows);
    if let Some(zoomed) = tab.zoomed.filter(|id| tab.pane(*id).is_some()) {
        panes.insert(
            zoomed,
            PaneGeometry {
                frame,
                content: frame,
            },
        );
    } else if let Some(layout) = &tab.layout {
        walk(
            layout,
            frame,
            matches!(layout, Layout::Split(_)),
            &mut panes,
        );
    }
    TabGeometry { size, panes }
}
