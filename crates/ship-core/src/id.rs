use std::{fmt, hash::Hash, marker::PhantomData, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use utoipa::openapi::{
    RefOr, Type,
    schema::{ObjectBuilder, Schema},
};
use uuid::Uuid;

use crate::{AppError, err};

/// Stable external kind prefix, e.g. `tab` in `tab:3f2a...`.
pub trait Prefixed {
    fn prefix() -> &'static str;
}

/// Associates an entity, or a choice of entities, with its typed identity.
pub trait Identified {
    type Id: Clone + Eq + Hash + fmt::Debug + Serialize + for<'de> Deserialize<'de>;
}

pub type IdOf<T> = <T as Identified>::Id;

/// UUID v4 tagged with its entity kind. Displayed and serialized as
/// `{prefix}:{uuid in simple form}`. Parsing rejects any other prefix and
/// accepts any UUID spelling `Uuid::parse_str` does.
#[derive(SerializeDisplay, DeserializeFromStr)]
pub struct Id<T> {
    uuid: Uuid,
    kind: PhantomData<fn() -> T>,
}

impl<T: Prefixed> Id<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            kind: PhantomData,
        }
    }
}

impl<T: Prefixed> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", T::prefix(), self.uuid.simple())
    }
}

impl<T: Prefixed> FromStr for Id<T> {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let expected = T::prefix();
        let Some((prefix, hex)) = value.split_once(':') else {
            return Err(err!(
                Validation,
                "expected {} ID, got '{}'",
                expected,
                value
            ));
        };
        if prefix != expected {
            return Err(err!(Validation, "expected {} ID, got {}", expected, prefix));
        }
        let uuid = Uuid::parse_str(hex)
            .map_err(|error| err!(Validation, "malformed {} ID", expected, @external: error))?;
        Ok(Self {
            uuid,
            kind: PhantomData,
        })
    }
}

// Hand-written so they need no bounds on the marker type.
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}
impl<T> Eq for Id<T> {}
impl<T> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}
impl<T: Prefixed> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// A string matching `^{prefix}:[0-9a-f]{32}$`. Utoipa 6 derives
/// `PartialSchema` from this for generic types. The schema derive passes the
/// entity's schema for `IdOf<T>`; an ID's schema does not depend on it.
impl<T: Prefixed> utoipa::__dev::ComposeSchema for Id<T> {
    fn compose(_: Vec<RefOr<Schema>>) -> RefOr<Schema> {
        let prefix = T::prefix();
        ObjectBuilder::new()
            .schema_type(Type::String)
            .pattern(Some(format!("^{prefix}:[0-9a-f]{{32}}$")))
            .description(Some(format!("{prefix} ID, e.g. {prefix}:3f2a...")))
            .into()
    }
}

// The default name is `Id`; Utoipa appends the entity, e.g. `Id_Tab`.
impl<T: Prefixed> utoipa::ToSchema for Id<T> {}

/// Marker for stream attachments; never instantiated.
pub enum Attachment {}

impl Prefixed for Attachment {
    fn prefix() -> &'static str {
        "attachment"
    }
}

impl Identified for Attachment {
    type Id = Id<Attachment>;
}
