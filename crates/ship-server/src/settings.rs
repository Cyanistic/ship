//! `[server]` in the config file. Read whenever a shell starts, never cached.

use std::path::{Path, PathBuf};

use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::Deserialize;
use ship_core::prelude::*;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSettings {
    /// Absent: the login shell.
    pub shell: Option<PathBuf>,
}

/// The file's top level as the server sees it: only `server`, with the
/// client's part and any typo left to the client and `ship config check`.
#[derive(Deserialize)]
struct Root {
    #[serde(default)]
    server: ServerSettings,
}

impl ServerSettings {
    /// `server` from the file through figment. A missing file is the
    /// defaults; a relative `shell` resolves from the file's folder (FR-005).
    /// Also what `ship config check` runs. `[server]` has one field, so its
    /// first error is the only one.
    pub fn load(path: &Path) -> Result<Self> {
        let Root { mut server } = Figment::from(Toml::file(path))
            .extract()
            .map_err(|error| located(path, error))?;
        if let (Some(shell), Some(folder)) = (&mut server.shell, path.parent()) {
            *shell = folder.join(&*shell);
        }
        Ok(server)
    }
}

/// Figment's own message prefixes the key with its `default.` profile, so
/// build the location from the key path instead. Syntax errors have no key
/// path, and their message carries the line and column.
fn located(path: &Path, error: figment::Error) -> AppError {
    let kind = error.kind.to_string();
    let kind = kind.trim_end();
    match error.path.join(".") {
        key if key.is_empty() => err!(Configuration, "{}: {kind}", path.display()),
        key => err!(Configuration, "{}: {key}: {kind}", path.display()),
    }
}
