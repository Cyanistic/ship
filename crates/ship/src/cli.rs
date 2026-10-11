use std::path::{self, PathBuf};

use clap::{Args, Parser, Subcommand};
use etcetera::BaseStrategy;
use ship_core::{DEFAULT_PORT, DEFAULT_SERVER_URL, command::Command as ServerAction, prelude::*};

#[derive(Parser)]
#[command(
    version,
    about = "Open the Ship client, run a loopback server or script tabs and panes",
    long_about = "Open the Ship client, run a loopback server or script tabs and panes.\n\nBare ship opens a full-screen client on the whole server; alt-q detaches.\nBare ship and tab and pane commands reuse the default-local server or start a missing one in the background.\nThat server stays running after the client and launching terminal exit."
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
    /// Config file; defaults to ship/config.toml in the platform config directory
    #[arg(long, global = true, env = "SHIP_CONFIG", value_name = "FILE")]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    /// `--config`, `SHIP_CONFIG`, then etcetera's config directory. Absolute,
    /// so the background server and relative settings see the same file.
    pub fn config_path(&self) -> Result<PathBuf> {
        let path = match &self.config {
            Some(path) => path.clone(),
            None => etcetera::choose_base_strategy()
                .map_err(|error| err!(Configuration, "cannot find the config directory", @external: error))?
                .config_dir()
                .join("ship/config.toml"),
        };
        path::absolute(&path).map_err(
            |error| err!(Io, "cannot resolve config path {}", path.display(), @external: error),
        )
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Run a foreground server on 127.0.0.1, or stop or check one
    Server(ServerArgs),
    #[command(flatten)]
    Action(ServerAction),
    /// Check the config file
    #[command(subcommand)]
    Config(ConfigCommand),
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Report every error in a config file and exit non-zero if any
    Check {
        /// File to check; defaults to the config file in use
        file: Option<PathBuf>,
    },
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
