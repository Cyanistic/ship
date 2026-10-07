use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Terminal size in cells. Also the viewer size and tab size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

impl Size {
    pub const FALLBACK: Size = Size { cols: 80, rows: 24 };
}
