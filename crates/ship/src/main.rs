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
    let explicit_target = matches.value_source("server_url") == Some(ValueSource::CommandLine);
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    match run(cli, explicit_target) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ship: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli, explicit_target: bool) -> Result<()> {
    if matches!(
        cli.command,
        Some(Command::Server(cli::ServerArgs {
            background_child: true,
            ..
        }))
    ) {
        nix::unistd::setsid().map_err(
            |error| err!(Io, "cannot establish background server session", @external: error),
        )?;
    }
    diagnostics::init()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| err!(Internal, "cannot create runtime", @external: error))?;
    runtime.block_on(dispatch(cli, explicit_target))
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

async fn dispatch(cli: Cli, explicit_target: bool) -> Result<()> {
    match cli.command {
        Some(Command::Server(_)) if explicit_target => Err(err!(
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
        Some(Command::Session(command)) => {
            let client = connect(&cli.server_url, explicit_target).await?;
            commands::session(&client, command).await
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
            let client = client(&cli.server_url)?;
            let response = if explicit_target {
                health(&client).await?
            } else {
                local::default_health(&client).await?
            };
            let json = serde_json::to_string(&response).map_err(
                |error| err!(Serialization, "cannot serialize health result", @external: error),
            )?;
            println!("{json}");
            Ok(())
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
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(1))
            .build()
            .map_err(|error| err!(Configuration, "cannot initialize HTTP client", @external: error.without_url()))?,
        url,
    })
}
