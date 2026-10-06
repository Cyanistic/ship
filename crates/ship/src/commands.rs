use serde::Serialize;
use ship_client::Client;
use ship_core::{model::Session, prelude::*};

use crate::cli::SessionCommand;

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
            print(&client.rename::<Session>(id, &args.name.to_string()).await?)
        }
        SessionCommand::Rm(args) => {
            let id = client.resolve_session(&args.session).await?;
            client.remove::<Session>(id).await
        }
    }
}

fn print(value: &impl Serialize) -> Result<()> {
    let json = serde_json::to_string(value)
        .map_err(|error| err!(Serialization, "cannot serialize result", @external: error))?;
    println!("{json}");
    Ok(())
}
