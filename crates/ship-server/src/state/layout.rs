//! Local tree rewrites and old-tree close selection repair.

use ship_core::{
    id::IdOf,
    layout::{Axis, Layout, Ratio, Split},
    model::{Pane, Tab},
    prelude::*,
    protocol::SplitDirection,
};

pub(crate) fn axis(direction: SplitDirection) -> Axis {
    match direction {
        SplitDirection::Right => Axis::Horizontal,
        SplitDirection::Down => Axis::Vertical,
    }
}

pub(crate) fn split(
    tab: &mut Tab,
    anchor: Option<IdOf<Pane>>,
    direction: SplitDirection,
    pane: Pane,
) -> Result<()> {
    fn insert(layout: &mut Layout, anchor: IdOf<Pane>, axis: Axis, pane: &Pane) -> bool {
        match layout {
            Layout::Pane(before) if before.id == anchor => {
                *layout = Layout::Split(Split {
                    axis,
                    ratio: Ratio::HALF,
                    first: Box::new(Layout::Pane(before.clone())),
                    second: Box::new(Layout::Pane(pane.clone())),
                });
                true
            }
            Layout::Pane(_) => false,
            Layout::Split(split) => {
                insert(&mut split.first, anchor, axis, pane)
                    || insert(&mut split.second, anchor, axis, pane)
            }
        }
    }
    match (&mut tab.layout, anchor) {
        (None, None) => tab.layout = Some(Layout::Pane(pane)),
        (Some(layout), Some(anchor)) => {
            if !insert(layout, anchor, axis(direction), &pane) {
                return Err(err!(InvalidStructure, "split anchor is not in the tab"));
            }
        }
        _ => return Err(err!(InvalidStructure, "split anchor is not in the tab")),
    }
    tab.zoomed = None;
    Ok(())
}

pub(crate) fn close(tab: &mut Tab, id: IdOf<Pane>) -> Result<()> {
    fn remove(layout: Layout, id: IdOf<Pane>) -> Option<Layout> {
        match layout {
            Layout::Pane(pane) => (pane.id != id).then_some(Layout::Pane(pane)),
            Layout::Split(split) => {
                let first = remove(*split.first, id);
                let second = remove(*split.second, id);
                match (first, second) {
                    (Some(first), Some(second)) => Some(Layout::Split(Split {
                        first: Box::new(first),
                        second: Box::new(second),
                        ..split
                    })),
                    (remaining, None) | (None, remaining) => remaining,
                }
            }
        }
    }
    if tab.pane(id).is_none() {
        return Err(err!(NotFound, "pane {} not found", id));
    }
    tab.layout = tab.layout.take().and_then(|layout| remove(layout, id));
    tab.zoomed = None;
    Ok(())
}

/// At same-axis splits descend toward the closed side; otherwise take first.
pub(crate) fn successor(layout: &Layout, closed: IdOf<Pane>) -> Option<IdOf<Pane>> {
    fn edge(layout: &Layout, axis: Axis, closed_first: bool) -> IdOf<Pane> {
        match layout {
            Layout::Pane(pane) => pane.id,
            Layout::Split(split) => {
                let child = if split.axis == axis && !closed_first {
                    &split.second
                } else {
                    &split.first
                };
                edge(child, axis, closed_first)
            }
        }
    }
    let Layout::Split(split) = layout else {
        return None;
    };
    match (&*split.first, &*split.second) {
        (Layout::Pane(pane), sibling) if pane.id == closed => Some(edge(sibling, split.axis, true)),
        (sibling, Layout::Pane(pane)) if pane.id == closed => {
            Some(edge(sibling, split.axis, false))
        }
        _ => successor(&split.first, closed).or_else(|| successor(&split.second, closed)),
    }
}
