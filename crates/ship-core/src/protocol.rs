use std::sync::Arc;

use crossterm::event::{KeyEvent, KeyModifiers, MouseEventKind};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{Object, ObjectBuilder, Ref, RefOr, schema::Schema},
};
use uuid::Uuid;

use crate::{
    command::Direction,
    id::{Attachment, IdOf},
    model::{NodeId, OptionalName, Pane, Switch, Tab, tabs_schema},
    screen::{Screen, Size, is_false},
    tree::Tabs,
};

pub const DEFAULT_PORT: u16 = 43179;
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:43179";
pub const HEALTH_PATH: &str = "/health";
pub const PROTOCOL_VERSION: u32 = 13;
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
    /// The log part the server is writing.
    pub log: String,
}

/// PUT /api/v0/server/log: {"set": "<filter>"} or "reset".
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum LogChange {
    Set(String),
    Reset,
}

/// The running server's filters and current log part.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogFilter {
    pub active: String,
    pub startup: String,
    pub file: String,
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
    /// Select the new tab, or its starter pane, in the same commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(schema_with = attachment_schema)]
    pub select: Option<IdOf<Attachment>>,
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
    /// Select the new pane in the same commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(schema_with = attachment_schema)]
    pub select: Option<IdOf<Attachment>>,
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
    /// Start with right-click passthrough on.
    #[serde(default, skip_serializing_if = "is_false")]
    pub right_click_passthrough: bool,
    #[serde(flatten)]
    pub spec: PaneSpec,
}

/// PATCH /tabs/{id} body. Absent or null leaves the name; blank clears it.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetTab {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "set_name"
    )]
    #[schema(value_type = Option<String>)]
    pub name: Option<OptionalName>,
}

/// PATCH /panes/{id} body. Absent or null fields stay as they are.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetPane {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "set_name"
    )]
    #[schema(value_type = Option<String>)]
    pub name: Option<OptionalName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_click_passthrough: Option<Switch>,
}

/// A present blank name must serialize as `""` to clear, not `null` to leave.
fn set_name<S: serde::Serializer>(
    name: &Option<OptionalName>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    name.as_ref()
        .map(|name| name.get().unwrap_or(""))
        .serialize(serializer)
}

/// POST /panes/{id}/zoom body. `{}` toggles; `false` unzooms the pane's tab
/// and `true` zooms the pane.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ZoomPane {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoomed: Option<bool>,
}

/// POST /panes/{id}/swap body: the pane to exchange places with, in the same
/// tab. Adjacency is not required.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwapPane {
    pub other: IdOf<Pane>,
}

/// POST /panes/{id}/resize body. At most one of `amount` and `to`.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResizePane {
    /// Which way the edge moves: the pane's edge on that side, or the
    /// opposite edge when that side is the tab's.
    pub direction: Direction,
    /// Cells; absent is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u16>,
    /// Where the edge's border is drawn, from the tab's left or top edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<u16>,
}

/// POST /panes/{id}/scroll body.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScrollPane {
    pub direction: ScrollDirection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u32>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Top,
    Bottom,
}

/// GET /panes/{id}/text query.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ReadPane {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<u32>,
    pub unwrap: bool,
    pub ansi: bool,
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

/// PUT /attach/view body: the client's area only.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViewInput {
    /// Where the client draws the viewed tab, after its own chrome.
    pub area: Size,
}

/// PUT /views/{attachment}/selection body; `null` selects nothing.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelectView {
    pub selection: Option<NodeId>,
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
    /// Text for this client's clipboard: its own mouse selection, or a
    /// program's clipboard write in a pane of its viewed tab.
    Clipboard(ClipboardText),
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
pub struct ClipboardText {
    pub text: String,
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
    Mouse(MouseInput),
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

/// A pointer relative to the pane's content; drags may be outside it.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MouseInput {
    pub pane: IdOf<Pane>,
    #[schema(value_type = Object)]
    pub kind: MouseEventKind,
    #[schema(value_type = String)]
    pub modifiers: KeyModifiers,
    pub x: i32,
    pub y: i32,
}

/// Text sent as one paste; bracketed when the program asked for it.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasteInput {
    pub pane: IdOf<Pane>,
    pub text: String,
}
