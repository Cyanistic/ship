//! Server-computed rectangles, relative to the viewed tab's top-left corner.

use indexmap::IndexMap;
use ratatui_core::layout::Rect;
use serde::{Deserialize, Serialize};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{Object, ObjectBuilder, Ref},
};

use crate::{id::IdOf, model::Pane, screen::Size};

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
}

fn rect_schema() -> Object {
    let integer = || {
        ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::Integer)
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
