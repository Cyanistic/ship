use clap::{Parser, Subcommand};
use ship_core::{DEFAULT_PORT, DEFAULT_SERVER_URL};

#[derive(Parser)]
#[command(
    version,
    about = "Check Ship health or run a loopback server",
    args_conflicts_with_subcommands = true,
    long_about = "Check Ship health or run a loopback server.\n\nBare ship reuses the default-local server or starts a missing one in the background.\nThat server stays running after the client and launching terminal exit."
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
    Server {
        #[arg(long, default_value_t = DEFAULT_PORT, value_parser = clap::value_parser!(u16).range(1..))]
        port: u16,
        #[arg(long, hide = true)]
        background_child: bool,
    },
}
