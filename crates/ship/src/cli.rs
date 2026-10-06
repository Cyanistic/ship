use std::str::FromStr;

use clap::{Args, Parser, Subcommand};
use ship_client::{SessionRef, TabParentRef};
use ship_core::{
    AppError, DEFAULT_PORT, DEFAULT_SERVER_URL,
    id::{IdOf, Identified},
    model::{SessionName, Tab},
};

#[derive(Parser)]
#[command(
    version,
    about = "Check Ship health, run a loopback server or manage sessions",
    long_about = "Check Ship health, run a loopback server or manage sessions.\n\nBare ship and session commands reuse the default-local server or start a missing one in the background.\nThat server stays running after the client and launching terminal exit."
)]
pub struct Cli {
    /// Connect only to this HTTP/HTTPS server; never start or fall back locally
    #[arg(long, value_name = "URL", default_value = DEFAULT_SERVER_URL)]
    pub server_url: String,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run a foreground server on 127.0.0.1; stop with SIGINT or SIGTERM
    Server(ServerArgs),
    /// Create, inspect, rename or remove sessions
    #[command(subcommand)]
    Session(SessionCommand),
    /// Create, inspect, rename, remove or move tabs
    #[command(subcommand)]
    Tab(TabCommand),
}

#[derive(Subcommand)]
pub enum SessionCommand {
    /// List sessions as JSON, keyed by ID
    List,
    /// Create a session and print it as JSON
    Create(NameArgs),
    /// Print a session as JSON
    Get(SessionArgs),
    /// Rename a session and print it as JSON
    Rename(RenameSessionArgs),
    /// Remove a session and everything in it
    Rm(SessionArgs),
}

#[derive(Subcommand)]
pub enum TabCommand {
    /// Append a tab to a session or tab and print it as JSON
    Create(CreateTabArgs),
    /// Print a tab and its descendants as JSON
    Get(IdArgs<Tab>),
    /// Rename a tab and print it as JSON
    Rename(RenameArgs<Tab>),
    /// Remove a tab and all its descendants
    Rm(IdArgs<Tab>),
    /// Move under PARENT; appends unless --before or --after names a sibling
    Move(MoveTabArgs),
}

#[derive(Args)]
pub struct ServerArgs {
    #[arg(long, default_value_t = DEFAULT_PORT, value_parser = clap::value_parser!(u16).range(1..))]
    pub port: u16,
    #[arg(long, hide = true)]
    pub background_child: bool,
}

#[derive(Args)]
pub struct NameArgs {
    /// Session name; must not contain ':'
    pub name: SessionName,
}

#[derive(Args)]
pub struct SessionArgs {
    /// Session name or ID
    pub session: SessionRef,
}

#[derive(Args)]
pub struct RenameSessionArgs {
    /// Session name or ID
    pub session: SessionRef,
    /// New session name; must not contain ':'
    pub name: SessionName,
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
    pub name: String,
}

#[derive(Args)]
pub struct CreateTabArgs {
    /// Session name or ID, or tab ID
    pub parent: TabParentRef,
    pub name: String,
}

#[derive(Args)]
pub struct MoveTabArgs {
    pub id: IdOf<Tab>,
    /// Session name or ID, or tab ID
    pub parent: TabParentRef,
    /// Insert before this sibling in PARENT
    #[arg(long, conflicts_with = "after")]
    pub before: Option<IdOf<Tab>>,
    /// Insert after this sibling in PARENT
    #[arg(long)]
    pub after: Option<IdOf<Tab>>,
}
