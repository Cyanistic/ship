use clap::{Args, Parser, Subcommand};
use ship_client::SessionRef;
use ship_core::{DEFAULT_PORT, DEFAULT_SERVER_URL, model::SessionName};

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
