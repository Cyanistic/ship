//! `[server]` in the config file. Read whenever a pane starts, never cached.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use ship_core::{config, prelude::*};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSettings {
    /// Absent: the login shell.
    pub shell: Option<PathBuf>,
    /// History cap for panes created after the file is saved.
    pub scrollback_bytes: usize,
}

/// `defaults.toml`'s.
impl Default for ServerSettings {
    fn default() -> Self {
        config::defaults::<Root>().server
    }
}

/// The file's top level as the server sees it: only `server`, with the
/// client's part and any typo left to the client and `ship config check`.
#[derive(Deserialize)]
struct Root {
    server: ServerSettings,
}

impl ServerSettings {
    /// `server` from the file. A missing file is the defaults; a relative
    /// `shell` resolves from the file's folder (FR-005). Also what
    /// `ship config check` runs.
    pub fn load(path: &Path) -> Result<Self> {
        let Root { mut server } = config::load(path)?;
        if let (Some(shell), Some(folder)) = (&mut server.shell, path.parent()) {
            *shell = folder.join(&*shell);
        }
        Ok(server)
    }
}
