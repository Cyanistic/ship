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
    /// `C-b n`, `C-b p`
    NextPane,
    PrevPane,
    /// `C-b )`, `C-b (`
    NextSession,
    PrevSession,
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
                let prefixed = std::mem::take(&mut self.prefixed);
                match (prefixed, is_prefix(&key)) {
                    (false, true) => {
                        self.prefixed = true;
                        return Action::None;
                    }
                    (true, false) => return bound(&key),
                    _ => {}
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

/// The action for a key after the prefix.
fn bound(key: &KeyEvent) -> Action {
    // Terminals differ on whether `(` and `)` carry Shift.
    if !(key.modifiers - KeyModifiers::SHIFT).is_empty() {
        return Action::None;
    }
    match key.code {
        KeyCode::Char('n') => Action::NextPane,
        KeyCode::Char('p') => Action::PrevPane,
        KeyCode::Char(')') => Action::NextSession,
        KeyCode::Char('(') => Action::PrevSession,
        KeyCode::Char('d') => Action::Detach,
        _ => Action::None,
    }
}

fn is_prefix(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL
}
