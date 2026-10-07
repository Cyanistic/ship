use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use libghostty_vt::terminal::{Mode, Terminal};
use libghostty_vt::{key, mouse};
use ratatui_ghostty::input::{self, IntoKeyInput, IntoMouseInput};

fn make_terminal() -> Terminal<'static, 'static> {
    let mut terminal = Terminal::new(80, 24).unwrap();
    terminal.set_scrollback_max_lines(Some(0)).unwrap();
    terminal
}

#[test]
fn encode_focus_gained() {
    assert_eq!(input::encode_focus(true), b"\x1b[I");
}

#[test]
fn encode_focus_lost() {
    assert_eq!(input::encode_focus(false), b"\x1b[O");
}

#[test]
fn encode_paste_without_bracketed_paste() {
    let terminal = make_terminal();
    let data = b"hello world";
    let result = input::encode_paste(data, &terminal);
    assert_eq!(result, b"hello world");
}

#[test]
fn encode_paste_with_bracketed_paste() {
    let mut terminal = make_terminal();
    terminal.set_mode(Mode::BRACKETED_PASTE, true).unwrap();
    let data = b"hello world";
    let result = input::encode_paste(data, &terminal);
    assert_eq!(result, b"\x1b[200~hello world\x1b[201~");
}

fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

#[test]
fn encode_key_printable_char() {
    let terminal = make_terminal();
    let mut encoder = key::Encoder::new().unwrap();
    let event = make_key_event(KeyCode::Char('a'), KeyModifiers::NONE);
    let result = input::encode_key(&mut encoder, &event.into_key_input(), &terminal).unwrap();
    assert_eq!(result, b"a");
}

#[test]
fn encode_key_enter() {
    let terminal = make_terminal();
    let mut encoder = key::Encoder::new().unwrap();
    let event = make_key_event(KeyCode::Enter, KeyModifiers::NONE);
    let result = input::encode_key(&mut encoder, &event.into_key_input(), &terminal).unwrap();
    assert_eq!(result, b"\r");
}

#[test]
fn encode_key_escape() {
    let terminal = make_terminal();
    let mut encoder = key::Encoder::new().unwrap();
    let event = make_key_event(KeyCode::Esc, KeyModifiers::NONE);
    let result = input::encode_key(&mut encoder, &event.into_key_input(), &terminal).unwrap();
    assert_eq!(result, b"\x1b");
}

#[test]
fn encode_key_tab() {
    let terminal = make_terminal();
    let mut encoder = key::Encoder::new().unwrap();
    let event = make_key_event(KeyCode::Tab, KeyModifiers::NONE);
    let result = input::encode_key(&mut encoder, &event.into_key_input(), &terminal).unwrap();
    assert_eq!(result, b"\t");
}

#[test]
fn encode_key_backspace() {
    let terminal = make_terminal();
    let mut encoder = key::Encoder::new().unwrap();
    let event = make_key_event(KeyCode::Backspace, KeyModifiers::NONE);
    let result = input::encode_key(&mut encoder, &event.into_key_input(), &terminal).unwrap();
    assert_eq!(result, b"\x7f");
}

#[test]
fn encode_key_arrow_up() {
    let terminal = make_terminal();
    let mut encoder = key::Encoder::new().unwrap();
    let event = make_key_event(KeyCode::Up, KeyModifiers::NONE);
    let result = input::encode_key(&mut encoder, &event.into_key_input(), &terminal).unwrap();
    assert_eq!(result, b"\x1b[A");
}

#[test]
fn encode_key_ctrl_c() {
    let terminal = make_terminal();
    let mut encoder = key::Encoder::new().unwrap();
    let event = make_key_event(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let result = input::encode_key(&mut encoder, &event.into_key_input(), &terminal).unwrap();
    assert_eq!(result, b"\x03");
}

#[test]
fn encode_key_returns_empty_for_unmapped() {
    let terminal = make_terminal();
    let mut encoder = key::Encoder::new().unwrap();
    let event = make_key_event(KeyCode::Null, KeyModifiers::NONE);
    let result = input::encode_key(&mut encoder, &event.into_key_input(), &terminal).unwrap();
    assert!(result.is_empty());
}

fn make_mouse_event(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn default_encoder_size() -> mouse::EncoderSize {
    mouse::EncoderSize {
        screen_width: 640,
        screen_height: 480,
        cell_width: 8,
        cell_height: 16,
        padding_top: 0,
        padding_bottom: 0,
        padding_right: 0,
        padding_left: 0,
    }
}

#[test]
fn encode_mouse_no_tracking_produces_empty() {
    let terminal = make_terminal();
    let mut encoder = mouse::Encoder::new().unwrap();
    let event = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 10);
    let result = input::encode_mouse(
        &mut encoder,
        &event.into_mouse_input().unwrap(),
        &terminal,
        default_encoder_size(),
    )
    .unwrap();
    assert!(result.is_empty());
}

#[test]
fn encode_mouse_with_normal_tracking() {
    let mut terminal = make_terminal();
    // Enable normal mouse tracking and SGR format via escape sequences
    terminal.vt_write(b"\x1b[?1000h"); // normal mouse
    terminal.vt_write(b"\x1b[?1006h"); // SGR mouse format
    let mut encoder = mouse::Encoder::new().unwrap();
    let event = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 10);
    let result = input::encode_mouse(
        &mut encoder,
        &event.into_mouse_input().unwrap(),
        &terminal,
        default_encoder_size(),
    )
    .unwrap();
    assert!(!result.is_empty(), "expected non-empty mouse encoding");
    let s = String::from_utf8_lossy(&result);
    assert!(s.starts_with("\x1b[<"), "expected SGR format, got: {s:?}");
}

#[test]
fn encode_mouse_scroll_up_no_tracking_produces_empty() {
    let terminal = make_terminal();
    let mut encoder = mouse::Encoder::new().unwrap();
    let event = make_mouse_event(MouseEventKind::ScrollUp, 5, 10);
    let result = input::encode_mouse(
        &mut encoder,
        &event.into_mouse_input().unwrap(),
        &terminal,
        default_encoder_size(),
    )
    .unwrap();
    assert!(
        result.is_empty(),
        "scroll up without tracking should produce empty (triggers viewport scroll)"
    );
}

#[test]
fn encode_mouse_scroll_down_no_tracking_produces_empty() {
    let terminal = make_terminal();
    let mut encoder = mouse::Encoder::new().unwrap();
    let event = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
    let result = input::encode_mouse(
        &mut encoder,
        &event.into_mouse_input().unwrap(),
        &terminal,
        default_encoder_size(),
    )
    .unwrap();
    assert!(
        result.is_empty(),
        "scroll down without tracking should produce empty (triggers viewport scroll)"
    );
}

#[test]
fn encode_mouse_scroll_up_with_tracking_produces_output() {
    let mut terminal = make_terminal();
    terminal.vt_write(b"\x1b[?1000h");
    terminal.vt_write(b"\x1b[?1006h");
    let mut encoder = mouse::Encoder::new().unwrap();
    let event = make_mouse_event(MouseEventKind::ScrollUp, 5, 10);
    let result = input::encode_mouse(
        &mut encoder,
        &event.into_mouse_input().unwrap(),
        &terminal,
        default_encoder_size(),
    )
    .unwrap();
    assert!(
        !result.is_empty(),
        "scroll up with tracking should produce output (forwarded to PTY)"
    );
}

#[test]
fn encode_mouse_scroll_down_with_tracking_produces_output() {
    let mut terminal = make_terminal();
    terminal.vt_write(b"\x1b[?1000h");
    terminal.vt_write(b"\x1b[?1006h");
    let mut encoder = mouse::Encoder::new().unwrap();
    let event = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
    let result = input::encode_mouse(
        &mut encoder,
        &event.into_mouse_input().unwrap(),
        &terminal,
        default_encoder_size(),
    )
    .unwrap();
    assert!(
        !result.is_empty(),
        "scroll down with tracking should produce output (forwarded to PTY)"
    );
}

#[test]
fn encode_mouse_scroll_left_dropped() {
    let event = make_mouse_event(MouseEventKind::ScrollLeft, 0, 0);
    assert!(event.into_mouse_input().is_none());
}

#[test]
fn encode_mouse_scroll_right_dropped() {
    let event = make_mouse_event(MouseEventKind::ScrollRight, 0, 0);
    assert!(event.into_mouse_input().is_none());
}
