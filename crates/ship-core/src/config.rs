//! The config file: one TOML file that the server and each client read their
//! own part of, over the defaults in `defaults.toml`.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, de::DeserializeOwned};
use toml::de::{DeTable, DeValue, Deserializer};

use crate::{model::ExitBehavior, prelude::*};

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

/// `[server.defaults.pane]`: what a new pane starts with unless its create
/// request says otherwise.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneDefaults {
    /// Absent: the login shell. Relative paths resolve from the config file's folder.
    pub shell: Option<PathBuf>,
    pub scrollback_bytes: usize,
    pub right_click_passthrough: bool,
    pub exit_behavior: ExitBehavior,
}

/// `[server]`. The server reads it whenever a pane is created, never cached.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSettings {
    pub defaults: ServerDefaults,
}

/// `[server.defaults]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerDefaults {
    pub pane: PaneDefaults,
}

/// `defaults.toml`'s.
impl Default for ServerSettings {
    fn default() -> Self {
        defaults::<ServerRoot>().server
    }
}

/// The file's top level as the server sees it: only `server`, with the
/// client's part and any typo left to the client and `ship config check`.
#[derive(Deserialize)]
struct ServerRoot {
    server: ServerSettings,
}

impl ServerSettings {
    /// `server` from the file. A missing file is the defaults; a relative
    /// `defaults.pane.shell` resolves from the file's folder (FR-005). Also
    /// what `ship config check` runs.
    pub fn load(path: &Path) -> Result<Self> {
        let ServerRoot { mut server } = load(path)?;
        if let (Some(shell), Some(folder)) = (&mut server.defaults.pane.shell, path.parent()) {
            *shell = folder.join(&*shell);
        }
        Ok(server)
    }
}

/// `defaults.toml`'s pane history size, for saved panes that lack one.
pub fn builtin_scrollback_bytes() -> usize {
    ServerSettings::default().defaults.pane.scrollback_bytes
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
