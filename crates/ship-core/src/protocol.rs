use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{id::IdOf, model::Creatable};

pub const DEFAULT_PORT: u16 = 43179;
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:43179";
pub const HEALTH_PATH: &str = "/health";
pub const PROTOCOL_VERSION: u32 = 1;
/// Names the attachment a selection or session-switch request controls.
pub const ATTACHMENT_HEADER: &str = "x-ship-attachment-id";

/// Server identity and protocol compatibility, not an authentication boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub service: String,
    pub protocol_version: u32,
    pub version: String,
}

/// Create body: `{"parent": "...", "name": "..."}`. Sessions take only the
/// input, since their parent is the server root.
#[derive(Serialize, Deserialize)]
pub struct Create<T: Creatable> {
    pub parent: IdOf<T::Parent>,
    #[serde(flatten)]
    pub input: T::Input,
}
