use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{ObjectBuilder, RefOr, schema::Schema},
};

use crate::{
    id::IdOf,
    model::{Creatable, Named, Tab, TabParent},
};

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

/// Hand-written: Utoipa's derive cannot see through `IdOf<T::Parent>` and
/// `T::Input`. Covers every entity created from a plain name (tabs, panes);
/// routes append the entity, naming the component e.g. `Create_Tab`.
impl<T: Creatable<Input = Named>> utoipa::__dev::ComposeSchema for Create<T>
where
    IdOf<T::Parent>: PartialSchema,
{
    fn compose(_: Vec<RefOr<Schema>>) -> RefOr<Schema> {
        ObjectBuilder::new()
            .property("parent", <IdOf<T::Parent> as PartialSchema>::schema())
            .required("parent")
            .property("name", String::schema())
            .required("name")
            .into()
    }
}

impl<T: Creatable<Input = Named>> ToSchema for Create<T>
where
    IdOf<T::Parent>: PartialSchema,
{
    fn name() -> Cow<'static, str> {
        "Create".into()
    }
}

/// POST /tabs/{id}/move body. No placement appends.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MoveTab {
    #[schema(inline)]
    pub parent: IdOf<TabParent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<Placement>,
}

/// A sibling in the destination to insert next to.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Placement {
    Before(IdOf<Tab>),
    After(IdOf<Tab>),
}
