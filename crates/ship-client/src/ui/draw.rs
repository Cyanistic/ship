use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Paragraph, Wrap},
};
use std::path::Path;

use ship_core::{
    id::IdOf,
    model::{Pane, PaneStatus, Tab},
    screen::{Cursor, Size},
    tree::{self, Tabs},
};

use super::{
    observer::{Observer, Selected},
    panes,
};
use crate::keymap::ModeName;

/// What the status line shows besides the selection.
pub(super) struct Status<'a> {
    /// The active mode, outside normal.
    pub mode: Option<&'a ModeName>,
    /// The last error, or "not available yet".
    pub message: Option<&'a str>,
}

/// Rows the status line takes at the bottom.
const STATUS_ROWS: u16 = 1;

/// The area a terminal of `cols` by `rows` leaves for the tab. The server
/// sizes panes to it.
pub(super) fn tab_area(cols: u16, rows: u16) -> Size {
    Size {
        cols,
        rows: rows.saturating_sub(STATUS_ROWS),
    }
}

/// The selected tab's panes over the area above the status line, or a hint
/// when it has none or nothing is selected. The status line on the last row.
pub(super) fn draw(frame: &mut Frame, observer: &Observer, status: &Status) -> Option<Cursor> {
    let [main, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(STATUS_ROWS)])
            .areas(frame.area());
    let selected = observer.selected();
    let mut cursor = None;
    match selected.as_ref().and_then(|selected| {
        let geometry = selected.tab.geometry.as_ref()?;
        (!geometry.panes.is_empty()).then_some((selected, geometry))
    }) {
        Some((selected, geometry)) => {
            cursor = panes::draw(
                frame,
                main,
                selected.tab,
                geometry,
                &observer.screens,
                selected.pane.map(|pane| pane.id),
            );
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
    cursor
}

/// "connecting" while the observer has no record; with nothing selected,
/// `3 tabs · alt-right to open one` or `no tabs · ship tab create`; with a tab
/// and no pane, `no pane: ship pane create --tab <tab-id>`.
fn hint(observer: &Observer, selected: Option<&Selected>) -> String {
    let (Some(replica), Some(_)) = (&observer.replica, observer.record()) else {
        return "connecting".into();
    };
    match (selected, replica.tabs.len()) {
        (Some(selected), _) => format!("no pane: ship pane create --tab {}", selected.tab.id),
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
    // A config error spans several lines; its first says where it is.
    if let Some(message) = status.message.and_then(|message| message.lines().next()) {
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
        .or_else(|| tab.panes().next().map(pane_label))
        .map_or_else(|| (position + 1).to_string(), str::to_owned)
}

/// The tab's 0-based index among its siblings.
fn position(tabs: &Tabs, tab: IdOf<Tab>) -> usize {
    tree::siblings(tabs, tab)
        .and_then(|siblings| siblings.get_index_of(&tab))
        .unwrap_or_default()
}
