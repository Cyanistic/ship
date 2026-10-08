mod cli;
mod commands;
mod diagnostics;
mod local;

use std::{process::ExitCode, time::Duration};

use clap::{CommandFactory, FromArgMatches, parser::ValueSource};
use cli::{Cli, Command};
use ship_client::Client;
use ship_core::{HealthResponse, PROTOCOL_VERSION, prelude::*};
use url::Url;

fn main() -> ExitCode {
    let matches = Cli::command().get_matches();
    let source = matches.value_source("server_url");
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    match run(cli, source) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ship: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli, source: Option<ValueSource>) -> Result<()> {
    if matches!(
        cli.command,
        Some(Command::Server(cli::ServerArgs {
            background_child: true,
            ..
        }))
    ) {
        detach()?;
    }
    diagnostics::init()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| err!(Internal, "cannot create runtime", @external: error))?;
    runtime.block_on(dispatch(cli, source))
}

/// Leave the launching terminal's session so the background server outlives it.
#[cfg(unix)]
fn detach() -> Result<()> {
    nix::unistd::setsid()
        .map(drop)
        .map_err(|error| err!(Io, "cannot establish background server session", @external: error))
}

/// Application compatibility boundary, shared by explicit requests and local readiness.
async fn health(client: &Client) -> Result<HealthResponse> {
    let response = client.health().await?;
    if response.service != "ship" || response.protocol_version != PROTOCOL_VERSION {
        return Err(err!(
            Validation,
            "incompatible health response from {}: expected service ship and protocol {}",
            client.url.origin().ascii_serialization(),
            PROTOCOL_VERSION
        ));
    }
    Ok(response)
}

/// `--server-url` or `SHIP_SERVER_URL` makes the target explicit for client
/// commands. `ship server` refuses only the flag, so an exported variable
/// doesn't stop a terminal from running a server.
async fn dispatch(cli: Cli, source: Option<ValueSource>) -> Result<()> {
    let explicit_target = matches!(
        source,
        Some(ValueSource::CommandLine | ValueSource::EnvVariable)
    );
    match cli.command {
        // Never start a server, so a stopped default server stays stopped.
        Some(Command::Server(cli::ServerArgs {
            command: Some(cli::ServerCommand::Stop),
            ..
        })) => commands::stop_server(&client(&cli.server_url)?).await,
        Some(Command::Server(cli::ServerArgs {
            command: Some(cli::ServerCommand::Status),
            ..
        })) => commands::print(&health(&client(&cli.server_url)?).await?),
        Some(Command::Server(_)) if source == Some(ValueSource::CommandLine) => Err(err!(
            Configuration,
            "--server-url selects a server for clients; ship server does not take it"
        )),
        Some(Command::Server(args)) => {
            ship_server::serve(
                ship_server::loopback_addr(args.port)?,
                diagnostics::shutdown()?,
            )
            .await
        }
        Some(Command::Tab(command)) => {
            let client = connect(&cli.server_url, explicit_target).await?;
            commands::tab(&client, command).await
        }
        Some(Command::Pane(command)) => {
            let client = connect(&cli.server_url, explicit_target).await?;
            commands::pane(&client, command).await
        }
        None => {
            let client = connect(&cli.server_url, explicit_target).await?;
            // A full-screen client must not misread another protocol.
            if explicit_target {
                health(&client).await?;
            }
            ship_client::ui::run(&client).await
        }
    }
}

/// An explicit `--server-url` is used as is; otherwise the default local
/// server is reused or started.
async fn connect(server_url: &str, explicit_target: bool) -> Result<Client> {
    let client = client(server_url)?;
    if !explicit_target {
        local::default_health(&client).await?;
    }
    Ok(client)
}

fn client(server_url: &str) -> Result<Client> {
    // Never include the input: even a malformed URL can contain credentials.
    let url = Url::parse(server_url)
        .map_err(|error| err!(Configuration, "invalid server URL", @external: error))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(err!(
            Configuration,
            "server URL requires absolute HTTP/HTTPS and a host"
        ));
    }
    Ok(Client {
        http: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(1))
            .build()
            .map_err(|error| err!(Configuration, "cannot initialize HTTP client", @external: error.without_url()))?,
        url,
    })
}
