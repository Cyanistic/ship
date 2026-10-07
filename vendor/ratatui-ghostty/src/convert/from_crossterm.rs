use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use libghostty_vt::{key, mouse};

pub fn key_code(code: &KeyCode) -> Option<key::Key> {
    Some(match code {
        KeyCode::Backspace => key::Key::Backspace,
        KeyCode::Enter => key::Key::Enter,
        KeyCode::Left => key::Key::ArrowLeft,
        KeyCode::Right => key::Key::ArrowRight,
        KeyCode::Up => key::Key::ArrowUp,
        KeyCode::Down => key::Key::ArrowDown,
        KeyCode::Home => key::Key::Home,
        KeyCode::End => key::Key::End,
        KeyCode::PageUp => key::Key::PageUp,
        KeyCode::PageDown => key::Key::PageDown,
        KeyCode::Tab | KeyCode::BackTab => key::Key::Tab,
        KeyCode::Delete => key::Key::Delete,
        KeyCode::Insert => key::Key::Insert,
        KeyCode::Esc => key::Key::Escape,
        KeyCode::CapsLock => key::Key::CapsLock,
        KeyCode::ScrollLock => key::Key::ScrollLock,
        KeyCode::NumLock => key::Key::NumLock,
        KeyCode::PrintScreen => key::Key::PrintScreen,
        KeyCode::Pause => key::Key::Pause,
        KeyCode::Menu => key::Key::ContextMenu,
        KeyCode::KeypadBegin => key::Key::NumpadBegin,
        KeyCode::Null => return None,
        KeyCode::F(n) => match n {
            1 => key::Key::F1,
            2 => key::Key::F2,
            3 => key::Key::F3,
            4 => key::Key::F4,
            5 => key::Key::F5,
            6 => key::Key::F6,
            7 => key::Key::F7,
            8 => key::Key::F8,
            9 => key::Key::F9,
            10 => key::Key::F10,
            11 => key::Key::F11,
            12 => key::Key::F12,
            13 => key::Key::F13,
            14 => key::Key::F14,
            15 => key::Key::F15,
            16 => key::Key::F16,
            17 => key::Key::F17,
            18 => key::Key::F18,
            19 => key::Key::F19,
            20 => key::Key::F20,
            21 => key::Key::F21,
            22 => key::Key::F22,
            23 => key::Key::F23,
            24 => key::Key::F24,
            25 => key::Key::F25,
            _ => return None,
        },
        KeyCode::Char(c) => match c {
            'a'..='z' | 'A'..='Z' => {
                // Map to the uppercase letter key
                match c.to_ascii_uppercase() {
                    'A' => key::Key::A,
                    'B' => key::Key::B,
                    'C' => key::Key::C,
                    'D' => key::Key::D,
                    'E' => key::Key::E,
                    'F' => key::Key::F,
                    'G' => key::Key::G,
                    'H' => key::Key::H,
                    'I' => key::Key::I,
                    'J' => key::Key::J,
                    'K' => key::Key::K,
                    'L' => key::Key::L,
                    'M' => key::Key::M,
                    'N' => key::Key::N,
                    'O' => key::Key::O,
                    'P' => key::Key::P,
                    'Q' => key::Key::Q,
                    'R' => key::Key::R,
                    'S' => key::Key::S,
                    'T' => key::Key::T,
                    'U' => key::Key::U,
                    'V' => key::Key::V,
                    'W' => key::Key::W,
                    'X' => key::Key::X,
                    'Y' => key::Key::Y,
                    'Z' => key::Key::Z,
                    _ => unreachable!(),
                }
            }
            '0' => key::Key::Digit0,
            '1' => key::Key::Digit1,
            '2' => key::Key::Digit2,
            '3' => key::Key::Digit3,
            '4' => key::Key::Digit4,
            '5' => key::Key::Digit5,
            '6' => key::Key::Digit6,
            '7' => key::Key::Digit7,
            '8' => key::Key::Digit8,
            '9' => key::Key::Digit9,
            ' ' => key::Key::Space,
            '`' => key::Key::Backquote,
            '\\' => key::Key::Backslash,
            '[' => key::Key::BracketLeft,
            ']' => key::Key::BracketRight,
            ',' => key::Key::Comma,
            '=' => key::Key::Equal,
            '-' => key::Key::Minus,
            '.' => key::Key::Period,
            '\'' => key::Key::Quote,
            ';' => key::Key::Semicolon,
            '/' => key::Key::Slash,
            _ => return None,
        },
        KeyCode::Media(media) => {
            use crossterm::event::MediaKeyCode;
            match media {
                MediaKeyCode::PlayPause => key::Key::MediaPlayPause,
                MediaKeyCode::Stop => key::Key::MediaStop,
                MediaKeyCode::TrackNext => key::Key::MediaTrackNext,
                MediaKeyCode::TrackPrevious => key::Key::MediaTrackPrevious,
                MediaKeyCode::LowerVolume => key::Key::AudioVolumeDown,
                MediaKeyCode::RaiseVolume => key::Key::AudioVolumeUp,
                MediaKeyCode::MuteVolume => key::Key::AudioVolumeMute,
                _ => return None,
            }
        }
        KeyCode::Modifier(modifier) => {
            use crossterm::event::ModifierKeyCode;
            match modifier {
                ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift => key::Key::ShiftLeft,
                ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => {
                    key::Key::ControlLeft
                }
                ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => key::Key::AltLeft,
                ModifierKeyCode::LeftSuper | ModifierKeyCode::RightSuper => key::Key::MetaLeft,
                _ => return None,
            }
        }
    })
}

pub fn key_modifiers(mods: KeyModifiers) -> key::Mods {
    let mut result = key::Mods::empty();
    if mods.contains(KeyModifiers::SHIFT) {
        result |= key::Mods::SHIFT;
    }
    if mods.contains(KeyModifiers::CONTROL) {
        result |= key::Mods::CTRL;
    }
    if mods.contains(KeyModifiers::ALT) {
        result |= key::Mods::ALT;
    }
    if mods.contains(KeyModifiers::SUPER) {
        result |= key::Mods::SUPER;
    }
    result
}

pub fn mouse_button(btn: MouseButton) -> mouse::Button {
    match btn {
        MouseButton::Left => mouse::Button::Left,
        MouseButton::Right => mouse::Button::Right,
        MouseButton::Middle => mouse::Button::Middle,
    }
}

/// Returns (action, button) for a mouse event kind.
/// ScrollLeft/Right are silently dropped (returns None).
pub fn mouse_action(kind: &MouseEventKind) -> Option<(mouse::Action, Option<mouse::Button>)> {
    match kind {
        MouseEventKind::Down(btn) => Some((mouse::Action::Press, Some(mouse_button(*btn)))),
        MouseEventKind::Up(btn) => Some((mouse::Action::Release, Some(mouse_button(*btn)))),
        MouseEventKind::Drag(btn) => Some((mouse::Action::Motion, Some(mouse_button(*btn)))),
        MouseEventKind::Moved => Some((mouse::Action::Motion, None)),
        MouseEventKind::ScrollUp => Some((mouse::Action::Press, Some(mouse::Button::Four))),
        MouseEventKind::ScrollDown => Some((mouse::Action::Press, Some(mouse::Button::Five))),
        MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => None,
    }
}
