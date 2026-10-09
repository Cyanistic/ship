use std::time::Duration;

use serde::Serialize;
use ship_client::{Client, Outcome, Scope};
use ship_core::{command::Command, prelude::*};

use tokio::time::{Instant, sleep};

/// Panes get two seconds to exit and the drain five, so ten covers a stop.
const STOP_WAIT: Duration = Duration::from_secs(10);

/// `ship tab …` and `ship pane …`: execute under `Scope::Cli`, then print
/// the outcome as one JSON line, or nothing for close.
pub async fn action(client: &Client, command: Command) -> Result<()> {
    match client.execute(command, &Scope::Cli).await? {
        Outcome::Closed => Ok(()),
        outcome => print(&outcome),
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

pub(crate) fn print(value: &impl Serialize) -> Result<()> {
    let json = serde_json::to_string(value)
        .map_err(|error| err!(Serialization, "cannot serialize result", @external: error))?;
    println!("{json}");
    Ok(())
}
