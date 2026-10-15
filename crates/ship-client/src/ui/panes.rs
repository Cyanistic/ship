//! The pane area: frames, labels and screens, from the published geometry.

use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{self as tui, Modifier, Style, Stylize},
    symbols::merge::MergeStrategy,
    text::Line,
    widgets::{Block, BorderType, Widget},
};
use ship_core::{
    id::IdOf,
    layout::Axis,
    model::Pane,
    screen::{Attr, Color, Cursor, Row, Run, Screen},
};

use super::{Scene, draw::pane_label, lane::Edge};

/// Marks pane-area cells no pane covers: space beyond the shared geometry on
/// a larger client, or content a lagging screen doesn't reach yet.
const FILLER: &str = "·";

/// A visible pane offset to the pane area's top-left corner and clipped to
/// it, with the part of its content its screen covers.
pub(super) struct Placed<'a> {
    pub id: IdOf<Pane>,
    pub frame: Rect,
    pub content: Rect,
    pub screen: Option<&'a Arc<Screen>>,
    /// Empty without a screen.
    pub shown: Rect,
}

/// Each pane of the scene's geometry, placed.
pub(super) fn placed<'a>(scene: &Scene<'a>) -> impl Iterator<Item = Placed<'a>> {
    let area = scene.panes;
    let screens = scene.screens;
    scene
        .geometry
        .into_iter()
        .flat_map(|geometry| &geometry.panes)
        .map(move |(id, pane)| {
            let content = offset(area, pane.content);
            let screen = screens.get(id);
            let shown = screen.map_or(Rect::default(), |screen| {
                content.intersection(Rect {
                    width: screen.size.cols,
                    height: screen.size.rows,
                    ..content
                })
            });
            Placed {
                id: *id,
                frame: offset(area, pane.frame),
                content,
                screen,
                shown,
            }
        })
}

/// `rect` from the shared geometry moved to `area`'s top-left corner and
/// clipped to it.
fn offset(area: Rect, rect: Rect) -> Rect {
    area.intersection(Rect {
        x: area.x.saturating_add(rect.x),
        y: area.y.saturating_add(rect.y),
        ..rect
    })
}

/// Everything in the pane area but the screens. Two or more panes get merged
/// shared borders with the pane's label on top, the selected one thick and
/// colored; a lone pane fills its frame unbordered. The filler marks the
/// cells no screen covers.
pub(super) fn chrome(buf: &mut Buffer, scene: &Scene, held: impl Iterator<Item = Edge>) {
    let (Some(tab), Some(geometry)) = (scene.tab, scene.geometry) else {
        return;
    };
    let area = scene.panes;
    let selected = scene.selected;
    let size = geometry.size;
    fill(
        buf,
        area,
        offset(area, Rect::new(0, 0, size.cols, size.rows)),
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
            block.render(offset(area, pane.frame), buf);
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
            for at in offset(area, border).positions() {
                buf[at].set_style(accent.remove_modifier(Modifier::DIM));
            }
        }
        // Labels after every border, since a border merges over text.
        for (id, pane) in &geometry.panes {
            let label = tab.pane(*id).map(pane_label).unwrap_or_default();
            let top = offset(area, pane.frame);
            let line = Line::from(format!(" {label} "));
            let line = match Some(*id) == selected {
                true => line.style(accent),
                false => line.dim(),
            };
            line.render(
                Rect::new(
                    top.x + 1,
                    top.y,
                    top.width.saturating_sub(2),
                    top.height.min(1),
                ),
                buf,
            );
        }
    }
    for pane in placed(scene) {
        fill(buf, pane.content, pane.shown);
    }
}

/// `[offset/history]` over each scrolled pane's top row, on its border when
/// it has one.
pub(super) fn scrolled(buf: &mut Buffer, scene: &Scene) {
    let bordered = scene
        .geometry
        .is_some_and(|geometry| geometry.panes.len() > 1);
    for pane in placed(scene) {
        let Some(scrolled) = pane.screen.and_then(|screen| screen.scrolled) else {
            continue;
        };
        let top = if bordered {
            Rect::new(
                pane.frame.x.saturating_add(1),
                pane.frame.y,
                pane.frame.width.saturating_sub(2),
                pane.frame.height.min(1),
            )
        } else {
            let content = pane.content;
            Rect::new(content.x, content.y, content.width, content.height.min(1))
        };
        Line::from(format!("[{}/{}]", scrolled.offset, scrolled.history))
            .right_aligned()
            .style(Style::new().bg(tui::Color::Reset).fg(tui::Color::Cyan))
            .render(top, buf);
    }
}

/// The selected pane's cursor and where it falls on the frame.
pub(super) fn cursor(scene: &Scene) -> Option<(Position, Cursor)> {
    let pane = placed(scene).find(|pane| Some(pane.id) == scene.selected)?;
    let at = pane.screen?.cursor?;
    (at.x < pane.shown.width && at.y < pane.shown.height)
        .then(|| (Position::new(pane.shown.x + at.x, pane.shown.y + at.y), at))
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

/// One screen row into `area`, a single row no wider than the screen.
pub(super) fn paint_row(row: &Row, area: Rect, buf: &mut Buffer) {
    let mut utf8 = [0; 4];
    let y = area.y;
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
