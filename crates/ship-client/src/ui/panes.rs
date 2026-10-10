//! The pane area: frames, labels and screens, from the published geometry.

use std::{collections::HashMap, sync::Arc};

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{self as tui, Modifier, Style, Stylize},
    symbols::merge::MergeStrategy,
    text::Line,
    widgets::{Block, BorderType},
};
use ship_core::{
    geometry::TabGeometry,
    id::IdOf,
    model::{Pane, Tab},
    screen::{Attr, Cell, Color, Cursor, Screen},
};

use super::draw::pane_label;

/// Marks pane-area cells no pane covers: space beyond the shared geometry on
/// a larger client, or content a lagging screen doesn't reach yet.
const FILLER: &str = "·";

/// Each visible pane's frame, offset to `area`'s top-left corner. Two or more
/// panes get merged shared borders with the pane's label on top, the selected
/// one thick and colored; a lone pane fills its frame unbordered. Each screen
/// paints into its content rectangle, clipped to it and to `area`, and the
/// filler marks the rest of `area`. Returns the selected pane's cursor once
/// placed on the frame.
pub(super) fn draw(
    frame: &mut Frame,
    area: Rect,
    tab: &Tab,
    geometry: &TabGeometry,
    screens: &HashMap<IdOf<Pane>, Arc<Screen>>,
    selected: Option<IdOf<Pane>>,
) -> Option<Cursor> {
    let offset = |rect: Rect| {
        area.intersection(Rect {
            x: area.x.saturating_add(rect.x),
            y: area.y.saturating_add(rect.y),
            ..rect
        })
    };
    let size = geometry.size;
    fill(
        frame.buffer_mut(),
        area,
        offset(Rect::new(0, 0, size.cols, size.rows)),
    );
    let accent = Style::new().fg(tui::Color::Cyan).bold();
    if geometry.panes.len() > 1 {
        // The selected pane's border last, so it wins on shared edges.
        let order = geometry
            .panes
            .iter()
            .filter(|(id, _)| Some(**id) != selected)
            .chain(
                geometry
                    .panes
                    .iter()
                    .filter(|(id, _)| Some(**id) == selected),
            );
        for (id, pane) in order {
            let block = Block::bordered().merge_borders(MergeStrategy::Exact);
            let block = match Some(*id) == selected {
                true => block.border_type(BorderType::Thick).border_style(accent),
                false => block.border_style(Style::new().dim()),
            };
            frame.render_widget(block, offset(pane.frame));
        }
        // Labels after every border, since a border merges over text.
        for (id, pane) in &geometry.panes {
            let label = tab.pane(*id).map(pane_label).unwrap_or_default();
            let top = offset(pane.frame);
            let line = Line::from(format!(" {label} "));
            let line = match Some(*id) == selected {
                true => line.style(accent),
                false => line.dim(),
            };
            frame.render_widget(
                line,
                Rect::new(
                    top.x + 1,
                    top.y,
                    top.width.saturating_sub(2),
                    top.height.min(1),
                ),
            );
        }
    }
    let mut cursor = None;
    for (id, pane) in &geometry.panes {
        let content = offset(pane.content);
        let screen = screens.get(id);
        let shown = screen.map_or(Rect::default(), |screen| {
            content.intersection(Rect {
                width: screen.size.cols,
                height: screen.size.rows,
                ..content
            })
        });
        fill(frame.buffer_mut(), content, shown);
        let Some(screen) = screen else {
            continue;
        };
        paint(screen, shown, frame.buffer_mut());
        if Some(*id) == selected
            && let Some(at) = screen.cursor
            && at.x < shown.width
            && at.y < shown.height
        {
            frame.set_cursor_position((shown.x + at.x, shown.y + at.y));
            cursor = Some(at);
        }
    }
    cursor
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
    if area.is_empty() || screen.size.cols == 0 {
        return;
    }
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
