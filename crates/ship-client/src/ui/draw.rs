use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{self as tui, Modifier, Style, Stylize},
    text::Line,
    widgets::{Paragraph, Wrap},
};
use std::{path::Path, sync::Arc};

use ship_core::{
    id::IdOf,
    model::{NodeId, Pane, PaneStatus, Tab},
    screen::{Attr, Cell, Color, Screen},
    tree::{self, Sessions},
};

use super::observer::{Observer, Selected};

/// Marks the client's area outside the pane, when the tab is sized to a
/// smaller client.
const FILLER: &str = "·";

/// The selected pane's screen at its own size in the top-left corner, the
/// filler pattern over the rest, and the status line on the last row. With
/// no pane selected, a hint for creating one instead of the screen.
pub(super) fn draw(frame: &mut Frame, observer: &Observer) {
    let [main, status] =
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
            Paragraph::new(hint(selected.as_ref()))
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
        Line::from(status_line(observer, selected.as_ref())).reversed(),
        status,
    );
}

/// `no pane: ship pane create <tab-id>`, or the tab to create first.
fn hint(selected: Option<&Selected>) -> String {
    let Some(selected) = selected else {
        return "connecting".into();
    };
    match selected
        .tab
        .or_else(|| selected.session.tabs.values().next().map(Arc::as_ref))
    {
        Some(tab) => format!("no pane: ship pane create {}", tab.id),
        None => format!("no tab: ship tab create {}", selected.session.name),
    }
}

/// `session › tab › pane` labels as far as the selection goes, then the
/// pane's exit status and the connection.
fn status_line(observer: &Observer, selected: Option<&Selected>) -> String {
    let mut line = String::from(" ");
    if let (Some(selected), Some(replica)) = (selected, &observer.replica) {
        line.push_str(&selected.session.name.to_string());
        if let Some(tab) = selected.tab {
            line.push_str(" › ");
            line.push_str(&tab_label(tab, position(&replica.sessions, tab.id)));
        }
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
fn position(sessions: &Sessions, tab: IdOf<Tab>) -> usize {
    let siblings = match tree::path(sessions, NodeId::Tab(tab)).as_deref() {
        Some([.., NodeId::Session(session), _]) => {
            sessions.get(session).map(|session| &session.tabs)
        }
        Some([.., NodeId::Tab(parent), _]) => {
            tree::tab(sessions, *parent).ok().map(|parent| &parent.tabs)
        }
        _ => None,
    };
    siblings
        .and_then(|tabs| tabs.get_index_of(&tab))
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
