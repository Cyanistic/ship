use std::{borrow::Cow, sync::Arc};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{Object, ObjectBuilder, Ref, RefOr, schema::Schema},
};
use uuid::Uuid;

use crate::{
    id::{Attachment, IdOf},
    model::{Creatable, Named, NodeId, Session, Tab, TabParent},
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

/// POST /attach body. The selection is what a reconnecting observer last had.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachRequest {
    pub session: IdOf<Session>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
}

/// Server-owned view of one attachment. Always names a session; the record
/// is deleted when the attachment ends or its session is removed.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViewingRecord {
    pub session: IdOf<Session>,
    pub selection: NodeId,
}

/// Complete replicated state. Session `Arc`s are shared with the state actor.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Replica {
    /// UUID v7 chosen at server start; a later start sorts higher.
    #[schema(value_type = String, format = "uuid")]
    pub incarnation: Uuid,
    pub revision: u64,
    #[schema(schema_with = sessions_schema)]
    pub sessions: IndexMap<IdOf<Session>, Arc<Session>>,
    #[schema(schema_with = viewers_schema)]
    pub viewers: IndexMap<IdOf<Attachment>, ViewingRecord>,
}

/// Sessions keyed by ID, in order. Written by hand like `Session::tabs`, so
/// the map key is described as an ID rather than an inlined entity.
fn sessions_schema() -> Object {
    ObjectBuilder::new()
        .property_names(Some(IdOf::<Session>::schema()))
        .additional_properties(Some(Ref::from_schema_name("Session")))
        .build()
}

/// `Attachment` is a marker with no schema of its own for the derive to pass.
fn attachment_schema() -> RefOr<Schema> {
    IdOf::<Attachment>::schema()
}

fn viewers_schema() -> Object {
    ObjectBuilder::new()
        .property_names(Some(IdOf::<Attachment>::schema()))
        .additional_properties(Some(Ref::from_schema_name("ViewingRecord")))
        .build()
}

/// One `data:` line of the attach stream.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SseEvent {
    /// Always first: the new attachment's ID and the state it starts from.
    Attached(Attached),
    /// A newer complete state, replacing the previous one.
    State(Arc<Replica>),
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Attached {
    #[schema(schema_with = attachment_schema)]
    pub attachment: IdOf<Attachment>,
    pub replica: Arc<Replica>,
}

/// PUT /attach/selection body. The selection must be in the attached session.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SelectRequest {
    pub selection: NodeId,
}

/// PUT /attach/session body. The new selection is the session itself.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SwitchSessionRequest {
    pub session: IdOf<Session>,
}
