use std::{path::PathBuf, str::FromStr};

use clap::{Args, Parser, Subcommand};
use ship_core::{
    AppError, DEFAULT_PORT, DEFAULT_SERVER_URL,
    id::{IdOf, Identified},
    model::{Pane, Tab},
    protocol::MoveTab,
};

#[derive(Parser)]
#[command(
    version,
    about = "Open the Ship client, run a loopback server or script tabs and panes",
    long_about = "Open the Ship client, run a loopback server or script tabs and panes.\n\nBare ship opens a full-screen client on the whole server; C-b d detaches.\nBare ship and tab and pane commands reuse the default-local server or start a missing one in the background.\nThat server stays running after the client and launching terminal exit."
)]
pub struct Cli {
    /// Connect only to this HTTP/HTTPS server; never start or fall back locally
    #[arg(
        long,
        global = true,
        env = "SHIP_SERVER_URL",
        value_name = "URL",
        default_value = DEFAULT_SERVER_URL
    )]
    pub server_url: String,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run a foreground server on 127.0.0.1, or stop or check one
    Server(ServerArgs),
    /// List, create, inspect, rename, remove or move tabs
    #[command(subcommand)]
    Tab(TabCommand),
    /// Create, inspect, rename or remove panes
    #[command(subcommand)]
    Pane(PaneCommand),
}

#[derive(Subcommand)]
pub enum TabCommand {
    /// List top-level tabs and their descendants as JSON, keyed by ID
    List,
    /// Append a tab to a tab, or to the top level without one, and print it as JSON
    Create(CreateTabArgs),
    /// Print a tab and its descendants as JSON
    Get(IdArgs<Tab>),
    /// Rename a tab and print it as JSON
    Rename(RenameArgs<Tab>),
    /// Remove a tab and all its descendants
    Rm(IdArgs<Tab>),
    /// Append under PARENT or to the top level, or place before or after a sibling
    Move(MoveTabArgs),
}

#[derive(Subcommand)]
pub enum PaneCommand {
    /// Append a pane running a program to a tab and print it as JSON
    Create(CreatePaneArgs),
    /// Print a pane as JSON
    Get(IdArgs<Pane>),
    /// Rename a pane and print it as JSON
    Rename(RenameArgs<Pane>),
    /// Remove a pane
    Rm(IdArgs<Pane>),
}

/// Bare `ship server` runs one; stop it with SIGINT, SIGTERM or `ship server stop`.
#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ServerArgs {
    #[command(subcommand)]
    pub command: Option<ServerCommand>,
    #[arg(long, default_value_t = DEFAULT_PORT, value_parser = clap::value_parser!(u16).range(1..))]
    pub port: u16,
    /// Start with one tab holding a shell in your home directory
    #[arg(long)]
    pub starter: bool,
    #[arg(long, hide = true)]
    pub background_child: bool,
}

#[derive(Subcommand)]
pub enum ServerCommand {
    /// Stop the server, ending every program in it, and wait until it exits
    Stop,
    /// Print the server's health as JSON; never starts a server
    Status,
}

#[derive(Args)]
pub struct IdArgs<T: Identified>
where
    IdOf<T>: FromStr<Err = AppError> + Send + Sync + 'static,
{
    pub id: IdOf<T>,
}

#[derive(Args)]
pub struct RenameArgs<T: Identified>
where
    IdOf<T>: FromStr<Err = AppError> + Send + Sync + 'static,
{
    pub id: IdOf<T>,
    /// New name; omitted or blank clears it
    pub name: Option<String>,
}

#[derive(Args)]
pub struct CreateTabArgs {
    /// Parent tab ID; omitted means the top level
    pub parent: Option<IdOf<Tab>>,
    /// Tab name; omitted or blank means none
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Args)]
pub struct CreatePaneArgs {
    /// Tab ID
    pub tab: IdOf<Tab>,
    /// Pane name; omitted or blank means none
    #[arg(long)]
    pub name: Option<String>,
    /// Starting directory; defaults to the current directory
    #[arg(long)]
    pub cwd: Option<PathBuf>,
    /// Command and arguments; defaults to your login shell
    #[arg(last = true, value_name = "COMMAND")]
    pub command: Vec<String>,
}

#[derive(Args)]
pub struct MoveTabArgs {
    pub id: IdOf<Tab>,
    #[command(flatten)]
    pub to: Destination,
}

/// At most one; none means the top level.
#[derive(Args)]
#[group(multiple = false)]
pub struct Destination {
    /// Parent tab ID; omitted means the top level
    pub parent: Option<IdOf<Tab>>,
    /// Insert before this sibling, under its parent
    #[arg(long)]
    pub before: Option<IdOf<Tab>>,
    /// Insert after this sibling, under its parent
    #[arg(long)]
    pub after: Option<IdOf<Tab>>,
}

impl From<Destination> for MoveTab {
    fn from(to: Destination) -> Self {
        match (to.before, to.after) {
            (Some(sibling), _) => Self::Before(sibling),
            (None, Some(sibling)) => Self::After(sibling),
            (None, None) => Self::Parent(to.parent),
        }
    }
}
