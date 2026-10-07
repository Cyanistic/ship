use libghostty_vt::render::RenderState;
use libghostty_vt::terminal::{ScrollViewport, Terminal};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::widgets::Widget;
use ratatui_ghostty::widget::TerminalWidget;

fn make_terminal(cols: u16, rows: u16) -> Terminal<'static, 'static> {
    make_terminal_scrollback(cols, rows, 0)
}

fn make_terminal_scrollback(
    cols: u16,
    rows: u16,
    max_scrollback: usize,
) -> Terminal<'static, 'static> {
    let mut terminal = Terminal::new(cols, rows).unwrap();
    terminal
        .set_scrollback_max_lines(Some(max_scrollback))
        .unwrap();
    terminal
}

fn render_widget<'a>(
    terminal: &mut Terminal<'a, 'a>,
    render_state: &mut RenderState<'a>,
    area: Rect,
    focused: bool,
) -> Buffer {
    let mut buf = Buffer::empty(area);
    let mut widget = TerminalWidget::new(terminal, render_state).focused(focused);
    (&mut widget).render(area, &mut buf);
    buf
}

#[test]
fn plain_text_at_home() {
    let mut terminal = make_terminal(10, 3);
    let mut rs = RenderState::new().unwrap();
    terminal.vt_write(b"Hello");
    let area = Rect::new(0, 0, 10, 3);
    let buf = render_widget(&mut terminal, &mut rs, area, false);
    let cell = &buf[(0, 0)];
    assert_eq!(cell.symbol(), "H");
    let cell = &buf[(4, 0)];
    assert_eq!(cell.symbol(), "o");
}

#[test]
fn sgr_bold_italic() {
    let mut terminal = make_terminal(10, 3);
    let mut rs = RenderState::new().unwrap();
    // ESC[1m = bold, ESC[3m = italic
    terminal.vt_write(b"\x1b[1;3mAB");
    let area = Rect::new(0, 0, 10, 3);
    let buf = render_widget(&mut terminal, &mut rs, area, false);
    let cell = &buf[(0, 0)];
    assert_eq!(cell.symbol(), "A");
    assert!(cell.modifier.contains(Modifier::BOLD));
    assert!(cell.modifier.contains(Modifier::ITALIC));
}

#[test]
fn sgr_rgb_fg_color() {
    let mut terminal = make_terminal(10, 3);
    let mut rs = RenderState::new().unwrap();
    // ESC[38;2;255;0;128m = RGB foreground
    terminal.vt_write(b"\x1b[38;2;255;0;128mX");
    let area = Rect::new(0, 0, 10, 3);
    let buf = render_widget(&mut terminal, &mut rs, area, false);
    let cell = &buf[(0, 0)];
    assert_eq!(cell.symbol(), "X");
    assert_eq!(cell.fg, Color::Rgb(255, 0, 128));
}

#[test]
fn sgr_inverse() {
    let mut terminal = make_terminal(10, 3);
    let mut rs = RenderState::new().unwrap();
    // ESC[7m = inverse
    terminal.vt_write(b"\x1b[7mR");
    let area = Rect::new(0, 0, 10, 3);
    let buf = render_widget(&mut terminal, &mut rs, area, false);
    let cell = &buf[(0, 0)];
    assert_eq!(cell.symbol(), "R");
    assert!(cell.modifier.contains(Modifier::REVERSED));
}

#[test]
fn cursor_visible_focused() {
    use ratatui::layout::Position;
    use ratatui_ghostty::widget::CursorStyle;

    let mut terminal = make_terminal(10, 3);
    let mut rs = RenderState::new().unwrap();
    terminal.vt_write(b"AB");
    let area = Rect::new(0, 0, 10, 3);
    let mut buf = Buffer::empty(area);
    let mut widget = TerminalWidget::new(&mut terminal, &mut rs).focused(true);
    (&mut widget).render(area, &mut buf);
    let cursor = widget.cursor();
    assert_eq!(
        cursor.position,
        Some(Position::new(2, 0)),
        "cursor should be at (2, 0) after writing 'AB'"
    );
    assert_eq!(cursor.style, CursorStyle::Block);
}

#[test]
fn cursor_hidden_no_position() {
    let mut terminal = make_terminal(10, 3);
    let mut rs = RenderState::new().unwrap();
    terminal.vt_write(b"AB");
    // ESC[?25l = hide cursor
    terminal.vt_write(b"\x1b[?25l");
    let area = Rect::new(0, 0, 10, 3);
    let mut buf = Buffer::empty(area);
    let mut widget = TerminalWidget::new(&mut terminal, &mut rs).focused(true);
    (&mut widget).render(area, &mut buf);
    assert!(
        widget.cursor().position.is_none(),
        "hidden cursor should not have a position"
    );
}

#[test]
fn wide_character() {
    let mut terminal = make_terminal(10, 3);
    let mut rs = RenderState::new().unwrap();
    // Write a wide character (Chinese character)
    terminal.vt_write("漢".as_bytes());
    let area = Rect::new(0, 0, 10, 3);
    let buf = render_widget(&mut terminal, &mut rs, area, false);
    let cell0 = &buf[(0, 0)];
    assert_eq!(cell0.symbol(), "漢");
    // Wide tail cell should be a space
    let cell1 = &buf[(1, 0)];
    assert_eq!(cell1.symbol(), " ");
}

#[test]
fn render_smaller_than_terminal() {
    let mut terminal = make_terminal(20, 10);
    let mut rs = RenderState::new().unwrap();
    terminal.vt_write(b"ABCDEFGHIJKLMNOPQRST");
    // Render into a smaller area — should not panic
    let area = Rect::new(0, 0, 5, 3);
    let buf = render_widget(&mut terminal, &mut rs, area, false);
    let cell = &buf[(0, 0)];
    assert_eq!(cell.symbol(), "A");
    let cell = &buf[(4, 0)];
    assert_eq!(cell.symbol(), "E");
}

#[test]
fn viewport_follows_output_after_scroll_to_bottom() {
    let mut terminal = make_terminal_scrollback(10, 3, 100);
    let mut rs = RenderState::new().unwrap();

    // Write 10 lines into a 3-row terminal — lines 1..7 go to scrollback.
    for i in 1..=10 {
        let line = format!("L{i:02}\r\n");
        terminal.vt_write(line.as_bytes());
    }
    // Mimic what pump() does: snap viewport to the active area.
    terminal.scroll_viewport(ScrollViewport::Bottom);

    let area = Rect::new(0, 0, 10, 3);
    let buf = render_widget(&mut terminal, &mut rs, area, false);

    // The last 3 visible rows should be L09, L10, and an empty prompt line.
    assert_eq!(buf[(0, 0)].symbol(), "L");
    assert_eq!(buf[(1, 0)].symbol(), "0");
    assert_eq!(buf[(2, 0)].symbol(), "9");

    assert_eq!(buf[(0, 1)].symbol(), "L");
    assert_eq!(buf[(1, 1)].symbol(), "1");
    assert_eq!(buf[(2, 1)].symbol(), "0");

    // Row 2 should be empty (cursor line after last \r\n).
    assert_eq!(buf[(0, 2)].symbol(), " ");
}

#[test]
fn viewport_shows_top_without_scroll_to_bottom() {
    let mut terminal = make_terminal_scrollback(10, 3, 100);
    let mut rs = RenderState::new().unwrap();

    for i in 1..=10 {
        let line = format!("L{i:02}\r\n");
        terminal.vt_write(line.as_bytes());
    }
    // Explicitly scroll viewport to the top of scrollback.
    terminal.scroll_viewport(ScrollViewport::Top);

    let area = Rect::new(0, 0, 10, 3);
    let buf = render_widget(&mut terminal, &mut rs, area, false);

    // Should see the oldest lines (L01, L02, L03).
    assert_eq!(buf[(0, 0)].symbol(), "L");
    assert_eq!(buf[(1, 0)].symbol(), "0");
    assert_eq!(buf[(2, 0)].symbol(), "1");

    assert_eq!(buf[(0, 1)].symbol(), "L");
    assert_eq!(buf[(1, 1)].symbol(), "0");
    assert_eq!(buf[(2, 1)].symbol(), "2");
}
