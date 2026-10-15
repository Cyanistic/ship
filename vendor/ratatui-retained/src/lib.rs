//! Retained rendering for ratatui. A plain ratatui frame repaints every
//! widget into an empty buffer and diffs the whole screen. Here the last
//! frame is kept, layers report which cells changed, and only those cells
//! are reset, replayed through every layer above and below them, and diffed.

mod compositor;
#[cfg(feature = "reactive")]
mod reactive;
mod region;

pub use compositor::{Compositor, Layer, Stack, Stats, footprint, render_full};
#[cfg(feature = "reactive")]
pub use reactive::{Ui, View, rows, widget};
#[cfg(feature = "reactive")]
pub use reactive_scope;
pub use region::Region;
