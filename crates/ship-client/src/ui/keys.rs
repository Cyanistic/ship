//! The `C-b` prefix: terminal events to client actions.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ship_core::{
    id::IdOf,
    model::Pane,
    protocol::{InputFrame, KeyInput, PasteInput},
};

pub(super) enum Action {
    /// A key or paste for the selected pane; dropped while disconnected.
    Frame(InputFrame),
    /// `C-b d`
    Detach,
    /// The prefix, an unbound key after it, or input with no pane selected.
    None,
}

#[derive(Default)]
pub(super) struct Keys {
    prefixed: bool,
}

impl Keys {
    /// `selected` is the pane frames go to. With no pane selected, keys after
    /// the prefix still act and other input is dropped. `C-b C-b` sends one
    /// `C-b`.
    pub fn handle(&mut self, event: Event, selected: Option<IdOf<Pane>>) -> Action {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if std::mem::take(&mut self.prefixed) {
                    match key.code {
                        KeyCode::Char('d') if key.modifiers.is_empty() => return Action::Detach,
                        _ if is_prefix(&key) => {}
                        _ => return Action::None,
                    }
                } else if is_prefix(&key) {
                    self.prefixed = true;
                    return Action::None;
                }
                selected.map_or(Action::None, |pane| {
                    Action::Frame(InputFrame::Key(KeyInput { pane, key }))
                })
            }
            Event::Paste(text) => {
                self.prefixed = false;
                selected.map_or(Action::None, |pane| {
                    Action::Frame(InputFrame::Paste(PasteInput { pane, text }))
                })
            }
            _ => Action::None,
        }
    }
}

fn is_prefix(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL
}
