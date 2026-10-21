//! The config file: one TOML file that the server and each client read their
//! own part of, over the defaults in `defaults.toml`.

use std::{fs, io, path::Path};

use serde::de::DeserializeOwned;
use toml::de::{DeTable, DeValue, Deserializer};

use crate::prelude::*;

/// Every default setting, key and menu, as a config file would write them.
pub const DEFAULTS: &str = include_str!("defaults.toml");

/// `path` over the defaults, parsed as `T`; a missing file is the defaults.
/// The error is toml's, which gives the line, column and offending text.
pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(err!(Configuration, "{}: {error}", path.display())),
    };
    let mut file = DeTable::parse(&text)
        .map_err(|error| err!(Configuration, "{}: {error}", path.display()))?;
    fill(file.get_mut(), defaults_table());
    serde_path_to_error::deserialize(Deserializer::from(file)).map_err(|error| {
        // Built from a table rather than text, the error can't quote the
        // line until it's given the file back.
        let at = error.path().clone();
        let mut error = error.into_inner();
        error.set_input(Some(&text));
        let error = serde_path_to_error::Error::new(at, error);
        err!(Configuration, "{}: {error}", path.display())
    })
}

/// The defaults alone, parsed as `T`.
pub fn defaults<T: DeserializeOwned>() -> T {
    match toml::from_str(DEFAULTS) {
        Ok(defaults) => defaults,
        Err(error) => panic!("invalid defaults.toml: {error}"),
    }
}

/// The defaults that fill a file key by key. Keys are left out: the client
/// merges them by chord, so two spellings of one chord meet, and a mode's
/// `clear_defaults` can drop them.
fn defaults_table() -> DeTable<'static> {
    let mut defaults = match DeTable::parse(DEFAULTS) {
        Ok(defaults) => defaults.into_inner(),
        Err(error) => panic!("invalid defaults.toml: {error}"),
    };
    if let Some(DeValue::Table(client)) = defaults.get_mut("client").map(|client| client.get_mut())
    {
        client.remove("modes");
    }
    defaults
}

/// Fills what `file` leaves out from `defaults`. A table on both sides merges
/// key by key; any other value in `file` replaces the default whole.
fn fill<'i>(file: &mut DeTable<'i>, defaults: DeTable<'i>) {
    for (key, value) in defaults {
        let Some(written) = file.get_mut(key.get_ref().as_ref()) else {
            file.insert(key, value);
            continue;
        };
        if let (DeValue::Table(written), DeValue::Table(default)) =
            (written.get_mut(), value.into_inner())
        {
            fill(written, default);
        }
    }
}
