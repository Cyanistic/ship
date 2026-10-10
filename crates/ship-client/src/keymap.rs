//! `[client]` in the config file: modes and their bindings.

use std::{collections::HashMap, path::Path};

use crokey::KeyCombination;
use serde::{Deserialize, Deserializer, de::IgnoredAny};
use ship_core::{
    command::{Command, Direction},
    config,
    prelude::*,
};

const DEFAULTS: &str = include_str!("keymap/defaults.toml");

pub struct Keymap {
    pub modes: HashMap<ModeName, Mode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct ModeName(pub String);

impl ModeName {
    pub const NORMAL: &str = "normal";

    pub fn normal() -> Self {
        Self(Self::NORMAL.into())
    }
}

pub struct Mode {
    pub kind: ModeKind,
    /// Only bound chords: `[]` leaves the chord out.
    pub keys: HashMap<KeyCombination, Binding>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind {
    #[default]
    Sticky,
    Oneshot,
}

/// Actions run in order, stopping at the first failure. Empty unbinds the
/// chord, so the key reaches the program.
#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct Binding(pub Vec<Action>);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Server(Command),
    Client(ClientAction),
}

/// Actions without arguments are empty struct variants (`Detach {}`), not
/// unit variants: the config writes them as `{}`, which figment accepts for a
/// struct variant and rejects for a unit one.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientAction {
    Tab(ClientTab),
    Pane(ClientPane),
    Sidebar(Sidebar),
    Mode(ModeName),
    Send(KeyCombination),
    Detach {},
    Config(ConfigAction),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientTab {
    Next {},
    Prev {},
    Select { row: u16 },
    Expand {},
    Collapse {},
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientPane {
    Next {},
    Prev {},
    Focus { direction: Direction },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Sidebar {
    Toggle {},
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigAction {
    Reload {},
}

/// The client's part of the config file. The defaults and the user's file
/// both read as this, and the file then goes over the defaults.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    /// Left to the server; named so a misspelled table is still an error.
    #[serde(default, rename = "server")]
    _server: IgnoredAny,
    #[serde(default)]
    client: Client,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Client {
    #[serde(default)]
    modes: HashMap<ModeName, ModeFile>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModeFile {
    kind: Option<ModeKind>,
    /// Drop the mode's default bindings before the file's apply.
    #[serde(default)]
    clear_defaults: bool,
    #[serde(default, deserialize_with = "normalized")]
    keys: HashMap<KeyCombination, Binding>,
}

/// crokey parses chords without normalizing them, and lookups use the
/// normalized form. Normalizing before the merge also lets a file chord
/// replace a default spelled differently.
fn normalized<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<HashMap<KeyCombination, Binding>, D::Error> {
    let keys = HashMap::<KeyCombination, Binding>::deserialize(deserializer)?;
    Ok(keys
        .into_iter()
        .map(|(chord, binding)| (chord.normalized(), binding))
        .collect())
}

impl Keymap {
    /// The file over the defaults; a missing file is the defaults. A file
    /// chord replaces the default on that chord. Used at startup, on reload
    /// and by `ship config check`.
    pub fn load(path: &Path) -> Result<Self> {
        Ok(Self::over_defaults(config::load(path)?))
    }

    pub fn defaults() -> Self {
        Self::over_defaults(File::default())
    }

    fn over_defaults(file: File) -> Self {
        let defaults: File = match toml::from_str(DEFAULTS) {
            Ok(defaults) => defaults,
            Err(error) => panic!("invalid default keys: {error}"),
        };
        let mut modes = defaults.client.modes;
        for (name, over) in file.client.modes {
            let mode = modes.entry(name).or_default();
            if over.clear_defaults {
                mode.keys.clear();
            }
            mode.keys.extend(over.keys);
            mode.kind = over.kind.or(mode.kind);
        }
        let modes = modes
            .into_iter()
            .map(|(name, mode)| (name, mode.into()))
            .collect();
        Self { modes }
    }
}

impl From<ModeFile> for Mode {
    fn from(mut file: ModeFile) -> Self {
        file.keys.retain(|_, binding| !binding.0.is_empty());
        Self {
            kind: file.kind.unwrap_or_default(),
            keys: file.keys,
        }
    }
}

impl Binding {
    /// Whether running this enters a mode, so a one-shot mode doesn't leave
    /// for normal on its own.
    pub fn enters_mode(&self) -> bool {
        self.0
            .iter()
            .any(|action| matches!(action, Action::Client(ClientAction::Mode(_))))
    }
}
