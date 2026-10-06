//! Shared contextual diagnostics. No HTTP or process ownership.

pub mod error;
pub mod id;
pub mod model;

/// Common operation errors and construction helpers.
pub mod prelude {
    pub use crate::{AppError, ErrorCode, Result, ResultExt, err};
}

pub use error::{AppError, ErrorCode, ExternalError, InternalError, Result, ResultExt};

pub mod protocol;
pub use protocol::{
    DEFAULT_PORT, DEFAULT_SERVER_URL, HEALTH_PATH, HealthResponse, PROTOCOL_VERSION,
};
