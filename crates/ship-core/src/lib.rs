//! Shared contextual diagnostics. No HTTP or process ownership.

pub mod error;

pub use error::{AppError, ErrorCode, ExternalError, InternalError, Result, ResultExt};
