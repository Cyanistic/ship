//! Temporary same-process observer controls. Delete with the real client UI;
//! observe.rs then drops its stdin branch and nothing else changes.

use ship_client::{Client, SessionRef};
use ship_core::{
    id::{Attachment, IdOf},
    model::NodeId,
    prelude::*,
};
use tokio::sync::mpsc;

/// `select <id>` or `switch <session name or ID>`, one per stdin line.
pub enum Control {
    Select(NodeId),
    Switch(SessionRef),
}

pub fn parse(line: &str) -> Result<Control> {
    match line.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["select", id] => id.parse().map(Control::Select),
        ["switch", session] => session.parse().map(Control::Switch),
        _ => Err(err!(
            Validation,
            "expected `select <id>` or `switch <session>`, got '{}'",
            line.trim()
        )),
    }
}

/// Send `control` for this observer's attachment. `switch` resolves a session
/// name through `Client::resolve_session` first and never creates a session.
pub async fn send(client: &Client, attachment: IdOf<Attachment>, control: Control) -> Result<()> {
    match control {
        Control::Select(selection) => client.select(attachment, selection).await?,
        Control::Switch(session) => {
            let session = client.resolve_session(&session).await?;
            client.switch_session(attachment, session).await?
        }
    };
    Ok(())
}

/// Stdin lines, read on a plain thread so a blocked read never delays exit.
/// The channel closes at end of input.
pub fn lines() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel(16);
    std::thread::spawn(move || {
        for line in std::io::stdin().lines() {
            let Ok(line) = line else { break };
            if tx.blocking_send(line).is_err() {
                break;
            }
        }
    });
    rx
}
