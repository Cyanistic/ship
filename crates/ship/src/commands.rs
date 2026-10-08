use std::{path::PathBuf, time::Duration};

use serde::Serialize;
use ship_client::Client;
use ship_core::{
    model::{Pane, Session, Tab},
    prelude::*,
    protocol::{CreateSession, MoveTab, PaneInput, PaneSpec, Placement},
};

use tokio::time::{Instant, sleep};

use crate::cli::{PaneCommand, SessionCommand, TabCommand};

/// Panes get two seconds to exit and the drain five, so ten covers a stop.
const STOP_WAIT: Duration = Duration::from_secs(10);

/// Each command prints its JSON result on stdout, or nothing for removal.
pub async fn session(client: &Client, command: SessionCommand) -> Result<()> {
    match command {
        SessionCommand::List => print(&client.sessions().await?),
        SessionCommand::Create(args) => {
            let body = CreateSession {
                name: args.name,
                starter: None,
            };
            print(&client.create_session(&body).await?)
        }
        SessionCommand::Get(args) => {
            let id = client.resolve_session(&args.session).await?;
            print(&client.get::<Session>(id).await?)
        }
        SessionCommand::Rename(args) => {
            let id = client.resolve_session(&args.session).await?;
            print(
                &client
                    .rename::<Session>(id, Some(&args.name.to_string()))
                    .await?,
            )
        }
        SessionCommand::Rm(args) => {
            let id = client.resolve_session(&args.session).await?;
            client.remove::<Session>(id).await
        }
    }
}

pub async fn tab(client: &Client, command: TabCommand) -> Result<()> {
    match command {
        TabCommand::Create(args) => {
            let parent = client.resolve_parent(&args.parent).await?;
            print(&client.create_tab(parent, args.name.as_deref()).await?)
        }
        TabCommand::Get(args) => print(&client.get::<Tab>(args.id).await?),
        TabCommand::Rename(args) => {
            print(&client.rename::<Tab>(args.id, args.name.as_deref()).await?)
        }
        TabCommand::Rm(args) => client.remove::<Tab>(args.id).await,
        TabCommand::Move(args) => {
            let to = MoveTab {
                parent: client.resolve_parent(&args.parent).await?,
                placement: args
                    .before
                    .map(Placement::Before)
                    .or(args.after.map(Placement::After)),
            };
            print(&client.move_tab(args.id, &to).await?)
        }
    }
}

pub async fn pane(client: &Client, command: PaneCommand) -> Result<()> {
    match command {
        PaneCommand::Create(args) => {
            let cwd = match args.cwd {
                Some(cwd) => cwd,
                None => current_dir()?,
            };
            let cwd = std::path::absolute(&cwd).map_err(
                |error| err!(Io, "cannot resolve '{}'", cwd.display(), @external: error),
            )?;
            let input = PaneInput {
                name: args.name.into(),
                spec: PaneSpec {
                    command: Some(args.command).filter(|command| !command.is_empty()),
                    cwd: Some(cwd.display().to_string()),
                },
            };
            print(&client.create_pane(args.tab, &input).await?)
        }
        PaneCommand::Get(args) => print(&client.get::<Pane>(args.id).await?),
        PaneCommand::Rename(args) => {
            print(&client.rename::<Pane>(args.id, args.name.as_deref()).await?)
        }
        PaneCommand::Rm(args) => client.remove::<Pane>(args.id).await,
    }
}

/// `ship server stop`: start shutdown, then wait until the server refuses
/// connections. Prints nothing, like removal.
pub async fn stop_server(client: &Client) -> Result<()> {
    client.stop_server().await?;
    let deadline = Instant::now() + STOP_WAIT;
    loop {
        match client.health().await {
            Err(error) if *error.code() == ErrorCode::ConnectionRefused => return Ok(()),
            _ if Instant::now() >= deadline => {
                return Err(err!(Network, "server still running after ten seconds"));
            }
            _ => sleep(Duration::from_millis(50)).await,
        }
    }
}

/// `$PWD` when it names the current directory, so a symlinked path such as
/// macOS's `/tmp` is kept as the user typed it, as shells do.
pub fn current_dir() -> Result<PathBuf> {
    let current = std::env::current_dir()
        .map_err(|error| err!(Io, "cannot read the current directory", @external: error))?;
    let real = std::fs::canonicalize(&current).ok();
    Ok(std::env::var_os("PWD")
        .map(PathBuf::from)
        .filter(|pwd| {
            pwd.is_absolute() && real.is_some() && std::fs::canonicalize(pwd).ok() == real
        })
        .unwrap_or(current))
}

fn print(value: &impl Serialize) -> Result<()> {
    let json = serde_json::to_string(value)
        .map_err(|error| err!(Serialization, "cannot serialize result", @external: error))?;
    println!("{json}");
    Ok(())
}
