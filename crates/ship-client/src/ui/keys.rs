//! The mode machine: terminal events to bindings in the active mode.

use crokey::KeyCombination;
use crossterm::event::{Event, KeyEventKind};
use ship_core::prelude::*;

use super::lane::Item;
use crate::keymap::{Action, Binding, ClientAction, Keymap, ModeKind, ModeName};

pub(super) enum Handled {
    /// A bound action, key or paste with its target resolved at its turn.
    Queue(Item),
    /// Mode, sidebar and reload actions apply at press, ahead of the lane.
    Now(ClientAction),
    /// An unbound key outside normal.
    None,
}

/// The same press-time/ordered-lane split for bindings and menu picks.
pub(super) fn handled(action: Action) -> Handled {
    match action {
        Action::Client(
            action @ (ClientAction::Mode(_) | ClientAction::Sidebar(_) | ClientAction::Config(_)),
        ) => Handled::Now(action),
        action => Handled::Queue(Item::Action(action)),
    }
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
    /// a queued key in normal, dropped in any other mode. A one-shot mode
    /// returns to normal after any key unless the binding enters a mode itself.
    pub fn handle(&mut self, event: Event) -> Handled {
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
                match binding.and_then(|binding| binding.action) {
                    Some(action) => handled(action),
                    None if normal => Handled::Queue(Item::Key(key)),
                    None => Handled::None,
                }
            }
            Event::Paste(text) => Handled::Queue(Item::Paste(text)),
            _ => Handled::None,
        }
    }

    pub fn enter(&mut self, mode: ModeName) -> Result<()> {
        if !self.keymap.modes.contains_key(&mode) {
            return Err(err!(Configuration, "no mode named \"{}\"", mode.0));
        }
        self.active = mode;
        Ok(())
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
