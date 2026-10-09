use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{self as tui, Modifier, Style, Stylize},
    text::Line,
    widgets::{Paragraph, Wrap},
};
use std::path::Path;

use ship_core::{
    id::IdOf,
    model::{Pane, PaneStatus, Tab},
    screen::{Attr, Cell, Color, Screen},
    tree::{self, Tabs},
};

use super::observer::{Observer, Selected};
use crate::keymap::ModeName;

/// What the status line shows besides the selection.
pub(super) struct Status<'a> {
    /// The active mode, outside normal.
    pub mode: Option<&'a ModeName>,
    /// The last error, or "not available yet".
    pub message: Option<&'a str>,
}

/// Marks the client's area outside the pane, when the tab is sized to a
/// smaller client.
const FILLER: &str = "·";

/// The selected pane's screen at its own size in the top-left corner, the
/// filler pattern over the rest, and the status line on the last row. With
/// no pane selected, a hint instead of the screen.
pub(super) fn draw(frame: &mut Frame, observer: &Observer, status: &Status) {
    let [main, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
    let selected = observer.selected();
    match selected.as_ref().and_then(|selected| selected.pane) {
        Some(pane) => {
            let screen = observer.screens.get(&pane.id);
            let shown = screen.map_or(Rect::default(), |screen| {
                main.intersection(Rect::new(
                    main.x,
                    main.y,
                    screen.size.cols,
                    screen.size.rows,
                ))
            });
            fill(frame.buffer_mut(), main, shown);
            if let Some(screen) = screen {
                paint(screen, shown, frame.buffer_mut());
                if let Some(cursor) = screen.cursor
                    && cursor.x < shown.width
                    && cursor.y < shown.height
                {
                    frame.set_cursor_position((shown.x + cursor.x, shown.y + cursor.y));
                }
            }
        }
        None => frame.render_widget(
            Paragraph::new(hint(observer, selected.as_ref()))
                .centered()
                .wrap(Wrap { trim: true }),
            Rect {
                y: main.y + main.height / 2,
                height: main.height - main.height / 2,
                ..main
            },
        ),
    }
    frame.render_widget(
        Line::from(status_line(observer, selected.as_ref(), status)).reversed(),
        status_area,
    );
}

/// "connecting" while the observer has no record; with nothing selected,
/// `3 tabs · alt-right to open one` or `no tabs · ship tab create`; with a tab
/// and no pane, `no pane: ship pane create <tab-id>`.
fn hint(observer: &Observer, selected: Option<&Selected>) -> String {
    let (Some(replica), Some(_)) = (&observer.replica, observer.record()) else {
        return "connecting".into();
    };
    match (selected, replica.tabs.len()) {
        (Some(selected), _) => format!("no pane: ship pane create {}", selected.tab.id),
        (None, 0) => "no tabs · ship tab create".into(),
        (None, 1) => "1 tab · alt-right to open one".into(),
        (None, count) => format!("{count} tabs · alt-right to open one"),
    }
}

/// The active mode outside normal, `tab › pane` labels as far as the
/// selection goes, then the pane's exit status, the connection and the last
/// message.
fn status_line(observer: &Observer, selected: Option<&Selected>, status: &Status) -> String {
    let mut line = String::from(" ");
    if let Some(mode) = status.mode {
        line.push_str(&format!("[{}] ", mode.0));
    }
    if let (Some(selected), Some(replica)) = (selected, &observer.replica) {
        let tab = selected.tab;
        line.push_str(&tab_label(tab, position(&replica.tabs, tab.id)));
        if let Some(pane) = selected.pane {
            line.push_str(" › ");
            line.push_str(pane_label(pane));
        }
        if let Some(PaneStatus::Exited(status)) = selected.pane.map(|pane| &pane.status) {
            match &status.signal {
                Some(signal) => line.push_str(&format!("  exited ({signal})")),
                None => line.push_str(&format!("  exited ({})", status.code)),
            }
        }
    }
    if !observer.connected {
        line.push_str("  disconnected, reconnecting");
    }
    if let Some(message) = status.message {
        line.push_str("  · ");
        line.push_str(message);
    }
    line
}

/// The pane's name, else its program's title, else its command's file name.
pub(super) fn pane_label(pane: &Pane) -> &str {
    pane.name
        .get()
        .or(pane.title.as_deref())
        .or_else(|| {
            let program = Path::new(pane.command.first()?).file_name()?;
            program.to_str()
        })
        .unwrap_or_default()
}

/// The tab's name, else its first pane's label, else its 1-based `position`
/// among its siblings.
pub(super) fn tab_label(tab: &Tab, position: usize) -> String {
    tab.name
        .get()
        .or_else(|| tab.panes.values().next().map(pane_label))
        .map_or_else(|| (position + 1).to_string(), str::to_owned)
}

/// The tab's 0-based index among its siblings.
fn position(tabs: &Tabs, tab: IdOf<Tab>) -> usize {
    tree::siblings(tabs, tab)
        .and_then(|siblings| siblings.get_index_of(&tab))
        .unwrap_or_default()
}

/// The dim filler over `area` outside `shown`.
fn fill(buf: &mut Buffer, area: Rect, shown: Rect) {
    for position in area.positions() {
        if !shown.contains(position) {
            buf[position]
                .set_symbol(FILLER)
                .set_style(Style::new().dim());
        }
    }
}

/// The screen's cells into `area`, which is no larger than the screen.
fn paint(screen: &Screen, area: Rect, buf: &mut Buffer) {
    let cols = usize::from(screen.size.cols);
    for (index, cell) in screen.cells.iter().enumerate() {
        let (x, y) = (index % cols, index / cols);
        let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
            break;
        };
        if x < area.width && y < area.height {
            buf[(area.x + x, area.y + y)]
                .set_symbol(&cell.symbol)
                .set_style(style(cell));
        }
    }
}

const MODIFIERS: [(Attr, Modifier); 9] = [
    (Attr::Bold, Modifier::BOLD),
    (Attr::Dim, Modifier::DIM),
    (Attr::Italic, Modifier::ITALIC),
    (Attr::Underlined, Modifier::UNDERLINED),
    (Attr::SlowBlink, Modifier::SLOW_BLINK),
    (Attr::RapidBlink, Modifier::RAPID_BLINK),
    (Attr::Reversed, Modifier::REVERSED),
    (Attr::Hidden, Modifier::HIDDEN),
    (Attr::CrossedOut, Modifier::CROSSED_OUT),
];

fn style(cell: &Cell) -> Style {
    let modifier = MODIFIERS
        .iter()
        .filter(|(attr, _)| cell.attrs.contains(attr))
        .fold(Modifier::empty(), |modifier, (_, add)| modifier | *add);
    Style::new()
        .fg(color(cell.fg))
        .bg(color(cell.bg))
        .add_modifier(modifier)
}

fn color(color: Color) -> tui::Color {
    match color {
        Color::Default => tui::Color::Reset,
        Color::Indexed(index) => tui::Color::Indexed(index),
        Color::Rgb(r, g, b) => tui::Color::Rgb(r, g, b),
    }
}
