//! Server-computed rectangles, relative to the viewed tab's top-left corner.

use indexmap::IndexMap;
use ratatui_core::layout::Rect;
use serde::{Deserialize, Serialize};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{Object, ObjectBuilder, Ref, Type},
};

use crate::{command::Direction, id::IdOf, model::Pane, screen::Size};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TabGeometry {
    pub size: Size,
    #[schema(schema_with = panes_schema)]
    pub panes: IndexMap<IdOf<Pane>, PaneGeometry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaneGeometry {
    #[schema(schema_with = rect_schema)]
    pub frame: Rect,
    #[schema(schema_with = rect_schema)]
    pub content: Rect,
}

impl TabGeometry {
    /// Largest actual content area; equal areas keep layout order.
    pub fn largest(&self) -> Option<IdOf<Pane>> {
        self.panes
            .iter()
            .fold(None, |best, (id, pane)| {
                let area = pane.content.area();
                match best {
                    Some((_, largest)) if largest >= area => best,
                    _ => Some((*id, area)),
                }
            })
            .map(|(id, _)| id)
    }

    /// The pane whose frame shares part of `from`'s edge on the `direction`
    /// side. Several go to the first of them in `recent`, else the nearest
    /// center, else layout order.
    pub fn neighbor(
        &self,
        from: IdOf<Pane>,
        direction: Direction,
        recent: &[IdOf<Pane>],
    ) -> Option<IdOf<Pane>> {
        let from_frame = self.panes.get(&from)?.frame;
        let candidates: Vec<_> = self
            .panes
            .iter()
            .filter(|(id, pane)| **id != from && adjacent(from_frame, pane.frame, direction))
            .map(|(id, pane)| (*id, pane.frame))
            .collect();
        recent
            .iter()
            .find(|id| candidates.iter().any(|(candidate, _)| candidate == *id))
            .copied()
            .or_else(|| {
                // `min_by_key` keeps the first of equal distances.
                candidates
                    .iter()
                    .min_by_key(|(_, frame)| distance(from_frame, *frame))
                    .map(|(id, _)| *id)
            })
    }
}

/// Whether `to` lies beyond `from`'s `direction` edge, touching it, with more
/// than a shared corner in common. Bordered neighbors overlap by one cell.
fn adjacent(from: Rect, to: Rect, direction: Direction) -> bool {
    let touches = |far: u16, near: u16| far.saturating_sub(1) <= near && near <= far;
    let shares = |start: u16, end: u16, other_start: u16, other_end: u16| {
        end.min(other_end).saturating_sub(start.max(other_start)) > 1
    };
    let touching = match direction {
        Direction::Left => touches(to.right(), from.left()),
        Direction::Right => touches(from.right(), to.left()),
        Direction::Up => touches(to.bottom(), from.top()),
        Direction::Down => touches(from.bottom(), to.top()),
    };
    touching
        && match direction {
            Direction::Left | Direction::Right => {
                shares(from.top(), from.bottom(), to.top(), to.bottom())
            }
            Direction::Up | Direction::Down => {
                shares(from.left(), from.right(), to.left(), to.right())
            }
        }
}

/// Squared distance between the centers, in half cells to stay integral.
fn distance(a: Rect, b: Rect) -> u32 {
    let center = |start: u16, length: u16| 2 * i32::from(start) + i32::from(length);
    let dx = center(a.x, a.width).abs_diff(center(b.x, b.width));
    let dy = center(a.y, a.height).abs_diff(center(b.y, b.height));
    dx * dx + dy * dy
}

fn rect_schema() -> Object {
    let integer = || {
        ObjectBuilder::new()
            .schema_type(Type::Integer)
            .minimum(Some(0.0))
            .maximum(Some(f64::from(u16::MAX)))
            .build()
    };
    ObjectBuilder::new()
        .property("x", integer())
        .required("x")
        .property("y", integer())
        .required("y")
        .property("width", integer())
        .required("width")
        .property("height", integer())
        .required("height")
        .build()
}

fn panes_schema() -> Object {
    ObjectBuilder::new()
        .property_names(Some(IdOf::<Pane>::schema()))
        .additional_properties(Some(Ref::from_schema_name("PaneGeometry")))
        .build()
}
