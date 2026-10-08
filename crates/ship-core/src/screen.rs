use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A pane's visible grid at one moment, row-major. Full snapshot; zstd on the
/// stream supplies the savings between frames.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    pub size: Size,
    /// `size.cols * size.rows` cells.
    pub cells: Vec<Cell>,
    /// Absent when the cursor is hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

/// Default-valued fields are omitted on the wire, so a blank cell is `{}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    /// Grapheme; " " when omitted. A wide character's trailing cell is blank.
    #[serde(default = "space", skip_serializing_if = "is_space")]
    pub symbol: String,
    #[serde(default, skip_serializing_if = "Color::is_default")]
    pub fg: Color,
    #[serde(default, skip_serializing_if = "Color::is_default")]
    pub bg: Color,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attrs: Vec<Attr>,
}

fn space() -> String {
    " ".into()
}

fn is_space(symbol: &str) -> bool {
    symbol == " "
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Color {
    /// The viewing terminal's own default color.
    #[default]
    Default,
    /// The viewing terminal's palette; 0 to 15 are the named ANSI colors.
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl Color {
    fn is_default(&self) -> bool {
        *self == Self::Default
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Attr {
    Bold,
    Dim,
    Italic,
    Underlined,
    SlowBlink,
    RapidBlink,
    Reversed,
    Hidden,
    CrossedOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Cursor {
    pub x: u16,
    pub y: u16,
    pub shape: CursorShape,
    pub blinking: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum CursorShape {
    Block,
    Underline,
    Bar,
}

/// Terminal size in cells. Also the viewer size and tab size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

impl Size {
    pub const FALLBACK: Size = Size { cols: 80, rows: 24 };
}
