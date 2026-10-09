# Keys and config program

Status: Locked on 2026-10-09. Cyan approved it in chat after two Plannotator rounds ("oh alright. fair enough. alright... i think we're ready for openspec docs??"). The rounds dropped config warnings (product amendment A-2) and confirmed by probe that argument-free client actions are empty struct variants (U-1). Written from the locked [product](product.md) paper (including amendments A-1 and A-2) and the locked [architecture](architecture.md) paper, against the crates at 9a05ab8. Nothing here is created source, and no snippet has been compiled. Approval doesn't start implementation; Cyan requests that separately, slice by slice.

Needs Cyan's attention:

- **`ship server --starter` goes through `CreateTab`.** The startup-only `state::Starter` message is deleted, because a `CreateTab` with `starter: Some(Shell)` does the same thing. That leaves one way to make a tab with a shell instead of two. The architecture paper said "reuses the `state::Starter` path", and this keeps the behavior while dropping the duplicate.
- **The configured shell goes in through `SHELL`.** portable-pty's default program reads `SHELL` from the command builder and still starts it as a login shell (`-zsh`), so the server sets `SHELL` on the builder and leaves the rest alone. Configuring the shell doesn't need a code path of its own.
- **`CreateTab` on the wire changes shape.** `parent` becomes `at`, the same `MoveTab` that move takes, so the body reads `{"at": {"after": "tab:…"}, "starter": "shell"}`. `PROTOCOL_VERSION` goes to 6.
- **`SHIP_PANE_ID` is read in two places.** clap reads it to fill a missing `--pane`, as the architecture says, and the binary reads it again to build `Scope::Cli`, which the `--tab` default needs. The alternative is a hidden global clap argument, which clashes with `--pane`.
- **U-1, resolved by probe:** figment rejects `{}` for a unit variant ("invalid type: found map, expected unit"), while the `toml` crate accepts it. Empty struct variants (`Next {}`) and structs with all-default fields both accept `{}` under figment, through the whole value and through one `Value` per binding. So client actions with no arguments are empty struct variants, on purpose (see the comment on `ClientAction`). Figment errors name the key path with a `default.` profile prefix (`default.keys.a.client.tab.next.x`), which `Keymap::load` strips.

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
use serde::{Deserialize, Serialize};
use crate::{id::IdOf, model::{Pane, Tab}, protocol::{MoveTab, Starter}};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args))]
#[serde(transparent)]
pub struct PaneTarget {
    #[cfg_attr(feature = "clap", arg(long = "pane", env = "SHIP_PANE_ID", value_name = "ID"))]
    pub id: Option<IdOf<Pane>>,
}

/// `--tab ID`; a missing ID is the tab holding the current pane on the CLI,
/// or the selected tab on keys.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args))]
#[serde(transparent)]
pub struct TabTarget {
    #[cfg_attr(feature = "clap", arg(long = "tab", value_name = "ID"))]
    pub id: Option<IdOf<Tab>>,
}

/// At most one. Moved here from `ship/src/cli.rs`, now with serde.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum Direction { Left, Down, Up, Right }

pub mod tab {
    #[derive(Clone, Debug, Serialize, Deserialize)]
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

Every struct derives `Clone, Debug, Serialize, Deserialize` and, behind the feature, `Args`. The `#[serde(default, deny_unknown_fields)]` shown on `Create` applies to every struct, so `{}` works wherever every field is optional.

#### `crates/ship-client/src/execute.rs` (slice 1)

```rust
//! One path from a command to the API, for the CLI and for keys.

use ship_core::{command::{Command, PaneTarget, TabTarget}, model::{NodeId, Pane, Tab}, tree::Tabs};

/// Where a missing target comes from.
pub enum Scope {
    /// The pane the CLI runs in, from `SHIP_PANE_ID`. CLI-made panes default
    /// to the process's current directory.
    Cli { pane: Option<IdOf<Pane>> },
    /// The client's selection and the tabs it was made in. Key-made tabs go
    /// after the selected tab (FR-021); key-made panes start in the server's
    /// home directory (architecture R-3).
    Keys { selection: Option<NodeId>, tabs: Tabs },
}

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

    /// Under `Scope::Cli`, a missing tab is `tree::pane_owner` of the current
    /// pane over `GET /tabs`.
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
use figment::{Figment, Provider, providers::{Format, Toml}};
use ship_core::command::{Command, Direction};

const DEFAULTS: &str = include_str!("keymap/defaults.toml");

pub struct Keymap { pub modes: HashMap<ModeName, Mode> }

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct ModeName(pub String);
impl ModeName { pub const NORMAL: &str = "normal"; }

pub struct Mode { pub kind: ModeKind, pub keys: HashMap<KeyCombination, Binding> }

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind { #[default] Sticky, Oneshot }

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Binding { Server(Command), Client(ClientAction), None }

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
/// 1 to 9.
pub struct Row(u8);

/// The file's shape before chords are parsed. The root holds only `server`
/// (ignored here) and `client`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Root { #[serde(default)] server: serde::de::IgnoredAny, #[serde(default)] client: RawClient }
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClient { #[serde(default)] modes: BTreeMap<String, RawMode> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMode {
    kind: Option<ModeKind>,
    #[serde(default)] clear_defaults: bool,
    #[serde(default)] keys: BTreeMap<String, figment::value::Value>,
}

/// Rewrites every key under `client.modes.*.keys` to crokey's canonical
/// spelling, drops `cleared` modes' keys, and fails on two spellings of one
/// chord in a mode (FR-010).
struct Chords<P> { inner: P, cleared: BTreeSet<String> }
impl<P: Provider> Provider for Chords<P>;

impl Keymap {
    /// Defaults merged under the file. A missing file is the defaults.
    /// Each binding deserializes on its own, so every error is collected,
    /// each naming the file and its key path.
    /// Used at startup, on reload and by `ship config check`, so check
    /// reports exactly what loading rejects (FR-016, product A-2).
    pub fn load(path: &Path) -> Result<Self, Vec<AppError>>;
    pub fn defaults() -> Self;
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

Adds `mod execute;` and `pub use execute::{Outcome, Scope, current_dir};` in slice 1, `pub mod keymap;` in slice 3, and `mod watch;` in slice 4.

#### `crates/ship-client/src/ui/keys.rs` (slice 3)

Rewritten as the mode machine (architecture decision 9). The `C-b` prefix, `is_prefix` and `bound` go away.

```rust
pub(super) enum Action {
    /// A key or paste for the selected pane; dropped while disconnected.
    Frame(InputFrame),
    /// A bound `server.` action, queued by the loop.
    Server(Command),
    /// A bound `client.` action other than `mode`, which `Keys` handles.
    Client(ClientAction),
    None,
}

pub(super) struct Keys {
    pub keymap: Keymap,
    pub active: ModeName,
}

impl Keys {
    pub fn new(keymap: Keymap) -> Self;
    /// `KeyCombination::from(key)`, looked up in the active mode. Unbound:
    /// a frame in normal, dropped in a sticky mode. A one-shot mode returns
    /// to normal after any key unless the binding was `client.mode`.
    pub fn handle(&mut self, event: Event, selected: Option<IdOf<Pane>>) -> Action;
    /// Swap in a reloaded keymap; an active mode it no longer defines falls
    /// back to normal.
    pub fn replace(&mut self, keymap: Keymap);
}
```

`client.send` turns its `KeyCombination` back into a `KeyEvent` for the frame. Whether crokey provides that conversion gets checked in slice 3. If it doesn't, the code builds the `KeyEvent` from the chord's single code and its modifiers.

#### `crates/ship-client/src/ui/mod.rs` (slices 3 and 4)

```rust
pub async fn run(client: &Client, open_first: bool, config: &Path) -> Result<()>;   // was (client, open_first)
```

Inside `drive`:

- `Keys::new(Keymap::load(config))`. On `Err`, it runs on `Keymap::defaults()` and puts the first error on the status line (FR-007).
- `queued: VecDeque<Command>` and `running: Pending<Outcome>`. A key's `Action::Server` is pushed onto the queue, and the slot runs the front with `Scope::Keys { selection, tabs }` taken from the observer. A `Tab` or `Pane` outcome becomes `chosen`, so the next view PUT selects it. A tab with a starter pane selects that pane (FR-021, FR-029). An error goes to the status line.
- `Action::Client`: `tab.next`/`prev` and `pane.next`/`prev` go through the existing `navigate`. `detach` exits. `config.reload` reloads. `send` becomes a frame. The layout actions (`tab.select`, `tab.expand`, `tab.collapse`, `pane.focus` and `sidebar.toggle`) put "not available yet" on the status line.
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
/// `ship tab …` and `ship pane …`: execute under `Scope::Cli` with
/// `SHIP_PANE_ID`, then print the outcome.
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

Empty.
