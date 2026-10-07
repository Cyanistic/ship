use serde::Serialize;
use ship_client::Client;
use ship_core::{
    model::{Pane, Session, Tab},
    prelude::*,
    protocol::{MoveTab, Placement},
};

use crate::cli::{PaneCommand, SessionCommand, TabCommand};

/// Each command prints its JSON result on stdout, or nothing for removal.
pub async fn session(client: &Client, command: SessionCommand) -> Result<()> {
    match command {
        SessionCommand::List => print(&client.sessions().await?),
        SessionCommand::Create(args) => print(&client.create_session(&args.name).await?),
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
            print(&client.create_pane(args.tab, args.name.as_deref()).await?)
        }
        PaneCommand::Get(args) => print(&client.get::<Pane>(args.id).await?),
        PaneCommand::Rename(args) => {
            print(&client.rename::<Pane>(args.id, args.name.as_deref()).await?)
        }
        PaneCommand::Rm(args) => client.remove::<Pane>(args.id).await,
    }
}

fn print(value: &impl Serialize) -> Result<()> {
    let json = serde_json::to_string(value)
        .map_err(|error| err!(Serialization, "cannot serialize result", @external: error))?;
    println!("{json}");
    Ok(())
}
