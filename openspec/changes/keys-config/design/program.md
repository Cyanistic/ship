# Keys and config program

Status: Locked on 2026-10-09. Re-approved in chat during slice 3 with product amendment A-4 (the `keymap.rs`, `keys.rs` and `ui/mod.rs` skeletons): bindings are lists of actions, the defaults go through a list-wrapping provider in place of the `Chords` provider, and the drive loop queues each key's actions. Cyan asked for the code first and the papers once it worked; see the slice 3 entry in the deviation log. Before that, Cyan approved it in chat after two Plannotator rounds ("oh alright. fair enough. alright... i think we're ready for openspec docs??"). The rounds dropped config warnings (product amendment A-2) and confirmed by probe that argument-free client actions are empty struct variants (U-1). Written from the locked [product](product.md) paper (including amendments A-1 and A-2) and the locked [architecture](architecture.md) paper, against the crates at 9a05ab8. Nothing here is created source, and no snippet has been compiled. Approval doesn't start implementation; Cyan requests that separately, slice by slice. Re-approved in chat after slice 1 with product amendment A-3 ("sounds good! go for it! i don't need to review your edits"): `TabTarget` gains `--pane`, `Scope::Cli` becomes a unit variant, and `commands::action` no longer reads `SHIP_PANE_ID`.

Needs Cyan's attention:

- **`ship server --starter` goes through `CreateTab`.** The startup-only `state::Starter` message is deleted, because a `CreateTab` with `starter: Some(Shell)` does the same thing. That leaves one way to make a tab with a shell instead of two. The architecture paper said "reuses the `state::Starter` path", and this keeps the behavior while dropping the duplicate.
- **The configured shell goes in through `SHELL`.** portable-pty's default program reads `SHELL` from the command builder and still starts it as a login shell (`-zsh`), so the server sets `SHELL` on the builder and leaves the rest alone. Configuring the shell doesn't need a code path of its own.
- **`CreateTab` on the wire changes shape.** `parent` becomes `at`, the same `MoveTab` that move takes, so the body reads `{"at": {"after": "tab:…"}, "starter": "shell"}`. `PROTOCOL_VERSION` goes to 6.
- **Superseded by A-3:** clap is now the only reader of `SHIP_PANE_ID`, through `--pane` on both targets. The original note follows. **`SHIP_PANE_ID` is read in two places.** clap reads it to fill a missing `--pane`, as the architecture says, and the binary reads it again to build `Scope::Cli`, which the `--tab` default needs. The alternative is a hidden global clap argument, which clashes with `--pane`.
- **U-1, resolved by probe:** figment rejects `{}` for a unit variant ("invalid type: found map, expected unit"), while the `toml` crate accepts it. Empty struct variants (`Next {}`) and structs with all-default fields both accept `{}` under figment, through the whole value and through one `Value` per binding. So client actions with no arguments are empty struct variants, on purpose (see the comment on `ClientAction`). Figment errors name the key path with a `default.` profile prefix (`default.keys.a.client.tab.next.x`), so `Keymap::load` builds the location from figment's key path instead of its message.

## Rationale

The skeleton follows the architecture's crate graph (shape B) one file per box:

- **The command tree (decisions 2 to 4)** is a new `ship-core/src/command.rs`. Each `server.` action is one struct in nested `tab` and `pane` modules, so the Rust path matches the config path (`command::tab::Close` is `server.tab.close`). Serde derives are always on, and clap derives sit behind the new `clap` feature. `cli.rs` loses its argument structs and includes the tree directly.
- **`execute` (decision 3)** is a new `ship-client/src/execute.rs`. It takes `(Command, Scope)` and calls the existing `api.rs` wrappers. It replaces the `tab()` and `pane()` dispatchers in `ship/src/commands.rs`, and `current_dir()` moves with it.
- **Placement and starter (decisions 6 and 7)** extend `protocol::CreateTab` and its state handler, which now uses the existing `tree::place`.
- **Server settings (decisions 1 and 12)** are a new `ship-server/src/settings.rs`. `PaneEnv` carries the config path, and `pane::spawn` reads the shell when a pane has no command.
- **The keymap (decisions 5, 8 and 9)** is a new `ship-client/src/keymap.rs` with an embedded `keymap/defaults.toml`. `ui/keys.rs` becomes the mode machine over a `Keymap`.
- **Watching (decision 11)** is a new `ship-client/src/watch.rs` that feeds the `drive` loop.
- **The command queue (decision 10)** is one more `Pending` slot and a `VecDeque` in `drive`.

Estimated change: implementation Rust grows by about 350 to 550 lines from today's roughly 5,070. The keymap and loader account for about 300, the command tree about 200, execute about 150 and watching about 50, offset by about 200 deleted from `cli.rs`, `commands.rs` and `keys.rs`. That's an estimate from the declarations below. The docs slice reports the measured number. Tests stay at zero.

## Skeleton map

### Proposed settings

Workspace `Cargo.toml`, with versions pinned to the current release at implementation time, checked with `cargo search`. These aren't guessed here.

```toml
[workspace.dependencies]
crokey = "1.5"                                    # crossterm 0.29, matches the workspace
etcetera = "…"
figment = { version = "0.10", features = ["toml"] }
notify-debouncer-mini = "…"
```

Per crate:

```toml
# crates/ship-core/Cargo.toml
[features]
clap = ["dep:clap"]
[dependencies]
clap = { workspace = true, optional = true }

# crates/ship-client/Cargo.toml
crokey.workspace = true
figment.workspace = true
notify-debouncer-mini.workspace = true

# crates/ship-server/Cargo.toml
figment.workspace = true

# crates/ship/Cargo.toml
etcetera.workspace = true
ship-core = { workspace = true, features = ["clap"] }
```

### Added

#### `crates/ship-core/src/command.rs` (slice 1)

```rust
//! Every `server.` action: the config's `server.*` bindings, the `ship tab`
//! and `ship pane` subcommands, and the input to `ship_client::execute`.

#[cfg(feature = "clap")]
use clap::{Args, Subcommand};
use serde::Deserialize;
use crate::{id::IdOf, model::{Pane, Tab}, protocol::{MoveTab, Starter}};

#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "clap", derive(Subcommand))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    /// List, create, inspect, rename, close or move tabs
    #[cfg_attr(feature = "clap", command(subcommand))]
    Tab(tab::TabCommand),
    /// Create, inspect, rename, close or resize panes
    #[cfg_attr(feature = "clap", command(subcommand))]
    Pane(pane::PaneCommand),
}

/// `--pane ID`; a missing ID comes from `SHIP_PANE_ID` on the CLI and the
/// selection on keys. `{}` in the config.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args))]
#[serde(transparent)]
pub struct PaneTarget {
    #[cfg_attr(feature = "clap", arg(long = "pane", env = "SHIP_PANE_ID", value_name = "ID"))]
    pub id: Option<IdOf<Pane>>,
}

/// `--tab ID`, or `--pane ID` for the tab holding it; `--tab` wins (A-3).
/// A missing target is the tab holding the current pane on the CLI, or the
/// selected tab on keys. The config writes one ID, `"tab:…"` or `"pane:…"`.
/// Not a clap group: clap counts an env-filled `--pane` as given.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args))]
#[serde(from = "Option<NodeId>")]
pub struct TabTarget {
    #[cfg_attr(feature = "clap", arg(long = "tab", value_name = "ID"))]
    pub id: Option<IdOf<Tab>>,
    #[cfg_attr(feature = "clap", arg(long = "pane", env = "SHIP_PANE_ID", value_name = "ID"))]
    pub pane: Option<IdOf<Pane>>,
}

/// At most one. Moved here from `ship/src/cli.rs`, now with serde.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args), group(multiple = false))]
#[serde(deny_unknown_fields)]
pub struct Destination {
    pub parent: Option<IdOf<Tab>>,
    #[cfg_attr(feature = "clap", arg(long))]
    pub before: Option<IdOf<Tab>>,
    #[cfg_attr(feature = "clap", arg(long))]
    pub after: Option<IdOf<Tab>>,
}

impl Destination {
    /// `None` when empty, so the caller picks the default; an error when the
    /// config sets more than one (clap's group already stops that on the CLI).
    pub fn resolve(self) -> Result<Option<MoveTab>>;
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum Direction { Left, Down, Up, Right }

pub mod tab {
    #[derive(Clone, Debug, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Subcommand))]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub enum TabCommand {
        /// List top-level tabs and their descendants as JSON, keyed by ID
        #[serde(skip)]
        List,
        /// Create a tab, at the top level or where given, and print it as JSON
        Create(Create),
        /// Print a tab and its descendants as JSON
        #[serde(skip)]
        Get(Get),
        /// Rename a tab and print it as JSON
        Rename(Rename),
        /// Close a tab and everything in it
        Close(Close),
        /// Append under PARENT or to the top level, or place before or after a sibling
        Move(Move),
    }

    #[serde(default, deny_unknown_fields)]
    pub struct Create {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub to: Destination,
        /// Tab name; omitted or blank means none
        #[cfg_attr(feature = "clap", arg(long))]
        pub name: Option<String>,
        /// Open the tab with one pane running the shell
        #[cfg_attr(feature = "clap", arg(long, num_args = 0..=1, default_missing_value = "shell"))]
        pub starter: Option<Starter>,
    }
    pub struct Get { #[cfg_attr(feature = "clap", command(flatten))] pub tab: TabTarget }
    pub struct Rename {
        #[cfg_attr(feature = "clap", command(flatten))] pub tab: TabTarget,
        /// New name; omitted or blank clears it
        pub name: Option<String>,
    }
    pub struct Close { #[cfg_attr(feature = "clap", command(flatten))] pub tab: TabTarget }
    pub struct Move {
        #[cfg_attr(feature = "clap", command(flatten))] pub tab: TabTarget,
        #[cfg_attr(feature = "clap", command(flatten))] pub to: Destination,
    }
}

pub mod pane {
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub enum PaneCommand {
        /// Create a pane running a program and print it as JSON
        Create(Create),
        #[serde(skip)]
        Get(Get),
        Rename(Rename),
        /// Close a pane
        Close(Close),
        /// Resize a pane (not available yet)
        Resize(Resize),
    }

    #[serde(default, deny_unknown_fields)]
    pub struct Create {
        #[cfg_attr(feature = "clap", command(flatten))] pub tab: TabTarget,
        /// Ignored until layout lands (FR-019)
        #[cfg_attr(feature = "clap", arg(long))] pub direction: Option<Split>,
        #[cfg_attr(feature = "clap", arg(long))] pub name: Option<String>,
        /// Starting directory; defaults to the current directory on the CLI
        #[cfg_attr(feature = "clap", arg(long))] pub cwd: Option<PathBuf>,
        /// Command and arguments; defaults to the configured or login shell
        #[cfg_attr(feature = "clap", arg(last = true, value_name = "COMMAND"))]
        pub command: Vec<String>,
    }
    #[serde(rename_all = "snake_case")]
    pub enum Split { Right, Down }
    pub struct Get { pub pane: PaneTarget }
    pub struct Rename { pub pane: PaneTarget, pub name: Option<String> }
    pub struct Close { pub pane: PaneTarget }
    pub struct Resize {
        pub pane: PaneTarget,
        pub direction: Direction,
        /// Cells; absent means one step
        pub amount: Option<u16>,
    }
}
```

Every struct derives `Clone, Debug, Deserialize` (no `Serialize`: nothing serializes a command) and, behind the feature, `Args`. The `#[serde(default, deny_unknown_fields)]` shown on `Create` applies to every struct, so `{}` works wherever every field is optional.

#### `crates/ship-client/src/execute.rs` (slice 1)

```rust
//! One path from a command to the API, for the CLI and for keys.

use ship_core::{command::{Command, PaneTarget, TabTarget}, model::{NodeId, Pane, Tab}, tree::Tabs};

/// Where a missing target comes from.
pub enum Scope {
    /// The CLI; clap already filled `--pane` from `SHIP_PANE_ID` (A-3).
    /// CLI-made panes default to the process's current directory.
    Cli,
    /// A key binding, resolved against the client's view.
    Keys(KeyScope),
}

/// The client's selection and the tabs it was made in. Key-made tabs go
/// after the selected tab (FR-021); key-made panes start in the server's
/// home directory (architecture R-3).
pub struct KeyScope { pub selection: Option<NodeId>, pub tabs: Tabs }

/// What a command produced. Serializes as the resource itself; `Closed`
/// prints nothing (FR-028).
#[derive(Serialize)]
#[serde(untagged)]
pub enum Outcome { Tabs(Tabs), Tab(Tab), Pane(Pane), Closed }

impl Client {
    /// Resolves targets (explicit ID, then scope, then FR-026's error), calls
    /// `api.rs`, and returns `Unavailable` ("not available yet") for
    /// `pane resize`.
    pub async fn execute(&self, command: Command, scope: &Scope) -> Result<Outcome>;

    /// `--tab`, else `tree::pane_owner` of `--pane` (over `GET /tabs` on the
    /// CLI), else the selected tab under keys.
    async fn tab_target(&self, target: TabTarget, scope: &Scope) -> Result<IdOf<Tab>>;
    fn pane_target(target: PaneTarget, scope: &Scope) -> Result<IdOf<Pane>>;
}

/// Moved from `ship/src/commands.rs`, unchanged.
pub fn current_dir() -> Result<PathBuf>;
```

#### `crates/ship-server/src/settings.rs` (slice 2)

```rust
//! `[server]` in the config file. Read whenever a shell starts, never cached.

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSettings {
    /// Absent: the login shell.
    pub shell: Option<PathBuf>,
}

impl ServerSettings {
    /// `server` from the file through figment. A missing file is the
    /// defaults; a relative `shell` resolves from the file's folder (FR-005).
    /// Also what `ship config check` runs. `[server]` has one field, so its
    /// first error is the only one.
    pub fn load(path: &Path) -> Result<Self>;
}
```

#### `crates/ship-client/src/keymap.rs` (slice 3)

```rust
//! `[client]` in the config file: modes and their bindings.

use crokey::KeyCombination;
use figment::{Figment, Provider, providers::{Format, Toml}, value::{Dict, Value}};
use ship_core::command::{Command, Direction};

const DEFAULTS: &str = include_str!("keymap/defaults.toml");
/// The defaults' figment source name, which tells their bindings apart from
/// the file's after the merge.
const DEFAULTS_NAME: &str = "Ship's default keys";

pub struct Keymap { pub modes: HashMap<ModeName, Mode> }

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct ModeName(pub String);
impl ModeName { pub const NORMAL: &str = "normal"; pub fn normal() -> Self; }

/// Only bound chords: `"none"` leaves the chord out.
pub struct Mode { pub kind: ModeKind, pub keys: HashMap<KeyCombination, Binding> }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind { #[default] Sticky, Oneshot }

/// One action, or a list run in order that stops at the first failure (A-4).
#[derive(Clone, Debug)]
pub struct Binding(pub Vec<Action>);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Action { Server(Command), Client(ClientAction), None }

/// Actions without arguments are empty struct variants (`Detach {}`), not
/// unit variants: the config writes them as `{}`, which figment accepts for a
/// struct variant and rejects for a unit one (probe, U-1).
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
pub enum ClientTab { Next {}, Prev {}, Select { row: Row }, Expand {}, Collapse {} }
pub enum ClientPane { Next {}, Prev {}, Focus { direction: Direction } }
pub enum Sidebar { Toggle {} }
pub enum ConfigAction { Reload {} }
/// 1 to 9, through `serde(try_from = "u8")`.
pub struct Row(pub u8);

/// The top level, checked once: only `server` (left to the server) and
/// `client`. Modes and bindings are then read from figment's tree, because
/// deserializing a `Value` drops the tags that say which source it came from.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Root { #[serde(default)] server: IgnoredAny, #[serde(default)] client: RawClient }
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClient { #[serde(default)] modes: BTreeMap<String, IgnoredAny> }
/// Each mode deserializes on its own, so one bad mode doesn't hide the rest.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMode {
    kind: Option<ModeKind>,
    #[serde(default)] clear_defaults: bool,
    #[serde(default)] keys: BTreeMap<String, IgnoredAny>,
}

/// The embedded defaults with every binding wrapped in a one-item list, so
/// figment's merge replaces a default whole instead of blending a file table
/// into it (architecture decision 8).
struct Defaults;
impl Provider for Defaults;

/// A chord's binding and where it came from, while a mode's keys resolve.
struct Found { spelling: String, default: bool, binding: Option<Binding> }

impl Keymap {
    /// `Figment::from(Defaults).merge(Toml::file(path))`, then per mode:
    /// parse each chord with crokey, drop default-tagged bindings under
    /// `clear_defaults`, let a file chord replace a default on the same parsed
    /// chord, and report two file spellings of one chord (FR-010). A table is
    /// one action and a list several, dispatched on the value so figment's
    /// key path stays in the error. Every error is collected, each as
    /// `<file>: <key path>: <message>`. Used at startup, on reload and by
    /// `ship config check`, so check reports exactly what loading rejects.
    pub fn load(path: &Path) -> Result<Self, Vec<AppError>>;
    /// The defaults alone; panics if they don't load.
    pub fn defaults() -> Self;
}

impl Binding {
    /// An empty list, `"none"` inside a list, a `client.mode` naming an
    /// undefined mode.
    fn check(&self, modes: &Dict) -> Result<(), String>;
    /// So a one-shot mode doesn't leave for normal on its own.
    pub fn enters_mode(&self) -> bool;
}
```

#### `crates/ship-client/src/keymap/defaults.toml` (slice 3)

The defaults block from the product paper's Interface, verbatim from `[client.modes.normal.keys]` down. `[server]` is left out, because the server's default is "no setting". `alt-1` through `alt-9` are written out.

#### `crates/ship-client/src/watch.rs` (slice 4)

```rust
//! Reload signals for the config file (architecture decision 11).

/// Watches the file's directory, and the symlink target's directory when
/// they differ, non-recursively, debounced. Yields once per change to the
/// file's name. Dropping the stream stops the watcher.
pub fn changes(path: &Path) -> Result<impl Stream<Item = ()>>;
```

### Moved

- `current_dir()`: from `crates/ship/src/commands.rs` to `crates/ship-client/src/execute.rs` (slice 1).
- `Destination` and its `From<Destination> for MoveTab`: from `crates/ship/src/cli.rs` to `crates/ship-core/src/command.rs`, where the `From` becomes `Destination::resolve` (slice 1).

### Replaced

#### `crates/ship-core/src/protocol.rs` (slice 1)

```rust
pub const PROTOCOL_VERSION: u32 = 6;              // was 5: CreateTab's shape

/// POST /tabs body. `at` defaults to the end of the top level.
pub struct CreateTab {
    #[serde(default = "MoveTab::top")]
    pub at: MoveTab,                              // was `parent: Option<IdOf<Tab>>`
    #[serde(default)]
    pub name: OptionalName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter: Option<Starter>,
}

/// What a new tab starts with.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum Starter { Shell }

impl MoveTab { pub fn top() -> Self; }            // Parent(None)
```

#### `crates/ship-core/src/lib.rs` (slice 1)

Adds `pub mod command;`.

#### `crates/ship-server/src/state.rs` (slice 1)

- `Message<CreateTab>`: builds the tab, starting one shell pane first when `starter` is `Some(Shell)` (as `Starter` did), then inserts it with `tree::place(tabs, tab, create.at)` in one commit. A sibling or parent that doesn't exist is `NotFound`, as in move.
- `pub struct Starter` and its `Message` impl are deleted.

#### `crates/ship-server/src/lib.rs` (slices 1 and 2)

```rust
pub async fn serve(
    address: SocketAddr,
    starter: bool,
    config: PathBuf,                              // slice 2
    shutdown: impl Future<Output = Result<()>> + Send + 'static,
) -> Result<()>;
// starter: state.ask(CreateTab { at: MoveTab::top(), name: default, starter: Some(Starter::Shell) })  (slice 1)
```

`mod settings;` is added in slice 2.

#### `crates/ship-server/src/pane.rs` (slice 2)

```rust
pub(crate) struct PaneEnv {
    server_url: String,
    bin: PathBuf,
    config: PathBuf,                              // new
}
impl PaneEnv { pub fn new(address: SocketAddr, config: PathBuf) -> Result<Self>; }
```

In `spawn`, when `command` is `None`: `ServerSettings::load(&env.config)`. On `Ok` with a shell, it calls `builder.env("SHELL", shell)` on the `new_default_prog` builder, so portable-pty still starts it as a login shell. On `Err`, it logs with `tracing::warn!` and leaves `SHELL` alone (FR-008).

#### `crates/ship-server/src/routes.rs` (slice 1)

The `create_tab` utoipa docs change from "Parent not found" to "Parent or sibling not found". The handler itself is unchanged.

#### `crates/ship-client/src/api.rs` (slice 1)

```rust
pub async fn create_tab(&self, body: &CreateTab) -> Result<Tab>;   // was (parent, name)
```

The other wrappers don't change.

#### `crates/ship-client/src/lib.rs` (slices 1, 3 and 4)

Adds `mod execute;` and `pub use execute::{KeyScope, Outcome, Scope, current_dir};` in slice 1, `pub mod keymap;` in slice 3, and `mod watch;` in slice 4.

#### `crates/ship-client/src/ui/keys.rs` (slice 3)

Rewritten as the mode machine (architecture decision 9). The `C-b` prefix, `is_prefix` and `bound` go away.

```rust
pub(super) enum Handled {
    /// A key or paste for the selected pane; dropped while disconnected.
    Frame(InputFrame),
    /// A bound key's actions, which the loop runs in order.
    Run(Binding),
    /// An unbound key outside normal, or input with no pane selected.
    None,
}

pub(super) struct Keys {
    pub keymap: Keymap,
    pub active: ModeName,
}

impl Keys {
    pub fn new(keymap: Keymap) -> Self;
    /// `KeyCombination::from(key)`, looked up in the active mode. Unbound:
    /// a frame in normal, dropped in any other mode. A one-shot mode returns
    /// to normal after any key unless the binding enters a mode itself.
    pub fn handle(&mut self, event: Event, selected: Option<IdOf<Pane>>) -> Handled;
    /// `client.mode`, run by the loop in its turn.
    pub fn enter(&mut self, mode: ModeName);
    /// Swap in a reloaded keymap; an active mode it no longer defines falls
    /// back to normal.
    pub fn replace(&mut self, keymap: Keymap);
    /// The active mode's name, outside normal.
    pub fn shown_mode(&self) -> Option<&ModeName>;
}
```

`client.send` turns its `KeyCombination` back into a `KeyEvent` for the frame with crokey's `Into<KeyEvent>` (first code, its modifiers, `Press`).

#### `crates/ship-client/src/ui/mod.rs` (slices 3 and 4)

```rust
pub async fn run(client: &Client, open_first: bool, config: &Path) -> Result<()>;   // was (client, open_first)
```

Inside `drive`:

- `Keys::new(Keymap::load(config))`. On `Err`, it runs on `Keymap::defaults()` and puts the first error, and how many more, on the status line (FR-007).
- `queued: VecDeque<VecDeque<Action>>`, one entry per bound key, and `running: Pending<Option<NodeId>>`. After every event the loop runs queued actions in order until it reaches a `server.` action, which goes in the slot with `Scope::Keys(KeyScope { selection, tabs })` taken from the observer; the actions after it wait for it (A-4). A created tab or pane becomes `chosen`, so the next view PUT selects it, and a tab with a starter pane selects that pane (FR-021, FR-029). A failure, from the server or a client action, goes to the status line and drops the rest of that key's actions.
- `Action::Client`: `tab.next`/`prev` and `pane.next`/`prev` go through the existing `navigate`. `mode` calls `Keys::enter`. `detach` exits. `config.reload` reloads. `send` becomes a frame for the selected pane, or "no pane selected". The layout actions (`tab.select`, `tab.expand`, `tab.collapse`, `pane.focus` and `sidebar.toggle`) put "not available yet" on the status line.
- Slice 4: `watch::changes(config)` becomes one more `select!` arm, the same as `config.reload`. A failed reload keeps the running keymap and shows the error.
- The doc comment loses its `C-b )` reference.

`navigate` takes a small `Step { Tab, Pane } × forward` instead of matching on the old `Action` variants.

#### `crates/ship-client/src/ui/draw.rs` (slice 3)

```rust
/// What the status line shows besides the selection.
pub(super) struct Status<'a> { pub mode: Option<&'a ModeName>, pub message: Option<&'a str> }
pub(super) fn draw(frame: &mut Frame, observer: &Observer, status: &Status);
```

The mode name is shown outside `normal` (FR-017), along with the last error or "not available yet" message. The empty-selection hint `C-b ) to open one` becomes `alt-right to open one`. It's hard-coded to the default; showing whatever the keymap actually binds is left for later.

#### `crates/ship/src/cli.rs` (slices 1 to 3)

`TabCommand`, `PaneCommand`, `IdArgs`, `RenameArgs`, `CreateTabArgs`, `CreatePaneArgs`, `MoveTabArgs` and `Destination` are deleted.

```rust
pub struct Cli {
    pub server_url: String,
    /// Config file; defaults to the platform config directory's ship/config.toml
    #[arg(long, global = true, env = "SHIP_CONFIG", value_name = "FILE")]
    pub config: Option<PathBuf>,                  // slice 2
    #[command(subcommand)]
    pub command: Option<Command>,
}

pub enum Command {
    Server(ServerArgs),
    #[command(flatten)]
    Action(ship_core::command::Command),          // `ship tab …`, `ship pane …` (slice 1)
    /// Check the config file
    #[command(subcommand)]
    Config(ConfigCommand),                        // slice 2
}

pub enum ConfigCommand {
    /// Report every error in a config file and exit non-zero if any
    Check { file: Option<PathBuf> },
}

impl Cli {
    /// `--config`, `SHIP_CONFIG`, then etcetera's config dir + ship/config.toml.
    pub fn config_path(&self) -> Result<PathBuf>;   // slice 2
}
```

`long_about` changes `C-b d detaches` to `alt-q detaches` (slice 3).

#### `crates/ship/src/commands.rs` (slices 1 and 2)

```rust
/// `ship tab …` and `ship pane …`: execute under `Scope::Cli`, then print
/// the outcome.
pub async fn action(client: &Client, command: Command) -> Result<()>;   // replaces tab() and pane()
/// `ship config check`: runs `ServerSettings::load` and `Keymap::load`, prints
/// every error with its location on stderr, exits non-zero on any. No warnings.
pub fn check_config(path: &Path) -> Result<()>;                          // slice 2 server, slice 3 client
```

`stop_server` and `print` stay. `current_dir` moves out.

#### `crates/ship/src/main.rs` (slices 1 and 2)

`dispatch` routes `Command::Action` to `commands::action` (slice 1). It passes `cli.config_path()?` to `serve`, `ui::run` and `local::default_health`, and routes `Command::Config` (slice 2).

#### `crates/ship/src/local.rs` (slice 2)

`default_health(client, config: &Path)` forwards `--config <path>` to the spawned `ship server … --background-child`.

#### `README.md` (slice 5)

The `C-b` keys become the Alt defaults, `rm` becomes `close`, the README shows `--tab`/`--pane` and adds the config file section, and chord spelling links to crokey.

### Untouched

- `ship-core` `id.rs`, `model.rs`, `tree.rs`, `screen.rs`, `relay.rs` and `error.rs`. `tree::pane_owner` already finds a pane's tab, and `ErrorCode::Unavailable` already exists.
- `ship-server` `attach.rs`, `input.rs`, `health.rs`, `app.rs` and `state/tree.rs`. `tree::place` already handles all three destinations.
- `ship-client` `ui/observer.rs` and `ui/terminal.rs`.
- `ship-macros`, `vendor/ratatui-ghostty`.
- `openspec/specs/*`. The change's spec deltas are written when it's archived, not in a slice.

## Build order

1. **One command tree for the CLI.**
   - Files: `ship-core` `command.rs`, `lib.rs` and `protocol.rs`; `ship-core/Cargo.toml` (the `clap` feature); `ship-server` `state.rs`, `lib.rs` (starter) and `routes.rs`; `ship-client` `execute.rs`, `api.rs` and `lib.rs`; `ship` `cli.rs`, `commands.rs`, `main.rs` and `Cargo.toml`.
   - Delivers:
     - `ship tab close`, `ship pane close`, `--tab`/`--pane` with the in-pane defaults and the outside-a-pane error (FR-024 to FR-028).
     - `ship tab create --starter` and `--before`/`--after` (FR-029, decision 6).
     - `ship pane resize` reporting "not available yet".
     - `ship server --starter` seeding through `CreateTab`.
   - Checks:
     - Build, fmt and Clippy, with and without the `clap` feature on `ship-core`.
     - Inside a Ship pane, `ship pane close` closes that pane, and `ship tab close` closes its tab.
     - Outside a pane, `ship pane close` fails with FR-026's message.
     - `ship tab create --starter --after <id>` puts a tab with a shell after `<id>` in `ship tab list`.
     - `ship tab rm` is gone, and `ship --help` lists `close`.

2. **Config path and server settings.**
   - Files: `ship` `cli.rs` (`--config`, `config_path`, `config check`), `commands.rs`, `main.rs` and `local.rs`; `ship-server` `settings.rs`, `pane.rs` and `lib.rs`; workspace and crate manifests (figment, etcetera).
   - Delivers:
     - FR-001, with `--config` forwarded to the background server.
     - FR-005 and FR-008: the configured shell, applied on the next pane, with a bad file falling back to the login shell plus a log line.
     - `ship config check` for `[server]`.
   - Checks:
     - `shell = "/bin/sh"` in the file, then `ship pane create` without a command runs `/bin/sh` as a login shell, with `-sh` in `ps`.
     - Breaking the file and creating a pane gives the login shell and a warning in the server log.
     - `ship config check` on a bad `shell` value names `server.shell` and exits 1.
     - `SHIP_CONFIG=/elsewhere ship` starts a server that reads `/elsewhere`.

3. **Keymap and modes.**
   - Files: `ship-client` `keymap.rs`, `keymap/defaults.toml`, `ui/keys.rs`, `ui/mod.rs`, `ui/draw.rs` and `lib.rs`; `ship` `commands.rs` (client check) and `cli.rs` (`long_about`); manifests (crokey, figment in ship-client).
   - Delivers:
     - The P1 stories with no config file (SC-001).
     - FR-002, FR-004, FR-007 (startup half) and FR-009 to FR-023.
     - User modes, sticky and one-shot.
     - `"none"`, `clear_defaults`, and duplicate-chord, undefined-mode and unknown top-level table errors in `ship config check` (SC-003). No warnings.
   - Checks:
     - With no config, each day-one default chord does what the Interface says, verified with `ship tab list`.
     - `ctrl-b` reaches `cat -v` in a pane.
     - A sticky user mode swallows unbound keys and shows its name.
     - A one-shot mode exits on an unbound key.
     - `ship config check` on files with a typo'd action, field and mode, a `[srever]` table, and `"alt-n"` plus `"Alt-N"`, reports each with its key path.
     - A binding and its CLI command give the same result (SC-004).

4. **Reload on save.**
   - Files: `ship-client` `watch.rs`, `ui/mod.rs` and `lib.rs`; the manifest (notify-debouncer-mini).
   - Delivers: FR-006 and FR-007's reload half (SC-002).
   - Checks, each verified on macOS and reported unverified on Linux unless run there:
     - With a client attached, adding `"alt-y" = { server.tab.create = {} }` and saving makes `alt-y` work.
     - The same works when the file is saved with vim's rename-save, and when it's a symlink into a dotfiles directory.
     - A broken save keeps the old keys and shows the error.
     - `client.config.reload` bound to a key reloads.

5. **Docs and size.**
   - Files: `README.md`.
   - Delivers: the README matches the new keys, commands and config file, and links to crokey for chord spelling.
   - Checks:
     - `grep -rn 'C-b\|tab rm\|pane rm' README.md crates` finds nothing.
     - Implementation Rust and test line counts are reported separately against the estimate.

## Deviation log

### Slice 1: one command tree (2026-10-09, macOS)

All slice 1 checks passed on macOS. Linux and Windows are unverified. No locked paper reopened. Implementation Rust went from 5,244 to 5,532 lines (+288), with tests at zero; slices 3 and 4 still have to delete the `C-b` keys.

Small departures from the skeleton, none changing behavior the papers specify:

- **`pane resize` takes `--direction` and `--amount` as flags** (FR-024 says "as flags"). The skeleton left their clap attributes out.
- **`Resize` has no struct-level `#[serde(default)]`**, because `direction` is required. Its optional fields default one by one, so "`{}` works wherever every field is optional" still holds.
- **The `Command` enums don't carry `deny_unknown_fields`.** An externally tagged enum already rejects unknown variants.
- **Superseded by A-3:** a malformed `SHIP_PANE_ID` used to fail every `ship tab` and `ship pane` command, `tab list` included, because `commands::action` parsed it up front. See the A-3 entry below.

One surprise, already true before this change: with an explicit `--server-url`, `ship tab …` and `ship pane …` skip the health check, so a protocol-5 `ship tab list` still talks to a protocol-6 server. The protocol check therefore ran through `server status` and the bare client, which do check health.

Evidence, each run against a foreground `ship server --port <p>` with `--server-url`:

- `ship --help` lists `tab` and `pane` with `close`, no `rm`. `ship tab rm` exits 2 with "unrecognized subcommand 'rm'".
- `curl POST /api/v0/tabs` with `{"at":{"after":"<a>"},"starter":"shell"}` returned 201 with one `/bin/zsh` pane, and `ship tab list` showed it right after `a`. `{}` appended an empty tab at the end. A missing sibling with a starter returned 404, and the tab count stayed at 4.
- `ship server --starter` listed one tab with one `/bin/zsh` pane.
- A pane running `"$SHIP_BIN" pane close` disappeared while its sibling `sleep 100` pane stayed. A pane running `"$SHIP_BIN" tab close` took its tab with it.
- Outside a pane (`env -u SHIP_PANE_ID`), `ship pane close` printed "pass --pane or run inside a pane" and `ship tab close` printed "pass --tab or run inside a pane", each exiting 1.
- `ship tab create --starter --after <first> --name started` printed a tab with one `/bin/zsh` pane, listed between `first` and `last`.
- `ship pane create --tab <t> --cwd /var -- sh -c 'sleep 100'` printed `cwd: "/var"`, running.
- `ship pane resize --pane <p> --direction left` printed "pane resize is not available yet" and exited 1.
- `ship tab move --tab <id> <parent> --before <sib>` exited 2: "the argument '[PARENT]' cannot be used with '--before <ID>'".
- The pre-change binary (protocol 5) running `server status` and the bare client both reported "incompatible health response … expected service ship and protocol 5" and exited 1.
- A disposable consumer in the scratchpad (build output in ignored `target/keys-config-proof`) printed `CreateTab` with `at` (`$ref MoveTab`), `name` and `starter` (`Starter`, enum `["shell"]`), and the create route's 404 as "Parent or sibling not found".
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, Clippy on `-p ship-core` with and without `--features clap`, debug builds of `ship-core` with and without `clap`, and the workspace debug and release builds all passed.

### A-3 follow-up: `--pane` on tab targets (2026-10-09, macOS)

Product amendment A-3 landed on top of slice 1. `TabTarget` gained an env-backed `--pane`, `Scope::Cli` became a unit variant, and `commands::action` stopped reading `SHIP_PANE_ID`, so clap is its only reader. Implementation Rust is now 5,569 lines (+37 over slice 1), with tests at zero. Linux and Windows are unverified.

Two clap probes outside the repo (clap 4.6.7) decided the shape. With `group(multiple = false)`, `PROBE_PANE=p5 probe --tab t3` exited 2: "the argument '--tab <ID>' cannot be used with '--pane <PANE>'". `overrides_with = "pane"` gave the same error for an env-filled value. With plain fields, both parse, so `tab_target` lets `--tab` win. In the config, `TabTarget` writes as one `NodeId` through `serde(from, into)`. Making it a nested table would have broken the `server.tab.close.tab = "…"` spelling, and `flatten` doesn't work with `deny_unknown_fields`.

What's left of the malformed-ID case: `SHIP_PANE_ID=garbage` no longer affects `tab list` or `tab create`. Any command with a `--pane` field still exits 2 on it ("invalid value 'garbage' for '--pane <ID>'"), even when `--tab` is given, because clap parses the env value before the command runs.

Evidence, against a foreground server with `--server-url`:

- Inside a pane, `"$SHIP_BIN" pane close` and `"$SHIP_BIN" tab close` still close that pane and its tab. `"$SHIP_BIN" tab rename --tab <other> renamed` renamed `<other>` and left the pane's own tab alone.
- Outside a pane, `ship tab close --pane <p>` closed the tab holding `<p>` and exited 0. `ship tab rename --tab <o> --pane <p> both` renamed `<o>`. `ship pane create --pane <p>` added the pane to `<p>`'s tab.
- Outside a pane without flags, `ship tab close` printed "pass --tab or --pane, or run inside a pane", and `ship pane close` printed "pass --pane or run inside a pane", each exiting 1.
- A serde probe on `Command` accepted `{"tab":{"close":{}}}`, `"tab": "tab:…"` and `"tab": "pane:…"` (the last becoming `pane: Some(…)`), and `pane.create.tab = "pane:…"`. It rejected `"tab": "nope"` (no `NodeId` variant) and `"pane": "x"` ("unknown field `pane`, expected `tab`").
- The slice 1 workflow script passed again: `--starter --after`, `--cwd /var`, the resize message, and the exit 2 for parent with `--before`.
- fmt, workspace Clippy, Clippy and debug builds on `ship-core` with and without `clap`, and the workspace debug and release builds passed.

### Review follow-up: `KeyScope` and no `Serialize` on commands (2026-10-09, macOS)

Cyan's Plannotator review of slice 1 pointed out that `Scope::Keys { selection, tabs }` broke `docs/agents/code.md`'s "name your shapes". It was also the only inline struct variant in the crates, where every other enum carries a named payload (`InputFrame::Key(KeyInput)`, `SseEvent::Attached(Attached)`). It's now `Scope::Keys(KeyScope)`, and `ship_client` exports `KeyScope`. The skeleton above is updated to match.

Checking the rest of the diff against `code.md` found one more fix, which Cyan approved with the first: the command tree derived `Serialize`, and nothing serializes a command. Dropping it also removed `From<TabTarget> for Option<NodeId>`, which existed only for `serde(into)` and silently dropped `pane` when both were set. Two other findings stayed as they are. The "at most one destination" rule lives in both the clap group and `Destination::resolve`, because the config has no clap and the spec needs exit 2 on the CLI. `KeyScope` owns a per-call snapshot of `Arc`s, which isn't stored state.

Implementation Rust is now 5,566 lines, with tests at zero. fmt, workspace Clippy, Clippy and debug builds on `ship-core` with and without `clap`, and the workspace debug and release builds passed. The serde probe accepted and rejected the same inputs as before, and both slice 1 workflow scripts gave the same results as before.

### Slice 2: config path and server settings (2026-10-09, macOS)

All slice 2 checks passed on macOS. Linux and Windows are unverified. No locked paper reopened. Implementation Rust went from 5,566 to 5,707 lines (+141), with tests at zero.

Cyan's review: `ship config check` first failed on a missing file. Cyan rejected that ("you're merging... nothing... with the defaults"), so a missing or empty file passes, the same as loading. Removing the guard exposed a bug: `Toml::file_exact` treats a missing file as an error, so with no config file every shell spawn logged a warning (still falling back to the login shell). `Toml::file` treats it as empty, and with the absolute path from `config_path` it doesn't search parent directories. A server started with `--config` naming a missing file then ran `/bin/zsh` with no WARN line.

Small departures from the skeleton, none changing behavior the papers specify:

- **`ui::run` doesn't take the config path yet.** Nothing in the client reads it until slice 3, so the parameter arrives with the keymap instead of as an unused argument.
- **`ServerSettings::load` extracts a private `Root { server }`** instead of focusing on `server`. Focusing drops `server` from error key paths. Errors read `<file>: <key path>: <message>`, built from figment's key path because its own message adds the `default.` profile. Syntax errors have no key path and carry line and column.
- **`shell` is a plain `PathBuf` joined onto the file's folder**, not figment's `RelativePathBuf`, which adds a `___figment_relative_path` segment to every error's key path (probe).
- **`Cli::config_path` makes the path absolute**, so the background server and a relative `shell` resolve against the same file whatever directory the server runs in. It's only computed where a server is started or the file is read, so `ship server status`, `ship server stop` and explicit-target commands don't depend on finding a config directory.
- **`ship config check` prints each error on stderr, then exits 1 with `ship: invalid config`** through the usual error path.

Surprises:

- portable-pty's `get_shell` falls back to the password-database shell when `SHELL` isn't executable, logging through the `log` crate, which Ship doesn't route. So a well-formed `shell = "/nonexistent"` gives the login shell with no line in Ship's log. FR-008 only promises the log line for a bad file, so this isn't a spec gap, but it's quiet.
- figment's unknown-field message doubles the backticks around the expected field names. Cosmetic, from figment.
- A `[srever]` table passes `ship config check` until slice 3 adds the client half, as planned: the server's `Root` ignores other tables.

Evidence, against foreground servers with `--server-url` unless noted:

- `ship --help` shows `--config <FILE>` with `[env: SHIP_CONFIG=]`, and `ship config --help` lists `check`.
- `ship config check bad.toml` with `shell = 3` printed ``bad.toml: server.shell: invalid type: found signed int `3`, expected path string`` and exited 1. A valid file exited 0 with no output. `shel = …` printed `server.shel: unknown field`, and a syntax error printed the line and column, each exiting 1. `SHIP_CONFIG=<bad>` with no `FILE` checked that file. A missing file and an empty file each exited 0 with no output.
- With `shell = "/bin/sh"`, `ship pane create --tab <t>` ran `-sh` in `ps`. Changing the file to `shell = "shells/mysh"` (a symlink to bash next to the file) made the next pane run `<config folder>/shells/mysh` as `-mysh`. Breaking the file's syntax made the next pane run `/bin/zsh`, with `WARN … using the login shell error=<file>: TOML parse error at line 1, column 8` in the server log. All three panes stayed running with their own shells.
- With no default server running, `SHIP_CONFIG=<scratch>/elsewhere.toml ship` (under `script` for a pty) started `ship server --port 43179 --starter --background-child --config <scratch>/elsewhere.toml`, whose starter pane ran `/bin/sh`.
- `ship server --port <p> --starter --config elsewhere.toml` listed one tab whose pane ran `/bin/sh`.
- Every test server was stopped afterward, and no test shells remained.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and the workspace debug and release builds passed. No route or protocol type changed, so the OpenAPI consumer wasn't rerun.

### Slice 3: keymap and modes (2026-10-09, macOS)

Tasks 3.1 to 3.4 passed on macOS, and so did every 3.5 check except two: judging key latency by feel is Cyan's to do, and Linux and Windows are unverified, so 3.5 stays open. The papers reopened under amendment A-4, which Cyan approved in chat (product status line). Implementation Rust went from 5,703 lines at the slice 2 commit to 6,281 (+578), counted as every `.rs` file under `crates`, with tests at zero. The slice 2 entry's 5,707 came from a slightly different count.

Surprises:

- **figment blends a rebound binding into the default it replaces.** It merges two tables key by key and replaces anything else whole. With `alt-n = { server.tab.create = {} }` over the default `{ server.tab.create.starter = "shell" }`, the merged value kept `starter`. A table over a table of a different action came out holding both actions, and an externally tagged enum then deserialized whichever key came first alphabetically, with no error (merge probe in the scratchpad). figment2 0.11.5 merges the same way. A-4 is the fix: the `Defaults` provider wraps each default binding in a one-item list, and figment replaces a list whole. Rejected along the way, as recorded in the architecture paper's decision 8: resolving chords in a provider before the merge, pruning default-tagged values, typed per-layer merging, and a custom merge.
- **Deserializing a figment `Value` into another `Value` drops the tags.** The first probe deserialized each mode to `BTreeMap<String, Value>`, so every binding read as `Tag::Default`. `clear_defaults` then did nothing, and `"Alt-N"` in the file was reported as a duplicate of the default `alt-n`. `build` now reads bindings from `figment.find_value("client.modes")`, which keeps them, and `Root`/`RawClient`/`RawMode` hold `IgnoredAny` where the tree is read instead.
- **`serde_with::OneOrMany` gave poor errors, so it isn't used.** It buffers the value and tries one shape, then the other, so a typo inside an action reported only the chord (`client.modes.normal.keys.alt-z`) with a multi-line "data did not match any variant" message. `build` dispatches on the value's shape instead: an array deserializes as `Vec<Action>`, anything else as one `Action`. The same typos now read `…keys.alt-z.server.tab.clse: unknown variant …` and `…keys.alt-z.1.client.mdoe: …`. This departs from Cyan's suggestion in chat; ship-client doesn't depend on `serde_with`.
- **On macOS, Alt was dropped when a key passed through to a pane.** With `"alt-q" = "none"`, `cat -v` showed `q`, not `^[q`. libghostty-vt's `set_options_from_terminal` (`key.rs:134`) resets `macos_option_as_alt` to false, so Option was treated as a macOS text modifier. This predated slice 3, which only made it visible through `"none"`. Herdr sets it back to true right after reading the terminal's modes (`src/pane/input.rs`, since `a124eed7`), because Alt is already decoded from the host terminal. Cyan asked for the same fix in this commit: `encode_key` in `vendor/ratatui-ghostty/src/input.rs` now sets `OptionAsAlt::True` after the modes. `cat -v` then received `^[q`, and plain and Ctrl keys arrived unchanged. Linux doesn't read this option, but it is unverified there.
- **A top-level error still stops the load before mode errors.** `[srever]` or `[client.mdoes]` fails `Root` and returns that one error, so mode and binding errors in the same file show up only after it's fixed. Errors inside `[client.modes]` are all collected.
- **A list's later server action reads the replica, not the earlier action's result.** In `[{ server.tab.create.starter = "shell" }, { server.pane.create = {} }]`, the second action's `KeyScope` takes the selection from `chosen` (set from the first result) but the tabs from the replica. If the server's event for the new tab hadn't reached the replica yet, the second action could miss the tab. It passed every time on loopback.

Small departures from the skeleton, none changing behavior the papers specify:

- **`Keymap::defaults()` panics** if the embedded defaults fail to load, since that's a build defect, not a user error.
- **A failed load shows the first error and a count**: `<first> (and N more; ship config check)`.
- **The status line shows `[mode]` before the selection and ` · message` after it**, and the message clears on the next bound key.
- **`client.send` with nothing selected shows "no pane selected"** and stops the rest of the list, like a failed server action.
- **`Row(pub u8)` deserializes through `try_from = "u8"`**, accepting 1 to 9.
- **No `Chords` provider.** Chords resolve after the merge, from the tags (architecture decision 8).
- **3.2 was verified end to end in a pty, not with a probe calling `Keys::handle`**: the same sequences, through the real client.

Evidence, with a disposable probe binary and a Python harness (pyte in a pty, one foreground server per scenario) in the scratchpad:

- Probe: a rebound `alt-n` loaded as `Create { starter: None }`, `"Alt-N"` in the file replaced the default with no error, `clear_defaults` left `resize` with only the file's `esc`, and `"none"` removed the chord. `"alt-n"` plus `"Alt-N"` in the file, `"ctrl-alt-x"` plus `"alt-ctrl-x"`, an undefined mode, `kind` on `normal`, `[srever]` and a typo'd action each failed with its key path.
- `ship config check` on a file with a typo'd action, an unknown field, an undefined mode and a duplicate chord printed all four and exited 1. A file with an unentered mode exited 0 with no output.
- Defaults, each checked against `ship tab list`: `alt-n` made a tab with a shell after the selected tab and selected its pane. `alt-|` and `alt--` added panes, `alt-tab` cycled, `alt-shift-x` closed the pane, `alt-left`/`alt-right` moved between tabs, and `alt-x` closed the tab. `alt-b`, `alt-h` and `alt-1` showed "not available yet", and `alt-r` then `h` showed "pane resize is not available yet". In `[tabs]`, keys were swallowed and `j` moved to the next tab. `alt-shift-x` with only a tab selected showed "no pane selected" and changed nothing. `ctrl-b` reached `cat -v` as `^B`. `alt-q` detached, printing "detached" and exiting 0.
- User config: a `prefix` on `ctrl-b` with `ctrl-b ctrl-b` sent one `^B`, and with `c` created a tab and returned to normal. `"alt-q" = "none"` stayed attached, and `cat -v` received `^[q`. A sticky `panes` mode showed `[panes]`, swallowed `x`, cycled panes on `n` and left on `esc`. `resize` with `clear_defaults` ignored `h`. The rebound `alt-n` made an empty tab. The `alt-m` list made a tab with two panes, the `alt-f` list failed on resize and created no tab, and the `alt-d` list (mode, then detach) exited 0.
- One-shot: in `prefix`, an unbound `z` was swallowed and returned to normal, so only the following `y` reached `cat -v`. `prefix` then `p` entered the sticky `panes` mode and stayed there.
- A broken file at startup ran the defaults and showed the error on the status line. `config.reload` on a broken file kept the old keymap and showed the error. After the file was fixed, the next reload applied the new binding.
- A binding and the matching CLI command gave the same tab shape in the same position.
- `rg 'C-b' crates` found nothing. Every test server was stopped afterward.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and the workspace debug and release builds passed.

### Slice 4: reload on save (2026-10-09, macOS)

All slice 4 checks passed on macOS. Linux and Windows are unverified. No locked paper reopened. Implementation Rust went from 6,281 to 6,366 lines (+85, against the estimate of about 50 for watching), with tests at zero. `watch.rs` is 63 of those; the rest is the `select!` arm, the watcher's setup and a shared `reload` in `ui/mod.rs`.

Surprise:

- **vim doesn't rename-save under `/tmp`.** Its default `backupskip` covers `/tmp/*` and `$TMPDIR/*`, and with no backup to make it writes the file in place, even with `backupcopy=no`. The scratchpad lives under `/private/tmp`, so the first vim run kept the inode and tested nothing. With `backupskip=` cleared, each vim save changed the inode. This is a test-setup trap, not a Ship behavior.

Small departures from the skeleton, none changing behavior the papers specify:

- **A config folder that doesn't exist isn't watched, and that isn't an error.** With no config folder, which is the default for anyone without a config, the watcher has nothing to watch and never yields, and the status line stays clear. If the folder is created later, the client won't notice saves until it restarts, though `client.config.reload` still works. Any other watch failure goes on the status line, after a keymap error if there is one. Cyan should confirm this choice.
- **A successful save clears the status line message**, so fixing a broken file removes its error. `client.config.reload` leaves the message alone, as before, since the key press already cleared it.
- **Events match on file name across the watched folders.** A file in the link's folder with the target's name, or the other way round, also triggers a reload. A spurious reload costs one file read and is otherwise harmless.
- **The debounce is 100 ms.**
- **The symlink is resolved once, at startup.** If the link is repointed into a third folder, saves there should go unnoticed until restart. Not exercised.

Evidence, from a Python harness (pyte in a pty, one foreground server per scenario, `--config` on server and client) in the scratchpad:

- In-place write: before the save `alt-y` did nothing. After adding `"alt-y" = { server.tab.create = {} }`, `alt-y` made an empty tab and `alt-n` still made a tab with a shell.
- Broken save: the status line showed `<file>: TOML parse error at line 10, column 6 …`, and `alt-y` still made a tab with the old keys. A key bound to `client.config.reload`, pressed on the still-broken file, put the same error back on the status line.
- Removing the active mode: `alt-p` entered the sticky `panes` mode (`[panes]` shown, `x` swallowed). Saving a file without `panes` dropped the indicator, and `alt-y` then worked, so the client was back in normal.
- vim rename-save (`set backupcopy=no writebackup backupskip=`): the inode changed (86318731 to 86318753) and `alt-y` worked afterward.
- Symlink: `links/config.toml` pointed to `dotfiles/ship.toml`, and vim rename-saved the target. The inode changed, `alt-y` worked afterward, and the link stayed a link. A second vim save then added `alt-u`, which worked, so the watch survives a rename.
- Writing `other.toml` in both folders didn't reload: with the file broken and its message cleared by a key, the message stayed clear until the config itself was touched again.
- With `--config` in a folder that doesn't exist, the client started with an empty status line and the default `alt-n` worked.
- Every test server was stopped afterward, and no `ship` processes remained.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and the workspace debug and release builds passed. No route or protocol type changed, so the OpenAPI consumer wasn't rerun.
