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
async fn dispatch(mut cli: Cli, source: Option<ValueSource>) -> Result<()> {
    let explicit_target = matches!(
        source,
        Some(ValueSource::CommandLine | ValueSource::EnvVariable)
    );
    match cli.command.take() {
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
                args.starter,
                cli.config_path()?,
                diagnostics::shutdown()?,
            )
            .await
        }
        Some(Command::Config(cli::ConfigCommand::Check { file })) => {
            commands::check_config(&match file {
                Some(file) => file,
                None => cli.config_path()?,
            })
        }
        Some(Command::Action(command)) => {
            let client = connect(&cli, explicit_target).await?;
            commands::action(&client, command).await
        }
        None => {
            let client = client(&cli.server_url)?;
            // A full-screen client must not misread another protocol. Only a
            // server this call launched holds a starter tab to open.
            let open_first = if explicit_target {
                health(&client).await?;
                false
            } else {
                local::default_health(&client, &cli.config_path()?).await?
            };
            ship_client::ui::run(&client, open_first, &cli.config_path()?).await
        }
    }
}

/// An explicit `--server-url` is used as is; otherwise the default local
/// server is reused or started.
async fn connect(cli: &Cli, explicit_target: bool) -> Result<Client> {
    let client = client(&cli.server_url)?;
    if !explicit_target {
        local::default_health(&client, &cli.config_path()?).await?;
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
