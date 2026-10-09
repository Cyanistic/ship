//! The mode machine: terminal events to bindings in the active mode.

use crokey::KeyCombination;
use crossterm::event::{Event, KeyEventKind};
use ship_core::{
    id::IdOf,
    model::Pane,
    protocol::{InputFrame, KeyInput, PasteInput},
};

use crate::keymap::{Binding, Keymap, ModeKind, ModeName};

pub(super) enum Handled {
    /// A key or paste for the selected pane; dropped while disconnected.
    Frame(InputFrame),
    /// A bound key's actions, which the loop runs in order.
    Run(Binding),
    /// An unbound key outside normal, or input with no pane selected.
    None,
}

pub(super) struct Keys {
    pub keymap: Keymap,
    pub active: ModeName,
}

impl Keys {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            active: ModeName::normal(),
        }
    }

    /// `KeyCombination::from(key)`, looked up in the active mode. Unbound:
    /// a frame in normal, dropped in any other mode. A one-shot mode returns
    /// to normal after any key unless the binding enters a mode itself.
    pub fn handle(&mut self, event: Event, selected: Option<IdOf<Pane>>) -> Handled {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                let Some(mode) = self.keymap.modes.get(&self.active) else {
                    self.active = ModeName::normal();
                    return Handled::None;
                };
                let binding = mode.keys.get(&KeyCombination::from(key)).cloned();
                let normal = self.active.0 == ModeName::NORMAL;
                if mode.kind == ModeKind::Oneshot
                    && !binding.as_ref().is_some_and(Binding::enters_mode)
                {
                    self.active = ModeName::normal();
                }
                match (binding, selected) {
                    (Some(binding), _) => Handled::Run(binding),
                    (None, Some(pane)) if normal => {
                        Handled::Frame(InputFrame::Key(KeyInput { pane, key }))
                    }
                    (None, _) => Handled::None,
                }
            }
            Event::Paste(text) => selected.map_or(Handled::None, |pane| {
                Handled::Frame(InputFrame::Paste(PasteInput { pane, text }))
            }),
            _ => Handled::None,
        }
    }

    /// Enter `mode`; the keymap checked at load that it exists.
    pub fn enter(&mut self, mode: ModeName) {
        self.active = mode;
    }

    /// Swap in a reloaded keymap; an active mode it no longer defines falls
    /// back to normal.
    pub fn replace(&mut self, keymap: Keymap) {
        if !keymap.modes.contains_key(&self.active) {
            self.active = ModeName::normal();
        }
        self.keymap = keymap;
    }

    /// The active mode's name, outside normal.
    pub fn shown_mode(&self) -> Option<&ModeName> {
        Some(&self.active).filter(|mode| mode.0 != ModeName::NORMAL)
    }
}
