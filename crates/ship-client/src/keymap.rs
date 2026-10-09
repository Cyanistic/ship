//! `[client]` in the config file: modes and their bindings.

use std::{
    collections::{BTreeMap, HashMap, hash_map::Entry},
    path::Path,
};

use crokey::KeyCombination;
use figment::{
    Figment, Metadata, Profile, Provider,
    providers::{Format, Toml},
    value::{Dict, Map, Tag, Value},
};
use serde::{Deserialize, de::IgnoredAny};
use ship_core::{
    command::{Command, Direction},
    prelude::*,
};

const DEFAULTS: &str = include_str!("keymap/defaults.toml");
/// The defaults' figment source name, which tells their bindings apart from
/// the file's after the merge.
const DEFAULTS_NAME: &str = "Ship's default keys";

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
    /// Only bound chords: `"none"` leaves the chord out.
    pub keys: HashMap<KeyCombination, Binding>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind {
    #[default]
    Sticky,
    Oneshot,
}

/// One action, or a list run in order that stops at the first failure.
#[derive(Clone, Debug)]
pub struct Binding(pub Vec<Action>);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Server(Command),
    Client(ClientAction),
    None,
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
    Select { row: Row },
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

/// 1 to 9.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(try_from = "u8")]
pub struct Row(pub u8);

impl TryFrom<u8> for Row {
    type Error = String;

    fn try_from(row: u8) -> std::result::Result<Self, String> {
        match row {
            1..=9 => Ok(Self(row)),
            _ => Err(format!("row {row} is not between 1 and 9")),
        }
    }
}

/// The file's shape down to each mode, which deserializes on its own so one
/// bad mode doesn't hide the others' errors. The root holds only `server`
/// (the server's part) and `client`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Root {
    #[serde(default)]
    #[expect(dead_code, reason = "accepted and left to the server")]
    server: IgnoredAny,
    #[serde(default)]
    #[expect(dead_code, reason = "checked here, read from figment's tree")]
    client: RawClient,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClient {
    #[serde(default)]
    #[expect(dead_code, reason = "read from figment's tree, which keeps tags")]
    modes: BTreeMap<String, IgnoredAny>,
}

/// Bindings are read from figment's tree instead, because deserializing a
/// `Value` drops the tags that say which source it came from.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMode {
    kind: Option<ModeKind>,
    #[serde(default)]
    clear_defaults: bool,
    #[serde(default)]
    #[expect(dead_code, reason = "read from figment's tree, which keeps tags")]
    keys: BTreeMap<String, IgnoredAny>,
}

/// The embedded defaults with every binding wrapped in a one-item list.
/// figment merges two tables key by key, so a file binding over a default
/// one would blend both actions; a list under a table is replaced whole.
struct Defaults;

impl Provider for Defaults {
    fn metadata(&self) -> Metadata {
        Metadata::named(DEFAULTS_NAME)
    }

    fn data(&self) -> std::result::Result<Map<Profile, Dict>, figment::Error> {
        let mut data = Toml::string(DEFAULTS).data()?;
        for root in data.values_mut() {
            let Some(Value::Dict(_, client)) = root.get_mut("client") else {
                continue;
            };
            let Some(Value::Dict(_, modes)) = client.get_mut("modes") else {
                continue;
            };
            for mode in modes.values_mut() {
                if let Value::Dict(_, mode) = mode
                    && let Some(Value::Dict(_, keys)) = mode.get_mut("keys")
                {
                    for binding in keys.values_mut() {
                        *binding = Value::Array(Tag::Default, vec![binding.clone()]);
                    }
                }
            }
        }
        Ok(data)
    }
}

/// A chord's binding and where it came from, while a mode's keys resolve.
struct Found {
    spelling: String,
    default: bool,
    binding: Option<Binding>,
}

impl Keymap {
    /// Defaults merged under the file; a missing file is the defaults. A
    /// file chord replaces the default on the same chord, however it's
    /// spelled. Each mode and binding deserializes on its own, so every
    /// error is collected, each naming the file and its key path. Used at
    /// startup, on reload and by `ship config check`, so check reports
    /// exactly what loading rejects.
    pub fn load(path: &Path) -> std::result::Result<Self, Vec<AppError>> {
        Self::build(&Figment::from(Defaults).merge(Toml::file(path)), path)
    }

    pub fn defaults() -> Self {
        match Self::build(&Figment::from(Defaults), Path::new(DEFAULTS_NAME)) {
            Ok(keymap) => keymap,
            Err(errors) => panic!("invalid default keys: {errors:?}"),
        }
    }

    fn build(figment: &Figment, path: &Path) -> std::result::Result<Self, Vec<AppError>> {
        figment
            .extract::<Root>()
            .map_err(|error| vec![located(path, &[], error)])?;
        let Ok(Value::Dict(_, defined)) = figment.find_value("client.modes") else {
            unreachable!("the defaults define modes");
        };
        let mut errors = Vec::new();
        let mut modes = HashMap::new();
        for (name, tree) in &defined {
            let at = ["client", "modes", name.as_str()];
            let raw: RawMode = match tree.deserialize() {
                Ok(raw) => raw,
                Err(error) => {
                    errors.push(located(path, &at, error));
                    continue;
                }
            };
            if name == ModeName::NORMAL && raw.kind.is_some() {
                errors.push(invalid(
                    path,
                    &[&at[..], &["kind"]].concat(),
                    "normal is always sticky",
                ));
            }
            let keys_at = [&at[..], &["keys"]].concat();
            let mut found: HashMap<KeyCombination, Found> = HashMap::new();
            for (spelling, value) in raw_keys(tree).into_iter().flatten() {
                let default = figment
                    .get_metadata(value.tag())
                    .is_some_and(|source| source.name == DEFAULTS_NAME);
                if default && raw.clear_defaults {
                    continue;
                }
                let spelling = spelling.clone();
                let at = [&keys_at[..], &[spelling.as_str()]].concat();
                let chord = match crokey::parse(&spelling) {
                    Ok(chord) => chord.normalized(),
                    Err(error) => {
                        errors.push(invalid(path, &at, error));
                        continue;
                    }
                };
                // A table is one action and a list is several. Dispatching
                // here keeps figment's key path in the error, which
                // trying one shape and then the other would lose.
                let binding = match value {
                    Value::Array(..) => value.deserialize(),
                    _ => value.deserialize().map(|action| vec![action]),
                }
                .map(Binding)
                .map_err(|error| located(path, &at, error))
                .and_then(|binding| {
                    binding
                        .check(&defined)
                        .map(|()| binding)
                        .map_err(|message| invalid(path, &at, message))
                })
                .map_err(|error| errors.push(error))
                .ok();
                let new = Found {
                    spelling,
                    default,
                    binding,
                };
                match found.entry(chord) {
                    Entry::Vacant(entry) => {
                        entry.insert(new);
                    }
                    Entry::Occupied(mut entry) => match (entry.get().default, new.default) {
                        (true, false) => {
                            entry.insert(new);
                        }
                        (false, true) => {}
                        _ => errors.push(invalid(
                            path,
                            &keys_at,
                            format!(
                                "\"{}\" and \"{}\" are the same chord",
                                entry.get().spelling,
                                new.spelling
                            ),
                        )),
                    },
                }
            }
            let keys = found
                .into_iter()
                .filter_map(|(chord, found)| Some((chord, found.binding?)))
                .filter(|(_, binding)| !binding.unbinds())
                .collect();
            modes.insert(
                ModeName(name.clone()),
                Mode {
                    kind: raw.kind.unwrap_or_default(),
                    keys,
                },
            );
        }
        match errors.is_empty() {
            true => Ok(Self { modes }),
            false => Err(errors),
        }
    }
}

impl Binding {
    /// Whether this is `"none"`, which leaves the chord to the program.
    fn unbinds(&self) -> bool {
        matches!(self.0.as_slice(), [Action::None])
    }

    /// What deserializing can't see: an empty list, `"none"` beside other
    /// actions, and modes the file doesn't define.
    fn check(&self, modes: &Dict) -> std::result::Result<(), String> {
        match self.0.as_slice() {
            [] => return Err("an empty list binds nothing; write \"none\" to unbind".into()),
            [_] => {}
            actions if actions.iter().any(|action| matches!(action, Action::None)) => {
                return Err("\"none\" can't be part of a list".into());
            }
            _ => {}
        }
        for action in &self.0 {
            if let Action::Client(ClientAction::Mode(ModeName(mode))) = action
                && !modes.contains_key(mode)
            {
                return Err(format!("no mode named \"{mode}\""));
            }
        }
        Ok(())
    }

    /// Whether running this enters a mode, so a one-shot mode doesn't leave
    /// for normal on its own.
    pub fn enters_mode(&self) -> bool {
        self.0
            .iter()
            .any(|action| matches!(action, Action::Client(ClientAction::Mode(_))))
    }
}

/// A mode's `keys` table in figment's tree.
fn raw_keys(mode: &Value) -> Option<&Dict> {
    match mode {
        Value::Dict(_, mode) => match mode.get("keys")? {
            Value::Dict(_, keys) => Some(keys),
            _ => None,
        },
        _ => None,
    }
}

/// `<file>: <key path>: <message>`. Figment's own message prefixes the key
/// with its `default.` profile, and a value deserialized on its own only
/// knows the path below `at`, so the location comes from both. Syntax errors
/// have no key path, and their message carries the line and column.
fn located(path: &Path, at: &[&str], error: figment::Error) -> AppError {
    let kind = error.kind.to_string();
    let at: Vec<&str> = at
        .iter()
        .copied()
        .chain(error.path.iter().map(String::as_str))
        .collect();
    invalid(path, &at, kind.trim_end())
}

fn invalid(path: &Path, at: &[&str], message: impl std::fmt::Display) -> AppError {
    match at.join(".") {
        key if key.is_empty() => err!(Configuration, "{}: {message}", path.display()),
        key => err!(Configuration, "{}: {key}: {message}", path.display()),
    }
}
