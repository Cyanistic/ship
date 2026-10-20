//! CLI input to a pane, without an attachment or selection.

use std::io::{self, Read};

use crokey::KeyCombination;
use crossterm::event::KeyEvent;
use futures_util::stream;
use ship_core::{
    id::IdOf,
    model::Pane,
    prelude::*,
    protocol::{InputFrame, KeyInput, PasteInput},
};

use crate::Client;

/// One paste followed by keys, all parsed before writing.
pub(crate) struct Input {
    text: Option<String>,
    keys: Vec<KeyEvent>,
}

impl Input {
    pub(crate) fn new(text: Option<String>, keys: Vec<KeyCombination>) -> Result<Self> {
        let text = match text.as_deref() {
            Some("-") => {
                let mut text = String::new();
                io::stdin()
                    .read_to_string(&mut text)
                    .map_err(|error| err!(Io, "cannot read input from stdin", @external: error))?;
                Some(text)
            }
            _ => text,
        };
        Ok(Self {
            text,
            keys: keys.into_iter().map(Into::into).collect(),
        })
    }

    fn frames(self, pane: IdOf<Pane>) -> Vec<InputFrame> {
        self.text
            .into_iter()
            .map(|text| InputFrame::Paste(PasteInput { pane, text }))
            .chain(
                self.keys
                    .into_iter()
                    .map(|key| InputFrame::Key(KeyInput { pane, key })),
            )
            .collect()
    }
}

impl Client {
    /// Check the target exists, then write without attaching. Exited programs
    /// accept and drop input just as they do for the full-screen client.
    pub(crate) async fn send(&self, pane: IdOf<Pane>, input: Input) -> Result<()> {
        self.get::<Pane>(pane).await?;
        self.write(input.frames(pane)).await?;
        Ok(())
    }

    /// The response revision separates before and after the queued input.
    async fn write(&self, frames: Vec<InputFrame>) -> Result<u64> {
        self.input(None, stream::iter(frames)).await?;
        Ok(self.written())
    }
}
