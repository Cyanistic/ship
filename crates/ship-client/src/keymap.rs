//! `[client]` in the config file: settings, modes and their bindings.

use std::{borrow::Cow, collections::HashMap, path::Path};

use crokey::KeyCombination;
use indexmap::IndexMap;
use serde::{
    Deserialize, Deserializer,
    de::{Error as _, IgnoredAny},
};
use ship_core::{
    command::{Command, Direction, pane::PaneCommand},
    config,
    prelude::*,
};

const DEFAULTS: &str = include_str!("keymap/defaults.toml");

pub struct ClientSettings {
    pub mouse: bool,
    pub keymap: Keymap,
    pub sidebar_width: u16,
}

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
    /// Only bound chords: `{}` leaves the chord out.
    pub keys: IndexMap<KeyCombination, Binding>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind {
    #[default]
    Sticky,
    Oneshot,
}

/// One action and its status-line hint. `{}` unbinds the chord, so the
/// key reaches the program.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    #[serde(default)]
    pub action: Option<Action>,
    /// Completed with its chord and the action's fallback word on load.
    #[serde(default)]
    hint: Option<String>,
}

impl Mode {
    /// Each chord's `chord label` hint, in the file's order.
    pub fn hints(&self) -> impl Iterator<Item = &str> {
        self.keys
            .values()
            .filter(|binding| binding.action.is_some())
            .filter_map(|binding| binding.hint.as_deref())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Server(Command),
    Client(ClientAction),
}

impl Action {
    /// The mode, direction or sent chord, otherwise the action's config name.
    pub fn hint(&self) -> Cow<'static, str> {
        let name: &'static str = match self {
            Self::Server(Command::Tab(action)) => action.into(),
            Self::Server(Command::Pane(PaneCommand::Resize(action))) => (&action.direction).into(),
            Self::Server(Command::Pane(action)) => action.into(),
            Self::Client(ClientAction::Mode(mode)) => return Cow::Owned(mode.0.clone()),
            Self::Client(ClientAction::Send(chord)) => return Cow::Owned(chord.to_string()),
            Self::Client(ClientAction::Tab(action)) => action.into(),
            Self::Client(ClientAction::Pane(
                ClientPane::Focus { direction } | ClientPane::Swap { direction },
            )) => direction.into(),
            Self::Client(ClientAction::Pane(action)) => action.into(),
            Self::Client(ClientAction::Sidebar(action)) => action.into(),
            Self::Client(ClientAction::Config(action)) => action.into(),
            Self::Client(action) => action.into(),
        };
        Cow::Borrowed(name)
    }
}

/// Actions without arguments are empty struct variants (`Detach {}`), not
/// unit variants: the config writes them as `{}`, which figment accepts for a
/// struct variant and rejects for a unit one.
#[derive(Clone, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[strum(serialize_all = "snake_case")]
pub enum ClientAction {
    Tab(ClientTab),
    Pane(ClientPane),
    Sidebar(Sidebar),
    Mode(ModeName),
    Send(KeyCombination),
    Detach {},
    Config(ConfigAction),
}

#[derive(Clone, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[strum(serialize_all = "snake_case")]
pub enum ClientTab {
    Next {},
    Prev {},
    Select { row: u16 },
    Expand {},
    Collapse {},
}

#[derive(Clone, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[strum(serialize_all = "snake_case")]
pub enum ClientPane {
    Next {},
    Prev {},
    Focus {
        direction: Direction,
    },
    /// Exchange with the neighbor `focus` would pick, through `server.pane.swap`.
    Swap {
        direction: Direction,
    },
}

#[derive(Clone, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[strum(serialize_all = "snake_case")]
pub enum Sidebar {
    Toggle {},
}

#[derive(Clone, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[strum(serialize_all = "snake_case")]
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
    mouse: Option<bool>,
    sidebar_width: Option<u16>,
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
    keys: IndexMap<KeyCombination, Binding>,
}

/// crokey parses chords without normalizing them, and lookups use the
/// normalized form. Normalizing before the merge also lets a file chord
/// replace a default spelled differently.
fn normalized<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<IndexMap<KeyCombination, Binding>, D::Error> {
    let keys = IndexMap::<String, Binding>::deserialize(deserializer)?;
    keys.into_iter()
        .map(|(spelling, mut binding)| {
            let chord: KeyCombination = spelling.parse().map_err(D::Error::custom)?;
            if let Some(action) = &binding.action {
                let hint = binding
                    .hint
                    .take()
                    .map_or_else(|| action.hint(), Cow::Owned);
                binding.hint = Some(format!("{spelling} {hint}"));
            }
            Ok((chord.normalized(), binding))
        })
        .collect()
}

impl ClientSettings {
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
            // File chords follow untouched defaults in their file order.
            for chord in over.keys.keys() {
                mode.keys.shift_remove(chord);
            }
            mode.keys.extend(over.keys);
            mode.kind = over.kind.or(mode.kind);
        }
        let modes = modes
            .into_iter()
            .map(|(name, mode)| (name, mode.into()))
            .collect();
        Self {
            keymap: Keymap { modes },
            sidebar_width: file.client.sidebar_width.unwrap_or(26),
            mouse: file.client.mouse.unwrap_or(true),
        }
    }
}

impl From<ModeFile> for Mode {
    fn from(mut file: ModeFile) -> Self {
        file.keys.retain(|_, binding| binding.action.is_some());
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
        matches!(self.action, Some(Action::Client(ClientAction::Mode(_))))
    }
}
