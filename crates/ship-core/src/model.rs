use std::{fmt, str::FromStr};

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{Object, ObjectBuilder, Ref},
};

use crate::{
    AppError, err,
    id::{Id, IdOf, Identified, Prefixed, ServerRoot, UntaggedEither},
    protocol::{CreateSession, PaneInput},
};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: IdOf<Session>,
    pub name: SessionName,
    #[schema(schema_with = tabs_schema)]
    pub tabs: IndexMap<IdOf<Tab>, Tab>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: IdOf<Tab>,
    #[serde(default)]
    pub name: OptionalName,
    #[schema(schema_with = tabs_schema)]
    pub tabs: IndexMap<IdOf<Tab>, Tab>,
    #[schema(schema_with = panes_schema)]
    pub panes: IndexMap<IdOf<Pane>, Pane>,
}

/// A leaf running one program in its own terminal. Owns no layout or children.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Pane {
    pub id: IdOf<Pane>,
    #[serde(default)]
    pub name: OptionalName,
    /// The argv the pane started; the shell's path for the default login shell.
    pub command: Vec<String>,
    /// Starting directory.
    pub cwd: String,
    /// The title the program last set; absent when none or cleared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub status: PaneStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum PaneStatus {
    Running,
    Exited(ExitStatus),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExitStatus {
    pub code: u32,
    /// The signal's name when a signal ended the program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
}

/// Ordered child tabs keyed by ID. Hand-written because the derive inlines
/// `Tab`'s schema for the `IdOf<Tab>` key, recursing without end.
fn tabs_schema() -> Object {
    ObjectBuilder::new()
        .property_names(Some(IdOf::<Tab>::schema()))
        .additional_properties(Some(Ref::from_schema_name("Tab")))
        .build()
}

/// Ordered panes keyed by ID, written like `tabs_schema` so the map key is
/// described as a pane ID rather than an inlined `Pane`.
fn panes_schema() -> Object {
    ObjectBuilder::new()
        .property_names(Some(IdOf::<Pane>::schema()))
        .additional_properties(Some(Ref::from_schema_name("Pane")))
        .build()
}

impl Prefixed for Session {
    fn prefix() -> &'static str {
        "session"
    }
}

impl Identified for Session {
    type Id = Id<Session>;
}

impl Prefixed for Tab {
    fn prefix() -> &'static str {
        "tab"
    }
}

impl Identified for Tab {
    type Id = Id<Tab>;
}

impl Prefixed for Pane {
    fn prefix() -> &'static str {
        "pane"
    }
}

impl Identified for Pane {
    type Id = Id<Pane>;
}

pub type TabParent = UntaggedEither<Session, Tab>;

/// Associates an entity with its parent kind and creation input.
pub trait Creatable: Identified {
    type Parent: Identified;
    type Input;
}

impl Creatable for Session {
    type Parent = ServerRoot;
    type Input = CreateSession;
}

impl Creatable for Tab {
    type Parent = TabParent;
    type Input = Named<OptionalName>;
}

impl Creatable for Pane {
    type Parent = Tab;
    type Input = PaneInput;
}

/// Creation and rename input. Sessions use `SessionName`; tabs and panes
/// `OptionalName`.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Named<N> {
    pub name: N,
}

/// Optional tab or pane name. Blank or whitespace-only input means none, so
/// missing, `null`, `""` and `"  "` all deserialize to `None`. The only place
/// the rule lives.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, ToSchema)]
#[schema(value_type = Option<String>)]
pub struct OptionalName(Option<String>);

impl OptionalName {
    pub fn get(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl From<Option<String>> for OptionalName {
    fn from(name: Option<String>) -> Self {
        Self(name.filter(|name| !name.trim().is_empty()))
    }
}

impl<'de> Deserialize<'de> for OptionalName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<String>::deserialize(deserializer).map(Self::from)
    }
}

/// A session name: not blank and without ':'. The only place the rule lives.
/// The server gets it by deserializing session create/rename bodies; the CLI
/// gets it by parsing `SessionRef`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr, ToSchema)]
#[schema(value_type = String, pattern = "^[^:]+$")]
pub struct SessionName(String);

impl FromStr for SessionName {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.trim().is_empty() {
            return Err(err!(Validation, "session name must not be blank"));
        }
        if value.contains(':') {
            return Err(err!(
                Validation,
                "session name '{}' must not contain ':'",
                value
            ));
        }
        Ok(Self(value.to_owned()))
    }
}

impl fmt::Display for SessionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Any selectable entity. Untagged; the prefix decides the variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum NodeId {
    Session(IdOf<Session>),
    Tab(IdOf<Tab>),
    Pane(IdOf<Pane>),
}

impl FromStr for NodeId {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split_once(':').map(|(prefix, _)| prefix) {
            Some("session") => value.parse().map(Self::Session),
            Some("tab") => value.parse().map(Self::Tab),
            Some("pane") => value.parse().map(Self::Pane),
            _ => Err(err!(
                Validation,
                "expected session, tab or pane ID, got '{}'",
                value
            )),
        }
    }
}

impl From<IdOf<TabParent>> for NodeId {
    fn from(parent: IdOf<TabParent>) -> Self {
        match parent {
            UntaggedEither::Left(session) => Self::Session(session),
            UntaggedEither::Right(tab) => Self::Tab(tab),
        }
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(id) => id.fmt(f),
            Self::Tab(id) => id.fmt(f),
            Self::Pane(id) => id.fmt(f),
        }
    }
}
