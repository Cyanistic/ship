use crossterm::event::{
    KeyCode, KeyModifiers, MediaKeyCode, ModifierKeyCode, MouseButton, MouseEventKind,
};
use libghostty_vt::{key, mouse};
use ratatui_ghostty::convert::from_crossterm;

#[test]
fn key_code_basic_keys() {
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Backspace),
        Some(key::Key::Backspace)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Enter),
        Some(key::Key::Enter)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Left),
        Some(key::Key::ArrowLeft)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Right),
        Some(key::Key::ArrowRight)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Up),
        Some(key::Key::ArrowUp)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Down),
        Some(key::Key::ArrowDown)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Home),
        Some(key::Key::Home)
    );
    assert_eq!(from_crossterm::key_code(&KeyCode::End), Some(key::Key::End));
    assert_eq!(
        from_crossterm::key_code(&KeyCode::PageUp),
        Some(key::Key::PageUp)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::PageDown),
        Some(key::Key::PageDown)
    );
    assert_eq!(from_crossterm::key_code(&KeyCode::Tab), Some(key::Key::Tab));
    assert_eq!(
        from_crossterm::key_code(&KeyCode::BackTab),
        Some(key::Key::Tab)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Delete),
        Some(key::Key::Delete)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Insert),
        Some(key::Key::Insert)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Esc),
        Some(key::Key::Escape)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::CapsLock),
        Some(key::Key::CapsLock)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::ScrollLock),
        Some(key::Key::ScrollLock)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::NumLock),
        Some(key::Key::NumLock)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::PrintScreen),
        Some(key::Key::PrintScreen)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Pause),
        Some(key::Key::Pause)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Menu),
        Some(key::Key::ContextMenu)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::KeypadBegin),
        Some(key::Key::NumpadBegin)
    );
}

#[test]
fn key_code_chars() {
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('a')),
        Some(key::Key::A)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('A')),
        Some(key::Key::A)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('z')),
        Some(key::Key::Z)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('0')),
        Some(key::Key::Digit0)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('9')),
        Some(key::Key::Digit9)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char(' ')),
        Some(key::Key::Space)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('`')),
        Some(key::Key::Backquote)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('\\')),
        Some(key::Key::Backslash)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('[')),
        Some(key::Key::BracketLeft)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char(']')),
        Some(key::Key::BracketRight)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char(',')),
        Some(key::Key::Comma)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('=')),
        Some(key::Key::Equal)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('-')),
        Some(key::Key::Minus)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('.')),
        Some(key::Key::Period)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('\'')),
        Some(key::Key::Quote)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char(';')),
        Some(key::Key::Semicolon)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Char('/')),
        Some(key::Key::Slash)
    );
}

#[test]
fn key_code_unknown_char_returns_none() {
    assert_eq!(from_crossterm::key_code(&KeyCode::Char('€')), None);
    assert_eq!(from_crossterm::key_code(&KeyCode::Null), None);
}

#[test]
fn key_code_function_keys() {
    assert_eq!(from_crossterm::key_code(&KeyCode::F(1)), Some(key::Key::F1));
    assert_eq!(
        from_crossterm::key_code(&KeyCode::F(12)),
        Some(key::Key::F12)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::F(25)),
        Some(key::Key::F25)
    );
}

#[test]
fn key_code_f_key_boundary() {
    assert_eq!(from_crossterm::key_code(&KeyCode::F(0)), None);
    assert_eq!(from_crossterm::key_code(&KeyCode::F(26)), None);
}

#[test]
fn key_code_media_keys() {
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Media(MediaKeyCode::PlayPause)),
        Some(key::Key::MediaPlayPause)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Media(MediaKeyCode::Stop)),
        Some(key::Key::MediaStop)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Media(MediaKeyCode::TrackNext)),
        Some(key::Key::MediaTrackNext)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Media(MediaKeyCode::TrackPrevious)),
        Some(key::Key::MediaTrackPrevious)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Media(MediaKeyCode::LowerVolume)),
        Some(key::Key::AudioVolumeDown)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Media(MediaKeyCode::RaiseVolume)),
        Some(key::Key::AudioVolumeUp)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Media(MediaKeyCode::MuteVolume)),
        Some(key::Key::AudioVolumeMute)
    );
}

#[test]
fn key_code_unmapped_media_returns_none() {
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Media(MediaKeyCode::Play)),
        None
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Media(MediaKeyCode::Rewind)),
        None
    );
}

#[test]
fn key_code_modifier_keys() {
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Modifier(ModifierKeyCode::LeftShift)),
        Some(key::Key::ShiftLeft)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Modifier(ModifierKeyCode::RightShift)),
        Some(key::Key::ShiftLeft)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Modifier(ModifierKeyCode::LeftControl)),
        Some(key::Key::ControlLeft)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Modifier(ModifierKeyCode::LeftAlt)),
        Some(key::Key::AltLeft)
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Modifier(ModifierKeyCode::LeftSuper)),
        Some(key::Key::MetaLeft)
    );
}

#[test]
fn key_code_unmapped_modifier_returns_none() {
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Modifier(ModifierKeyCode::LeftHyper)),
        None
    );
    assert_eq!(
        from_crossterm::key_code(&KeyCode::Modifier(ModifierKeyCode::LeftMeta)),
        None
    );
}

#[test]
fn key_modifiers_single() {
    assert_eq!(
        from_crossterm::key_modifiers(KeyModifiers::SHIFT),
        key::Mods::SHIFT
    );
    assert_eq!(
        from_crossterm::key_modifiers(KeyModifiers::CONTROL),
        key::Mods::CTRL
    );
    assert_eq!(
        from_crossterm::key_modifiers(KeyModifiers::ALT),
        key::Mods::ALT
    );
    assert_eq!(
        from_crossterm::key_modifiers(KeyModifiers::SUPER),
        key::Mods::SUPER
    );
}

#[test]
fn key_modifiers_empty() {
    assert_eq!(
        from_crossterm::key_modifiers(KeyModifiers::NONE),
        key::Mods::empty()
    );
}

#[test]
fn key_modifiers_combinations() {
    let combo = KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT;
    let expected = key::Mods::SHIFT | key::Mods::CTRL | key::Mods::ALT;
    assert_eq!(from_crossterm::key_modifiers(combo), expected);
}

#[test]
fn mouse_button_all() {
    assert_eq!(
        from_crossterm::mouse_button(MouseButton::Left),
        mouse::Button::Left
    );
    assert_eq!(
        from_crossterm::mouse_button(MouseButton::Right),
        mouse::Button::Right
    );
    assert_eq!(
        from_crossterm::mouse_button(MouseButton::Middle),
        mouse::Button::Middle
    );
}

#[test]
fn mouse_action_press() {
    let result = from_crossterm::mouse_action(&MouseEventKind::Down(MouseButton::Left));
    assert_eq!(
        result,
        Some((mouse::Action::Press, Some(mouse::Button::Left)))
    );
}

#[test]
fn mouse_action_release() {
    let result = from_crossterm::mouse_action(&MouseEventKind::Up(MouseButton::Right));
    assert_eq!(
        result,
        Some((mouse::Action::Release, Some(mouse::Button::Right)))
    );
}

#[test]
fn mouse_action_drag() {
    let result = from_crossterm::mouse_action(&MouseEventKind::Drag(MouseButton::Middle));
    assert_eq!(
        result,
        Some((mouse::Action::Motion, Some(mouse::Button::Middle)))
    );
}

#[test]
fn mouse_action_motion() {
    let result = from_crossterm::mouse_action(&MouseEventKind::Moved);
    assert_eq!(result, Some((mouse::Action::Motion, None)));
}

#[test]
fn mouse_action_scroll_up() {
    let result = from_crossterm::mouse_action(&MouseEventKind::ScrollUp);
    assert_eq!(
        result,
        Some((mouse::Action::Press, Some(mouse::Button::Four)))
    );
}

#[test]
fn mouse_action_scroll_down() {
    let result = from_crossterm::mouse_action(&MouseEventKind::ScrollDown);
    assert_eq!(
        result,
        Some((mouse::Action::Press, Some(mouse::Button::Five)))
    );
}

#[test]
fn mouse_action_scroll_left_dropped() {
    assert_eq!(
        from_crossterm::mouse_action(&MouseEventKind::ScrollLeft),
        None
    );
}

#[test]
fn mouse_action_scroll_right_dropped() {
    assert_eq!(
        from_crossterm::mouse_action(&MouseEventKind::ScrollRight),
        None
    );
}
