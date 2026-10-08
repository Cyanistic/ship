use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseEvent};
use libghostty_vt::terminal::{Mode, Terminal};
use libghostty_vt::{key, mouse};

use crate::convert::from_crossterm;

/// Decoded key input data that can be sent across threads.
#[derive(Debug, Clone)]
pub struct KeyInput {
    pub action: key::Action,
    pub key: key::Key,
    pub mods: key::Mods,
    /// UTF-8 text for character keys.
    pub utf8: Option<String>,
    /// The unshifted codepoint, if applicable.
    pub unshifted_codepoint: Option<char>,
}

/// Decoded mouse input data that can be sent across threads.
///
/// Coordinates are session-relative cell positions.
#[derive(Debug, Clone)]
pub struct MouseInput {
    pub action: mouse::Action,
    pub button: Option<mouse::Button>,
    pub mods: key::Mods,
    /// Column in session-relative cell coordinates.
    pub col: u16,
    /// Row in session-relative cell coordinates.
    pub row: u16,
}

/// Trait for types that can be converted into a [`KeyInput`].
pub trait IntoKeyInput {
    fn into_key_input(self) -> KeyInput;
}

/// Trait for types that can be converted into a [`MouseInput`].
///
/// Implementations must provide session-relative coordinates.
/// Returns `None` when the event has no meaningful mouse encoding (e.g. scroll left/right).
pub trait IntoMouseInput {
    fn into_mouse_input(self) -> Option<MouseInput>;
}

impl IntoKeyInput for KeyInput {
    fn into_key_input(self) -> KeyInput {
        self
    }
}

impl IntoMouseInput for MouseInput {
    fn into_mouse_input(self) -> Option<MouseInput> {
        Some(self)
    }
}

impl IntoMouseInput for MouseEvent {
    fn into_mouse_input(self) -> Option<MouseInput> {
        let (action, button) = from_crossterm::mouse_action(&self.kind)?;
        let mods = from_crossterm::key_modifiers(self.modifiers);

        Some(MouseInput {
            action,
            button,
            mods,
            col: self.column,
            row: self.row,
        })
    }
}

impl IntoKeyInput for KeyEvent {
    fn into_key_input(self) -> KeyInput {
        let action = match self.kind {
            KeyEventKind::Press => key::Action::Press,
            KeyEventKind::Repeat => key::Action::Repeat,
            KeyEventKind::Release => key::Action::Release,
        };
        let ghostty_key = from_crossterm::key_code(&self.code).unwrap_or(key::Key::Unidentified);
        let mods = from_crossterm::key_modifiers(self.modifiers);

        let (utf8, unshifted_codepoint) = if let KeyCode::Char(c) = self.code {
            let unshifted = if self
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT)
            {
                c.to_ascii_lowercase()
            } else {
                c
            };
            (Some(c.to_string()), Some(unshifted))
        } else {
            (None, None)
        };

        KeyInput {
            action,
            key: ghostty_key,
            mods,
            utf8,
            unshifted_codepoint,
        }
    }
}

/// Builds a [`KeyInput`] into a ghostty-vt [`key::Event`] for encoding.
pub(crate) fn build_key_event(
    input: &KeyInput,
) -> Result<key::Event<'static>, libghostty_vt::Error> {
    let mut event = key::Event::new()?;
    event
        .set_action(input.action)
        .set_key(input.key)
        .set_mods(input.mods);
    if let Some(ref text) = input.utf8 {
        event.set_utf8(Some(text.clone()));
    }
    if let Some(cp) = input.unshifted_codepoint {
        event.set_unshifted_codepoint(cp);
    }
    Ok(event)
}

/// Builds a [`MouseInput`] into a ghostty-vt [`mouse::Event`] for encoding.
pub(crate) fn build_mouse_event(
    input: &MouseInput,
    size: mouse::EncoderSize,
) -> Result<mouse::Event<'static>, libghostty_vt::Error> {
    let mut event = mouse::Event::new()?;
    event
        .set_action(input.action)
        .set_button(input.button)
        .set_mods(input.mods)
        .set_position(mouse::Position {
            x: input.col as f32 * size.cell_width as f32 + size.padding_left as f32,
            y: input.row as f32 * size.cell_height as f32 + size.padding_top as f32,
        });
    Ok(event)
}

pub fn encode_focus(gained: bool) -> &'static [u8] {
    if gained { b"\x1b[I" } else { b"\x1b[O" }
}

pub fn encode_key(
    encoder: &mut key::Encoder,
    input: &KeyInput,
    terminal: &Terminal,
) -> Result<Vec<u8>, libghostty_vt::Error> {
    encoder.set_options_from_terminal(terminal);
    let key_event = build_key_event(input)?;
    let mut buf = Vec::new();
    encoder.encode_to_vec(&key_event, &mut buf)?;
    Ok(buf)
}

pub fn encode_mouse(
    encoder: &mut mouse::Encoder,
    input: &MouseInput,
    terminal: &Terminal,
    size: mouse::EncoderSize,
) -> Result<Vec<u8>, libghostty_vt::Error> {
    encoder.set_options_from_terminal(terminal);
    encoder.set_size(size);
    let mouse_event = build_mouse_event(input, size)?;
    let mut buf = Vec::new();
    encoder.encode_to_vec(&mouse_event, &mut buf)?;
    Ok(buf)
}

pub fn encode_paste(data: &[u8], terminal: &Terminal) -> Vec<u8> {
    let bracketed = terminal.mode(Mode::BRACKETED_PASTE).unwrap_or(false);
    if bracketed {
        let mut buf = Vec::with_capacity(data.len() + 12);
        buf.extend_from_slice(b"\x1b[200~");
        buf.extend_from_slice(data);
        buf.extend_from_slice(b"\x1b[201~");
        buf
    } else {
        data.to_vec()
    }
}
