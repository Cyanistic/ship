use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use utoipa::ToSchema;

use crate::{
    AppError, err,
    id::{Id, IdOf, Identified, Prefixed, ServerRoot},
};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: IdOf<Session>,
    pub name: SessionName,
}

impl Prefixed for Session {
    fn prefix() -> &'static str {
        "session"
    }
}

impl Identified for Session {
    type Id = Id<Session>;
}

/// Associates an entity with its parent kind and creation input.
pub trait Creatable: Identified {
    type Parent: Identified;
    type Input;
}

impl Creatable for Session {
    type Parent = ServerRoot;
    type Input = Named<SessionName>;
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
