use std::{fmt, str::FromStr};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{Object, ObjectBuilder, Ref},
};

use crate::{
    AppError, err,
    id::{Id, IdOf, Identified, Prefixed, ServerRoot, UntaggedEither},
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
    pub name: String,
    #[schema(schema_with = tabs_schema)]
    pub tabs: IndexMap<IdOf<Tab>, Tab>,
}

/// Ordered child tabs keyed by ID. Hand-written because the derive inlines
/// `Tab`'s schema for the `IdOf<Tab>` key, recursing without end.
fn tabs_schema() -> Object {
    ObjectBuilder::new()
        .property_names(Some(IdOf::<Tab>::schema()))
        .additional_properties(Some(Ref::from_schema_name("Tab")))
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

pub type TabParent = UntaggedEither<Session, Tab>;

/// Associates an entity with its parent kind and creation input.
pub trait Creatable: Identified {
    type Parent: Identified;
    type Input;
}

impl Creatable for Session {
    type Parent = ServerRoot;
    type Input = Named<SessionName>;
}

impl Creatable for Tab {
    type Parent = TabParent;
    type Input = Named;
}

/// Creation and rename input. Sessions use `SessionName`; tabs and panes any string.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Named<N = String> {
    pub name: N,
}

/// A session name: non-empty and without ':'. The only place the rule lives.
/// The server gets it by deserializing session create/rename bodies; the CLI
/// gets it by parsing `SessionRef`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr, ToSchema)]
#[schema(value_type = String, pattern = "^[^:]+$")]
pub struct SessionName(String);

impl FromStr for SessionName {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(err!(Validation, "session name must not be empty"));
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
