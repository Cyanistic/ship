//! The pane area: frames, labels and screens, from the published geometry.

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Position, Rect},
    style::{self as tui, Modifier, Style, Stylize},
    symbols::merge::MergeStrategy,
    text::Line,
    widgets::{Block, BorderType},
};
use ship_core::{
    layout::Axis,
    screen::{Attr, Color, Cursor, Run, Screen},
};

use super::{Scene, draw::pane_label, lane::Edge};

/// Marks pane-area cells no pane covers: space beyond the shared geometry on
/// a larger client, or content a lagging screen doesn't reach yet.
const FILLER: &str = "·";

/// Each visible pane's frame, offset to `area`'s top-left corner. Two or more
/// panes get merged shared borders with the pane's label on top, the selected
/// one thick and colored; a lone pane fills its frame unbordered. Each screen
/// paints into its content rectangle, clipped to it and to `area`, and the
/// filler marks the rest of `area`. Returns the selected pane's cursor and
/// where it falls on the frame.
pub(super) fn draw(
    frame: &mut Frame,
    scene: &Scene,
    held: impl Iterator<Item = Edge>,
) -> Option<(Position, Cursor)> {
    let area = scene.panes;
    let tab = scene.tab?;
    let geometry = scene.geometry?;
    let selected = scene.selected;
    let screens = scene.screens;
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
        // Color only published border cells, never a predicted position.
        for edge in held {
            let Some(pane) = geometry.panes.get(&edge.pane) else {
                continue;
            };
            let border = match edge.axis {
                Axis::Horizontal => Rect::new(
                    pane.frame.right().saturating_sub(1),
                    pane.frame.y,
                    1,
                    pane.frame.height,
                ),
                Axis::Vertical => Rect::new(
                    pane.frame.x,
                    pane.frame.bottom().saturating_sub(1),
                    pane.frame.width,
                    1,
                ),
            };
            for at in offset(border).positions() {
                frame.buffer_mut()[at].set_style(accent.remove_modifier(Modifier::DIM));
            }
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
        if let Some(scrolled) = screen.scrolled {
            let top = if geometry.panes.len() > 1 {
                let frame = offset(pane.frame);
                Rect::new(
                    frame.x.saturating_add(1),
                    frame.y,
                    frame.width.saturating_sub(2),
                    frame.height.min(1),
                )
            } else {
                Rect::new(content.x, content.y, content.width, content.height.min(1))
            };
            frame.render_widget(
                Line::from(format!("[{}/{}]", scrolled.offset, scrolled.history))
                    .right_aligned()
                    .style(Style::new().bg(tui::Color::Reset).fg(tui::Color::Cyan)),
                top,
            );
        }
        if Some(*id) == selected
            && let Some(at) = screen.cursor
            && at.x < shown.width
            && at.y < shown.height
        {
            cursor = Some((Position::new(shown.x + at.x, shown.y + at.y), at));
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

/// The screen's rows into `area`, which is no larger than the screen.
fn paint(screen: &Screen, area: Rect, buf: &mut Buffer) {
    let mut utf8 = [0; 4];
    for (y, row) in (area.top()..area.bottom()).zip(&screen.rows) {
        let cols = area.left()..area.right();
        for x in cols.clone() {
            buf[(x, y)].reset();
        }
        for (x, symbol) in cols.clone().zip(row.text.chars()) {
            buf[(x, y)].set_symbol(symbol.encode_utf8(&mut utf8));
        }
        let mut unstyled = cols;
        for run in &row.runs {
            let style = style(run);
            for x in unstyled.by_ref().take(usize::from(run.cells)) {
                buf[(x, y)].set_style(style);
            }
        }
        for multi in &row.multi {
            if multi.col < area.width {
                buf[(area.x + multi.col, y)].set_symbol(&multi.symbol);
            }
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

fn style(run: &Run) -> Style {
    let modifier = MODIFIERS
        .iter()
        .filter(|(attr, _)| run.attrs.contains(attr))
        .fold(Modifier::empty(), |modifier, (_, add)| modifier | *add);
    Style::new()
        .fg(color(run.fg))
        .bg(color(run.bg))
        .underline_color(color(run.underline_color))
        .add_modifier(modifier)
}

fn color(color: Color) -> tui::Color {
    match color {
        Color::Default => tui::Color::Reset,
        Color::Indexed(index) => tui::Color::Indexed(index),
        Color::Rgb(r, g, b) => tui::Color::Rgb(r, g, b),
    }
}
