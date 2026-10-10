use std::sync::Arc;

use crossterm::event::KeyEvent;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{Object, ObjectBuilder, Ref, RefOr, schema::Schema},
};
use uuid::Uuid;

use crate::{
    id::{Attachment, IdOf},
    model::{NodeId, OptionalName, Pane, Tab, tabs_schema},
    screen::{Screen, Size},
    tree::Tabs,
};

pub const DEFAULT_PORT: u16 = 43179;
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:43179";
pub const HEALTH_PATH: &str = "/health";
pub const PROTOCOL_VERSION: u32 = 8;
/// Names the attachment a view or input request controls.
pub const ATTACHMENT_HEADER: &str = "x-ship-attachment-id";
/// On every response: the server's revision once the request was handled,
/// at least that of any commit it made. A client that waits for a replica of
/// this revision sees its own changes.
pub const REVISION_HEADER: &str = "x-ship-revision";
/// Longest line of the input stream, in bytes without the newline.
pub const INPUT_LINE_MAX: usize = 64 * 1024;

/// Server identity and protocol compatibility, not an authentication boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub service: String,
    pub protocol_version: u32,
    pub version: String,
}

/// POST /tabs body, e.g. `{"at": {"after": "tab:…"}, "starter": "shell"}`.
/// `at` defaults to the end of the top level.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTab {
    #[serde(default = "MoveTab::top")]
    pub at: MoveTab,
    #[serde(default)]
    pub name: OptionalName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter: Option<Starter>,
}

/// What a new tab starts with.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum Starter {
    /// One pane running the configured shell, else the login shell.
    Shell,
}

/// POST /panes body.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePane {
    pub at: PaneAt,
    #[serde(default)]
    pub direction: SplitDirection,
    #[serde(flatten)]
    pub input: PaneInput,
}

/// Split an explicit pane, or the largest pane of a tab (first pane if empty).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PaneAt {
    Tab(IdOf<Tab>),
    Pane(IdOf<Pane>),
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum SplitDirection {
    #[default]
    Right,
    Down,
}

/// What a pane runs. Pane creation input; also the `--starter` pane.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaneSpec {
    /// argv; absent means the server's login shell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    /// Absolute directory; absent means the server's home directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// POST /panes input, flattened next to `at` and `direction`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaneInput {
    #[serde(default)]
    pub name: OptionalName,
    #[serde(flatten)]
    pub spec: PaneSpec,
}

/// POST /tabs/{id}/move body: exactly one destination. `{"parent": "tab:…"}`,
/// `{"parent": null}` for the top level, `{"before": "tab:…"}` or
/// `{"after": "tab:…"}`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum MoveTab {
    /// Append under the parent, or to the top level when `null`.
    Parent(Option<IdOf<Tab>>),
    /// Insert before the sibling, under its parent.
    Before(IdOf<Tab>),
    /// Insert after the sibling, under its parent.
    After(IdOf<Tab>),
}

impl MoveTab {
    /// The end of the top level.
    pub fn top() -> Self {
        Self::Parent(None)
    }
}

/// POST /attach body. The selection is what a reconnecting client last had;
/// kept if it still exists, else nothing is selected.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
    /// Where the client draws the viewed tab, after its own chrome.
    pub area: Size,
}

/// PUT /attach/view body: the client's whole view. The server replaces the
/// attachment's record with it.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViewInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
    /// Where the client draws the viewed tab, after its own chrome.
    pub area: Size,
}

/// Server-owned view of one attachment. Deleted only when the attachment
/// ends; a removed selection falls back to an ancestor or to nothing.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViewingRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
    /// Where the client draws the viewed tab. Tab sizes derive from it.
    pub area: Size,
}

/// Complete published state, including each viewed tab's derived geometry.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Replica {
    /// UUID v7 chosen at server start; a later start sorts higher.
    #[schema(value_type = String, format = "uuid")]
    pub incarnation: Uuid,
    pub revision: u64,
    #[schema(schema_with = tabs_schema)]
    pub tabs: Tabs,
    #[schema(schema_with = viewers_schema)]
    pub viewers: IndexMap<IdOf<Attachment>, ViewingRecord>,
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

/// One event of the attach stream, sent as one SSE `data:` line, e.g.
/// `{"type": "screen", "data": {"pane": "pane:...", "screen": {...}}}`.
// Adjacently tagged: serde reads `type` first and deserializes `data` directly.
// Internally tagged, it buffered every screen cell to find the tag.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum SseEvent {
    /// Always first: the new attachment's ID and the state it starts from.
    Attached(Attached),
    /// A newer complete state, replacing the previous one.
    State(Arc<Replica>),
    /// The latest screen of a pane in the attachment's viewed tab. Sent when
    /// it changes, and for every such pane when the viewed tab changes.
    Screen(PaneScreen),
    /// Always last: why the server is closing the stream. A stream that ends
    /// without it was cut.
    Ended(Ended),
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaneScreen {
    pub pane: IdOf<Pane>,
    pub screen: Arc<Screen>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Ended {
    pub reason: EndReason,
}

/// Derived by each stream from the latest replica, never published.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum EndReason {
    /// The server is shutting down. Reattaching may succeed later.
    ServerShutdown,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Attached {
    #[schema(schema_with = attachment_schema)]
    pub attachment: IdOf<Attachment>,
    pub replica: Arc<Replica>,
}

/// One NDJSON line of POST /attach/input, e.g.
/// `{"type": "paste", "pane": "pane:...", "text": "ls"}`. View changes use
/// PUT /attach/view instead.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum InputFrame {
    Key(KeyInput),
    Paste(PasteInput),
}

/// A key press, encoded on the server against the pane's live terminal modes.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeyInput {
    pub pane: IdOf<Pane>,
    /// crossterm's own serialization, e.g. `{"code": "Enter", "modifiers": "",
    /// "kind": "Press", "state": ""}`; modifiers read `"SHIFT | CONTROL"`.
    #[schema(value_type = Object)]
    pub key: KeyEvent,
}

/// Text sent as one paste; bracketed when the program asked for it.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasteInput {
    pub pane: IdOf<Pane>,
    pub text: String,
}
