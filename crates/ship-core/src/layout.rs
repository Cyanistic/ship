//! Pane membership and order, stored once as leaves of a binary split tree.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{id::IdOf, model::Pane, prelude::*};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Layout {
    Pane(Pane),
    Split(Split),
}

/// First is left or above second, and receives `ratio` of the split area.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Split {
    pub axis: Axis,
    pub ratio: Ratio,
    #[schema(no_recursion)]
    pub first: Box<Layout>,
    #[schema(no_recursion)]
    pub second: Box<Layout>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// A finite first-child share of the total, strictly between zero and one.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "f32", into = "f32")]
#[schema(value_type = f32, exclusive_minimum = 0, exclusive_maximum = 1)]
pub struct Ratio(f32);

impl Ratio {
    pub const HALF: Self = Self(0.5);
}

impl TryFrom<f32> for Ratio {
    type Error = AppError;

    fn try_from(value: f32) -> Result<Self> {
        if value.is_finite() && value > 0.0 && value < 1.0 {
            Ok(Self(value))
        } else {
            Err(err!(
                Validation,
                "split ratio must be finite and between zero and one"
            ))
        }
    }
}

impl From<Ratio> for f32 {
    fn from(ratio: Ratio) -> Self {
        ratio.0
    }
}

impl Layout {
    /// Leaves in layout order: first child before second.
    pub fn panes(&self) -> Box<dyn Iterator<Item = &Pane> + '_> {
        match self {
            Self::Pane(pane) => Box::new(std::iter::once(pane)),
            Self::Split(split) => Box::new(split.first.panes().chain(split.second.panes())),
        }
    }

    pub fn pane(&self, id: IdOf<Pane>) -> Option<&Pane> {
        self.panes().find(|pane| pane.id == id)
    }

    pub fn pane_mut(&mut self, id: IdOf<Pane>) -> Option<&mut Pane> {
        match self {
            Self::Pane(pane) => (pane.id == id).then_some(pane),
            Self::Split(split) => split
                .first
                .pane_mut(id)
                .or_else(|| split.second.pane_mut(id)),
        }
    }
}
