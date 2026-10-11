//! Local tree rewrites and old-tree close selection repair. Structural edits
//! clear the tab's zoom.

use ratatui::layout::Rect;
use ship_core::{
    command::Direction,
    id::IdOf,
    layout::{Axis, Layout, Ratio, Split},
    model::{Pane, Tab},
    prelude::*,
    protocol::SplitDirection,
    screen::Size,
};

use super::geometry;

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

/// Exchanges the leaves of `a` and `b`, which must both be in `tab`.
pub(crate) fn swap(tab: &mut Tab, a: IdOf<Pane>, b: IdOf<Pane>) -> Result<()> {
    fn exchange(layout: &mut Layout, a: &Pane, b: &Pane) {
        match layout {
            Layout::Pane(pane) if pane.id == a.id => *pane = b.clone(),
            Layout::Pane(pane) if pane.id == b.id => *pane = a.clone(),
            Layout::Pane(_) => {}
            Layout::Split(split) => {
                exchange(&mut split.first, a, b);
                exchange(&mut split.second, a, b);
            }
        }
    }
    let pane = |id| {
        tab.pane(id)
            .cloned()
            .ok_or_else(|| err!(InvalidStructure, "pane {} is not in tab {}", id, tab.id))
    };
    let (a, b) = (pane(a)?, pane(b)?);
    if a.id == b.id {
        return Err(err!(
            InvalidStructure,
            "cannot swap pane {} with itself",
            a.id
        ));
    }
    if let Some(layout) = &mut tab.layout {
        exchange(layout, &a, &b);
    }
    tab.zoomed = None;
    Ok(())
}

/// Moves `pane`'s edge on the `direction` side by up to `cells` that way,
/// or, when that side is the tab's edge, its opposite edge that way. The edge
/// is the nearest ancestor split on that axis with `pane` on the right side
/// of it; its ratio becomes the one that puts the edge there at `size`. The
/// move stops before any pane would be pushed below `MIN_CONTENT`. Returns
/// whether the edge moved; when it didn't, `tab` is unchanged.
pub(crate) fn resize(
    tab: &mut Tab,
    size: Size,
    pane: IdOf<Pane>,
    direction: Direction,
    cells: u16,
) -> Result<bool> {
    let (axis, forward) = match direction {
        Direction::Left => (Axis::Horizontal, false),
        Direction::Right => (Axis::Horizontal, true),
        Direction::Up => (Axis::Vertical, false),
        Direction::Down => (Axis::Vertical, true),
    };
    let layout = tab
        .layout
        .as_ref()
        .filter(|layout| layout.pane(pane).is_some())
        .ok_or_else(|| err!(InvalidStructure, "pane {} is not in tab {}", pane, tab.id))?;
    let Some(path) =
        edge(layout, pane, axis, forward).or_else(|| edge(layout, pane, axis, !forward))
    else {
        return Ok(false);
    };
    let mut full = tab.clone();
    full.zoomed = None;
    let before = geometry::tab(&full, size);
    let split = split_at(full.layout.as_mut().expect("checked above"), &path);
    let frame = |layout: &Layout| {
        layout
            .panes()
            .filter_map(|pane| before.panes.get(&pane.id))
            .map(|pane| pane.frame)
            .reduce(|union, frame| union.union(frame))
            .unwrap_or_default()
    };
    let area = frame(&split.first).union(frame(&split.second));
    let length = |rect: Rect| match axis {
        Axis::Horizontal => rect.width,
        Axis::Vertical => rect.height,
    };
    let first = length(frame(&split.first));
    let total = length(area);
    // The ratio putting the edge `moved` cells along, if no pane goes below
    // the minimum it wasn't already below.
    let attempt = |moved: u16| {
        let target = if forward {
            first.checked_add(moved)?
        } else {
            first.checked_sub(moved)?
        };
        let ratio = geometry::ratio_for(area, axis, target)?;
        let mut trial = full.clone();
        split_at(trial.layout.as_mut()?, &path).ratio = ratio;
        let after = geometry::tab(&trial, size);
        let fits = after.panes.iter().all(|(id, pane)| {
            let old = before.panes[id].content;
            let new = pane.content;
            (new.width >= geometry::MIN_CONTENT.cols || new.width >= old.width)
                && (new.height >= geometry::MIN_CONTENT.rows || new.height >= old.height)
        });
        fits.then_some(ratio)
    };
    // The furthest fitting move: fitting shrinks as the edge goes further.
    let (mut low, mut high) = (0, cells.min(total));
    let mut found = None;
    while low < high {
        let middle = high - (high - low) / 2;
        match attempt(middle) {
            Some(ratio) => {
                found = Some(ratio);
                low = middle;
            }
            None => high = middle - 1,
        }
    }
    let Some(ratio) = found else {
        return Ok(false);
    };
    let layout = tab.layout.as_mut().expect("checked above");
    split_at(layout, &path).ratio = ratio;
    tab.zoomed = None;
    Ok(true)
}

/// The path, as "into second" flags, to the deepest split on `axis` holding
/// `pane` in its first child when `first`, else in its second.
fn edge(layout: &Layout, pane: IdOf<Pane>, axis: Axis, first: bool) -> Option<Vec<bool>> {
    let Layout::Split(split) = layout else {
        return None;
    };
    let in_second = split.first.pane(pane).is_none();
    let child = if in_second {
        &split.second
    } else {
        &split.first
    };
    if let Some(mut path) = edge(child, pane, axis, first) {
        path.insert(0, in_second);
        return Some(path);
    }
    (split.axis == axis && in_second != first).then(Vec::new)
}

fn split_at<'a>(layout: &'a mut Layout, path: &[bool]) -> &'a mut Split {
    let Layout::Split(split) = layout else {
        unreachable!("edge paths lead to splits");
    };
    match path.split_first() {
        None => split,
        Some((true, rest)) => split_at(&mut split.second, rest),
        Some((false, rest)) => split_at(&mut split.first, rest),
    }
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
