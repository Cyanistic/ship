//! The config file: one TOML file that the server and each client read their
//! own part of.

use std::{fs, io, path::Path};

use serde::de::DeserializeOwned;

use crate::prelude::*;

/// `path` parsed as `T`; a missing file is an empty one. The error is
/// toml's, which gives the line, column and offending text.
pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(err!(Configuration, "{}: {error}", path.display())),
    };
    toml::from_str(&text).map_err(|error| err!(Configuration, "{}: {error}", path.display()))
}
