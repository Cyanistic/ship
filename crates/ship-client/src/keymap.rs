//! `[client]` in the config file: settings, modes and their bindings.

use std::{
    borrow::Cow,
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    time::Duration,
};

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

use crate::sound::{Clip, Cue};

pub struct ClientSettings {
    pub mouse: bool,
    pub keymap: Keymap,
    pub sidebar_width: u16,
    pub agents: AgentSettings,
}

/// `[client.agents]` with defaults applied.
pub struct AgentSettings {
    /// The lines of an agent's entry in the sidebar.
    pub rows: Vec<Vec<Token>>,
    /// How long a new state must hold before it notifies.
    pub delay: Duration,
    /// Whether fired notifications are also written as OSC 9 popups.
    pub popups: bool,
    pub sounds: Sounds<Cue>,
}

/// One value in an agent's entry.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Token {
    Icon,
    App,
    State,
    Tab,
    Pane,
    Title,
    Msg,
}

/// The two kinds of notification. In config it also names a bundled
/// sound: `{ builtin = "done" }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Notice {
    Done,
    Attention,
}

/// `"off"`, `{ builtin = "done" }` or `{ path = "~/ding.wav" }`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sound {
    Off,
    Builtin(Notice),
    Path(PathBuf),
}

/// `[client.agents.sounds]`: the defaults and each app's overrides. `S` is
/// `Sound` as written, `Cue` once loaded.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, bound = "S: Deserialize<'de>")]
pub struct Sounds<S> {
    pub default: NoticeSounds<S>,
    /// Keyed by the record's `app`.
    pub apps: HashMap<String, NoticeSounds<S>>,
}

/// One slot per notice; unset falls through to the next level.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, bound = "S: Deserialize<'de>")]
pub struct NoticeSounds<S> {
    pub done: Option<S>,
    pub attention: Option<S>,
}

impl Sounds<Sound> {
    /// Each path read into a clip; a bad file becomes the bundled clip for
    /// its slot's notice, with a warning naming it.
    fn load(self, base: &Path) -> Sounds<Cue> {
        Sounds {
            default: self.default.load(base),
            apps: self
                .apps
                .into_iter()
                .map(|(app, sounds)| (app, sounds.load(base)))
                .collect(),
        }
    }
}

impl Sounds<Cue> {
    /// The app's cue, else the default's, else the notice's bundled clip;
    /// `None` for `off`.
    pub fn resolve(&self, app: Option<&str>, notice: Notice) -> Option<Clip> {
        let cue = app
            .and_then(|app| self.apps.get(app)?.get(notice))
            .or_else(|| self.default.get(notice));
        match cue {
            Some(Cue::Off) => None,
            Some(Cue::Play(clip)) => Some(clip.clone()),
            None => Some(Clip::bundled(notice)),
        }
    }
}

impl NoticeSounds<Sound> {
    fn load(self, base: &Path) -> NoticeSounds<Cue> {
        let load = |sound: Option<Sound>, notice| sound.map(|sound| sound.load(base, notice));
        NoticeSounds {
            done: load(self.done, Notice::Done),
            attention: load(self.attention, Notice::Attention),
        }
    }
}

impl<S> NoticeSounds<S> {
    fn get(&self, notice: Notice) -> Option<&S> {
        match notice {
            Notice::Done => self.done.as_ref(),
            Notice::Attention => self.attention.as_ref(),
        }
    }
}

impl Sound {
    /// A path resolves against `base`, the config file's directory, with a
    /// leading `~` taken as the home directory.
    fn load(self, base: &Path, notice: Notice) -> Cue {
        match self {
            Self::Off => Cue::Off,
            Self::Builtin(builtin) => Cue::Play(Clip::bundled(builtin)),
            Self::Path(path) => {
                let path = match (path.strip_prefix("~"), env::home_dir()) {
                    (Ok(rest), Some(home)) => home.join(rest),
                    _ => base.join(path),
                };
                Cue::Play(Clip::load(&path).unwrap_or_else(|error| {
                    tracing::warn!("{error}; using the bundled sound instead");
                    Clip::bundled(notice)
                }))
            }
        }
    }
}

pub struct Keymap {
    pub modes: HashMap<ModeName, Mode>,
    pub menus: Menus,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Menus {
    pub pane: Vec<MenuItem>,
    pub tab: Vec<MenuItem>,
    pub global: Vec<MenuItem>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuItem {
    pub action: Action,
    #[serde(default)]
    label: Option<String>,
}

impl MenuItem {
    pub fn label(&self) -> Cow<'_, str> {
        self.label
            .as_deref()
            .map_or_else(|| self.action.label(), Cow::Borrowed)
    }
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
    /// Completed into a `chord label` hint with the action's fallback on load.
    #[serde(default)]
    label: Option<String>,
}

impl Mode {
    /// Each chord's `chord label` hint, in the file's order.
    pub fn hints(&self) -> impl Iterator<Item = &str> {
        self.keys
            .values()
            .filter(|binding| binding.action.is_some())
            .filter_map(|binding| binding.label.as_deref())
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
    pub fn label(&self) -> Cow<'static, str> {
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
            Self::Client(ClientAction::Sidebar(Sidebar::Resize { direction, .. })) => {
                direction.into()
            }
            Self::Client(ClientAction::Sidebar(action)) => action.into(),
            Self::Client(ClientAction::Config(action)) => action.into(),
            Self::Client(ClientAction::Agents(action)) => action.into(),
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
    Agents(ClientAgents),
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
    Resize {
        /// Left and right resize the width; up and down move the split
        /// between the sections.
        direction: Direction,
        #[serde(default)]
        amount: Option<u16>,
    },
}

#[derive(Clone, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[strum(serialize_all = "snake_case")]
pub enum ClientAgents {
    /// Select the next pane that needs the user, else the next agent.
    Next {},
}

#[derive(Clone, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[strum(serialize_all = "snake_case")]
pub enum ConfigAction {
    Reload {},
}

/// The client's part of the config file, already over the defaults except
/// for modes, which `over_defaults` merges by chord.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    /// Left to the server; named so a misspelled table is still an error.
    #[serde(rename = "server")]
    _server: IgnoredAny,
    client: Client,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Client {
    mouse: bool,
    sidebar_width: u16,
    menus: Menus,
    /// Only the file's; the defaults' join in `over_defaults`.
    #[serde(default)]
    modes: HashMap<ModeName, ModeFile>,
    agents: AgentsFile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentsFile {
    rows: Vec<Vec<Token>>,
    delay_ms: u64,
    popups: bool,
    sounds: Sounds<Sound>,
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
                let label = binding
                    .label
                    .take()
                    .map_or_else(|| action.label(), Cow::Owned);
                binding.label = Some(format!("{spelling} {label}"));
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
        let base = path.parent().unwrap_or(Path::new(""));
        Ok(Self::over_defaults(config::load(path)?, base))
    }

    /// `load` without reading sound files, so a bad one isn't reported.
    pub fn check(path: &Path) -> Result<()> {
        config::load::<File>(path).map(drop)
    }

    pub fn defaults() -> Self {
        Self::over_defaults(config::defaults(), Path::new(""))
    }

    /// The file's modes over the default modes, chord by chord. Sound paths
    /// resolve against `base`.
    fn over_defaults(file: File, base: &Path) -> Self {
        let Client {
            mouse,
            sidebar_width,
            menus,
            modes: file_modes,
            agents,
        } = file.client;
        let mut modes = config::defaults::<File>().client.modes;
        for (name, over) in file_modes {
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
            keymap: Keymap { modes, menus },
            sidebar_width,
            mouse,
            agents: AgentSettings {
                rows: agents.rows,
                delay: Duration::from_millis(agents.delay_ms),
                popups: agents.popups,
                sounds: agents.sounds.load(base),
            },
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
